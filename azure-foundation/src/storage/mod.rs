// Pont vers azure-stockage (le daemon de stockage) : des expressions courtes
// pour les apps, comme les routes (voir `navigation::models::router`).
//
//     let store = Stockage::connect(10)?;
//     store.set("theme", "sombre")?;
//     let clics = store.update("clics", 0, |n: i64| n + 1)?;
//     let theme = store.get_or("theme", "clair".to_string());
//
//     store.share("journal")
//         .value("cher journal")
//         .protected()
//         .account("maman", "1234").reader()
//         .account("papa", "abcd").writer()
//         .save()?;
//
//     let recette: String = store.from(10).get("recette")?;
//     store.from(10).login("papa", "abcd").set("journal", "nouveau")?;
//
//     // RsS, le SQL d'Azure (voir `models::db`) :
//     let db = store.db();
//     db.run("INSERT INTO notes (titre) VALUES (?)", &params!["Courses"])?;
//     let titres = db.query("SELECT titre FROM notes ORDER BY id", &[])?.column::<String>("titre")?;
//
//     store.move_to("/mnt/disque/notes")?;   // ou vivent les donnees
pub mod models;

// Raccourcis : `use azure_foundation::storage::{Stockage, Role};`.
pub use azure_core::models::storage_model::{Role, ShareAccess};
pub use models::admin::{Admin, AdminApp, AdminTable};
pub use models::db::{Db, FromValue, Row, Rows};
pub use models::stockage::Stockage;
pub use azure_stockage::rss::value::Value;
