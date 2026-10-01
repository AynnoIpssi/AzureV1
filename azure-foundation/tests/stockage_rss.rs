// RsS et emplacement depuis azure-foundation (`Stockage::db`, `move_to`),
// contre le VRAI daemon azure-stockage sur un socket et un dossier prives.
use azure_foundation::params;
use azure_foundation::storage::{Stockage, Value};
use azure_foundation::storage::Role;
use azure_stockage::managers::daemon::start_daemon_at;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn daemon(name: &str) -> String {
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-foundation-rss-{name}-{}.sock"), std::process::id(), name = name);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("foundation-rss-{name}"));
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

fn schema() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/rss/schema.rss").to_string()
}

#[test]
fn todo_app_in_rss() {
    let store = connect(&daemon("todo"), 1);
    let db = store.db();
    db.run_file(&schema()).unwrap();

    db.run("INSERT INTO listes (nom) VALUES (?), (?)", &params!["Maison", "Travail"]).unwrap();
    let maison: i64 = db.value("SELECT id FROM listes WHERE nom = ?", &params!["Maison"]).unwrap().unwrap();
    for (titre, minutes) in [("Vaisselle", 15), ("Linge", 30), ("Courses", 45)] {
        db.run("INSERT INTO taches (liste_id, titre, minutes) VALUES (?, ?, ?)", &params![maison, titre, minutes]).unwrap();
    }
    db.run("INSERT INTO taches (liste_id, titre, minutes) VALUES (?, ?, ?)", &params![2, "Rapport", None::<i64>]).unwrap();
    assert_eq!(db.run("UPDATE taches SET fait = true WHERE titre = ?", &params!["Linge"]).unwrap(), 1);

    let rows = db.query("SELECT titre, minutes FROM taches WHERE liste_id = ? AND fait = ? ORDER BY minutes DESC", &params![maison, false]).unwrap();
    assert_eq!(rows.column::<String>("titre").unwrap(), ["Courses", "Vaisselle"]);
    let first = rows.iter().next().unwrap();
    assert_eq!(first.get::<i64>("minutes").unwrap(), 45);
    assert!(first.get::<String>("minutes").is_err(), "mauvais type : erreur claire");

    let bilan = db
        .query("SELECT l.nom, count(t.id) AS n, sum(t.minutes) AS total FROM listes l LEFT JOIN taches t ON t.liste_id = l.id GROUP BY l.nom ORDER BY l.nom", &[])
        .unwrap();
    let lignes: Vec<(String, i64, Option<i64>)> = bilan.iter().map(|r| (r.get("nom").unwrap(), r.get("n").unwrap(), r.get("total").unwrap())).collect();
    assert_eq!(lignes, [("Maison".to_string(), 3, Some(90)), ("Travail".to_string(), 1, None)]);
    assert_eq!(bilan.iter().nth(1).unwrap().get_or("total", 0i64), 0);

    assert_eq!(db.value::<i64>("SELECT count(*) FROM taches WHERE titre = ?", &params!["x' OR 1=1 --"]).unwrap(), Some(0));
    assert!(db.run("INSERT INTO listes (nom) VALUES (?)", &params!["Maison"]).unwrap_err().contains("unique"));
}

#[test]
fn transactions_are_all_or_nothing() {
    let store = connect(&daemon("tx"), 1);
    let db = store.db();
    db.script("CREATE TABLE compte (nom TEXT PRIMARY KEY, solde INT NOT NULL); INSERT INTO compte VALUES ('a', 100), ('b', 0)").unwrap();

    let virement = |montant: i64| {
        db.transaction(|db| {
            db.run("UPDATE compte SET solde = solde - ? WHERE nom = 'a'", &params![montant])?;
            db.run("UPDATE compte SET solde = solde + ? WHERE nom = 'b'", &params![montant])?;
            let solde: i64 = db.value("SELECT solde FROM compte WHERE nom = 'a'", &[])?.unwrap_or(0);
            if solde < 0 { Err(format!("solde insuffisant ({solde})")) } else { Ok(solde) }
        })
    };
    assert_eq!(virement(30), Ok(70));
    assert!(virement(500).unwrap_err().contains("insuffisant"));
    let soldes = db.query("SELECT solde FROM compte ORDER BY nom", &[]).unwrap().column::<i64>("solde").unwrap();
    assert_eq!(soldes, [70, 30], "le virement refuse n'a rien laisse");
}

#[test]
fn another_app_reads_a_shared_table_with_a_login() {
    let socket = daemon("share");
    let notes = connect(&socket, 10);
    notes.db().script("CREATE TABLE recettes (nom TEXT, minutes INT); INSERT INTO recettes VALUES ('crepes', 20), ('gateau', 60); SHARE TABLE recettes PROTECTED").unwrap();
    notes.add_account("maman", "1234", Role::Reader).unwrap();

    let other = connect(&socket, 11);
    assert!(other.db().query("SELECT * FROM @10.recettes", &[]).unwrap_err().contains("protegee"));
    let rapides = other.db().login(10, "maman", "1234").query("SELECT nom FROM @10.recettes WHERE minutes < ?", &params![30]).unwrap();
    assert_eq!(rapides.column::<String>("nom").unwrap(), ["crepes"]);
    assert!(other.db().login(10, "maman", "1234").run("DELETE FROM @10.recettes", &[]).unwrap_err().contains("lecture seule"));
}

#[test]
fn an_app_chooses_where_its_data_lives() {
    let store = connect(&daemon("move"), 1);
    store.set("theme", "sombre").unwrap();
    store.db().script("CREATE TABLE t (x INT); INSERT INTO t VALUES (7)").unwrap();
    assert_eq!(store.location().unwrap(), None);

    let disk = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("foundation-rss-move-disque");
    let _ = std::fs::remove_dir_all(&disk);
    store.move_to(disk.to_str().unwrap()).unwrap();
    assert_eq!(store.location().unwrap().as_deref(), disk.to_str());
    assert_eq!(store.get::<String>("theme").unwrap().as_deref(), Some("sombre"));
    assert_eq!(store.db().value::<i64>("SELECT x FROM t", &[]).unwrap(), Some(7));
    assert_eq!(std::fs::read_dir(&disk).unwrap().count(), 1);

    assert!(store.move_to("relatif").is_err());
    store.reset_location().unwrap();
    assert_eq!(store.location().unwrap(), None);
    assert_eq!(store.db().value::<i64>("SELECT x FROM t", &[]).unwrap(), Some(7));
}

#[test]
fn rows_feed_rsh_conditions() {
    let store = connect(&daemon("ctx"), 1);
    let db = store.db();
    db.script("CREATE TABLE t (etat TEXT); INSERT INTO t VALUES ('fini')").unwrap();
    let row = db.first("SELECT etat FROM t", &[]).unwrap().unwrap();
    assert_eq!(row.value("etat"), Some(&Value::Text("fini".into())));
    use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
    use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
    use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
    use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
    use azure_foundation::compiler::services::codegen::StyleSource;
    use azure_foundation::compiler::services::interpreter::build_ui_with_context;
    use azure_foundation::ui::models::ui_node::UiNode;
    let ast = parse_rsh(tokenize_rsh(r#"<if.etat == "fini"><text>Termine<!text><!if><else><text>En cours<!text><!else>"#)).unwrap();
    let sheet = parse_rsc(tokenize_rsc("")).unwrap();
    let nodes = build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &row.context());
    assert!(matches!(&nodes[..], [UiNode::Label(l)] if l.text == "Termine"));
    assert!(db.first("SELECT etat FROM t WHERE etat = 'rien'", &[]).unwrap().is_none());
}
