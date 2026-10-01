// Lancer, arreter et sonder un processus de service.
use crate::find_binary;
use crate::models::service::ServiceSpec;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Au-dela, le journal est renomme en `.log.1` (l'ancien `.log.1` perdu).
const MAX_LOG: u64 = 1024 * 1024;

pub fn log_file(log_dir: &Path, name: &str) -> PathBuf {
    log_dir.join(format!("{name}.log"))
}

fn open_log(log_dir: &Path, name: &str) -> Result<File, String> {
    std::fs::create_dir_all(log_dir).map_err(|e| format!("{} : {e}", log_dir.display()))?;
    let path = log_file(log_dir, name);
    if path.metadata().is_ok_and(|m| m.len() > MAX_LOG) {
        let _ = std::fs::rename(&path, path.with_extension("log.1"));
    }
    let mut file = OpenOptions::new().create(true).append(true).open(&path).map_err(|e| format!("{} : {e}", path.display()))?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let _ = writeln!(file, "--- azure-provider : demarrage (horodatage unix {now}) ---");
    Ok(file)
}

/// Lance le service ; sa sortie standard et d'erreur vont dans son journal.
pub fn spawn(spec: &ServiceSpec, log_dir: &Path) -> Result<Child, String> {
    let program = find_binary(&spec.command).ok_or_else(|| format!("executable '{}' introuvable", spec.command))?;
    let log = open_log(log_dir, &spec.name)?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    let mut command = Command::new(&program);
    command.args(&spec.args).stdin(Stdio::null()).stdout(log).stderr(err);
    // Sans socket, un service orphelin ne serait pas reconnu par un provider
    // relance (il en lancerait un second) : il s'arrete avec le provider. Un
    // daemon a socket survit, et sera repris (`State::External`).
    if spec.health.is_none() {
        // SAFETY : `prctl` est sur entre fork et exec (pas d'allocation).
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
    }
    // PDEATHSIG suit le THREAD qui lance, pas le processus : lance depuis le
    // thread d'une connexion (ENSURE d'azure-manager), le service recevait
    // SIGTERM des la fin de cette connexion. Tous les lancements passent
    // donc par un thread qui vit autant que le provider.
    launcher_spawn(command).map_err(|e| format!("{} : {e}", program.display()))
}

type Job = (Command, std::sync::mpsc::Sender<std::io::Result<Child>>);

fn launcher_spawn(command: Command) -> std::io::Result<Child> {
    static LAUNCHER: std::sync::OnceLock<std::sync::Mutex<std::sync::mpsc::Sender<Job>>> = std::sync::OnceLock::new();
    let launcher = LAUNCHER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("azure-provider-lanceur".into())
            .spawn(move || {
                for (mut command, reply) in rx {
                    let _ = reply.send(command.spawn());
                }
            })
            .expect("thread de lancement du provider");
        std::sync::Mutex::new(tx)
    });
    let (reply, answer) = std::sync::mpsc::channel();
    launcher.lock().unwrap_or_else(|e| e.into_inner()).send((command, reply)).map_err(|_| std::io::Error::other("thread de lancement arrete"))?;
    answer.recv().map_err(|_| std::io::Error::other("thread de lancement arrete"))?
}

/// SIGTERM, puis SIGKILL si le processus n'est pas sorti apres `timeout`.
pub fn terminate(child: &mut Child, timeout: Duration) {
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    // SAFETY : simple envoi de signal a notre propre enfant.
    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
    }
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Le socket accepte une connexion (fermee aussitot).
pub fn health_ok(socket: &str) -> bool {
    UnixStream::connect(socket).is_ok()
}

/// "code 3", "tue par le signal 9"...
pub fn describe_exit(status: ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(code), _) => format!("sorti avec le code {code}"),
        (None, Some(signal)) => format!("tue par le signal {signal}"),
        _ => "arrete".to_string(),
    }
}
