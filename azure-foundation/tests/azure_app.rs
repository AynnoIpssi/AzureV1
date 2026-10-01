// `AzureApp` : deux apps declarees par leur manifeste, qui se parlent par
// leur nom. Tous les daemons tournent dans ce test (pas d'azure-provider).
use azure_foundation::app::{AppSockets, AzureApp};
use azure_foundation::flux::Value;
use azure_foundation::navigation::managers::navigation_manager;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::ui::models::ui_node::UiNode;
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
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("azure-app");
    let _ = std::fs::remove_dir_all(&dir);
    let me = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let sockets = AppSockets {
        manager: format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-app-test-manager-{pid}.sock"), pid = pid),
        service: format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-app-test-service-{pid}.sock"), pid = pid),
        stockage: format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-app-test-stockage-{pid}.sock"), pid = pid),
        router: format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-app-test-router-{pid}.sock"), pid = pid),
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

fn manifest(app: &str) -> String {
    format!("{}/tests/app/{app}/app.azure", env!("CARGO_MANIFEST_DIR"))
}

fn texts(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(l) => out.push(l.text.clone()),
            UiNode::Container(c) => texts(&c.children, out),
            _ => {}
        }
    }
}

#[test]
fn two_apps_declared_by_their_manifest_talk_by_name() {
    let sockets = start_daemons();
    let boutique = AzureApp::from_manifest_at(manifest("boutique"), sockets.clone()).unwrap();
    let caisse = AzureApp::from_manifest_at(manifest("caisse"), sockets.clone()).unwrap();
    assert!(boutique.id() >= 1000 && caisse.id() > boutique.id(), "ids attribues par azure-manager");
    assert_eq!(caisse.resolve("boutique").unwrap(), boutique.id());

    // Flux : acces et filtre viennent des manifestes.
    let mut panier = boutique.share("panier").unwrap();
    panier.set("total", 12).unwrap();
    panier.set("client", "Ana").unwrap();
    let mut ecoute = caisse.listen("panier@boutique").unwrap().start().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while ecoute.get("total") != Some(&Value::Int(12)) {
        assert!(Instant::now() < deadline);
        ecoute.wait(Duration::from_millis(50));
    }
    assert_eq!(ecoute.get("client"), None, "paths = total dans le manifeste de caisse");
    assert!(boutique.share("secret").err().unwrap().contains("Declarez [share secret]"));
    assert!(caisse.listen("autre@boutique").err().unwrap().contains("Declarez [listen autre@boutique]"));

    // Message par nom, recu par la caisse.
    let mut recu = navigation_manager::connect_at(&sockets.router, caisse.id()).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    boutique.send("caisse", "bonjour").unwrap();
    assert_eq!(navigation_manager::receive(&mut recu).unwrap(), "bonjour");

    // Stockage de l'app.
    let store = caisse.stockage().unwrap();
    store.set("theme", "sombre").unwrap();
    assert_eq!(store.get::<String>("theme").unwrap().as_deref(), Some("sombre"));

    // Appel avec reponse, declare des deux cotes. Pas encore servi : `call`
    // echoue tout de suite, `call_wait` attend que la methode le soit.
    assert!(caisse.call("boutique", "prix", Value::Null).unwrap_err().contains("pas disponible"));
    let (_prix, attendu) = std::thread::scope(|scope| {
        let attente = scope.spawn(|| caisse.call_wait("boutique", "prix", Value::map([("produit", 2.into())]), Duration::from_secs(5)));
        std::thread::sleep(Duration::from_millis(300));
        let prix = boutique
            .serve("prix", |demande| {
                let produit = demande.args.get("produit").and_then(Value::as_i64).ok_or("produit attendu")?;
                Ok(Value::from(produit * 3))
            })
            .unwrap();
        (prix, attente.join().unwrap())
    });
    assert_eq!(attendu.unwrap(), Value::Int(6));
    assert_eq!(caisse.call("boutique", "prix", Value::map([("produit", 14.into())])).unwrap(), Value::Int(42));
    assert_eq!(caisse.call("boutique", "prix", Value::Null).unwrap_err(), "produit attendu");
    assert!(caisse.call("boutique", "stock", Value::Null).unwrap_err().contains("Declarez [use stock@boutique]"));
    assert!(boutique.serve("stock", |_| Ok(Value::Null)).err().unwrap().contains("Declarez [provide stock]"));

    // Point 5 et 6 : flux persistant (manifeste), evenements, plantage signale.
    let (flux_stats, _) = azure_service::flux::Flux::connect_at(&sockets.service, 42).unwrap().stats().unwrap();
    assert!(flux_stats.iter().any(|f| f.name == "panier" && f.persist), "persist = true dans le manifeste");
    boutique.error("stock epuise");
    let _ = std::thread::spawn(|| panic!("boum")).join();
    let mut admin = azure_manager::services::client::ManagerClient::connect_at(&sockets.manager).unwrap();
    let state = admin.state().unwrap();
    let messages: Vec<String> = state.get("evenements").unwrap().as_list().unwrap().iter().filter_map(|e| e.get("message").and_then(Value::as_str).map(String::from)).collect();
    assert!(messages.iter().any(|m| m == "stock epuise"), "{messages:?}");
    assert!(messages.iter().any(|m| m.starts_with("plantage : boum (") && m.contains("azure_app.rs")), "{messages:?}");

    // Fenetre et pages du manifeste.
    let mut out = Vec::new();
    texts(&boutique.routes().resolve(&Route::new("/accueil", "")).unwrap(), &mut out);
    assert_eq!(out, ["Boutique", "Page /accueil"]);
    let window = boutique.window().unwrap();
    assert_eq!(window.window_spec().unwrap().owner_app_id(), boutique.id());
    assert_eq!((window.window_spec().unwrap().size().width(), window.window_spec().unwrap().size().height()), (640, 480));
}

#[test]
fn the_manifest_is_found_installed_or_in_development() {
    use azure_foundation::app::locate_manifest;
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("locate-manifest");
    let (installed, dev) = (root.join("apps/notes"), root.join("src-notes"));
    std::fs::create_dir_all(&installed).unwrap();
    // Pas de manifeste a cote de l'executable : celui du crate.
    let _ = std::fs::remove_file(installed.join("app.azure"));
    assert_eq!(locate_manifest(None, Some(&installed), &dev), dev.join("app.azure"));
    // App installee : celui d'a cote.
    std::fs::write(installed.join("app.azure"), "[app]\nname = notes\n").unwrap();
    assert_eq!(locate_manifest(None, Some(&installed), &dev), installed.join("app.azure"));
    // La variable d'environnement passe avant tout.
    assert_eq!(locate_manifest(Some("/ailleurs/app.azure"), Some(&installed), &dev), PathBuf::from("/ailleurs/app.azure"));
}
