// azure-service : les taches complexes reutilisables d'Azure, servies par
// un daemon (`service_daemon`, surveille par azure-provider).
//
// - Le **flux** (`flux`) : une app partage en temps reel les modifications
//   d'un etat (et des evenements) ; les apps autorisees l'ecoutent,
//   recoivent d'abord l'etat actuel puis chaque modification.
// - Les **appels** (`call`) : une app sert des methodes, les apps
//   autorisees les appellent et attendent la reponse.
/// Id d'azure-manager aupres d'azure-service. Reserve : seul l'executable
/// d'azure-manager connu du daemon (`--manager`) peut le prendre, il n'est
/// jamais lie au premier executable venu (voir `managers::daemon`).
pub const MANAGER_APP_ID: u32 = 999;

pub mod call;
pub mod flux;
pub mod managers;

use std::path::PathBuf;

pub static SOCKET_PATH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| azure_core::paths::socket("service"));

/// Dossier du daemon (registre des apps) : `$XDG_DATA_HOME/azure/service`
/// (sinon `~/.local/share/...`, et `/tmp/azure-service` sans `HOME`).
pub fn default_data_dir() -> PathBuf {
    match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join("azure/service"),
        _ => std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share/azure/service")).unwrap_or_else(|| PathBuf::from("/tmp/azure-service")),
    }
}

/// Valeur de `--nom valeur` dans les arguments.
pub fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|arg| arg == name).and_then(|i| args.get(i + 1)).cloned()
}
