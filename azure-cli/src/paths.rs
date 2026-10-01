// Ou tout est installe (dossiers XDG, voir la doc de `lib.rs`).
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Paths {
    /// `~/.local/share/azure`
    pub root: PathBuf,
    /// `~/.local/share/applications`
    pub applications: PathBuf,
    /// `~/.config/systemd/user`
    pub systemd: PathBuf,
    /// `~/.local/state/azure/apps`
    pub logs: PathBuf,
    /// `~/.local/bin`
    pub local_bin: PathBuf,
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    match std::env::var_os(var) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => home().join(fallback),
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

impl Paths {
    pub fn from_env() -> Paths {
        let data = xdg("XDG_DATA_HOME", ".local/share");
        Paths {
            root: azure_provider::install_root(),
            applications: data.join("applications"),
            systemd: xdg("XDG_CONFIG_HOME", ".config").join("systemd/user"),
            logs: xdg("XDG_STATE_HOME", ".local/state").join("azure/apps"),
            local_bin: home().join(".local/bin"),
        }
    }

    pub fn bin(&self) -> PathBuf {
        self.root.join("bin")
    }

    pub fn apps(&self) -> PathBuf {
        self.root.join("apps")
    }

    pub fn app(&self, name: &str) -> PathBuf {
        self.apps().join(name)
    }

    /// Les taches de fond d'une app installee, pour azure-provider (voir
    /// `azure_provider::services_dir`).
    pub fn services(&self, name: &str) -> PathBuf {
        self.root.join("services").join(format!("{name}.conf"))
    }

    pub fn desktop(&self, name: &str) -> PathBuf {
        self.applications.join(format!("azure-{name}.desktop"))
    }
}
