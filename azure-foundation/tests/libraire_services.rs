// Les services de la librairie (azure-libraire, `service`) servis pour de
// vrai : une app les sert avec `AzureApp::servir`, une autre les appelle
// par azure-service. Tous les daemons tournent dans ce test.
use azure_foundation::app::{AppSockets, AzureApp};
use azure_foundation::flux::Value;
use azure_libraire::service::{manifeste, services};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn wait_socket(socket: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::os::unix::net::UnixStream::connect(socket).is_err() {
        assert!(Instant::now() < deadline, "{socket} ne repond pas");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn start_daemons() -> AppSockets {
    let pid = std::process::id();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("libraire-services");
    let _ = std::fs::remove_dir_all(&dir);
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let sockets = AppSockets {
        manager: format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/libraire-services-manager-{pid}.sock"), pid = pid),
        service: format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/libraire-services-service-{pid}.sock"), pid = pid),
        stockage: format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/libraire-services-stockage-{pid}.sock"), pid = pid),
        router: format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/libraire-services-router-{pid}.sock"), pid = pid),
        provider: false,
        sandbox: false,
    };
    // Les daemons du test demandent l'identite des apps a SON manager, pas
    // a celui d'un Azure qui tournerait sur la machine.
    azure_service::managers::daemon::set_manager_socket(&sockets.manager);
    azure_stockage::managers::daemon::set_manager_socket(&sockets.manager);
    azure_rooter::managers::router::set_manager_socket(&sockets.manager);
    let s = sockets.clone();
    let (d, m) = (dir.clone(), me.clone());
    std::thread::spawn(move || azure_service::managers::daemon::start_daemon_with(&s.service, &d.join("service"), Some(m)));
    let s = sockets.clone();
    let d = dir.clone();
    std::thread::spawn(move || azure_stockage::managers::daemon::start_daemon_at(&s.stockage, &d.join("stockage")));
    let s = sockets.clone();
    std::thread::spawn(move || azure_rooter::managers::router::start_router_at(&s.router));
    wait_socket(&sockets.service);
    let config = azure_manager::managers::daemon::ManagerConfig {
        socket: sockets.manager.clone(),
        data_dir: dir.join("manager"),
        service_socket: sockets.service.clone(),
        provider_socket: "/tmp/aucun-provider.sock".into(),
        admins: vec![me],
        provider_logs: dir.join("logs-provider"),
        app_logs: dir.join("logs-apps"),
    };
    std::thread::spawn(move || azure_manager::managers::daemon::run(config));
    for socket in [&sockets.manager, &sockets.stockage, &sockets.router] {
        wait_socket(socket);
    }
    sockets
}

#[test]
fn une_app_sert_les_services_de_la_librairie_et_une_autre_les_appelle() {
    let sockets = start_daemons();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("libraire-services-apps");
    let ecrire = |app: &str, texte: String| {
        let d = dir.join(app);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("app.azure"), texte).unwrap();
        d.join("app.azure")
    };
    // `outils` sert tout ; seul `client` peut appeler.
    let outils = ecrire("outils", format!("[app]\nname = outils\ntitle = Outils\n\n{}", manifeste(services(), "", "to = client")));
    let client = ecrire("client", "[app]\nname = client\ntitle = Client\n[use diff-unifie@outils]\n[use archive-creer@outils]\n[use archive-lister@outils]\n[use temps-lire@outils]\n".to_string());
    let outils = AzureApp::from_manifest_at(outils, sockets.clone()).unwrap();
    let client = AzureApp::from_manifest_at(client, sockets.clone()).unwrap();

    let mut servies = Vec::new();
    for s in services() {
        servies.extend(outils.servir(s).unwrap());
    }
    assert_eq!(servies.len(), services().iter().map(|s| s.methodes.len()).sum::<usize>());

    let attente = Duration::from_secs(5);
    let diff = client.call_wait("outils", "diff-unifie", Value::map([("avant", Value::from("a\nb\n")), ("apres", Value::from("a\nc\n"))]), attente).unwrap();
    assert_eq!(diff.as_str().unwrap(), "--- avant\n+++ apres\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n");

    // Une valeur imbriquee a l'aller, une table au retour.
    let fichiers = Value::list([Value::map([("nom", Value::from("un.txt")), ("texte", Value::from("bonjour"))]), Value::map([("nom", Value::from("deux.txt")), ("texte", Value::from("au revoir"))])]);
    let zip = client.call_wait("outils", "archive-creer", Value::map([("fichiers", fichiers)]), attente).unwrap();
    let noms = client.call("outils", "archive-lister", Value::map([("base64", zip.get("base64").unwrap().clone())])).unwrap();
    assert_eq!(noms, Value::list([Value::from("un.txt"), Value::from("deux.txt")]));

    // L'erreur de la methode revient telle quelle.
    let e = client.call("outils", "temps-lire", Value::map([("texte", Value::from("demain"))])).unwrap_err();
    assert!(e.contains("date illisible"), "{e}");
    // Une methode non declaree par [use] est refusee avant de partir.
    assert!(client.call("outils", "diff-mots", Value::Null).unwrap_err().contains("Declarez [use diff-mots@outils]"));
}
