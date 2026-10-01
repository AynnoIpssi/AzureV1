// Pont vers azure-rooter (le routeur IPC inter-apps, voir
// `azure_rooter::managers::router`) : les apps foundation passent par ici
// (`managers::navigation_manager` + `models::navigation_client`) plutot que
// de deposer `use azure_rooter::...` directement dans leur propre code, comme
// le faisaient jusqu'ici `azure-test/app-test/app-{a,b,c}`.
pub mod managers;
pub mod models;

/// Routeur interne a l'app (pages, sans passer par le daemon) : ici pour que
/// l'app n'ait pas a dependre d'azure-rooter.
pub use azure_rooter::managers::intra_router::IntraRouter;
pub use azure_core::rules::root_provider::AzureRouterProvider;
