// Installer, desinstaller, lister et lancer une app.
use crate::paths::Paths;
use crate::{copy_all, install_binary};
use azure_manager::models::manifest::Manifest;
use azure_manager::services::client::ManagerClient;
use std::path::{Path, PathBuf};

/// Une app installee.
#[derive(Clone, Debug)]
pub struct Installed {
    pub name: String,
    pub title: String,
    pub version: String,
    pub exe: PathBuf,
    /// Permissions reseau et processus du manifeste installe.
    pub network: bool,
    pub processes: bool,
}

/// Comment joindre azure-manager pendant l'installation.
pub enum ManagerAccess {
    /// Le daemon par defaut (lance par azure-provider si besoin).
    Default,
    /// Un autre socket (tests).
    At(String),
    /// Ne pas le contacter.
    None,
}

/// Le `target-dir` du `.cargo/config.toml` d'une app (ecrit par `azure
/// new`), sinon `$CARGO_TARGET_DIR`.
fn configured_target(dir: &Path) -> Option<PathBuf> {
    let config = std::fs::read_to_string(dir.join(".cargo/config.toml")).ok();
    let from_config = config.and_then(|text| {
        text.lines().find_map(|line| {
            let value = line.trim().strip_prefix("target-dir")?.trim_start().strip_prefix('=')?;
            Some(value.trim().trim_matches('"').to_string())
        })
    });
    let target = from_config.or_else(|| std::env::var("CARGO_TARGET_DIR").ok().filter(|t| !t.is_empty()))?;
    let target = PathBuf::from(target);
    Some(if target.is_absolute() { target } else { dir.join(target) })
}

/// Cherche le binaire `exec` d'une app dont le manifeste est dans `dir` :
/// dans `dir`, puis dans le `target-dir` de l'app (`.cargo/config.toml`,
/// voir `azure new`), puis dans `target/release` et `target/debug` d'un
/// dossier parent (projet cargo). Le `target-dir` passe avant : c'est la
/// que cargo compile l'app, un `target` voisin peut etre perime.
pub fn find_app_binary(dir: &Path, exec: &str) -> Option<PathBuf> {
    let direct = dir.join(exec);
    if direct.is_file() {
        return Some(direct);
    }
    let dir = dir.canonicalize().ok()?;
    let configured = configured_target(&dir).into_iter();
    configured
        .chain(dir.ancestors().map(|d| d.join("target")))
        .flat_map(|target| [target.join("release").join(exec), target.join("debug").join(exec)])
        .find(|p| p.is_file())
}

/// Installe l'app dont le manifeste est `<source>/app.azure`. `binary` :
/// l'executable a installer (sinon cherche par `find_app_binary`).
/// Retourne ce qui a ete installe, et les remarques a afficher.
pub fn install(paths: &Paths, source: &Path, binary: Option<&Path>, manager: ManagerAccess) -> Result<(Installed, Vec<String>), String> {
    let manifest = Manifest::load(&source.join("app.azure"))?;
    let exec = manifest.exec.clone().ok_or_else(|| format!("{}/app.azure : ajoutez `exec = <binaire>` dans [app]", source.display()))?;
    let binary = match binary {
        Some(path) => path.to_path_buf(),
        None => find_app_binary(source, &exec).ok_or_else(|| format!("binaire '{exec}' introuvable (compilez l'app : cargo build --release, ou donnez --bin <chemin>)"))?,
    };
    if !binary.is_file() {
        return Err(format!("{} : pas un fichier", binary.display()));
    }
    let files = manifest.bundle_files();
    for file in &files {
        if !file.exists() {
            return Err(format!("{} : fichier declare dans app.azure introuvable", file.display()));
        }
    }

    // Tout est prepare a cote, puis mis en place d'un coup.
    let target = paths.app(&manifest.name);
    let staging = paths.apps().join(format!(".{}.nouveau", manifest.name));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| format!("{} : {e}", staging.display()))?;
    copy_all(&source.join("app.azure"), &staging.join("app.azure"))?;
    for file in &files {
        let relative = file.strip_prefix(&manifest.dir).map_err(|_| format!("{} : hors du dossier de l'app", file.display()))?;
        copy_all(file, &staging.join(relative))?;
    }
    install_binary(&binary, &staging.join(&exec))?;
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|e| format!("{} : {e}", target.display()))?;
    }
    std::fs::rename(&staging, &target).map_err(|e| format!("{} : {e}", target.display()))?;

    let exe = target.join(&exec);
    let installed = Installed { name: manifest.name.clone(), title: manifest.title.clone(), version: manifest.version.clone(), exe: exe.clone(), network: manifest.permissions.network, processes: manifest.permissions.processes };
    let icon = manifest.icon.as_ref().and_then(|icon| icon.strip_prefix(&manifest.dir).ok()).map(|relative| target.join(relative));
    write_desktop(paths, &installed, icon.as_deref())?;

    // Le manifeste installe : ses commandes pointent sur le binaire copie.
    let installed_manifest = Manifest::load(&target.join("app.azure"))?;
    let services = installed_manifest.provider_services();
    write_services(paths, &manifest.name, &services)?;

    let mut notes = Vec::new();
    let contact = !matches!(manager, ManagerAccess::None);
    if let Some(note) = register_install(&manifest, &exe, manager) {
        notes.push(note);
    }
    if contact && !services.is_empty() {
        notes.push(declare_services(&services));
    }
    Ok((installed, notes))
}

/// Ecrit les taches de fond de l'app (format de provider.conf) : le
/// provider les connait des son demarrage et les lance a la demande. Aucune
/// tache : le fichier est retire.
fn write_services(paths: &Paths, name: &str, services: &[azure_provider::ServiceSpec]) -> Result<(), String> {
    let path = paths.services(name);
    if services.is_empty() {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    let arg = |a: &String| if a.chars().any(char::is_whitespace) || a.is_empty() { format!("\"{a}\"") } else { a.clone() };
    let mut text = format!("# Taches de fond de l'app {name}, ecrit par `azure install`.\n");
    for spec in services {
        text.push_str(&format!("\n[{}]\ncommand = {}\n", spec.name, spec.command));
        if !spec.args.is_empty() {
            text.push_str(&format!("args = {}\n", spec.args.iter().map(arg).collect::<Vec<_>>().join(" ")));
        }
        if let Some(health) = &spec.health {
            text.push_str(&format!("health = {health}\n"));
        }
        text.push_str(&format!("restart = {}\nautostart = {}\n", spec.restart.name(), spec.autostart));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    }
    std::fs::write(&path, text).map_err(|e| format!("{} : {e}", path.display()))
}

/// Fait connaitre les taches de fond au provider qui tourne deja (sinon il
/// les lira a son demarrage).
fn declare_services(services: &[azure_provider::ServiceSpec]) -> String {
    let Ok(mut provider) = azure_provider::ProviderClient::connect_at(&azure_provider::SOCKET_PATH) else {
        return "taches de fond enregistrees (connues d'azure-provider a son demarrage)".to_string();
    };
    let names: Vec<&str> = services.iter().map(|s| s.name.as_str()).collect();
    match services.iter().try_for_each(|spec| provider.declare(spec)) {
        Ok(()) => format!("taches de fond declarees a azure-provider : {}", names.join(", ")),
        Err(e) => format!("azure-provider : {e} (taches de fond connues a son prochain demarrage)"),
    }
}

fn quote(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\""))
}

fn write_desktop(paths: &Paths, app: &Installed, icon: Option<&Path>) -> Result<(), String> {
    std::fs::create_dir_all(&paths.applications).map_err(|e| format!("{} : {e}", paths.applications.display()))?;
    let icon = icon.map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "application-x-executable".to_string());
    // Par `azure run` quand Azure est installe : il demarre les daemons
    // avant l'app, qui une fois enfermee ne pourrait plus le faire.
    let azure = paths.bin().join("azure");
    let exec = if azure.is_file() { format!("{} run {}", quote(&azure), app.name) } else { quote(&app.exe) };
    // Affiche dans la recherche du bureau, sous le nom : la version seulement
    // si le manifeste en donne une.
    let comment = match app.version.as_str() {
        "" | "0" => "App Azure".to_string(),
        v => format!("App Azure, version {v}"),
    };
    let text = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={comment}\nExec={exec}\nIcon={icon}\nStartupWMClass=azure-{}\nCategories=Utility;\nX-Azure-App={}\n",
        app.title, app.name, app.name
    );
    let path = paths.desktop(&app.name);
    std::fs::write(&path, text).map_err(|e| format!("{} : {e}", path.display()))
}

/// Enregistre l'app installee aupres d'azure-manager : son executable et
/// son empreinte (SHA-256) deviennent son identite. Elle garde son id, ses
/// donnees et les reglages du tableau de bord si elle etait deja connue
/// (version de developpement, ou installation precedente).
fn register_install(manifest: &Manifest, exe: &Path, manager: ManagerAccess) -> Option<String> {
    let client = match manager {
        ManagerAccess::None => return Some("azure-manager non contacte : l'identite sera fixee au premier lancement".to_string()),
        ManagerAccess::Default => ManagerClient::connect(),
        ManagerAccess::At(socket) => ManagerClient::connect_at(&socket),
    };
    let fingerprint = match azure_core::managers::identity::fingerprint_file(exe) {
        Ok(f) => f,
        Err(e) => return Some(format!("empreinte impossible ({e})")),
    };
    let result = client.and_then(|mut c| c.install(&manifest.summary(), &exe.to_string_lossy(), &fingerprint));
    Some(match result {
        Ok(id) => format!("identite enregistree : id {id}, empreinte {}", azure_core::managers::identity::hex(&fingerprint[..8])),
        Err(e) => format!("azure-manager : {e} (l'identite sera fixee au premier lancement)"),
    })
}

pub fn uninstall(paths: &Paths, name: &str) -> Result<(), String> {
    let dir = paths.app(name);
    if !dir.join("app.azure").exists() {
        return Err(format!("'{name}' n'est pas installee"));
    }
    let services = Manifest::load(&dir.join("app.azure")).map(|m| m.provider_services()).unwrap_or_default();
    std::fs::remove_dir_all(&dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    let _ = std::fs::remove_file(paths.desktop(name));
    let _ = std::fs::remove_file(paths.services(name));
    if !services.is_empty()
        && let Ok(mut provider) = azure_provider::ProviderClient::connect_at(&azure_provider::SOCKET_PATH)
    {
        for spec in &services {
            let _ = provider.unregister(&spec.name);
        }
    }
    Ok(())
}

pub fn list(paths: &Paths) -> Vec<Installed> {
    let mut apps: Vec<Installed> = std::fs::read_dir(paths.apps())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let dir = entry.path();
            let manifest = Manifest::load(&dir.join("app.azure")).ok()?;
            let exe = dir.join(manifest.exec.as_deref()?);
            Some(Installed { name: manifest.name, title: manifest.title, version: manifest.version, exe, network: manifest.permissions.network, processes: manifest.permissions.processes })
        })
        .collect();
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps
}

/// Lance une app installee, detachee du terminal ; sa sortie va dans son
/// journal. Retourne son pid.
pub fn run(paths: &Paths, name: &str, args: &[String]) -> Result<u32, String> {
    use std::os::unix::process::CommandExt;
    let app = list(paths).into_iter().find(|a| a.name == name).ok_or_else(|| format!("'{name}' n'est pas installee (azure list)"))?;
    // Les daemons d'abord (voir `write_desktop`), si Azure est installe.
    if paths.bin().join("azure_provider").is_file() {
        for service in ["manager", "rooter", "stockage", "service"] {
            let _ = azure_provider::Provider::ensure(service);
        }
    }
    std::fs::create_dir_all(&paths.logs).map_err(|e| format!("{} : {e}", paths.logs.display()))?;
    let log_path = paths.logs.join(format!("{name}.log"));
    let log = std::fs::OpenOptions::new().create(true).append(true).open(&log_path).map_err(|e| format!("{} : {e}", log_path.display()))?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    let spawn = |isolated: bool| -> std::io::Result<std::process::Child> {
        let mut command = std::process::Command::new(&app.exe);
        command.args(args).current_dir(app.exe.parent().unwrap_or(Path::new("/"))).stdin(std::process::Stdio::null()).stdout(log.try_clone()?).stderr(err.try_clone()?);
        // Espaces de noms en plus de Landlock (voir azure_core::security::isolation).
        let isolation = isolated.then(|| azure_core::security::isolation::Isolation::for_app(app.network).voir_processus(app.processes));
        // SAFETY : `setsid` et `enter` ne font que des appels systeme.
        unsafe {
            command.pre_exec(move || {
                libc::setsid();
                match &isolation {
                    Some(isolation) => isolation.enter(),
                    None => Ok(()),
                }
            });
        }
        command.spawn()
    };
    // Isolation impossible (systeme qui la refuse a mi-chemin) : Landlock seul.
    let mut child = spawn(true).or_else(|e| {
        eprintln!("azure : isolation par espaces de noms impossible ({e}), l'app n'aura que Landlock");
        spawn(false)
    }).map_err(|e| format!("{} : {e}", app.exe.display()))?;
    let pid = child.id();
    std::thread::spawn(move || child.wait());
    Ok(pid)
}
