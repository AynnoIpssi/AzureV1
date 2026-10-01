// RsS (le SQL d'Azure) execute directement par le moteur, sans socket.
use azure_core::models::storage_model::Role;
use azure_stockage::managers::stockage::AzureStockage;
use azure_stockage::rss::engine::{execute, Login, RssResult, Session};
use azure_stockage::rss::value::Value;
use std::path::PathBuf;

fn root(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("rss-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    root
}

struct Db {
    store: AzureStockage,
    session: Session,
    app: u32,
}

impl Db {
    fn new(name: &str) -> Db {
        Db { store: AzureStockage::open(&root(name)).unwrap(), session: Session::new(), app: 1 }
    }

    fn run(&mut self, sql: &str) -> RssResult {
        self.run_with(sql, &[])
    }

    fn run_with(&mut self, sql: &str, params: &[Value]) -> RssResult {
        match execute(&mut self.store, &mut self.session, self.app, sql, params, &[]) {
            Ok(mut results) => results.pop().unwrap_or_default(),
            Err(e) => panic!("{sql}\n=> {e}"),
        }
    }

    fn fails(&mut self, sql: &str) -> String {
        match execute(&mut self.store, &mut self.session, self.app, sql, &[], &[]) {
            Ok(r) => panic!("{sql} aurait du echouer, a rendu {r:?}"),
            Err(e) => e,
        }
    }

    // Les lignes rendues, en texte, pour comparer facilement.
    fn rows(&mut self, sql: &str) -> Vec<String> {
        self.run(sql).rows.iter().map(|r| r.iter().map(Value::to_string).collect::<Vec<_>>().join("|")).collect()
    }
}

fn notes() -> Db {
    let mut db = Db::new(&format!("notes-{:?}", std::thread::current().id()));
    db.run(
        "CREATE TABLE notes (id INT PRIMARY KEY, titre TEXT NOT NULL, fait BOOL DEFAULT false, prio INT);
         INSERT INTO notes (titre, prio) VALUES ('Courses', 2), ('Sport', 1), ('Lire', 3);
         INSERT INTO notes (id, titre, fait, prio) VALUES (10, 'Impots', true, NULL);",
    );
    db
}

#[test]
fn create_insert_select() {
    let mut db = notes();
    assert_eq!(db.rows("SELECT id, titre, fait FROM notes"), ["1|Courses|false", "2|Sport|false", "3|Lire|false", "10|Impots|true"]);
    assert_eq!(db.rows("SELECT titre FROM notes WHERE fait = false ORDER BY prio DESC LIMIT 2"), ["Lire", "Courses"]);
    assert_eq!(db.rows("SELECT titre FROM notes ORDER BY id LIMIT 2 OFFSET 1"), ["Sport", "Lire"]);
    assert_eq!(db.rows("SELECT titre AS t, prio * 10 AS p FROM notes WHERE prio IS NOT NULL ORDER BY p"), ["Sport|10", "Courses|20", "Lire|30"]);
    let result = db.run("SELECT titre AS t, count(*) FROM notes GROUP BY titre LIMIT 1");
    assert_eq!(result.columns, ["t", "count(*)"]);
    assert_eq!(db.rows("SELECT 1 + 2 * 3, 'a' || 'b', 7 / 2, 7.0 / 2"), ["7|ab|3|3.5"]);
    assert_eq!(db.rows("select TITRE from NOTES where ID = 2"), ["Sport"], "mots et noms insensibles a la casse");
}

#[test]
fn primary_key_is_automatic_and_unique() {
    let mut db = notes();
    db.run("INSERT INTO notes (titre) VALUES ('Suivante')");
    assert_eq!(db.rows("SELECT id FROM notes WHERE titre = 'Suivante'"), ["11"]);
    let err = db.fails("INSERT INTO notes (id, titre) VALUES (2, 'Double')");
    assert!(err.contains("unique"), "{err}");
}

#[test]
fn constraints_and_types_are_checked() {
    let mut db = notes();
    assert!(db.fails("INSERT INTO notes (prio) VALUES (1)").contains("NULL"));
    assert!(db.fails("INSERT INTO notes (titre, prio) VALUES ('x', 'pas un nombre')").contains("INT"));
    assert!(db.fails("INSERT INTO notes (titre, inconnue) VALUES ('x', 1)").contains("inconnue"));
    assert!(db.fails("SELECT titre FROM notes WHERE titre = 3").contains("comparer"));
    db.run("CREATE TABLE u (email TEXT UNIQUE, score FLOAT DEFAULT 1)");
    db.run("INSERT INTO u (email) VALUES ('a@x'), (NULL), (NULL)");
    assert_eq!(db.rows("SELECT score FROM u LIMIT 1"), ["1"], "INT par defaut converti en FLOAT");
    assert!(db.fails("INSERT INTO u VALUES ('a@x', 2)").contains("unique"));
    assert!(db.fails("CREATE TABLE notes (x INT)").contains("existe"));
    db.run("CREATE TABLE IF NOT EXISTS notes (x INT)");
}

#[test]
fn a_failing_statement_changes_nothing() {
    let mut db = notes();
    // La 2e ligne viole la cle primaire : la 1re ne doit pas rester.
    db.fails("INSERT INTO notes (id, titre) VALUES (50, 'ok'), (1, 'double')");
    assert!(db.rows("SELECT id FROM notes WHERE id = 50").is_empty());
    db.fails("UPDATE notes SET id = 1");
    assert_eq!(db.rows("SELECT count(*) FROM notes WHERE id = 1"), ["1"]);
}

#[test]
fn update_and_delete() {
    let mut db = notes();
    assert_eq!(db.run("UPDATE notes SET fait = true, prio = prio + 1 WHERE prio <= 2").affected, 2);
    assert_eq!(db.rows("SELECT titre, prio FROM notes WHERE fait = true ORDER BY titre"), ["Courses|3", "Impots|NULL", "Sport|2"]);
    assert_eq!(db.run("DELETE FROM notes WHERE fait = true").affected, 3);
    assert_eq!(db.rows("SELECT titre FROM notes"), ["Lire"]);
    assert_eq!(db.run("DELETE FROM notes").affected, 1);
}

#[test]
fn where_operators() {
    let mut db = notes();
    assert_eq!(db.rows("SELECT titre FROM notes WHERE titre LIKE 'c%' OR titre LIKE '_mpots'"), ["Courses", "Impots"]);
    assert_eq!(db.rows("SELECT titre FROM notes WHERE prio IN (1, 3) ORDER BY prio"), ["Sport", "Lire"]);
    assert_eq!(db.rows("SELECT titre FROM notes WHERE prio NOT BETWEEN 2 AND 3"), ["Sport"]);
    assert_eq!(db.rows("SELECT titre FROM notes WHERE prio IS NULL"), ["Impots"]);
    assert_eq!(db.rows("SELECT titre FROM notes WHERE NOT (fait OR prio > 1)"), ["Sport"]);
    assert_eq!(db.rows("SELECT titre FROM notes WHERE prio > 1 AND prio < 3"), ["Courses"]);
    // NULL n'est ni egal ni different : jamais retenu par un WHERE.
    assert_eq!(db.rows("SELECT count(*) FROM notes WHERE prio = NULL OR prio <> NULL"), ["0"]);
}

#[test]
fn functions_distinct_and_params() {
    let mut db = notes();
    assert_eq!(db.rows("SELECT upper(titre), lower('ABC'), length(titre), abs(-3), round(2.567, 1), coalesce(prio, 0) FROM notes WHERE id = 10"), ["IMPOTS|abc|6|3|2.6|0"]);
    db.run("INSERT INTO notes (titre, prio) VALUES ('Courses', 2)");
    assert_eq!(db.rows("SELECT DISTINCT titre FROM notes WHERE titre = 'Courses'"), ["Courses"]);
    let r = db.run_with("SELECT titre FROM notes WHERE prio = ? AND titre <> ? ORDER BY id", &[2.into(), "rien".into()]);
    assert_eq!(r.rows.len(), 2);
    // Un parametre n'est jamais du code : pas d'injection.
    let r = db.run_with("SELECT count(*) FROM notes WHERE titre = ?", &["x' OR 1=1 --".into()]);
    assert_eq!(r.rows, [[Value::Int(0)]]);
    assert!(db.fails("SELECT titre FROM notes WHERE id = ?").contains("manquant"));
}

#[test]
fn aggregates_and_group_by() {
    let mut db = Db::new("groups");
    db.run(
        "CREATE TABLE ventes (id INT PRIMARY KEY, vendeur TEXT, montant FLOAT);
         INSERT INTO ventes (vendeur, montant) VALUES ('ana', 10), ('ana', 30), ('bob', 5), ('bob', 5), ('cle', NULL);",
    );
    assert_eq!(db.rows("SELECT count(*), count(montant), sum(montant), avg(montant), min(montant), max(montant) FROM ventes"), ["5|4|50|12.5|5|30"]);
    assert_eq!(
        db.rows("SELECT vendeur, count(*) AS n, sum(montant) AS total FROM ventes GROUP BY vendeur ORDER BY total DESC"),
        ["ana|2|40", "bob|2|10", "cle|1|NULL"]
    );
    assert_eq!(db.rows("SELECT vendeur FROM ventes GROUP BY vendeur HAVING sum(montant) > 20"), ["ana"]);
    assert_eq!(db.rows("SELECT count(DISTINCT montant) FROM ventes"), ["3"]);
    assert_eq!(db.rows("SELECT count(*) FROM ventes WHERE montant > 1000"), ["0"], "agregat sur zero ligne");
    assert!(db.fails("SELECT titre FROM ventes WHERE count(*) > 1").contains("SELECT, HAVING"));
}

#[test]
fn joins() {
    let mut db = Db::new("joins");
    db.run(
        "CREATE TABLE auteurs (id INT PRIMARY KEY, nom TEXT);
         CREATE TABLE livres (id INT PRIMARY KEY, titre TEXT, auteur_id INT);
         INSERT INTO auteurs VALUES (1, 'Hugo'), (2, 'Zola'), (3, 'Sand');
         INSERT INTO livres (titre, auteur_id) VALUES ('Les Miserables', 1), ('Germinal', 2), ('Notre-Dame', 1), ('Orphelin', 99);",
    );
    assert_eq!(
        db.rows("SELECT a.nom, l.titre FROM livres l JOIN auteurs a ON a.id = l.auteur_id ORDER BY l.titre"),
        ["Zola|Germinal", "Hugo|Les Miserables", "Hugo|Notre-Dame"]
    );
    assert_eq!(
        db.rows("SELECT a.nom, count(l.id) AS n FROM auteurs a LEFT JOIN livres l ON l.auteur_id = a.id GROUP BY a.nom ORDER BY n DESC, a.nom"),
        ["Hugo|2", "Zola|1", "Sand|0"]
    );
    assert_eq!(db.rows("SELECT livres.* FROM livres WHERE id = 2"), ["2|Germinal|2"]);
    assert!(db.fails("SELECT id FROM livres JOIN auteurs ON auteurs.id = livres.auteur_id").contains("ambigue"));
}

#[test]
fn indexes() {
    let mut db = notes();
    db.run("CREATE INDEX par_prio ON notes (prio)");
    assert_eq!(db.rows("SELECT titre FROM notes WHERE prio = 3"), ["Lire"]);
    assert!(db.fails("CREATE INDEX par_prio ON notes (titre)").contains("existe"));
    assert!(db.fails("CREATE UNIQUE INDEX titres ON notes (fait)").contains("unique"), "fait a deja des doublons");
    db.run("CREATE UNIQUE INDEX titres ON notes (titre)");
    assert!(db.fails("INSERT INTO notes (titre) VALUES ('Sport')").contains("unique"));
    db.run("DROP INDEX titres");
    db.run("INSERT INTO notes (titre) VALUES ('Sport')");
    assert!(db.fails("DROP INDEX titres").contains("introuvable"));
}

#[test]
fn an_index_makes_lookups_fast() {
    let mut db = Db::new("index-perf");
    db.run("CREATE TABLE mesures (id INT PRIMARY KEY, capteur INT, valeur FLOAT)");
    let values: Vec<String> = (0..20_000).map(|i| format!("({}, {})", i % 500, i)).collect();
    db.run(&format!("INSERT INTO mesures (capteur, valeur) VALUES {}", values.join(", ")));
    let start = std::time::Instant::now();
    for i in 1..=200 {
        assert_eq!(db.run_with("SELECT valeur FROM mesures WHERE id = ?", &[Value::Int(i)]).rows.len(), 1);
    }
    let by_key = start.elapsed();
    assert!(by_key.as_millis() < 500, "200 lectures par cle primaire en {by_key:?}");
}

#[test]
fn transactions_commit_and_rollback() {
    let mut db = notes();
    db.run("BEGIN");
    db.run("INSERT INTO notes (titre) VALUES ('Brouillon')");
    db.run("DELETE FROM notes WHERE id = 1");
    assert_eq!(db.rows("SELECT count(*) FROM notes"), ["4"], "la transaction voit ses propres changements");
    db.run("ROLLBACK");
    assert_eq!(db.rows("SELECT count(*) FROM notes"), ["4"]);
    assert!(db.rows("SELECT id FROM notes WHERE titre = 'Brouillon'").is_empty());

    db.run("BEGIN; UPDATE notes SET prio = 0; CREATE TABLE journal (ligne TEXT); INSERT INTO journal VALUES ('ok'); COMMIT");
    assert_eq!(db.rows("SELECT DISTINCT prio FROM notes"), ["0"]);
    assert_eq!(db.rows("SELECT ligne FROM journal"), ["ok"]);

    // Une erreur dans une transaction annule l'instruction, pas la transaction.
    db.run("BEGIN");
    db.run("INSERT INTO journal VALUES ('garde')");
    db.fails("INSERT INTO notes (id, titre) VALUES (1, 'double')");
    db.run("COMMIT");
    assert_eq!(db.rows("SELECT ligne FROM journal ORDER BY ligne"), ["garde", "ok"]);

    assert!(db.fails("COMMIT").contains("sans BEGIN"));
    db.run("BEGIN");
    assert!(db.fails("BEGIN").contains("deja"));
}

#[test]
fn concurrent_transactions_conflict_instead_of_losing_data() {
    let mut db = notes();
    let mut other = Session::new();
    db.run("BEGIN; UPDATE notes SET prio = 100 WHERE id = 1");
    // Une autre connexion modifie la meme table pendant ce temps.
    execute(&mut db.store, &mut other, 1, "UPDATE notes SET prio = 7 WHERE id = 2", &[], &[]).unwrap();
    assert!(db.fails("COMMIT").contains("Conflit"));
    assert_eq!(db.rows("SELECT prio FROM notes WHERE id IN (1, 2) ORDER BY id"), ["2", "7"]);
}

#[test]
fn tables_survive_a_restart_and_are_encrypted() {
    let path = root("persist");
    {
        let mut store = AzureStockage::open(&path).unwrap();
        execute(&mut store, &mut Session::new(), 1, "CREATE TABLE secrets (nom TEXT); INSERT INTO secrets VALUES ('code-coffre-4521')", &[], &[]).unwrap();
    }
    let mut store = AzureStockage::open(&path).unwrap();
    let r = execute(&mut store, &mut Session::new(), 1, "SELECT nom FROM secrets", &[], &[]).unwrap();
    assert_eq!(r[0].rows, [[Value::Text("code-coffre-4521".into())]]);
    for entry in walk(&path) {
        let data = std::fs::read(&entry).unwrap();
        assert!(!String::from_utf8_lossy(&data).contains("coffre"), "{} contient du clair", entry.display());
        assert!(!String::from_utf8_lossy(&data).contains("secrets"));
    }
    // Chaque app a sa propre base.
    assert!(execute(&mut store, &mut Session::new(), 2, "SELECT nom FROM secrets", &[], &[]).unwrap_err().contains("introuvable"));
}

fn walk(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() { out.extend(walk(&path)) } else { out.push(path) }
    }
    out
}

#[test]
fn show_describe_and_drop() {
    let mut db = notes();
    db.run("SHARE TABLE notes PUBLIC; CREATE TABLE b (x TEXT)");
    assert_eq!(db.rows("SHOW TABLES"), ["b|privee", "notes|publique"]);
    assert_eq!(db.rows("DESCRIBE notes"), ["id|INT|PRIMARY KEY|NULL", "titre|TEXT|NOT NULL|NULL", "fait|BOOL||false", "prio|INT||NULL"]);
    db.run("DROP TABLE b");
    assert_eq!(db.rows("SHOW TABLES"), ["notes|publique"]);
    assert!(db.fails("DROP TABLE b").contains("introuvable"));
    db.run("DROP TABLE IF EXISTS b");
}

#[test]
fn shared_tables_with_accounts_and_roles() {
    let mut db = notes();
    db.store.add_account(1, "maman", "1234", Role::Reader).unwrap();
    db.store.add_account(1, "papa", "abcd", Role::Writer).unwrap();
    let mut guest = Session::new();
    let mut as_app2 = |db: &mut Db, sql: &str, logins: &[Login]| execute(&mut db.store, &mut guest, 2, sql, &[], logins).map(|mut r| r.pop().unwrap_or_default());
    let maman = [Login { owner: 1, user: "maman".into(), password: "1234".into() }];
    let papa = [Login { owner: 1, user: "papa".into(), password: "abcd".into() }];

    let first = as_app2(&mut db, "SELECT titre FROM @1.notes", &[]);
    assert!(first.as_ref().unwrap_err().contains("non partagee"), "{first:?}");

    db.run("SHARE TABLE notes PUBLIC");
    assert_eq!(as_app2(&mut db, "SELECT count(*) FROM @1.notes", &[]).unwrap().rows, [[Value::Int(4)]]);
    assert!(as_app2(&mut db, "DELETE FROM @1.notes", &[]).unwrap_err().contains("refusee"));
    assert!(as_app2(&mut db, "SELECT * FROM @1.notes", &[Login { owner: 1, user: "maman".into(), password: "faux".into() }]).unwrap_err().contains("refuses"));

    db.run("SHARE TABLE notes PROTECTED");
    assert!(as_app2(&mut db, "SELECT * FROM @1.notes", &[]).unwrap_err().contains("protegee"));
    assert_eq!(as_app2(&mut db, "SELECT titre FROM @1.notes WHERE id = 2", &maman).unwrap().rows, [[Value::Text("Sport".into())]]);
    assert!(as_app2(&mut db, "UPDATE @1.notes SET prio = 0", &maman).unwrap_err().contains("lecture seule"));
    assert_eq!(as_app2(&mut db, "UPDATE @1.notes SET prio = 0 WHERE id = 2", &papa).unwrap().affected, 1);
    assert_eq!(db.rows("SELECT prio FROM notes WHERE id = 2"), ["0"]);

    // Jointure entre une table a soi et une table partagee.
    execute(&mut db.store, &mut Session::new(), 2, "CREATE TABLE favoris (note_id INT); INSERT INTO favoris VALUES (2), (3)", &[], &[]).unwrap();
    let r = as_app2(&mut db, "SELECT n.titre FROM favoris f JOIN @1.notes n ON n.id = f.note_id ORDER BY n.titre", &maman).unwrap();
    assert_eq!(r.rows, [[Value::Text("Lire".into())], [Value::Text("Sport".into())]]);

    assert!(as_app2(&mut db, "DROP TABLE notes", &papa).unwrap_err().contains("introuvable"), "l'app 2 n'a pas de table notes a elle");
    db.run("UNSHARE TABLE notes");
    assert!(as_app2(&mut db, "SELECT * FROM @1.notes", &maman).unwrap_err().contains("non partagee"));
}

#[test]
fn tables_follow_the_app_location() {
    let path = root("location");
    let disk = root("location-disque");
    let mut store = AzureStockage::open(&path).unwrap();
    execute(&mut store, &mut Session::new(), 1, "CREATE TABLE t (x INT); INSERT INTO t VALUES (42)", &[], &[]).unwrap();
    store.set_location(1, Some(&disk)).unwrap();
    let mut store = AzureStockage::open(&path).unwrap();
    let r = execute(&mut store, &mut Session::new(), 1, "SELECT x FROM t", &[], &[]).unwrap();
    assert_eq!(r[0].rows, [[Value::Int(42)]]);
    assert!(walk(&disk).iter().any(|p| p.extension().is_some_and(|e| e == "rss")));
}

#[test]
fn syntax_errors_say_where() {
    let mut db = Db::new("syntax");
    let err = db.fails("SELECT * FORM notes");
    assert!(err.contains("ligne 1"), "{err}");
    assert!(db.fails("CREATE TABLE t (x BLOB)").contains("type inconnu"));
    assert!(db.fails("SELECT 'pas ferme").contains("jamais ferme"));
    assert!(db.fails("SELECT * FROM @x.notes").contains("numero d'app"));
    db.run("-- un commentaire\nCREATE TABLE t (x INT) /* et un autre */;");
}
