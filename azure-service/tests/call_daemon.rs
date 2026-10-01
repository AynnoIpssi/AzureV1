// Appels entre apps, de bout en bout avec le vrai daemon (binaire).
use azure_service::call::{CallRequest, DEFAULT_TIMEOUT};
use azure_service::flux::{Access, Flux, Value};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const BOUTIQUE: u32 = 10;
const CAISSE: u32 = 11;
const AUTRE: u32 = 12;

struct Daemon {
    child: Child,
    socket: String,
    data: PathBuf,
}

impl Daemon {
    fn start(test: &str) -> Daemon {
        let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("call-{test}"));
        let _ = std::fs::remove_dir_all(&data);
        let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-call-test-{test}-{}.sock"), std::process::id(), test = test);
        Daemon::start_at(socket, data)
    }

    fn start_at(socket: String, data: PathBuf) -> Daemon {
        let child = Command::new(env!("CARGO_BIN_EXE_service_daemon")).args(["--socket", &socket, "--data", data.to_str().unwrap()]).stdout(std::process::Stdio::null()).spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while std::os::unix::net::UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        Daemon { child, socket, data }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket);
    }
}

/// La boutique sert `prix` : 3 par produit, erreur si le produit manque.
fn prix(demande: &CallRequest) -> Result<Value, String> {
    let produit = demande.args.get("produit").and_then(Value::as_i64).ok_or("produit attendu")?;
    Ok(Value::map([("produit", produit.into()), ("prix", (produit * 3).into()), ("appelant", demande.caller.into())]))
}

#[test]
fn an_app_asks_another_and_gets_the_answer() {
    let daemon = Daemon::start("answer");
    let boutique = Flux::connect_at(&daemon.socket, BOUTIQUE).unwrap();
    let _serveur = boutique.serve("prix", Access::Apps(vec![CAISSE]), prix).unwrap();

    let caisse = Flux::connect_at(&daemon.socket, CAISSE).unwrap();
    let reponse = caisse.call(BOUTIQUE, "prix", Value::map([("produit", 14.into())]), DEFAULT_TIMEOUT).unwrap();
    assert_eq!(reponse.get("prix"), Some(&Value::Int(42)));
    assert_eq!(reponse.get("appelant"), Some(&Value::Int(CAISSE as i64)), "l'appelant est verifie par le daemon");

    // L'erreur de l'app appelee revient telle quelle.
    assert_eq!(caisse.call(BOUTIQUE, "prix", Value::empty_map(), DEFAULT_TIMEOUT).unwrap_err(), "produit attendu");
    // Pas autorisee, methode inconnue.
    let autre = Flux::connect_at(&daemon.socket, AUTRE).unwrap();
    assert_eq!(autre.call(BOUTIQUE, "prix", Value::empty_map(), DEFAULT_TIMEOUT).unwrap_err(), "L'app 10 ne permet pas a l'app 12 d'appeler 'prix'");
    assert!(caisse.call(BOUTIQUE, "stock", Value::Null, DEFAULT_TIMEOUT).unwrap_err().contains("n'est pas disponible"));
    // La boutique peut s'appeler elle-meme.
    assert!(boutique.call(BOUTIQUE, "prix", Value::map([("produit", 1.into())]), DEFAULT_TIMEOUT).is_ok());
}

#[test]
fn slow_answers_time_out_and_calls_run_in_parallel() {
    let daemon = Daemon::start("slow");
    let boutique = Flux::connect_at(&daemon.socket, BOUTIQUE).unwrap();
    let _lent = boutique
        .serve("lent", Access::Public, |d| {
            std::thread::sleep(Duration::from_millis(d.args.as_i64().unwrap_or(0) as u64));
            Ok(Value::from("fini"))
        })
        .unwrap();
    let caisse = Flux::connect_at(&daemon.socket, CAISSE).unwrap();
    let debut = Instant::now();
    let erreur = caisse.call(BOUTIQUE, "lent", 2000i64, Duration::from_millis(200)).unwrap_err();
    assert_eq!(erreur, "'lent' de l'app 10 n'a pas repondu en 200 ms");
    assert!(debut.elapsed() < Duration::from_millis(1000));

    // 8 appels de 300 ms en meme temps : servis en parallele.
    let debut = Instant::now();
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let caisse = caisse.clone();
            std::thread::spawn(move || caisse.call(BOUTIQUE, "lent", 300i64, DEFAULT_TIMEOUT))
        })
        .collect();
    for t in threads {
        assert_eq!(t.join().unwrap().unwrap(), Value::from("fini"));
    }
    assert!(debut.elapsed() < Duration::from_millis(1500), "{:?}", debut.elapsed());
}

#[test]
fn a_crash_or_a_stop_is_reported_to_the_caller() {
    let daemon = Daemon::start("stop");
    let boutique = Flux::connect_at(&daemon.socket, BOUTIQUE).unwrap();
    let panique = boutique.serve("panique", Access::Public, |_| panic!("boum")).unwrap();
    let caisse = Flux::connect_at(&daemon.socket, CAISSE).unwrap();
    assert_eq!(caisse.call(BOUTIQUE, "panique", Value::Null, DEFAULT_TIMEOUT).unwrap_err(), "l'app a plante en repondant");

    // La methode n'est plus servie quand l'objet disparait.
    drop(panique);
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match caisse.call(BOUTIQUE, "panique", Value::Null, DEFAULT_TIMEOUT) {
            Err(e) if e.contains("n'est pas disponible") => break,
            _ => {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    // L'app qui sert s'arrete PENDANT un appel : erreur tout de suite.
    let lente = boutique.serve("lente", Access::Public, |_| {
        std::thread::sleep(Duration::from_secs(10));
        Ok(Value::Null)
    });
    let lente = lente.unwrap();
    let c = caisse.clone();
    let appel = std::thread::spawn(move || c.call(BOUTIQUE, "lente", Value::Null, Duration::from_secs(8)));
    std::thread::sleep(Duration::from_millis(200));
    let debut = Instant::now();
    drop(lente);
    assert_eq!(appel.join().unwrap().unwrap_err(), "l'app s'est arretee pendant l'appel");
    assert!(debut.elapsed() < Duration::from_secs(2));
}

#[test]
fn calls_come_back_after_the_daemon_restarts() {
    let daemon = Daemon::start("restart");
    let (socket, data) = (daemon.socket.clone(), daemon.data.clone());
    let compteur = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&compteur);
    let boutique = Flux::connect_at(&socket, BOUTIQUE).unwrap();
    let _serveur = boutique
        .serve("compte", Access::Public, move |_| Ok(Value::Int(c.fetch_add(1, Ordering::SeqCst) as i64 + 1)))
        .unwrap();
    let caisse = Flux::connect_at(&socket, CAISSE).unwrap();
    assert_eq!(caisse.call(BOUTIQUE, "compte", Value::Null, DEFAULT_TIMEOUT).unwrap(), Value::Int(1));

    drop(daemon);
    let _daemon = Daemon::start_at(socket, data);
    // La boutique se reconnecte d'elle-meme ; la caisse aussi, sans rejouer
    // d'appel (le compteur n'avance que d'un par appel reussi).
    let deadline = Instant::now() + Duration::from_secs(5);
    let valeur = loop {
        match caisse.call(BOUTIQUE, "compte", Value::Null, DEFAULT_TIMEOUT) {
            Ok(v) => break v,
            Err(_) => {
                assert!(Instant::now() < deadline, "pas revenu");
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    assert_eq!(valeur, Value::Int(2));
    assert_eq!(compteur.load(Ordering::SeqCst), 2);
}

#[test]
fn only_azure_manager_changes_who_can_call() {
    let daemon = Daemon::start("access");
    let boutique = Flux::connect_at(&daemon.socket, BOUTIQUE).unwrap();
    let _serveur = boutique.serve("prix", Access::Apps(vec![CAISSE]), prix).unwrap();
    let error = Flux::connect_at(&daemon.socket, CAISSE).unwrap().set_call_access(BOUTIQUE, "prix", &Access::Public).unwrap_err();
    assert_eq!(error, "SET_CALL_ACCESS reserve a azure-manager");
}
