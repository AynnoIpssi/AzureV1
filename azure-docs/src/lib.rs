// Azure Docs : la documentation d'Azure, et de quoi y chercher.
//
// - `apercu`  : le rendu en direct des exemples rsC ;
// - `contenu` : lit les pages (`contenu/<section>/<page>.page`) ;
// - `index`   : recherche dans les pages et les exemples de code (chacun a
//   un identifiant stable, `rsc.flex.1`) - utilisable hors de l'app, par
//   l'IDE par exemple ;
// - `ecrans`  : les donnees des vues (ui/docs.rsh) et les clics ;
// - `demos`   : les fenetres ouvertes par les blocs `> demo` (ui/demos/) ;
// - `service` : ce que l'app sert aux autres (recherche, pages).
pub mod apercu;
pub mod coloration;
pub mod contenu;
pub mod demos;
pub mod ecrans;
pub mod index;
pub mod service;

pub use contenu::{Docs, Exemple, Page};
pub use index::{chercher, Resultat};
