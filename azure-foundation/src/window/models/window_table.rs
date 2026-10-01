// Meme principe que `navigation::models::route_table::RouteTable`, sauf que
// l'expression finale resolue est une `AzureWindow` ENTIERE (pas encore
// lancee, voir `AzureWindow::run`) plutot qu'un arbre de `UiNode` - c'est
// l'"expression routeur" pour ouvrir des fenetres depuis une fenetre (voir
// `window_context::WindowContext::open_window`). Purement locale, comme
// `RouteTable` : ne parle jamais elle-meme a azure-rooter ni ne cree de
// thread - c'est `open_window` qui s'en charge une fois la fenetre resolue.
use crate::navigation::models::router::Router;
use crate::window::models::window::AzureWindow;

/// Meme syntaxe facon Laravel que `RouteTable` (voir `router::Router`) :
/// `.on(path, |payload| ...)`, `.route("/note/{id}", |r| ...)`, `.name()`,
/// `.group()`... Les handlers ne doivent jamais `.run()` la fenetre : c'est
/// `open_window` qui s'en charge, dans son propre thread (d'ou le `+ Send`).
pub type WindowTable = Router<AzureWindow>;

impl Router<AzureWindow> {
    /// La nouvelle `AzureWindow` (pas encore lancee) pour `path`/`payload`,
    /// ou `None` si aucune route ne correspond - a l'appelant (voir
    /// `open_window`) de decider quoi faire d'un `path` inconnu.
    pub fn resolve(&self, path: &str, payload: &str) -> Option<AzureWindow> {
        self.dispatch(path, payload)
    }
}
