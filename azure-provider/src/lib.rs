// azure-provider : s'assure que tous les processus d'arriere-plan d'Azure
// tournent. Il lance les daemons (routeur, stockage) et les services
// declares (config ou enregistres par une app), verifie qu'ils repondent,
// et les relance avec un delai croissant quand ils plantent.
//
// Une app n'a rien a faire : `ensure("stockage")` (appele par
// `Stockage::connect` et la navigation d'azure-foundation) lance le
// provider s'il ne tourne pas, puis attend que le service reponde.
pub mod managers;
pub mod models;
pub mod services;

pub use models::service::{Restart, ServiceSpec, ServiceStatus, State};
pub use services::client::{Provider, ProviderClient, Service};

use std::path::PathBuf;

pub static SOCKET_PATH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| azure_core::paths::socket("provider"));

/// Nom du binaire du provider (voir `find_binary`, lance par les apps).
pub const BINARY: &str = "azure_provider";

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn xdg(var: &str, fallback: &str) -> Option<PathBuf> {
    match std::env::var_os(var) {
        Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
        _ => home().map(|home| home.join(fallback)),
    }
}

/// `$XDG_CONFIG_HOME/azure/provider.conf` (sinon `~/.config/...`).
pub fn config_path() -> Option<PathBuf> {
    xdg("XDG_CONFIG_HOME", ".config").map(|dir| dir.join("azure/provider.conf"))
}

/// Dossier des journaux des services : `$XDG_STATE_HOME/azure/provider`
/// (sinon `~/.local/state/...`, et `/tmp/azure-provider` sans `HOME`).
pub fn log_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").map(|dir| dir.join("azure/provider")).unwrap_or_else(|| PathBuf::from("/tmp/azure-provider"))
}

/// Valeur de `--nom valeur` dans les arguments.
pub fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|arg| arg == name).and_then(|i| args.get(i + 1)).cloned()
}

/// Racine d'une installation d'Azure : `$XDG_DATA_HOME/azure` (sinon
/// `~/.local/share/azure`). Les daemons y sont dans `bin/`, les apps dans
/// `apps/<nom>/` (voir la commande `azure`).
pub fn install_root() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").map(|dir| dir.join("azure")).unwrap_or_else(|| PathBuf::from("/tmp/azure"))
}

/// Les taches de fond des apps installees, une fois par app
/// (`<app>.conf`) : chargees par le provider a son demarrage.
pub fn services_dir() -> PathBuf {
    install_root().join("services")
}

/// Cherche un executable : d'abord a cote de l'executable courant (et un
/// dossier au-dessus : `target/debug/examples/`, `target/debug/deps/`), ou
/// cargo range tous les binaires du workspace, puis dans les binaires
/// installes (`install_root()/bin`), puis dans le `PATH`. Un nom qui
/// contient `/` est pris tel quel.
pub fn find_binary(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        return Some(PathBuf::from(name));
    }
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(PathBuf::from)) {
        dirs.extend(dir.parent().map(PathBuf::from));
        dirs.insert(0, dir);
    }
    dirs.push(install_root().join("bin"));
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    dirs.into_iter().map(|dir| dir.join(name)).find(|file| is_executable(file))
}

fn is_executable(file: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    file.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}
