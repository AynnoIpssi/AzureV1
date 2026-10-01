// Les expressions de stockage d'azure-foundation (`storage::models`), contre
// le VRAI daemon azure-stockage sur un socket et un dossier prives.
use azure_core::models::storage_model::{Role, ShareAccess};
use azure_foundation::storage::models::stockage::Stockage;
use azure_foundation::window::models::window_context::WindowContext;
use azure_stockage::managers::daemon::start_daemon_at;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn daemon(name: &str) -> String {
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-foundation-stockage-{name}-{}.sock"), std::process::id(), name = name);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("foundation-stockage-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let s = socket.clone();
    std::thread::spawn(move || start_daemon_at(&s, &root).unwrap());
    socket
}

fn connect(socket: &str, app: u32) -> Stockage {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match Stockage::connect_at(socket, app) {
            Ok(store) => return store,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("connexion impossible : {e}"),
        }
    }
}

#[test]
fn typed_values_in_one_line() {
    let store = connect(&daemon("typed"), 1);
    store.set("theme", "sombre").unwrap();
    store.set("volume", 80u32).unwrap();
    store.set("plein-ecran", true).unwrap();
    store.set("ratio", 1.5f64).unwrap();

    assert_eq!(store.get::<String>("theme").unwrap().as_deref(), Some("sombre"));
    assert_eq!(store.get::<u32>("volume").unwrap(), Some(80));
    assert_eq!(store.get::<i64>("volume").unwrap(), Some(80), "un nombre se relit dans un autre type entier");
    assert_eq!(store.get::<bool>("plein-ecran").unwrap(), Some(true));
    assert_eq!(store.get::<f64>("ratio").unwrap(), Some(1.5));
    assert!(store.get::<u32>("theme").is_err(), "'sombre' n'est pas un nombre");

    assert_eq!(store.get_or("absent", 7i32), 7);
    assert_eq!(store.get_or("theme", 0u8), 0, "illisible : valeur par defaut");
    assert!(store.has("theme").unwrap());
    assert!(store.forget("theme").unwrap());
    assert!(!store.has("theme").unwrap());
}

#[test]
fn update_remember_scope_and_clear() {
    let store = connect(&daemon("helpers"), 1);
    assert_eq!(store.update("clics", 0, |n: i64| n + 1).unwrap(), 1);
    assert_eq!(store.update("clics", 0, |n: i64| n + 1).unwrap(), 2);

    let mut calls = 0;
    assert_eq!(store.remember("bienvenue", || { calls += 1; "Salut".to_string() }).unwrap(), "Salut");
    assert_eq!(store.remember("bienvenue", || { calls += 1; "Autre".to_string() }).unwrap(), "Salut");
    assert_eq!(calls, 1);

    let reglages = store.scope("reglages");
    reglages.set("theme", "sombre").unwrap();
    reglages.scope("son").set("volume", 3).unwrap();
    assert_eq!(store.get::<String>("reglages.theme").unwrap().as_deref(), Some("sombre"));
    assert_eq!(reglages.keys().unwrap(), ["son.volume", "theme"]);

    assert_eq!(reglages.clear().unwrap(), 2);
    assert!(reglages.keys().unwrap().is_empty());
    assert_eq!(store.keys().unwrap(), ["bienvenue", "clics"], "clear ne touche qu'au scope");
}

#[test]
fn sharing_in_accordion_and_reading_from_another_app() {
    let socket = daemon("share");
    let notes = connect(&socket, 10);
    let galerie = connect(&socket, 11);

    notes.share("recette").value("crepes").public().save().unwrap();
    notes.share("journal")
        .value("cher journal")
        .protected()
        .account("maman", "1234").reader()
        .account("papa", "abcd").writer()
        .save()
        .unwrap();

    assert_eq!(galerie.from(10).get::<String>("recette").unwrap(), "crepes");
    assert!(galerie.from(10).get::<String>("journal").is_err(), "protege : il faut un compte");
    assert_eq!(galerie.from(10).login("maman", "1234").get::<String>("journal").unwrap(), "cher journal");
    assert!(galerie.from(10).login("maman", "1234").set("journal", "x").is_err(), "maman est reader");
    galerie.from(10).login("papa", "abcd").set("journal", "cher journal, suite").unwrap();
    assert_eq!(notes.shared().get::<String>("journal").unwrap(), "cher journal, suite");
    assert_eq!(galerie.from(10).get_or("journal", "cache".to_string()), "cache");

    assert_eq!(notes.accounts().unwrap(), [("maman".to_string(), Role::Reader), ("papa".to_string(), Role::Writer)]);
    let list = galerie.from(10).list().unwrap();
    assert_eq!(list.iter().map(|i| (i.name.as_str(), i.access)).collect::<Vec<_>>(), [("journal", ShareAccess::Protected), ("recette", ShareAccess::Public)]);

    // Sans .value() : garde la valeur, change l'acces.
    notes.share("journal").public().save().unwrap();
    assert_eq!(galerie.from(10).get::<String>("journal").unwrap(), "cher journal, suite");
    assert!(notes.share("jamais-partage").public().save().is_err());

    assert!(notes.unshare("recette").unwrap());
    assert!(galerie.from(10).get::<String>("recette").is_err());
}

#[test]
fn a_share_is_protected_unless_said_otherwise() {
    let socket = daemon("default");
    let notes = connect(&socket, 10);
    notes.share("brouillon").value("secret").save().unwrap();
    assert!(connect(&socket, 11).from(10).get::<String>("brouillon").is_err());
}

#[test]
fn window_context_gives_the_store_and_clones_share_the_connection() {
    let store = connect(&daemon("ctx"), 1);
    let copy = store.clone();
    let ctx = WindowContext { stockage: Some(&store), ..Default::default() };
    ctx.stockage().unwrap().update("clics", 0, |n: i64| n + 1).unwrap();
    assert_eq!(copy.get::<i64>("clics").unwrap(), Some(1));
    assert!(WindowContext::default().stockage().is_none());
}

#[test]
fn store_values_feed_rsh_conditions() {
    use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
    use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
    use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
    use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
    use azure_foundation::compiler::services::codegen::StyleSource;
    use azure_foundation::compiler::services::interpreter::build_ui_with_context;
    use azure_foundation::ui::models::ui_node::UiNode;

    let store = connect(&daemon("rsh"), 1);
    store.set("theme", "sombre").unwrap();
    let ast = parse_rsh(tokenize_rsh(r#"<if.theme == "sombre"><text>Mode sombre<!text><!if><else><text>Mode clair<!text><!else>"#)).unwrap();
    let sheet = parse_rsc(tokenize_rsc("")).unwrap();
    let nodes = build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &store.context(&["theme", "absente"]));
    assert!(matches!(&nodes[..], [UiNode::Label(l)] if l.text == "Mode sombre"));
}
