// Les modules du front : tout ce qu'une page rsH appelle pour se
// construire (`<card>`, `<sidebar>`, `<modal>`...).
//
// - Un module = deux fichiers dans `modules/<categorie>/` : `<nom>.rsh`
//   (son modele) et `<nom>.rsc` (ses styles), declares dans `catalogue.rs`.
// - Les styles n'ecrivent aucune couleur en dur : ils nomment des jetons
//   (`$surface`, `$accent/14`, `$rayon`) que le theme remplace. Un theme
//   de plus = un fichier de plus dans `themes/` (voir `theme.rs`).
//
// Reference : INTERFACE.md.
pub mod catalogue;
pub mod theme;

pub use catalogue::{categories, module, modules, styles, Module};
pub use theme::Theme;
