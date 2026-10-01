// Cote app : parler au provider, et le lancer s'il ne tourne pas.
//
// ```text
// Provider::ensure("stockage")?;                        // lance si besoin, attend qu'il reponde
// Service::new("notes-sync", "notes_sync")              // une tache de fond de l'app
//     .arg("--rapide")
//     .health_socket("/tmp/notes-sync.sock")
//     .register()?;
// for s in Provider::status()? { println!("{} : {}", s.name, s.state.label()); }
// ```
use crate::models::request::*;
use crate::models::service::{ServiceSpec, ServiceStatus};
use crate::models::wire::Writer;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Temps laisse a un service pour etre pret (`ensure`).
pub const ENSURE_TIMEOUT: Duration = Duration::from_secs(10);
/// Temps laisse au provider fraichement lance pour ouvrir son socket.
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(3);

pub struct ProviderClient {
    stream: UnixStream,
}

impl ProviderClient {
    pub fn connect() -> Result<ProviderClient, String> {
        ProviderClient::connect_at(&crate::SOCKET_PATH)
    }

    pub fn connect_at(socket: &str) -> Result<ProviderClient, String> {
        let stream = UnixStream::connect(socket).map_err(|e| format!("provider injoignable ({socket}) : {e}"))?;
        Ok(ProviderClient { stream })
    }

    /// Se connecte, en lancant le provider s'il ne tourne pas.
    pub fn connect_or_launch(socket: &str) -> Result<ProviderClient, String> {
        if let Ok(client) = ProviderClient::connect_at(socket) {
            return Ok(client);
        }
        // Lance depuis une app enfermee, le provider (et tous les daemons)
        // le seraient avec elle : sans acces a leurs donnees.
        if azure_core::security::sandbox::is_sandboxed() {
            return Err("azure-provider ne tourne pas, et une app enfermee ne peut pas le lancer (azure autostart on)".to_string());
        }
        launch_provider(socket)?;
        let deadline = Instant::now() + LAUNCH_TIMEOUT;
        loop {
            match ProviderClient::connect_at(socket) {
                Ok(client) => return Ok(client),
                Err(e) if Instant::now() >= deadline => return Err(e),
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }

    fn call(&mut self, request: Writer) -> Result<Vec<u8>, String> {
        write_frame(&mut self.stream, &request.finish())?;
        let response = read_frame(&mut self.stream)?;
        check_status(&response)?;
        Ok(response)
    }

    fn call_name(&mut self, opcode: u32, name: &str) -> Result<(), String> {
        self.call(Writer::new().u32(opcode).str(name)).map(|_| ())
    }

    pub fn status(&mut self) -> Result<Vec<ServiceStatus>, String> {
        let response = self.call(Writer::new().u32(STATUS))?;
        let mut r = check_status(&response)?;
        let count = r.u32()?;
        (0..count).map(|_| read_status(&mut r)).collect()
    }

    pub fn start(&mut self, name: &str) -> Result<(), String> {
        self.call_name(START, name)
    }

    pub fn stop(&mut self, name: &str) -> Result<(), String> {
        self.call_name(STOP, name)
    }

    pub fn restart(&mut self, name: &str) -> Result<(), String> {
        self.call_name(RESTART, name)
    }

    pub fn register(&mut self, spec: &ServiceSpec) -> Result<(), String> {
        self.call(write_spec(Writer::new().u32(REGISTER), spec)).map(|_| ())
    }

    /// Fait connaitre `spec` sans le lancer (voir `DECLARE`).
    pub fn declare(&mut self, spec: &ServiceSpec) -> Result<(), String> {
        self.call(write_spec(Writer::new().u32(DECLARE), spec)).map(|_| ())
    }

    pub fn unregister(&mut self, name: &str) -> Result<(), String> {
        self.call_name(UNREGISTER, name)
    }

    /// Lance `name` si besoin et attend qu'il soit pret.
    pub fn ensure(&mut self, name: &str, timeout: Duration) -> Result<(), String> {
        let ms = timeout.as_millis().min(u32::MAX as u128) as u32;
        self.call(Writer::new().u32(ENSURE).str(name).u32(ms)).map(|_| ())
    }

    /// Arrete le provider et tous ses services.
    pub fn shutdown(&mut self) -> Result<(), String> {
        self.call(Writer::new().u32(SHUTDOWN)).map(|_| ())
    }
}

/// Lance le binaire du provider, detache (nouvelle session : il survit a
/// l'app qui l'a lance).
fn launch_provider(socket: &str) -> Result<(), String> {
    let program = crate::find_binary(crate::BINARY).ok_or_else(|| format!("'{}' introuvable (ni a cote de l'app, ni dans le PATH)", crate::BINARY))?;
    let mut command = Command::new(&program);
    command.arg("--socket").arg(socket).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    // SAFETY : `setsid` est sur entre fork et exec.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut child = command.spawn().map_err(|e| format!("{} : {e}", program.display()))?;
    // Ramasse le processus quand il sortira, sans bloquer l'app.
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Les expressions cote app.
pub struct Provider;

impl Provider {
    /// S'assure que le service tourne et repond (lance le provider s'il le
    /// faut). A appeler avant de se connecter a un daemon.
    pub fn ensure(name: &str) -> Result<(), String> {
        Provider::ensure_at(&crate::SOCKET_PATH, name)
    }

    pub fn ensure_at(socket: &str, name: &str) -> Result<(), String> {
        ProviderClient::connect_or_launch(socket)?.ensure(name, ENSURE_TIMEOUT)
    }

    pub fn status() -> Result<Vec<ServiceStatus>, String> {
        ProviderClient::connect()?.status()
    }

    pub fn start(name: &str) -> Result<(), String> {
        ProviderClient::connect_or_launch(&crate::SOCKET_PATH)?.start(name)
    }

    pub fn stop(name: &str) -> Result<(), String> {
        ProviderClient::connect()?.stop(name)
    }

    pub fn restart(name: &str) -> Result<(), String> {
        ProviderClient::connect_or_launch(&crate::SOCKET_PATH)?.restart(name)
    }

    pub fn unregister(name: &str) -> Result<(), String> {
        ProviderClient::connect()?.unregister(name)
    }
}

/// `Service::new(...)` cote app : meme chose que `ServiceSpec`.
pub type Service = ServiceSpec;

impl ServiceSpec {
    /// Confie le service au provider (lance s'il ne tourne pas) : il est
    /// lance tout de suite et relance s'il tombe.
    pub fn register(self) -> Result<(), String> {
        self.register_at(&crate::SOCKET_PATH)
    }

    pub fn register_at(self, socket: &str) -> Result<(), String> {
        ProviderClient::connect_or_launch(socket)?.register(&self)
    }

    /// Fait connaitre le service sans le lancer : il le sera a la
    /// premiere demande (`Provider::ensure`).
    pub fn declare(self) -> Result<(), String> {
        self.declare_at(&crate::SOCKET_PATH)
    }

    pub fn declare_at(self, socket: &str) -> Result<(), String> {
        ProviderClient::connect_or_launch(socket)?.declare(&self)
    }
}
