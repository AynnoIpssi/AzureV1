// Le provider en tant que daemon : un socket de controle (une connexion =
// un thread) et une boucle qui fait avancer le `Supervisor` toutes les
// 100 ms.
use crate::managers::supervisor::Supervisor;
use crate::models::policy::RestartPolicy;
use crate::models::request::*;
use crate::models::service::{ServiceSpec, State};
use crate::models::wire::{Reader, Writer};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(100);

/// Mis a vrai par SIGTERM/SIGINT (voir `stop_on_signals`).
static SIGNALED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: libc::c_int) {
    SIGNALED.store(true, Ordering::SeqCst);
}

/// SIGTERM et SIGINT (Ctrl+C) arretent proprement le provider : ses
/// services sont arretes avec lui.
pub fn stop_on_signals() {
    let handler = on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t;
    // SAFETY : le gestionnaire ne fait qu'ecrire un atomique.
    unsafe {
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGINT, handler);
    }
}

pub struct ProviderConfig {
    pub socket: String,
    pub services: Vec<ServiceSpec>,
    pub policy: RestartPolicy,
    pub log_dir: PathBuf,
}

/// Bloque tant que le provider tourne (jusqu'a SHUTDOWN ou un signal).
pub fn run(config: ProviderConfig) -> Result<(), String> {
    let socket = config.socket.clone();
    // Un socket laisse par un provider arrete est retire ; un provider qui
    // repond encore garde le sien.
    if UnixStream::connect(&socket).is_ok() {
        return Err(format!("un provider tourne deja sur {socket}"));
    }
    let listener = azure_core::daemon::bind(&socket)?;
    // Le provider lance des commandes : seul l'utilisateur peut lui parler.
    let _ = std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600));

    let supervisor = Arc::new(Mutex::new(Supervisor::new(config.services, config.policy, config.log_dir)));
    let stop = Arc::new(AtomicBool::new(false));
    supervisor.lock().unwrap_or_else(|e| e.into_inner()).start_autostart(Instant::now());

    {
        let supervisor = Arc::clone(&supervisor);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || azure_core::daemon::serve(listener, move |stream| handle_connection(stream, Arc::clone(&supervisor), Arc::clone(&stop))));
    }

    while !stop.load(Ordering::SeqCst) && !SIGNALED.load(Ordering::SeqCst) {
        if let Ok(mut supervisor) = supervisor.lock() {
            supervisor.tick(Instant::now());
        }
        std::thread::sleep(TICK);
    }

    if let Ok(mut supervisor) = supervisor.lock() {
        supervisor.shutdown();
    }
    let _ = std::fs::remove_file(&socket);
    Ok(())
}

fn handle_connection(mut stream: UnixStream, supervisor: Arc<Mutex<Supervisor>>, stop: Arc<AtomicBool>) {
    // Une app enfermee peut demander l'etat et qu'un service connu tourne
    // (ENSURE), rien d'autre : sinon elle ferait lancer par le provider,
    // hors de son enfermement, la commande de son choix.
    // Exception : les outils d'Azure installes a cote du provider (`azure`,
    // azure-manager), meme lances avec NoNewPrivs (service systemd durci).
    let restricted = azure_core::managers::identity::peer_restricted(&stream) && !is_azure_tool(&stream);
    while let Ok(request) = read_frame(&mut stream) {
        let response = response(handle_request(&request, &supervisor, &stop, restricted));
        if write_frame(&mut stream, &response).is_err() {
            break;
        }
    }
}

/// Le client est-il `azure` ou `manager_daemon`, dans le dossier du provider ?
fn is_azure_tool(stream: &UnixStream) -> bool {
    let (Ok(peer), Ok(me)) = (azure_core::managers::identity::peer_exe(stream), std::env::current_exe()) else { return false };
    let Some(dir) = me.parent() else { return false };
    ["azure", "manager_daemon"].iter().any(|tool| std::path::Path::new(&peer) == dir.join(tool))
}

fn lock(supervisor: &Mutex<Supervisor>) -> Result<std::sync::MutexGuard<'_, Supervisor>, String> {
    // Un thread qui a plante en le tenant ne bloque pas les autres.
    Ok(supervisor.lock().unwrap_or_else(|e| e.into_inner()))
}

fn handle_request(request: &[u8], supervisor: &Mutex<Supervisor>, stop: &AtomicBool, restricted: bool) -> Result<Vec<u8>, String> {
    let mut r = Reader::new(request);
    let now = Instant::now();
    let opcode = r.u32()?;
    if restricted && !matches!(opcode, STATUS | ENSURE) {
        return Err("refuse : un processus enferme ne pilote pas le provider (seuls l'etat et ENSURE)".to_string());
    }
    match opcode {
        STATUS => {
            let status = lock(supervisor)?.status(now);
            let mut w = Writer::new().u32(status.len() as u32);
            for s in &status {
                w = write_status(w, s);
            }
            Ok(w.finish())
        }
        START => lock(supervisor)?.start(&r.str()?, now).map(|_| Vec::new()),
        STOP => lock(supervisor)?.stop(&r.str()?).map(|_| Vec::new()),
        RESTART => lock(supervisor)?.restart(&r.str()?, now).map(|_| Vec::new()),
        REGISTER => {
            let spec = read_spec(&mut r)?;
            lock(supervisor)?.register(spec, now).map(|_| Vec::new())
        }
        DECLARE => {
            let spec = read_spec(&mut r)?;
            lock(supervisor)?.declare(spec).map(|_| Vec::new())
        }
        UNREGISTER => lock(supervisor)?.unregister(&r.str()?).map(|_| Vec::new()),
        ENSURE => {
            let name = r.str()?;
            let timeout = Duration::from_millis(r.u32()? as u64);
            ensure(supervisor, &name, timeout).map(|_| Vec::new())
        }
        SHUTDOWN => {
            stop.store(true, Ordering::SeqCst);
            Ok(Vec::new())
        }
        other => Err(format!("Opcode inconnu : {other}")),
    }
}

/// Lance le service s'il ne tourne pas et attend qu'il soit pret. Le verrou
/// est relache entre deux essais : la boucle principale continue ses `tick`.
fn ensure(supervisor: &Mutex<Supervisor>, name: &str, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    lock(supervisor)?.start(name, Instant::now())?;
    loop {
        {
            let mut supervisor = lock(supervisor)?;
            supervisor.tick(Instant::now());
            let state = supervisor.state(name).ok_or_else(|| format!("Service inconnu : '{name}'"))?;
            if state.is_ready() {
                return Ok(());
            }
            let message = supervisor.message(name).unwrap_or_default();
            if matches!(state, State::Failed | State::Exited | State::Stopped) {
                return Err(format!("'{name}' {} : {message}", state.label()));
            }
            if Instant::now() >= deadline {
                return Err(format!("'{name}' pas pret apres {} ms ({}{})", timeout.as_millis(), state.label(), if message.is_empty() { String::new() } else { format!(" : {message}") }));
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
