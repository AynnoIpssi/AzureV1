// Pendant INTRA-app de `navigation_manager` (inter-app, voir celui-ci) :
// memes noms/forme (`connect`, `navigate`, sonder pour recevoir), mais
// transporte par `azure_rooter::managers::intra_router::IntraRouter`
// (100% en memoire, voir sa documentation) plutot que par une socket Unix -
// ce fichier n'importe JAMAIS `azure_rooter::services::client` ni
// `azure_rooter::managers::router` (le routeur inter-app), exactement comme
// `intra_router.rs` n'importe jamais ce dernier cote azure-rooter.
use crate::navigation::models::intra_client::IntraClient;
use crate::navigation::models::route::Route;
use crate::navigation::models::route_table::RouteTable;
use crate::ui::models::ui_node::UiNode;
use azure_core::rules::root_provider::AzureRouterProvider;
use azure_rooter::managers::intra_router::IntraRouter;
use azure_rooter::models::intra_route::IntraRoute;

/// Enregistre la vue `view_id` aupres de `router` (une poignee CLONEE - voir
/// `IntraRouter::Clone`, l'etat interne reste partage avec l'original) et
/// retourne la poignee a reutiliser pour tous les appels suivants. `router`
/// est typiquement construit une seule fois par process (`IntraRouter::new`)
/// et partage entre toutes les vues qui doivent se parler localement.
pub fn connect(router: &IntraRouter, view_id: u32) -> IntraClient {
    let mut router = router.clone();
    router.register(view_id);
    IntraClient { view_id, router }
}

/// Envoie une demande de navigation vers l'ecran `path` de la vue
/// `target_view_id` - une AUTRE vue du meme process, OU `intra.view_id`
/// lui-meme pour une navigation purement locale a cette fenetre (voir
/// `window_context::WindowContext::goto`). Depose juste le message dans la
/// boite ciblee : sans effet visible avant le prochain `poll`.
pub fn navigate(intra: &IntraClient, target_view_id: u32, path: &str, payload: &str) {
    intra.router.navigate(intra.view_id, target_view_id, path, payload);
}

/// Sondage NON bloquant (contrairement a `navigation_manager::listen`, pas
/// de thread ni de canal a demarrer - voir `IntraRouter::receive`) : `Some(nodes)`
/// si un message attendait pour `intra.view_id` ET que son chemin est
/// enregistre dans `routes`, `None` dans tous les autres cas - y compris
/// quand un message attendait mais que son chemin, lui, n'est pas
/// enregistre.
///
/// A NE PAS UTILISER depuis une boucle d'evenements reelle : `poll` ne
/// retire QU'UN SEUL message de la boite, et retourne `None` des que ce
/// message ne resout dans aucune route enregistree - meme si un AUTRE
/// message, plus recent, attend juste derriere et resoudrait correctement.
/// `AzureWindow::run` (le bras `on_tick`) n'appelle jamais cette fonction
/// pour cette raison, seulement `drain` ci-dessous, qui vide la boite
/// entiere et ne perd que les messages perimes/inconnus. `poll` ne reste
/// public que pour des tests unitaires ou un seul message est en jeu (voir
/// `mod tests` ci-dessous) - preferer `drain` partout ailleurs.
pub fn poll(intra: &IntraClient, routes: &RouteTable) -> Option<Vec<UiNode>> {
    let raw = intra.router.receive(intra.view_id)?;
    let decoded = IntraRoute::decode(&raw);
    routes.resolve(&Route::new(&decoded.path, &decoded.payload))
}

/// Retire TOUS les messages actuellement en attente pour `intra.view_id`
/// (pas seulement jusqu'au premier qui ne resout pas, contrairement a une
/// boucle naive sur `poll`) et retourne le DERNIER resolu avec succes - le
/// reste (routes perimees ou vers un chemin inconnu) est simplement
/// consomme sans effet. A appeler depuis une boucle deja existante (voir
/// `AzureWindow::run`, le bras `on_tick`), pas dans un thread dedie.
pub fn drain(intra: &IntraClient, routes: &RouteTable) -> Option<Vec<UiNode>> {
    let mut resolved = None;
    while let Some(raw) = intra.router.receive(intra.view_id) {
        let decoded = IntraRoute::decode(&raw);
        if let Some(nodes) = routes.resolve(&Route::new(&decoded.path, &decoded.payload)) {
            resolved = Some(nodes);
        }
    }
    resolved
}
