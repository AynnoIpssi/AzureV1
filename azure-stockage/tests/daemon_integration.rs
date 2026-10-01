// Le VRAI daemon sur un socket et un dossier prives (jamais le vrai daemon),
// avec deux apps clientes.
use azure_core::models::storage_model::{Role, ShareAccess};
use azure_stockage::managers::daemon::start_daemon_at;
use azure_stockage::managers::stockage::{AzureStockage, Credentials};
use azure_stockage::services::client::StockageClient;
use std::path::PathBuf;
use std::time::{Duration, Instant};

// `prepare` agit sur le dossier AVANT que le daemon le prenne (ensuite, lui
// seul doit y toucher).
fn start(name: &str, prepare: impl FnOnce(&AzureStockage)) -> String {
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-stockage-test-{name}-{}.sock"), std::process::id(), name = name);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("stockage-daemon-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    prepare(&AzureStockage::open(&root).unwrap());
    let (s, r) = (socket.clone(), root.clone());
    std::thread::spawn(move || start_daemon_at(&s, &r).unwrap());
    socket
}

fn connect(socket: &str, app: u32) -> StockageClient {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match StockageClient::connect_at(socket, app) {
            Ok(client) => return client,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("connexion impossible : {e}"),
        }
    }
}

#[test]
fn two_apps_through_the_daemon() {
    let socket = start("apps", |_| {});
    let mut notes = connect(&socket, 10);
    let mut galerie = connect(&socket, 11);

    notes.put_text("brouillon", "liste de courses").unwrap();
    assert_eq!(notes.get_text("brouillon").unwrap().as_deref(), Some("liste de courses"));
    assert_eq!(galerie.get("brouillon").unwrap(), None, "prive a l'app notes");
    assert_eq!(notes.keys().unwrap(), ["brouillon"]);

    notes.share("recette", b"crepes", ShareAccess::Public).unwrap();
    notes.share("journal", b"cher journal", ShareAccess::Protected).unwrap();
    notes.add_account("maman", "1234", Role::Reader).unwrap();

    assert_eq!(galerie.read_shared(10, "recette", None).unwrap(), b"crepes");
    assert!(galerie.read_shared(10, "journal", None).is_err());
    let maman = Some(Credentials { user: "maman", password: "1234" });
    assert_eq!(galerie.read_shared(10, "journal", maman).unwrap(), b"cher journal");
    assert!(galerie.write_shared(10, "journal", b"x", maman).is_err());
    assert_eq!(galerie.shared_list(10).unwrap().len(), 2);
    assert_eq!(notes.accounts().unwrap(), [("maman".to_string(), Role::Reader)]);

    assert!(notes.delete("brouillon").unwrap());
    let err = notes.put("", b"x").unwrap_err();
    assert!(err.contains("empty"), "l'erreur du daemon remonte au client : {err}");
}

#[test]
fn an_app_id_owned_by_another_executable_is_refused() {
    // L'app 20 a deja ete prise par un autre programme.
    let socket = start("identity", |store| store.bind_app(20, "/usr/bin/autre-app").unwrap());
    let deadline = Instant::now() + Duration::from_secs(2);
    let err = loop {
        match StockageClient::connect_at(&socket, 20) {
            Err(e) if e.contains("/tmp/") && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => break e,
            Ok(_) => panic!("un autre executable ne doit pas pouvoir se presenter comme l'app 20"),
        }
    };
    assert!(err.contains("autre-app"), "{err}");
    // Le meme programme garde son id d'une connexion a l'autre.
    connect(&socket, 21);
    connect(&socket, 21).put("ok", b"1").unwrap();
}
