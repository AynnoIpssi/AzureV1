// Le tableau de bord d'Azure : pages (`routes`) et clics (`action`), sans
// fenetre, pour pouvoir les tester. `main` les branche sur une vraie
// fenetre, l'etat venant du flux `etat` d'azure-manager.
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::flux::{to_rsh, Value};
use azure_foundation::navigation::models::route_table::RouteTable;
pub use azure_manager::managers::manager::Kind;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Lit les dernieres lignes du journal d'un service ou d'une app.
pub type LogReader = Arc<dyn Fn(&str) -> Result<Vec<String>, String> + Send + Sync>;

/// Lance `azure <arguments>` (par azure-manager) : code de sortie, lignes.
pub type CommandRunner = Arc<dyn Fn(&[String]) -> Result<(i32, Vec<String>), String> + Send + Sync>;

/// L'etat affiche (dernier recu d'azure-manager), la page ouverte, de quoi
/// lire les journaux et lancer les commandes du terminal.
#[derive(Clone, Default)]
pub struct Dashboard {
    pub state: Arc<Mutex<Value>>,
    pub current: Arc<Mutex<String>>,
    pub logs: Option<LogReader>,
    /// Derniere action refusee (affichee en haut de la page), vide sinon.
    pub error: Arc<Mutex<String>>,
    pub runner: Option<CommandRunner>,
    /// Ce que montre le terminal : `{texte, genre}`, genre `cmd`, `out`,
    /// `err` ou `ok`.
    pub terminal: Arc<Mutex<Vec<Value>>>,
    /// La commande qui tourne (voir `start`), sinon `None`.
    pub running: Arc<Mutex<Option<String>>>,
    /// Le terminal a change depuis le dernier affichage.
    pub refresh: Arc<AtomicBool>,
    /// Ce qui est tape dans le champ, garde quand la page est redessinee.
    pub draft: Arc<Mutex<String>>,
    /// La derniere compilation lancee depuis la page d'une app (voir
    /// `start_build`).
    pub build: Arc<Mutex<Option<Build>>>,
}

/// Lignes de la sortie gardees pour la page de l'app (le terminal a tout).
pub const BUILD_LINES: usize = 12;

/// Une compilation lancee depuis la page d'une app.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Build {
    pub app: String,
    /// `en_cours`, `ok` ou `echec`.
    pub state: String,
    /// La fin de la sortie.
    pub lines: Vec<String>,
}

impl Dashboard {
    pub fn state(&self) -> Value {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn current(&self) -> String {
        self.current.lock().map(|c| c.clone()).unwrap_or_else(|_| "/".to_string())
    }

    /// Affiche `error` en haut de la page (vide : rien).
    pub fn set_error(&self, error: &str) {
        if let Ok(mut e) = self.error.lock() {
            *e = error.to_string();
        }
    }

    /// Les lignes du terminal.
    pub fn terminal(&self) -> Vec<Value> {
        self.terminal.lock().map(|t| t.clone()).unwrap_or_default()
    }

    fn print(&self, genre: &str, text: &str) {
        if let Ok(mut t) = self.terminal.lock() {
            t.push(Value::map([("texte", Value::Text(text.to_string())), ("genre", Value::Text(genre.to_string()))]));
            // Les plus anciennes s'en vont.
            let excess = t.len().saturating_sub(TERMINAL_LINES);
            t.drain(..excess);
        }
        self.refresh.store(true, Ordering::SeqCst);
    }

    /// La commande en cours, ou vide.
    pub fn running(&self) -> String {
        self.running.lock().ok().and_then(|r| r.clone()).unwrap_or_default()
    }

    /// Le terminal a-t-il change depuis le dernier appel ?
    pub fn take_refresh(&self) -> bool {
        self.refresh.swap(false, Ordering::SeqCst)
    }

    /// Comme `execute`, mais la commande `azure` tourne dans un thread a
    /// part (une compilation prend du temps) : la fenetre reste utilisable,
    /// et `take_refresh` dit quand redessiner. Une seule a la fois.
    pub fn start(&self, line: &str) {
        if !self.running().is_empty() {
            return self.print("err", &format!("« {} » tourne encore : attendez qu'elle finisse.", self.running()));
        }
        let Some(args) = self.prepare(line) else { return };
        if let Ok(mut r) = self.running.lock() {
            *r = Some(format!("azure {}", args.join(" ")));
        }
        let dashboard = self.clone();
        std::thread::spawn(move || {
            let _ = dashboard.run_remote(&args);
            if let Ok(mut r) = dashboard.running.lock() {
                *r = None;
            }
            dashboard.refresh.store(true, Ordering::SeqCst);
        });
    }

    /// Compile et installe l'app `name` (`azure build <name> --installer`)
    /// dans un thread a part, comme `start` ; la sortie va aussi au
    /// terminal. Erreur si une commande tourne deja.
    pub fn start_build(&self, name: &str) -> Result<(), String> {
        let running = self.running();
        if !running.is_empty() {
            return Err(format!("« {running} » tourne encore : attendez qu'elle finisse"));
        }
        let args: Vec<String> = ["build", name, "--installer"].map(String::from).to_vec();
        let line = format!("azure {}", args.join(" "));
        if let Ok(mut r) = self.running.lock() {
            *r = Some(line.clone());
        }
        self.set_build(Build { app: name.to_string(), state: "en_cours".to_string(), lines: Vec::new() });
        self.print("cmd", &format!("$ {line}"));
        let dashboard = self.clone();
        let name = name.to_string();
        std::thread::spawn(move || {
            let (ok, mut lines) = match dashboard.run_remote(&args) {
                Some((code, lines)) => (code == 0, lines),
                None => (false, vec!["commande impossible (voir le terminal)".to_string()]),
            };
            let excess = lines.len().saturating_sub(BUILD_LINES);
            lines.drain(..excess);
            dashboard.set_build(Build { app: name, state: if ok { "ok" } else { "echec" }.to_string(), lines });
            if let Ok(mut r) = dashboard.running.lock() {
                *r = None;
            }
            dashboard.refresh.store(true, Ordering::SeqCst);
        });
        Ok(())
    }

    fn set_build(&self, build: Build) {
        if let Ok(mut b) = self.build.lock() {
            *b = Some(build);
        }
    }

    /// La derniere compilation de `name`, s'il y en a une.
    pub fn build_of(&self, name: &str) -> Option<Build> {
        self.build.lock().ok()?.clone().filter(|b| b.app == name)
    }

    /// Execute une ligne tapee dans le terminal : `effacer`, `aide`, ou une
    /// commande `azure` (le mot `azure` devant est facultatif).
    pub fn execute(&self, line: &str) {
        if let Some(args) = self.prepare(line) {
            let _ = self.run_remote(&args);
        }
    }

    /// Traite les commandes locales ; pour une commande `azure`, l'affiche
    /// et rend ses arguments.
    fn prepare(&self, line: &str) -> Option<Vec<String>> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        let args = match split_command(line) {
            Ok(args) => args,
            Err(e) => {
                self.print("err", &e);
                return None;
            }
        };
        let args: Vec<String> = match args.first().map(String::as_str) {
            Some("azure") => args[1..].to_vec(),
            _ => args,
        };
        match args.first().map(String::as_str) {
            Some("effacer" | "clear") => {
                if let Ok(mut t) = self.terminal.lock() {
                    t.clear();
                }
                self.refresh.store(true, Ordering::SeqCst);
                return None;
            }
            Some("aide" | "help") | None => {
                self.print("cmd", &format!("$ {line}"));
                for l in HELP {
                    self.print("out", l);
                }
                return None;
            }
            _ => {}
        }
        self.print("cmd", &format!("$ azure {}", args.join(" ")));
        Some(args)
    }

    /// Lance `azure <args>` et affiche sa sortie ; rend code et lignes
    /// (`None` : pas lancee).
    fn run_remote(&self, args: &[String]) -> Option<(i32, Vec<String>)> {
        let Some(run) = &self.runner else {
            self.print("err", "terminal indisponible (azure-manager injoignable)");
            return None;
        };
        match run(args) {
            Ok((code, lines)) => {
                let genre = if code == 0 { "out" } else { "err" };
                for l in &lines {
                    self.print(genre, l);
                }
                if code == 0 {
                    self.print("ok", "terminé");
                } else {
                    self.print("err", &format!("échec (code {code})"));
                }
                Some((code, lines))
            }
            Err(e) => {
                self.print("err", &e);
                None
            }
        }
    }

    fn data(&self, page: &str, path: &str) -> Context {
        if let Ok(mut current) = self.current.lock() {
            *current = path.to_string();
        }
        let error = self.error.lock().map(|e| e.clone()).unwrap_or_default();
        Context::new().with_value("etat", to_rsh(&self.state())).with_text("page", page).with_text("erreur", &error)
    }
}

/// Lignes gardees par le terminal.
pub const TERMINAL_LINES: usize = 500;

const HELP: [&str; 10] = [
    "new <nom> [--titre <titre>] [--dans <dossier>] — crée une app vide (dans le projet Azure par défaut)",
    "build <nom | dossier> [--installer] — la compile (cargo build --release), puis l'installe avec --installer",
    "install <dossier> [--bin <exe>] — installe l'app dont app.azure est dans <dossier>",
    "uninstall <app> — la désinstalle (son stockage est gardé)",
    "list — les apps installées",
    "run <app> — lance une app installée",
    "autostart on | off | status — démarrer Azure à l'ouverture de session",
    "effacer — vide le terminal",
    "Le mot « azure » devant est facultatif ; ~ = dossier personnel.",
    "Exemple : new meteo, puis build meteo --installer, puis run meteo",
];

/// Coupe une ligne en arguments : espaces, guillemets simples ou doubles,
/// `\` pour garder le caractere suivant.
pub fn split_command(line: &str) -> Result<Vec<String>, String> {
    let (mut args, mut current, mut quote, mut started) = (Vec::new(), String::new(), None, false);
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match (c, quote) {
            ('\\', q) if q != Some('\'') => current.push(chars.next().ok_or("\\ en fin de ligne")?),
            (c, Some(q)) if c == q => quote = None,
            (c, Some(_)) => current.push(c),
            ('"' | '\'', None) => quote = Some(c),
            (c, None) if c.is_whitespace() => {
                if started || !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
                started = false;
                continue;
            }
            (c, None) => current.push(c),
        }
        started = true;
    }
    if quote.is_some() {
        return Err("guillemet non ferme".to_string());
    }
    if started || !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

/// L'app `name` dans l'etat.
pub fn find_app<'a>(state: &'a Value, name: &str) -> Option<&'a Value> {
    state.get("apps")?.as_list()?.iter().find(|app| app.get("nom").and_then(Value::as_str) == Some(name))
}

/// Les pages : `/` (apps), `/app/{nom}`, `/liens`, `/services`.
pub fn routes(ui_dir: &Path, dashboard: &Dashboard) -> RouteTable {
    let rsh = ui_dir.join("tableau.rsh").to_string_lossy().into_owned();
    let rsc = ui_dir.join("tableau.rsc").to_string_lossy().into_owned();
    let page = |name: &'static str| {
        let dashboard = dashboard.clone();
        move |request: &azure_foundation::navigation::models::router::Request| dashboard.data(name, &request.path)
    };
    let detail = {
        let dashboard = dashboard.clone();
        move |request: &azure_foundation::navigation::models::router::Request| {
            // Payload `oublier` : demander confirmation (voir `Action::AskForget`).
            let ctx = dashboard.data("app", &request.path).with_text("confirmer_oubli", if request.payload == "oublier" { "oui" } else { "non" });
            let state = dashboard.state();
            let name = request.param("nom").unwrap_or("");
            let build = dashboard.build_of(name).unwrap_or_default();
            let lines = azure_foundation::compiler::services::condition::ConditionValue::List(build.lines.into_iter().map(Into::into).collect());
            let ctx = ctx.with_text("compilation", &build.state).with_value("compilation_lignes", lines).with_text("en_cours", &dashboard.running());
            match find_app(&state, name) {
                Some(app) => ctx.with_value("app", to_rsh(app)),
                // App oubliee entre-temps : retour a la liste.
                None => ctx.with_text("page", "apps"),
            }
        }
    };
    let journal = {
        let dashboard = dashboard.clone();
        move |request: &azure_foundation::navigation::models::router::Request| {
            let name = request.param("nom").unwrap_or("").to_string();
            let ctx = dashboard.data("journal", &request.path).with_text("journal", &name);
            let lines = match &dashboard.logs {
                Some(read) => read(&name),
                None => Err("journaux indisponibles".to_string()),
            };
            match lines {
                Ok(lines) if lines.is_empty() => ctx.with_text("erreur_journal", "Journal vide.").with_value("lignes", azure_foundation::compiler::services::condition::ConditionValue::List(Vec::new())),
                Ok(lines) => ctx.with_text("erreur_journal", "").with_value("lignes", azure_foundation::compiler::services::condition::ConditionValue::List(lines.into_iter().map(Into::into).collect())),
                Err(e) => ctx.with_text("erreur_journal", &format!("Pas de journal : {e}")).with_value("lignes", azure_foundation::compiler::services::condition::ConditionValue::List(Vec::new())),
            }
        }
    };
    let terminal = {
        let dashboard = dashboard.clone();
        move |request: &azure_foundation::navigation::models::router::Request| {
            let lines = Value::list(dashboard.terminal());
            let count = dashboard.terminal.lock().map(|t| t.len()).unwrap_or(0);
            let draft = dashboard.draft.lock().map(|d| d.clone()).unwrap_or_default();
            dashboard.data("terminal", &request.path).with_value("terminal", to_rsh(&lines)).with_text("en_cours", &dashboard.running()).with_text("brouillon", &draft).with_value("nb_terminal", azure_foundation::compiler::services::condition::ConditionValue::Number(count as f64))
        }
    };
    RouteTable::new()
        .view_with("/", &rsh, &rsc, page("apps"))
        .view_with("/terminal", &rsh, &rsc, terminal)
        .view_with("/liens", &rsh, &rsc, page("liens"))
        .view_with("/services", &rsh, &rsc, page("services"))
        .view_with("/evenements", &rsh, &rsc, page("evenements"))
        .view_with("/journaux", &rsh, &rsc, page("journaux"))
        .view_with("/journaux/{nom}", &rsh, &rsc, journal)
        .view_with("/app/{nom}", &rsh, &rsc, detail)
}

/// Ce qu'un clic demande.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Goto(String),
    /// `name` : le flux ou la methode, selon `kind`.
    Grant { kind: Kind, owner: String, name: String, app: String },
    Revoke { kind: Kind, owner: String, name: String, app: String },
    SetPublic { kind: Kind, owner: String, name: String, public: bool },
    Reset { kind: Kind, owner: String, name: String },
    Restart(String),
    /// Demander confirmation avant d'oublier l'app.
    AskForget(String),
    /// Oublier l'app (`azure_manager forget`).
    Forget(String),
    /// Executer ce qui est tape dans le champ `#commande` du terminal.
    Execute,
    /// Executer cette ligne (boutons du terminal).
    Command(String),
    /// Compiler et installer cette app (`azure build <nom> --installer`).
    Build(String),
}

fn text_at(value: &Value, path: &str) -> Option<String> {
    value.get(path).and_then(Value::as_str).map(str::to_string)
}

/// Traduit le bouton clique (`#id` rsH) en action, selon la page ouverte et
/// l'etat affiche (les ids portent des numeros de ligne : `#retirer-0-1` =
/// 2e app autorisee du 1er flux ; prefixe `a` pour une methode :
/// `#aretirer-0-1`).
pub fn action(clicked: &str, current: &str, state: &Value) -> Option<Action> {
    let goto = |path: &str| Some(Action::Goto(path.to_string()));
    match clicked {
        "nav-apps" => return goto("/"),
        "nav-liens" => return goto("/liens"),
        "nav-services" => return goto("/services"),
        "nav-evenements" => return goto("/evenements"),
        "nav-journaux" => return goto("/journaux"),
        "nav-terminal" => return goto("/terminal"),
        // Entree dans le champ, ou le bouton.
        "commande" | "terminal-executer" => return Some(Action::Execute),
        "terminal-list" => return Some(Action::Command("list".to_string())),
        "terminal-aide" => return Some(Action::Command("aide".to_string())),
        "terminal-effacer" => return Some(Action::Command("effacer".to_string())),
        "journal-actualiser" => return goto(current),
        _ => {}
    }
    if let Some(name) = clicked.strip_prefix("journal-") {
        return goto(&format!("/journaux/{name}"));
    }
    if let Some(name) = clicked.strip_prefix("app-") {
        return goto(&format!("/app/{name}"));
    }
    if let Some(service) = clicked.strip_prefix("relancer-") {
        return Some(Action::Restart(service.to_string()));
    }
    if clicked == "erreur-fermer" {
        return goto(current);
    }
    // Les autres boutons sont sur la page d'une app.
    let owner = current.strip_prefix("/app/")?;
    match clicked {
        "oublier" => return Some(Action::AskForget(owner.to_string())),
        "oublier-oui" => return Some(Action::Forget(owner.to_string())),
        "oublier-non" => return goto(current),
        "compiler" => return Some(Action::Build(owner.to_string())),
        _ => {}
    }
    let app = find_app(state, owner)?;
    let (verb, rest) = clicked.split_once('-')?;
    let (kind, verb, list, key) = match verb.strip_prefix('a').filter(|v| ["retirer", "autoriser", "public", "prive", "reset"].contains(v)) {
        Some(verb) => (Kind::Call, verb, "fournit", "methode"),
        None => (Kind::Flux, verb, "partages", "flux"),
    };
    let mut numbers = rest.split('-').map(|n| n.parse::<usize>().ok());
    let index = numbers.next()??;
    let second = numbers.next().flatten();
    let item = format!("{list}.{index}");
    let name = text_at(app, &format!("{item}.{key}"))?;
    let owner = owner.to_string();
    match verb {
        "retirer" => Some(Action::Revoke { kind, app: text_at(app, &format!("{item}.autorises.{}", second?))?, owner, name }),
        "autoriser" => Some(Action::Grant { kind, app: text_at(app, &format!("{item}.autres.{}", second?))?, owner, name }),
        "public" => Some(Action::SetPublic { kind, owner, name, public: true }),
        "prive" => Some(Action::SetPublic { kind, owner, name, public: false }),
        "reset" => Some(Action::Reset { kind, owner, name }),
        _ => None,
    }
}
