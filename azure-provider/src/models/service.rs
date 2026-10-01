// Un service surveille par le provider : ce qu'il faut lancer, comment
// savoir qu'il repond, et quoi faire quand il s'arrete.

/// Quoi faire quand le processus s'arrete de lui-meme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restart {
    /// Toujours relancer (un daemon ne doit jamais s'arreter).
    Always,
    /// Relancer seulement s'il sort en erreur (code != 0 ou signal).
    OnFailure,
    /// Ne jamais relancer (tache qui se termine).
    Never,
}

impl Restart {
    pub fn name(self) -> &'static str {
        match self {
            Restart::Always => "always",
            Restart::OnFailure => "on-failure",
            Restart::Never => "never",
        }
    }

    pub fn from_name(name: &str) -> Option<Restart> {
        match name {
            "always" => Some(Restart::Always),
            "on-failure" => Some(Restart::OnFailure),
            "never" => Some(Restart::Never),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceSpec {
    pub name: String,
    /// Executable : un nom (cherche par `find_binary`) ou un chemin.
    pub command: String,
    pub args: Vec<String>,
    /// Socket Unix qui doit accepter une connexion quand le service est
    /// pret. Sans socket, le service est pret des qu'il est lance.
    pub health: Option<String>,
    pub restart: Restart,
    /// Lance au demarrage du provider ; sinon a la premiere demande
    /// (`ensure`, `start`).
    pub autostart: bool,
}

impl ServiceSpec {
    pub fn new(name: &str, command: &str) -> ServiceSpec {
        ServiceSpec { name: name.to_string(), command: command.to_string(), args: Vec::new(), health: None, restart: Restart::Always, autostart: false }
    }

    pub fn arg(mut self, arg: &str) -> ServiceSpec {
        self.args.push(arg.to_string());
        self
    }

    pub fn args<S: AsRef<str>>(mut self, args: &[S]) -> ServiceSpec {
        self.args.extend(args.iter().map(|arg| arg.as_ref().to_string()));
        self
    }

    pub fn health_socket(mut self, socket: &str) -> ServiceSpec {
        self.health = Some(socket.to_string());
        self
    }

    pub fn restart(mut self, restart: Restart) -> ServiceSpec {
        self.restart = restart;
        self
    }

    pub fn autostart(mut self, autostart: bool) -> ServiceSpec {
        self.autostart = autostart;
        self
    }

    /// Nom : lettres minuscules, chiffres, `-` et `_` (il sert aussi de nom
    /// au fichier journal) ; commande non vide.
    pub fn validate(&self) -> Result<(), String> {
        let valid_name = !self.name.is_empty()
            && self.name.len() <= 64
            && self.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
        if !valid_name {
            return Err(format!("Nom de service invalide : '{}' (a-z, 0-9, - et _)", self.name));
        }
        if self.command.trim().is_empty() {
            return Err(format!("Service '{}' : commande vide", self.name));
        }
        Ok(())
    }
}

/// Les daemons d'Azure, surveilles d'office (voir `config` pour les
/// modifier ou les desactiver).
pub fn builtin() -> Vec<ServiceSpec> {
    vec![
        ServiceSpec::new("rooter", "routeur_daemon").health_socket(&azure_core::paths::socket("router")).autostart(true),
        ServiceSpec::new("stockage", "stockage_daemon").health_socket(&azure_core::paths::socket("stockage")).autostart(true),
        ServiceSpec::new("service", "service_daemon").health_socket(&azure_core::paths::socket("service")).autostart(true),
        ServiceSpec::new("manager", "manager_daemon").health_socket(&azure_core::paths::socket("manager")).autostart(true),
    ]
}

pub fn is_builtin(name: &str) -> bool {
    builtin().iter().any(|spec| spec.name == name)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Pas lance (ou arrete a la main).
    Stopped,
    /// Lance, attend que son socket reponde.
    Starting,
    Running,
    /// Son socket repondait deja avant que le provider le lance (daemon
    /// lance a la main) : surveille, relance par le provider s'il tombe.
    External,
    /// A plante, relance prevue apres un delai.
    Backoff,
    /// A plante trop souvent : plus relance jusqu'a un `start` explicite.
    Failed,
    /// Termine normalement (restart `never`/`on-failure`).
    Exited,
}

impl State {
    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<State> {
        [State::Stopped, State::Starting, State::Running, State::External, State::Backoff, State::Failed, State::Exited].get(code as usize).copied()
    }

    pub fn label(self) -> &'static str {
        match self {
            State::Stopped => "arrete",
            State::Starting => "demarrage",
            State::Running => "actif",
            State::External => "actif (externe)",
            State::Backoff => "relance prevue",
            State::Failed => "en echec",
            State::Exited => "termine",
        }
    }

    /// Le service repond : une app peut s'en servir.
    pub fn is_ready(self) -> bool {
        matches!(self, State::Running | State::External)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceStatus {
    pub name: String,
    pub state: State,
    pub pid: Option<u32>,
    /// Nombre de relances depuis le demarrage du provider.
    pub restarts: u32,
    /// Secondes depuis que le service est pret (0 sinon).
    pub uptime_secs: u64,
    /// Derniere raison d'arret ou d'echec.
    pub message: String,
}
