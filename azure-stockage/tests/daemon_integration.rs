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

// L'admin des donnees (Azure Data) : ici ce test lui-meme, declare par son
// executable ; les autres daemons de ce fichier n'ont aucun admin.
#[test]
fn the_data_admin_opens_every_app_and_nobody_else_can() {
    use azure_stockage::managers::daemon::{start_daemon_with, Options};
    use azure_stockage::rss::value::Value;
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-stockage-test-admin-{}.sock"), std::process::id());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stockage-daemon-admin");
    let _ = std::fs::remove_dir_all(&root);
    let moi = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let (s, r) = (socket.clone(), root.clone());
    std::thread::spawn(move || start_daemon_with(&s, &r, Options { admins: vec![moi] }).unwrap());

    let mut notes = connect(&socket, 10);
    notes.rss("CREATE TABLE notes (id INT PRIMARY KEY, titre TEXT NOT NULL, photo BLOB, libre ANY); CREATE INDEX par_titre ON notes (titre); INSERT INTO notes (titre, photo, libre) VALUES ('Courses', x'CAFE', 3), ('Sport', NULL, 'x')", &[], &[]).unwrap();
    notes.put_text("theme", "sable").unwrap();

    let mut data = connect(&socket, 20);
    let apps = data.admin_apps().unwrap();
    let app = apps.iter().find(|a| a.id == 10).expect("l'app 10 est listee");
    assert_eq!((app.tables, app.keys), (1, 1));
    let schema = data.admin_schema(10).unwrap();
    assert_eq!(schema.len(), 1);
    assert_eq!((schema[0].name.as_str(), schema[0].rows, schema[0].columns.len()), ("notes", 2, 4));
    assert!(schema[0].columns[0].primary && schema[0].columns[1].not_null);
    assert_eq!(schema[0].indexes[0].column, "titre");
    let lignes = data.admin_rss(10, "SELECT titre, photo, libre FROM notes ORDER BY id", &[]).unwrap().remove(0).rows;
    assert_eq!(lignes, [vec![Value::Text("Courses".into()), Value::Blob(vec![0xCA, 0xFE]), Value::Int(3)], vec![Value::Text("Sport".into()), Value::Null, Value::Text("x".into())]]);
    // Il ecrit aussi : l'app voit le changement.
    data.admin_rss(10, "UPDATE notes SET titre = ? WHERE id = 2", &[Value::Text("Piscine".into())]).unwrap();
    assert_eq!(notes.rss("SELECT titre FROM notes WHERE id = 2", &[], &[]).unwrap().remove(0).rows, [[Value::Text("Piscine".into())]]);
    assert_eq!(data.admin_keys(10).unwrap(), [("theme".to_string(), 5)]);
    assert_eq!(data.admin_get(10, "theme").unwrap().as_deref(), Some(b"sable".as_slice()));
}

#[test]
fn admin_requests_are_refused_to_a_normal_app() {
    let socket = start("pas-admin", |_| {});
    let mut notes = connect(&socket, 10);
    notes.rss("CREATE TABLE t (x INT)", &[], &[]).unwrap();
    let mut curieuse = connect(&socket, 11);
    for err in [curieuse.admin_apps().map(drop), curieuse.admin_schema(10).map(drop), curieuse.admin_rss(10, "SELECT * FROM t", &[]).map(drop), curieuse.admin_keys(10).map(drop), curieuse.admin_get(10, "k").map(drop)] {
        assert!(err.unwrap_err().contains("Reserve a Azure Data"));
    }
}
