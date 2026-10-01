// azure-manager : facilite toute la communication entre apps.
//
// - Chaque app se declare dans un manifeste (`app.azure`, voir
//   `models::manifest`) : nom, fenetre, pages, flux partages / ecoutes,
//   taches de fond, stockage.
// - Le daemon (`manager_daemon`) attribue a chaque app un id a partir de
//   son nom, et les apps se designent par leur nom. Il sait qui tourne, qui
//   partage quoi avec qui, et applique les choix du tableau de bord.
// - Le tableau de bord (`azure_dashboard`) et la ligne de commande
//   (`azure_manager`, `azure`) montrent tout cela et le modifient.
pub mod managers;
pub mod models;
pub mod services;

use std::path::PathBuf;

pub static SOCKET_PATH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| azure_core::paths::socket("manager"));

/// Id du manager lui-meme aupres d'azure-service (il y partage le flux
/// `etat` ecoute par le tableau de bord). En dessous de
/// `managers::manager::FIRST_ID`, jamais attribue a une app.
pub const MANAGER_ID: u32 = azure_service::MANAGER_APP_ID;

/// Nom du flux d'etat partage par le manager.
pub const STATE_FLUX: &str = "etat";

/// `$XDG_DATA_HOME/azure/manager` (sinon `~/.local/share/...`).
pub fn default_data_dir() -> PathBuf {
    match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir).join("azure/manager"),
        _ => std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share/azure/manager")).unwrap_or_else(|| PathBuf::from("/tmp/azure-manager")),
    }
}

/// Le tableau de bord et la ligne de commande, a cote du daemon : les seuls
/// autorises a le piloter.
/// Plus le tableau de bord installe comme app (`azure install`).
pub fn default_admins() -> Vec<String> {
    let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(PathBuf::from)) else { return Vec::new() };
    let mut admins: Vec<String> = ["azure_dashboard", "azure_manager", "azure"].iter().map(|name| dir.join(name).to_string_lossy().into_owned()).collect();
    admins.push(azure_provider::install_root().join("apps/dashboard/azure_dashboard").to_string_lossy().into_owned());
    admins
}

pub fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|arg| arg == name).and_then(|i| args.get(i + 1)).cloned()
}

/// Toutes les valeurs de `--nom valeur` (option repetable).
pub fn arg_values(args: &[String], name: &str) -> Vec<String> {
    args.windows(2).filter(|pair| pair[0] == name).map(|pair| pair[1].clone()).collect()
}
