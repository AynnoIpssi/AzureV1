// De bout en bout : le vrai daemon azure-manager (binaire) + un daemon
// azure-service dans le test. Ce test joue les apps ET le tableau de bord
// (son executable est declare administrateur).
use azure_manager::managers::manager::Kind;
use azure_manager::models::manifest::Manifest;
use azure_manager::services::client::ManagerClient;
use azure_service::flux::{Flux, FluxEvent, Listener, Value};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

struct Setup {
    manager: Child,
    socket: String,
    service: String,
    dir: PathBuf,
}

impl Drop for Setup {
    fn drop(&mut self) {
        let _ = self.manager.kill();
        let _ = self.manager.wait();
        let _ = std::fs::remove_file(&self.socket);
    }
}

fn wait_socket(socket: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(socket).is_err() {
        assert!(Instant::now() < deadline, "{socket} ne repond pas");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn setup(test: &str, admin: &str) -> Setup {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("manager-daemon-{test}"));
    let _ = std::fs::remove_dir_all(&dir);
    let pid = std::process::id();
    let (socket, service) = (format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-manager-test-{test}-{pid}.sock"), test = test, pid = pid), format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-manager-test-{test}-svc-{pid}.sock"), test = test, pid = pid));
    let manager_exe = env!("CARGO_BIN_EXE_manager_daemon").to_string();
    // Ce service demande l'identite des apps a CE manager (pas a celui d'un
    // Azure qui tournerait sur la machine, ni a celui d'un autre test).
    let (s, d, m) = (service.clone(), dir.join("service"), socket.clone());
    std::thread::spawn(move || azure_service::managers::daemon::start_daemon_for(&s, &d, Some(manager_exe), Some(m)));
    wait_socket(&service);
    let manager = Command::new(env!("CARGO_BIN_EXE_manager_daemon"))
        .args(["--socket", &socket, "--data", dir.join("manager").to_str().unwrap(), "--service-socket", &service, "--provider-socket", "/tmp/aucun-provider.sock", "--admin", admin, "--provider-logs", dir.join("logs-provider").to_str().unwrap(), "--app-logs", dir.join("logs-apps").to_str().unwrap()])
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    wait_socket(&socket);
    Setup { manager, socket, service, dir }
}

/// Une app qui se presente au manager (connexion gardee = active).
fn app(setup: &Setup, manifest: &str) -> (ManagerClient, u32) {
    let summary = Manifest::parse(manifest, Path::new("/x")).unwrap().summary();
    let mut client = ManagerClient::connect_at(&setup.socket).unwrap();
    let id = client.register(&summary).unwrap();
    (client, id)
}

fn eventually<T>(mut attempt: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = attempt() {
            return value;
        }
        assert!(Instant::now() < deadline, "delai depasse");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn wait_for(listener: &mut Listener, mut done: impl FnMut(&Listener, &[FluxEvent]) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    while !done(listener, &events) {
        assert!(Instant::now() < deadline, "delai depasse : {} / {events:?}", listener.state());
        events.extend(listener.wait(Duration::from_millis(50)));
    }
}

fn text(v: Option<&Value>) -> String {
    v.map(|v| match v {
        Value::Text(t) => t.clone(),
        other => other.to_string(),
    })
    .unwrap_or_default()
}

const BOUTIQUE: &str = "[app]\nname = boutique\n[share panier]\nto = caisse";
const CAISSE: &str = "[app]\nname = caisse\n[listen panier@boutique]";
const STATS: &str = "[app]\nname = stats\n[listen panier@boutique]";

#[test]
fn apps_talk_by_name_and_the_dashboard_decides() {
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let setup = setup("full", &me);
    let (mut boutique, b) = app(&setup, BOUTIQUE);
    let (mut caisse, c) = app(&setup, CAISSE);
    let (_stats_client, s) = app(&setup, STATS);
    assert_eq!(caisse.resolve("boutique").unwrap(), b, "les apps se designent par leur nom");

    // La boutique partage avec l'acces donne par le manager (son manifeste).
    let access = boutique.access(Kind::Flux, "panier").unwrap();
    assert_eq!(access, azure_service::flux::Access::Apps(vec![c]));
    let mut panier = Flux::connect_at(&setup.service, b).unwrap().share("panier").to(&[c]).open().unwrap();
    panier.set("total", 10).unwrap();

    let mut ecoute_caisse = Flux::connect_at(&setup.service, c).unwrap().listen(b, "panier").start().unwrap();
    assert!(Flux::connect_at(&setup.service, s).unwrap().listen(b, "panier").start().is_err(), "stats pas autorisee");

    // Le tableau de bord (ce test) : autorise stats, retire caisse.
    let (mut tableau, t) = app(&setup, "[app]\nname = tableau");
    tableau.grant(Kind::Flux, "boutique", "panier", "stats").unwrap();
    let mut ecoute_stats = eventually(|| Flux::connect_at(&setup.service, s).unwrap().listen(b, "panier").start().ok());
    wait_for(&mut ecoute_stats, |l, _| l.get("total") == Some(&Value::Int(10)));

    tableau.revoke(Kind::Flux, "boutique", "panier", "caisse").unwrap();
    wait_for(&mut ecoute_caisse, |l, _| l.is_finished());

    // L'etat, publie en flux pour le tableau de bord.
    let mut etat = eventually(|| Flux::connect_at(&setup.service, t).unwrap().listen(azure_manager::MANAGER_ID, azure_manager::STATE_FLUX).start().ok());
    // Le manager applique les droits PUIS publie l'etat : on attend l'etat
    // qui suit le dernier reglage.
    let links = |l: &Listener| -> Vec<String> { l.get("liens").and_then(Value::as_list).unwrap_or(&[]).iter().map(|l| format!("{}->{}:{}", text(l.get("de")), text(l.get("vers")), text(l.get("statut")))).collect() };
    wait_for(&mut etat, |l, _| text(l.get("nb_apps")) == "4" && links(l) == ["boutique->caisse:refusé", "boutique->stats:autorisé"]);

    // Une app qui se ferme apparait arretee.
    drop(caisse);
    wait_for(&mut etat, |l, _| text(l.get("nb_actives")) == "3");
    let state = tableau.state().unwrap();
    assert_eq!(text(state.get("apps.1.nom")), "caisse");
    assert_eq!(text(state.get("apps.1.actif")), "false");
    assert_eq!(text(state.get("provider")), "false", "pas de provider dans ce test");
    drop(boutique); // gardee active jusqu'ici
}

#[test]
fn only_the_dashboard_can_change_things() {
    let setup = setup("admin", "/usr/bin/pas-moi");
    let (mut boutique, _) = app(&setup, BOUTIQUE);
    app(&setup, CAISSE);
    for error in [boutique.revoke(Kind::Flux, "boutique", "panier", "caisse").unwrap_err(), boutique.state().unwrap_err(), boutique.forget("caisse").unwrap_err(), boutique.logs("stockage", 5).unwrap_err()] {
        assert_eq!(error, "Reserve au tableau de bord d'Azure");
    }
}

#[test]
fn a_name_belongs_to_its_executable() {
    let setup = setup("names", "/usr/bin/pas-moi");
    let (_first, id) = app(&setup, BOUTIQUE);
    let (_again, same) = app(&setup, BOUTIQUE);
    assert_eq!(id, same, "le meme executable peut ouvrir plusieurs connexions");
    let mut client = ManagerClient::connect_at(&setup.socket).unwrap();
    assert!(client.access(Kind::Flux, "panier").unwrap_err().contains("REGISTER attendu"));
    assert!(client.resolve("personne").unwrap_err().contains("App inconnue"));
}


#[test]
fn the_manager_gives_call_access_to_azure_service() {
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let setup = setup("calls", &me);
    let (mut boutique, b) = app(&setup, "[app]\nname = boutique\n[provide prix]\nto = caisse");
    let (_c, c) = app(&setup, "[app]\nname = caisse\n[use prix@boutique]");
    let (_s, s) = app(&setup, "[app]\nname = stats\n[use prix@boutique]");
    let access = boutique.access(Kind::Call, "prix").unwrap();
    assert_eq!(access, azure_service::flux::Access::Apps(vec![c]));
    let _serveur = Flux::connect_at(&setup.service, b).unwrap().serve("prix", access, |_| Ok(Value::Int(3))).unwrap();
    let stats = Flux::connect_at(&setup.service, s).unwrap();
    assert!(stats.call(b, "prix", Value::Null, Duration::from_secs(1)).is_err());

    // Le tableau de bord autorise stats : le manager le dit a azure-service.
    let (mut tableau, _) = app(&setup, "[app]\nname = tableau");
    tableau.grant(Kind::Call, "boutique", "prix", "stats").unwrap();
    assert_eq!(eventually(|| stats.call(b, "prix", Value::Null, Duration::from_secs(1)).ok()), Value::Int(3));
}


#[test]
fn apps_report_events_and_the_dashboard_reads_logs() {
    use azure_manager::managers::manager::Level;
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let setup = setup("events", &me);
    let (mut boutique, _) = app(&setup, BOUTIQUE);
    boutique.report("boutique", Level::Error, "paiement refuse").unwrap();
    boutique.report("boutique", Level::Info, "demarree").unwrap();
    assert!(boutique.report("caisse", Level::Error, "faux").unwrap_err().contains("n'est pas l'app qui parle"));

    let (mut tableau, _) = app(&setup, "[app]\nname = tableau");
    let state = tableau.state().unwrap();
    assert_eq!(text(state.get("evenements.0.message")), "demarree", "les plus recents d'abord");
    assert_eq!(text(state.get("evenements.1.niveau")), "erreur");
    assert_eq!(text(state.get("apps.0.nb_erreurs")), "1");
    assert_eq!(text(state.get("apps.0.erreurs.0.message")), "paiement refuse");
    assert_eq!(text(state.get("apps.0.erreurs.0.heure")).len(), 8, "HH:MM:SS");

    // Journaux : d'un service (azure-provider), puis d'une app (azure run).
    std::fs::create_dir_all(setup.dir.join("logs-provider")).unwrap();
    std::fs::create_dir_all(setup.dir.join("logs-apps")).unwrap();
    std::fs::write(setup.dir.join("logs-provider/stockage.log"), "un\ndeux\ntrois\n").unwrap();
    std::fs::write(setup.dir.join("logs-apps/boutique.log"), "boutique prete\n").unwrap();
    assert_eq!(tableau.logs("stockage", 2).unwrap(), ["deux", "trois"]);
    assert_eq!(tableau.logs("boutique", 50).unwrap(), ["boutique prete"]);
    assert!(tableau.logs("../secret", 5).is_err());
}

#[test]
fn call_and_flux_counters_reach_the_dashboard() {
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let setup = setup("counters", &me);
    let (mut boutique, b) = app(&setup, "[app]\nname = boutique\n[provide prix]\npublic = true\n[share panier]\npublic = true");
    let (_c, c) = app(&setup, "[app]\nname = caisse\n[use prix@boutique]");
    let flux_b = Flux::connect_at(&setup.service, b).unwrap();
    let _serveur = flux_b.serve("prix", boutique.access(Kind::Call, "prix").unwrap(), |d| d.args.as_i64().map(Value::Int).ok_or("nombre".to_string())).unwrap();
    let mut panier = flux_b.share("panier").public().persist(true).open().unwrap();
    panier.set("n", 1).unwrap();
    let caisse = Flux::connect_at(&setup.service, c).unwrap();
    caisse.call(b, "prix", 3i64, Duration::from_secs(1)).unwrap();
    assert!(caisse.call(b, "prix", "x", Duration::from_secs(1)).is_err());

    let (mut tableau, _) = app(&setup, "[app]\nname = tableau");
    let state = eventually(|| {
        let s = tableau.state().ok()?;
        (text(s.get("apps.0.fournit.0.stats.appels")) == "2").then_some(s)
    });
    assert_eq!(text(state.get("apps.0.fournit.0.stats.erreurs")), "1");
    assert_eq!(text(state.get("apps.0.fournit.0.stats.derniere_erreur")), "nombre");
    assert_eq!(text(state.get("apps.0.fournit.0.stats.servie")), "true");
    assert_eq!(text(state.get("apps.0.partages.0.stats.modifications")), "1");
    assert_eq!(text(state.get("apps.0.partages.0.stats.persistant")), "true");
}

#[test]
fn an_app_opens_only_what_its_manifest_declares() {
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let setup = setup("open", &me);
    let docs = Manifest::parse("[app]\nname = docs", Path::new("/x")).unwrap().summary();
    let mut admin = ManagerClient::connect_at(&setup.socket).unwrap();
    let fp = azure_core::managers::identity::fingerprint_file(Path::new(&me)).unwrap();
    admin.install(&docs, &me, &fp).unwrap();
    let (mut portfolio, _) = app(&setup, "[app]\nname = portfolio\n[open docs]\n[open note]");
    let (mut autre, _) = app(&setup, "[app]\nname = autre");

    assert!(autre.open("docs", "").unwrap_err().contains("[open docs]"));
    assert!(portfolio.open("note", "").unwrap_err().contains("pas installee"));
    // Docs tourne deja : rien a lancer, la page lui arrive par le routeur.
    let (_docs, _) = app(&setup, "[app]\nname = docs");
    assert_eq!(portfolio.open("docs", ""), Ok(false));
    let mut anonyme = ManagerClient::connect_at(&setup.socket).unwrap();
    assert!(anonyme.open("docs", "").unwrap_err().contains("REGISTER attendu"));
}
