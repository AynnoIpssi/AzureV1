// De bout en bout : le vrai daemon (binaire), app B qui partage, app A qui
// ecoute, avec `Flux` comme dans une app.
use azure_service::flux::{Change, Flux, FluxEvent, Value};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const A: u32 = 1;
const B: u32 = 2;
const C: u32 = 3;

struct Daemon {
    child: Child,
    socket: String,
    data: PathBuf,
}

impl Daemon {
    fn start(test: &str) -> Daemon {
        let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("service-{test}"));
        let _ = std::fs::remove_dir_all(&data);
        std::fs::create_dir_all(&data).unwrap();
        let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-service-test-{test}-{}.sock"), std::process::id(), test = test);
        Daemon::start_at(socket, data)
    }

    fn start_at(socket: String, data: PathBuf) -> Daemon {
        let child = Command::new(env!("CARGO_BIN_EXE_service_daemon"))
            .args(["--socket", &socket, "--data", data.to_str().unwrap()])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while std::os::unix::net::UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < deadline, "le daemon ne demarre pas");
            std::thread::sleep(Duration::from_millis(10));
        }
        Daemon { child, socket, data }
    }

    fn kill(&mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket);
    }
}

/// Attend que `done` soit vrai en traitant les evenements de l'ecoute.
fn wait_until(listener: &mut azure_service::flux::Listener, mut done: impl FnMut(&azure_service::flux::Listener) -> bool) -> Vec<FluxEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    while !done(listener) {
        assert!(Instant::now() < deadline, "delai depasse, etat : {} ; evenements : {events:?}", listener.state());
        events.extend(listener.wait(Duration::from_millis(50)));
    }
    events
}

#[test]
fn b_shares_its_changes_and_a_follows_them_in_real_time() {
    let daemon = Daemon::start("live");
    let mut panier = Flux::connect_at(&daemon.socket, B).unwrap().share("panier").to(&[A]).open().unwrap();
    panier.set("client", "Ana").unwrap();
    panier.set("total", 0).unwrap();

    let totals = Arc::new(Mutex::new(Vec::new()));
    let payments = Arc::new(Mutex::new(Vec::new()));
    let (t, p) = (Arc::clone(&totals), Arc::clone(&payments));
    let mut ecoute = Flux::connect_at(&daemon.socket, A)
        .unwrap()
        .listen(B, "panier")
        .path("total")
        .path("items")
        .on("total", move |v| t.lock().unwrap().push(v.as_i64().unwrap_or(-1)))
        .on_event("paye", move |v| p.lock().unwrap().push(v.to_string()))
        .start()
        .unwrap();

    // D'abord l'etat actuel, filtre.
    let events = wait_until(&mut ecoute, |l| l.seq() == 2);
    assert_eq!(events, [FluxEvent::Snapshot { seq: 2 }]);
    assert_eq!(ecoute.state().to_string(), r#"{"total": 0}"#, "pas 'client' : A ne l'a pas demande");

    // Puis chaque modification.
    panier.push("items", Value::map([("nom", "pomme".into()), ("prix", 2.into())])).unwrap();
    panier.batch(|b| {
        b.set("total", 2).push("items", Value::map([("nom", "pain".into()), ("prix", 3.into())])).set("total", 5);
    })
    .unwrap();
    panier.set("client", "Bo").unwrap(); // pas ecoute par A
    panier.emit("paye", "carte").unwrap();
    let events = wait_until(&mut ecoute, |l| l.seq() == 6);
    assert_eq!(events.len(), 3, "le batch arrive en un seul envoi, 'client' pas du tout : {events:?}");
    assert!(events[1].touches("total") && events[1].touches("items.1.nom"));

    assert_eq!(ecoute.get("total"), Some(&Value::Int(5)));
    assert_eq!(ecoute.get("items.1.nom").and_then(Value::as_str), Some("pain"));
    assert_eq!(*totals.lock().unwrap(), [0, 5], "une fois a l'etat recu, une fois au batch");
    assert_eq!(*payments.lock().unwrap(), [r#""carte""#]);
    assert_eq!(panier.seq(), 6);
}

#[test]
fn the_state_stays_available_when_b_is_closed_and_b_resumes_it() {
    let daemon = Daemon::start("resume");
    {
        let mut panier = Flux::connect_at(&daemon.socket, B).unwrap().share("panier").public().open().unwrap();
        panier.set("total", 9).unwrap();
    } // B fermee.
    let mut ecoute = Flux::connect_at(&daemon.socket, C).unwrap().listen(B, "panier").start().unwrap();
    wait_until(&mut ecoute, |l| l.get("total") == Some(&Value::Int(9)));

    // B relancee : reprend son etat.
    let panier = Flux::connect_at(&daemon.socket, B).unwrap().share("panier").public().open().unwrap();
    assert_eq!(panier.get("total"), Some(&Value::Int(9)));
    assert_eq!(panier.seq(), 1);
    let streams = Flux::connect_at(&daemon.socket, C).unwrap().streams().unwrap();
    assert_eq!((streams[0].owner, streams[0].name.as_str(), streams[0].public), (B, "panier", true));

    panier.close().unwrap();
    wait_until(&mut ecoute, |l| l.state() == &Value::empty_map());
}

#[test]
fn b_decides_who_listens() {
    let daemon = Daemon::start("access");
    let mut panier = Flux::connect_at(&daemon.socket, B).unwrap().share("panier").to(&[A, C]).open().unwrap();
    let outsider = Flux::connect_at(&daemon.socket, 4).unwrap().listen(B, "panier").start();
    assert_eq!(outsider.err().unwrap(), "L'app 2 ne partage pas 'panier' avec l'app 4");

    let mut c = Flux::connect_at(&daemon.socket, C).unwrap().listen(B, "panier").start().unwrap();
    wait_until(&mut c, |l| l.seq() == 0 && l.is_connected());
    panier.share_with(azure_service::flux::Access::Apps(vec![A])).unwrap();
    let events = wait_until(&mut c, |l| l.is_finished());
    assert!(matches!(events.last(), Some(FluxEvent::Denied(m)) if m.contains("ne partage plus")));
    // Un flux pas partage par defaut : seule son app le voit.
    let prive = Flux::connect_at(&daemon.socket, B).unwrap().share("brouillon").open().unwrap();
    assert!(Flux::connect_at(&daemon.socket, A).unwrap().listen(B, "brouillon").start().is_err());
    drop(prive);
}

#[test]
fn an_app_cannot_pretend_to_be_another() {
    let daemon = Daemon::start("identity");
    std::fs::write(daemon.data.join("apps.txt"), "99\t/usr/bin/une-autre-app\n").unwrap();
    let error = Flux::connect_at(&daemon.socket, 99).err().unwrap();
    assert!(error.contains("L'app 99 appartient a l'executable /usr/bin/une-autre-app"), "{error}");
    // Le premier executable qui annonce un id le garde.
    Flux::connect_at(&daemon.socket, 50).unwrap();
    let registry = std::fs::read_to_string(daemon.data.join("apps.txt")).unwrap();
    assert!(registry.lines().any(|l| l.starts_with("50\t") && l.contains("flux_daemon")), "{registry}");
}

#[test]
fn everything_comes_back_after_the_daemon_restarts() {
    let mut daemon = Daemon::start("restart");
    let mut panier = Flux::connect_at(&daemon.socket, B).unwrap().share("panier").public().open().unwrap();
    panier.batch(|b| {
        b.set("client", "Ana").set("total", 3);
    })
    .unwrap();
    let mut ecoute = Flux::connect_at(&daemon.socket, A).unwrap().listen(B, "panier").start().unwrap();
    wait_until(&mut ecoute, |l| l.get("total") == Some(&Value::Int(3)));

    // Le daemon plante (azure-provider le relancerait) : il perd tout.
    daemon.kill();
    let events = wait_until(&mut ecoute, |l| !l.is_connected());
    assert_eq!(events.last(), Some(&FluxEvent::Disconnected));
    let (socket, data) = (daemon.socket.clone(), daemon.data.clone());
    drop(daemon);
    let _daemon = Daemon::start_at(socket, data);

    // B continue comme si de rien n'etait : son etat est rendu au daemon.
    panier.set("total", 4).unwrap();
    wait_until(&mut ecoute, |l| l.get("total") == Some(&Value::Int(4)) && l.get("client").is_some());
    assert_eq!(ecoute.state().to_string(), r#"{"client": "Ana", "total": 4}"#);
    assert!(ecoute.is_connected());
}

#[test]
fn a_change_refused_by_b_s_copy_is_never_sent() {
    let daemon = Daemon::start("refused");
    let mut flux = Flux::connect_at(&daemon.socket, B).unwrap().share("s").open().unwrap();
    flux.set("n", 1).unwrap();
    assert!(flux.push("n", 2).unwrap_err().contains("pas une liste"));
    assert!(flux.apply(vec![Change::set("a", 1), Change::set("", 5)]).is_err());
    assert_eq!(flux.seq(), 1);
    assert_eq!(flux.state().to_string(), r#"{"n": 1}"#);
}

#[test]
fn only_azure_manager_can_change_the_access_of_another_app() {
    let daemon = Daemon::start("setaccess");
    let _panier = Flux::connect_at(&daemon.socket, B).unwrap().share("panier").to(&[A]).open().unwrap();
    let error = Flux::connect_at(&daemon.socket, C).unwrap().set_access(B, "panier", &azure_service::flux::Access::Public).unwrap_err();
    assert_eq!(error, "SET_ACCESS reserve a azure-manager");
}

#[test]
fn a_persistent_flux_survives_a_daemon_crash() {
    let mut daemon = Daemon::start("persist");
    {
        let flux = Flux::connect_at(&daemon.socket, B).unwrap();
        let mut garde = flux.share("garde").to(&[A]).persist(true).open().unwrap();
        garde.set("total", 42).unwrap();
        garde.push("items", "pomme").unwrap();
        let mut volatil = flux.share("volatil").public().open().unwrap();
        volatil.set("x", 1).unwrap();
    }
    daemon.kill();
    let (socket, data) = (daemon.socket.clone(), daemon.data.clone());
    drop(daemon);
    let _daemon = Daemon::start_at(socket.clone(), data.clone());

    // Son app le retrouve tel quel (meme numero de modification).
    let flux = Flux::connect_at(&socket, B).unwrap();
    let garde = flux.share("garde").to(&[A]).persist(true).open().unwrap();
    assert_eq!(garde.state().to_string(), r#"{"items": ["pomme"], "total": 42}"#);
    assert_eq!(garde.seq(), 2);
    // Et A l'ecoute a nouveau.
    let mut ecoute = Flux::connect_at(&socket, A).unwrap().listen(B, "garde").start().unwrap();
    wait_until(&mut ecoute, |l| l.get("total") == Some(&Value::Int(42)));
    // Le flux ordinaire, lui, est reparti de zero.
    let volatil = flux.share("volatil").public().open().unwrap();
    assert_eq!((volatil.seq(), volatil.state().to_string()), (0, "{}".to_string()));
    // Ferme : son fichier disparait.
    garde.close().unwrap();
    assert!(!std::fs::read_dir(data.join("flux")).unwrap().flatten().any(|e| e.path().extension().is_some_and(|x| x == "flux")), "plus de fichier de flux (reste la cle)");
}

#[test]
fn only_the_manager_takes_the_manager_id() {
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("service-manager-id");
    let _ = std::fs::remove_dir_all(&data);
    std::fs::create_dir_all(&data).unwrap();
    let start = |manager: &str| {
        let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-service-test-manager-id-{}.sock"), std::process::id());
        let child = Command::new(env!("CARGO_BIN_EXE_service_daemon")).args(["--socket", &socket, "--data", data.to_str().unwrap(), "--manager", manager]).stdout(std::process::Stdio::null()).spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while std::os::unix::net::UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < deadline, "le daemon ne demarre pas");
            std::thread::sleep(Duration::from_millis(10));
        }
        Daemon { child, socket, data: data.clone() }
    };
    // Un autre executable (ce test) se fait passer pour le manager : refuse,
    // et l'id n'est pas lie a lui pour la suite.
    let daemon = start("/nulle-part/manager_daemon");
    let error = Flux::connect_at(&daemon.socket, azure_service::MANAGER_APP_ID).and_then(|f| f.share("etat").open().map(|_| ())).unwrap_err();
    assert!(error.contains("reservee a azure-manager"), "{error}");
    drop(daemon);
    // Le vrai manager (ici : ce test, declare comme tel) est accepte ensuite.
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let daemon = start(&me);
    let flux = Flux::connect_at(&daemon.socket, azure_service::MANAGER_APP_ID).unwrap();
    flux.share("etat").open().unwrap();
}
