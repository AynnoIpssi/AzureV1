use std::os::unix::net::UnixStream;

/// Poignee vers le routeur azure-rooter une fois cette app enregistree -
/// obtenue via `navigation::managers::navigation_manager::connect`, jamais
/// construite a la main (`connection` doit correspondre a un `register`
/// reellement envoye, sinon le routeur ignorera tout send/subscribe/publish
/// venant de cet identifiant).
///
/// `router_id`, pas `app_id` : ce champ identifie cette app aupres du
/// routeur INTER-app (un `u32`, voir `azure_rooter::managers::router`) - a
/// ne pas confondre avec `window::models::window::AzureWindow::app_id`
/// (une `String`, l'identifiant xdg-shell transmis au compositeur pour la
/// barre des taches/le dock), un concept totalement different malgre le nom
/// proche. Les deux se cotoient dans `AzureWindow`/`WindowContext` sans
/// aucun rapport entre eux.
pub struct NavigationClient {
    pub router_id: u32,
    pub(crate) connection: UnixStream,
}
