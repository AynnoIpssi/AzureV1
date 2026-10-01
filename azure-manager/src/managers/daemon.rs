// Le daemon azure-manager : une connexion par app (REGISTER avec son
// manifeste, verifie contre son vrai executable), et une boucle qui, chaque
// seconde, lit l'etat des services (azure-provider), applique les acces aux
// flux (azure-service) et publie l'etat pour le tableau de bord.
use crate::managers::manager::{Activity, Kind, Level, Manager};
use crate::models::manifest::Summary;
use crate::models::request::*;
use azure_core::managers::identity::{peer_exe, peer_fingerprint, peer_pid};
use azure_core::models::wire::{Reader, Writer};
use azure_provider::{ProviderClient, ServiceStatus};
use azure_service::flux::{Access, Change, Flux, Shared, Value};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const POLL: Duration = Duration::from_secs(1);
/// Le temps laisse a une tache de fond pour demarrer (voir `WAKE`).
const WAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct ManagerConfig {
    pub socket: String,
    pub data_dir: PathBuf,
    pub service_socket: String,
    pub provider_socket: String,
    pub admins: Vec<String>,
    /// Journaux des services (azure-provider) et des apps (`azure run`).
    pub provider_logs: PathBuf,
    pub app_logs: PathBuf,
}

/// Les journaux par defaut : `~/.local/state/azure/provider` et
/// `~/.local/state/azure/apps`.
pub fn default_app_logs() -> PathBuf {
    let state = match std::env::var_os("XDG_STATE_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")).unwrap_or_else(|| "/tmp".into()),
    };
    state.join("azure/apps")
}

/// Les `lines` dernieres lignes de `file`.
pub fn tail(file: &std::path::Path, lines: usize) -> Result<Vec<String>, String> {
    let text = std::fs::read(file).map_err(|e| format!("{} : {e}", file.display()))?;
    // Seulement la fin d'un gros fichier.
    let start = text.len().saturating_sub(256 * 1024);
    let text = String::from_utf8_lossy(&text[start..]);
    let all: Vec<&str> = text.lines().collect();
    Ok(all[all.len().saturating_sub(lines)..].iter().map(|l| l.to_string()).collect())
}

/// La connexion a azure-service : flux `etat` et acces des autres flux.
#[derive(Default)]
struct Publisher {
    flux: Option<Flux>,
    etat: Option<Shared>,
    readers: Vec<u32>,
    services: Vec<ServiceStatus>,
    provider_ok: bool,
    activity: Activity,
    // Derniere erreur de publication (voir `sync`), pour ne la noter qu'une fois.
    last_error: Option<String>,
}

struct Daemon {
    manager: Mutex<Manager>,
    publisher: Mutex<Publisher>,
    config: ManagerConfig,
}

/// Bloque tant que le daemon tourne.
pub fn run(config: ManagerConfig) -> Result<(), String> {
    let manager = Manager::open(&config.data_dir, config.admins.clone())?;
    let listener = azure_core::daemon::bind(&config.socket)?;
    let daemon = Arc::new(Daemon { manager: Mutex::new(manager), publisher: Mutex::new(Publisher::default()), config });

    {
        let daemon = Arc::clone(&daemon);
        std::thread::spawn(move || loop {
            daemon.refresh_services();
            daemon.sync();
            std::thread::sleep(POLL);
        });
    }
    azure_core::daemon::serve(listener, move |stream| daemon.handle_connection(stream));
    Ok(())
}

fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    // Un thread qui a plante en le tenant ne bloque pas les autres.
    Ok(mutex.lock().unwrap_or_else(|e| e.into_inner()))
}

impl Daemon {
    fn refresh_services(&self) {
        let status = ProviderClient::connect_at(&self.config.provider_socket).and_then(|mut client| client.status());
        if let Ok(mut publisher) = self.publisher.lock() {
            publisher.provider_ok = status.is_ok();
            publisher.services = status.unwrap_or_default();
        }
    }

    /// Donne a azure-service les acces a jour et publie l'etat s'il a
    /// change. Une erreur (azure-service absent...) sera retentee au tour
    /// suivant.
    fn sync(&self) {
        let Ok(mut publisher) = self.publisher.lock() else { return };
        // Compteurs d'azure-service (flux, appels).
        if let Some(flux) = &publisher.flux
            && let Ok((flux, methods)) = flux.stats() {
                publisher.activity = lock(&self.manager).map(|mut m| m.absorb(Activity { flux, methods })).unwrap_or_default();
            }
        if let Ok(mut m) = lock(&self.manager)
            && let Err(e) = m.save_activity() {
                eprintln!("azure-manager : evenements non enregistres : {e}");
            }
        let Ok((state, pushes, readers)) = lock(&self.manager).map(|m| (m.state_with(&publisher.services, publisher.provider_ok, &publisher.activity), m.pushes(), m.admin_ids())) else { return };
        match publish(&mut publisher, &self.config.service_socket, state, &pushes, readers) {
            Ok(()) => {
                if publisher.last_error.take().is_some() {
                    eprintln!("azure-manager : etat de nouveau publie");
                }
            }
            Err(e) => {
                // Au journal une fois par erreur differente (on reessaie
                // chaque seconde) : sans etat, le tableau de bord est fige.
                if publisher.last_error.as_deref() != Some(e.as_str()) {
                    eprintln!("azure-manager : etat non publie ({e}), nouvel essai chaque seconde");
                    publisher.last_error = Some(e);
                }
                // Reconnexion complete au prochain tour.
                publisher.flux = None;
                publisher.etat = None;
            }
        }
    }

    fn handle_connection(&self, mut stream: UnixStream) {
        let exe = peer_exe(&stream).unwrap_or_default();
        let mut session: Option<String> = None;
        while let Ok(request) = read_frame(&mut stream) {
            let result = self.handle_request(&request, &stream, &exe, &mut session);
            let changed = result.as_ref().map(|(_, changed)| *changed).unwrap_or(false);
            if write_frame(&mut stream, &response(result.map(|(values, _)| values))).is_err() {
                break;
            }
            if changed {
                self.sync();
            }
        }
        if let Some(name) = session {
            if let Ok(mut manager) = self.manager.lock() {
                manager.disconnected(&name);
            }
            self.sync();
        }
    }

    /// Retourne la reponse, et si l'etat a change (a publier).
    fn handle_request(&self, request: &[u8], stream: &UnixStream, exe: &str, session: &mut Option<String>) -> Result<(Vec<u8>, bool), String> {
        let mut r = Reader::new(request);
        let opcode = r.u32()?;
        let out = Writer::new();
        let admin = || if lock(&self.manager)?.is_admin(exe) { Ok(()) } else { Err("Reserve au tableau de bord d'Azure".to_string()) };
        Ok(match opcode {
            REGISTER => {
                let summary = Summary::read(&mut r)?;
                r.finish()?;
                if session.is_some() {
                    return Err("Deja enregistree sur cette connexion".to_string());
                }
                let pid = peer_pid(stream)? as u32;
                let name = summary.name.clone();
                let fingerprint = peer_fingerprint(stream).ok();
                let id = lock(&self.manager)?.register(summary, exe, fingerprint, pid)?;
                *session = Some(name);
                (out.u32(id).finish(), true)
            }
            IDENTIFY => {
                let id = r.u32()?;
                r.finish()?;
                let manager = lock(&self.manager)?;
                let w = match manager.by_id(id) {
                    Some(app) => out
                        .u8(1)
                        .str(&app.summary.name)
                        .str(&app.exe)
                        .u8(app.installed as u8)
                        .bytes(app.fingerprint.as_ref().map(|f| f.as_slice()).unwrap_or(&[]))
                        .u8(app.summary.permissions.storage as u8),
                    None => out.u8(0),
                };
                (w.finish(), false)
            }
            WAKE => {
                let (owner, method) = (r.str()?, r.str()?);
                r.finish()?;
                let caller = session.as_ref().ok_or("REGISTER attendu avant WAKE")?;
                let service = lock(&self.manager)?.wake_service(caller, &owner, &method)?;
                // Verrou rendu : le lancement peut prendre un moment.
                ProviderClient::connect_at(&self.config.provider_socket)?.ensure(&service, WAKE_TIMEOUT)?;
                (out.finish(), false)
            }
            OPEN => {
                let (target, activation) = (r.str()?, r.str()?);
                r.finish()?;
                let caller = session.as_ref().ok_or("REGISTER attendu avant OPEN")?;
                let launch = lock(&self.manager)?.open_check(caller, &target)?;
                if launch {
                    // `azure run` : comme depuis un terminal (isolation,
                    // journal) ; il rend la main des l'app lancee.
                    let mut command = std::process::Command::new(crate::managers::terminal::azure_binary());
                    // Premier plan : la convention des lanceurs Wayland.
                    if activation.is_empty() {
                        command.env_remove("XDG_ACTIVATION_TOKEN");
                    } else {
                        command.env("XDG_ACTIVATION_TOKEN", &activation);
                    }
                    let mut child = command
                        .args(["run", target.as_str()])
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn()
                        .map_err(|e| format!("azure run {target} : {e}"))?;
                    std::thread::spawn(move || child.wait());
                }
                (out.u8(launch as u8).finish(), false)
            }
            INSTALL => {
                let summary = Summary::read(&mut r)?;
                let (path, fp) = (r.str()?, r.bytes()?.to_vec());
                r.finish()?;
                admin()?;
                let fp: [u8; 32] = fp.try_into().map_err(|_| "empreinte invalide".to_string())?;
                let id = lock(&self.manager)?.install(summary, &path, fp)?;
                (out.u32(id).finish(), true)
            }
            RESOLVE => {
                let name = r.str()?;
                r.finish()?;
                (out.u32(lock(&self.manager)?.resolve(&name)?).finish(), false)
            }
            ACCESS => {
                let (kind, name) = (Kind::from_code(r.u8()?)?, r.str()?);
                r.finish()?;
                let owner = session.as_ref().ok_or("REGISTER attendu avant ACCESS")?;
                (lock(&self.manager)?.access(kind, owner, &name)?.write(out).finish(), false)
            }
            STATE => {
                r.finish()?;
                admin()?;
                let (services, ok, activity) = lock(&self.publisher).map(|p| (p.services.clone(), p.provider_ok, p.activity.clone()))?;
                (lock(&self.manager)?.state_with(&services, ok, &activity).write(out).finish(), false)
            }
            REPORT => {
                let (name, level, message) = (r.str()?, Level::from_code(r.u8()?), r.str()?);
                r.finish()?;
                let mut manager = lock(&self.manager)?;
                // Seulement en son propre nom.
                let own = session.as_deref() == Some(name.as_str()) || manager.app(&name).is_some_and(|a| a.exe == exe);
                if !own {
                    return Err(format!("'{name}' n'est pas l'app qui parle"));
                }
                manager.report(&name, level, &message);
                (out.finish(), true)
            }
            LOGS => {
                let (name, lines) = (r.str()?, r.u32()?.min(2000) as usize);
                r.finish()?;
                admin()?;
                crate::models::manifest::check_app_name(&name)?;
                let service = self.config.provider_logs.join(format!("{name}.log"));
                let file = if service.exists() { service } else { self.config.app_logs.join(format!("{name}.log")) };
                let lines = tail(&file, lines)?;
                (lines.iter().fold(out.u32(lines.len() as u32), |w, l| w.str(l)).finish(), false)
            }
            GRANT | REVOKE => {
                let (kind, owner, name, app) = (Kind::from_code(r.u8()?)?, r.str()?, r.str()?, r.str()?);
                r.finish()?;
                admin()?;
                let mut manager = lock(&self.manager)?;
                if opcode == GRANT { manager.grant(kind, &owner, &name, &app)? } else { manager.revoke(kind, &owner, &name, &app)? }
                (out.finish(), true)
            }
            SET_PUBLIC => {
                let (kind, owner, name, public) = (Kind::from_code(r.u8()?)?, r.str()?, r.str()?, r.u8()? != 0);
                r.finish()?;
                admin()?;
                lock(&self.manager)?.set_public(kind, &owner, &name, public)?;
                (out.finish(), true)
            }
            RESET_ACCESS => {
                let (kind, owner, name) = (Kind::from_code(r.u8()?)?, r.str()?, r.str()?);
                r.finish()?;
                admin()?;
                lock(&self.manager)?.reset(kind, &owner, &name)?;
                (out.finish(), true)
            }
            FORGET => {
                let name = r.str()?;
                r.finish()?;
                admin()?;
                lock(&self.manager)?.forget(&name)?;
                (out.finish(), true)
            }
            RESTART_SERVICE => {
                let name = r.str()?;
                r.finish()?;
                admin()?;
                ProviderClient::connect_at(&self.config.provider_socket)?.restart(&name)?;
                (out.finish(), false)
            }
            AZURE_COMMAND => {
                let args = (0..r.u32()?.min(64)).map(|_| r.str()).collect::<Result<Vec<_>, _>>()?;
                r.finish()?;
                admin()?;
                // Sans verrou : `azure install` rappelle ce daemon.
                let (code, lines) = crate::managers::terminal::run(&crate::managers::terminal::azure_binary(), &args, crate::managers::terminal::timeout_for(args.first().map(String::as_str).unwrap_or("")))?;
                (lines.iter().fold(out.u32(code as u32).u32(lines.len() as u32), |w, l| w.str(l)).finish(), true)
            }
            other => return Err(format!("Opcode inconnu : {other}")),
        })
    }
}

fn publish(publisher: &mut Publisher, socket: &str, state: Value, pushes: &[crate::managers::manager::Push], readers: Vec<u32>) -> Result<(), String> {
    if publisher.flux.is_none() {
        publisher.flux = Some(Flux::connect_at(socket, crate::MANAGER_ID)?);
    }
    let flux = publisher.flux.clone().expect("connecte juste au-dessus");
    for push in pushes {
        // Pas encore partage / servi par son app : elle recevra l'acces en
        // le faisant (voir `ACCESS`).
        let _ = match push.kind {
            Kind::Flux => flux.set_access(push.owner, &push.name, &push.access),
            Kind::Call => flux.set_call_access(push.owner, &push.name, &push.access),
        };
    }
    if publisher.etat.is_none() {
        publisher.etat = Some(flux.share(crate::STATE_FLUX).to(&readers).open()?);
        publisher.readers = readers.clone();
    }
    let etat = publisher.etat.as_mut().expect("ouvert juste au-dessus");
    if publisher.readers != readers {
        etat.share_with(Access::Apps(readers.clone()))?;
        publisher.readers = readers;
    }
    if etat.state() != &state {
        etat.apply(vec![Change::Set(String::new(), state)])?;
    }
    Ok(())
}
