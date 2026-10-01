pub mod crypto;
pub mod managers;
pub mod models;
pub mod rss;
pub mod services;

/// Socket par defaut du daemon de stockage (voir `managers::daemon`).
/// `start_daemon_at` / `services::client::connect_at` en acceptent un autre,
/// pour que les tests ne touchent jamais au vrai daemon.
pub static SOCKET_PATH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| azure_core::paths::socket("stockage"));

/// Le dossier du stockage, dans cet ordre :
/// 1. l'argument `--root <dossier>` (ou `--root=<dossier>`) du daemon ;
/// 2. la variable d'environnement `AZURE_STOCKAGE_ROOT` ;
/// 3. la ligne `root = <dossier>` du fichier de config
///    (`$XDG_CONFIG_HOME/azure/stockage.conf`, sinon `~/.config/azure/stockage.conf`) ;
/// 4. `default_root()`.
pub fn resolve_root(args: &[String]) -> Result<std::path::PathBuf, String> {
    let config = config_path().and_then(|path| std::fs::read_to_string(path).ok());
    resolve_root_from(args, std::env::var("AZURE_STOCKAGE_ROOT").ok(), config.as_deref())
}

/// `resolve_root` sans lire l'environnement ni le disque (testable).
pub fn resolve_root_from(args: &[String], env: Option<String>, config: Option<&str>) -> Result<std::path::PathBuf, String> {
    if let Some(root) = arg_value(args, "--root") {
        return Ok(root.into());
    }
    if let Some(root) = env.filter(|r| !r.is_empty()) {
        return Ok(root.into());
    }
    if let Some(root) = config.and_then(|text| config_value(text, "root")) {
        return Ok(root.into());
    }
    default_root()
}

/// La valeur de `--nom <valeur>` ou `--nom=<valeur>` dans `args`.
pub fn arg_value(args: &[String], name: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == name {
            return iter.next().cloned();
        }
        if let Some(value) = arg.strip_prefix(name).and_then(|rest| rest.strip_prefix('=')) {
            return Some(value.to_string());
        }
    }
    None
}

// Ligne `cle = valeur` d'un fichier de config (`#` = commentaire).
fn config_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter_map(|line| line.split_once('='))
        .find(|(k, _)| k.trim() == key)
        .map(|(_, v)| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
}

fn config_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|d| !d.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    Some(base.join("azure/stockage.conf"))
}

/// Dossier par defaut ou le daemon garde tout le stockage :
/// `$XDG_DATA_HOME/azure/stockage`, sinon `~/.local/share/azure/stockage`.
pub fn default_root() -> Result<std::path::PathBuf, String> {
    if let Some(data) = std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
        return Ok(std::path::PathBuf::from(data).join("azure/stockage"));
    }
    let home = std::env::var_os("HOME").ok_or("HOME n'est pas defini")?;
    Ok(std::path::PathBuf::from(home).join(".local/share/azure/stockage"))
}
