pub mod window;
pub mod layout;
pub mod compiler;
pub mod ui;
pub mod style;
pub mod event;
pub mod navigation;
pub mod cursor;
pub mod storage;
pub mod provider;
pub mod flux;
pub mod app;
pub mod perf;
pub mod inspector;
pub mod selecteur;
pub mod essai;

/// La librairie des modules d'Azure (`libraire::interface` : modules du
/// front et themes).
pub use azure_libraire as libraire;
/// Les themes : `theme::choisir("ivoire")`, `theme::definir(Theme::fichier("ui/app.theme")?)`.
pub use azure_libraire::interface::theme;
