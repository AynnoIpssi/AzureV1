// Pendant INTRA-app de `NavigationClient` (inter-app, voir
// `navigation_client.rs`) : aucune socket ici, juste une poignee partagee
// vers l'`IntraRouter` d'azure-rooter (100% en memoire, voir sa
// documentation) et l'id de CETTE vue au sein du process. Construit par
// `managers::intra_navigation_manager::connect`.
use azure_rooter::managers::intra_router::IntraRouter;

pub struct IntraClient {
    pub view_id: u32,
    pub(crate) router: IntraRouter,
}
