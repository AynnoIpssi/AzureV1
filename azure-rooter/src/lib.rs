pub mod models;
pub mod managers;
pub mod services;

/// Chemin par defaut du routeur INTER-app (voir `managers::router`) - UNE
/// SEULE definition, partagee par `managers::router::start_router` et
/// `services::client::register` (avant, chacun avait sa propre copie
/// litterale du meme chemin, jamais reellement garanties identiques).
/// `start_router_at`/`services::client::register_at` acceptent un AUTRE
/// chemin, notamment pour qu'un test d'integration puisse faire tourner son
/// propre routeur SANS jamais toucher a celui-ci - le confondre avec le
/// chemin reel a deja casse un routeur lance manuellement par un `cargo test`
/// concurrent (le fichier de socket supprime/re-cree en cours de route).
pub static SOCKET_PATH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| azure_core::paths::socket("router"));


/// Dossier des messages en attente : `$XDG_DATA_HOME/azure/rooter` (sinon
/// `~/.local/share/azure/rooter`).
pub fn default_data_dir() -> std::path::PathBuf {
    match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) if !dir.is_empty() => std::path::PathBuf::from(dir).join("azure/rooter"),
        _ => std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share/azure/rooter")).unwrap_or_else(|| "/tmp/azure-rooter".into()),
    }
}
