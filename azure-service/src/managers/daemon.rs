// Le daemon azure-service : une connexion = une app (HELLO verifie contre
// son vrai executable, voir `AppRegistry`). Une connexion sert soit a
// publier (requete / reponse), soit, apres LISTEN, a recevoir un flux.
use crate::call::broker::Broker;
use crate::flux::change::Filter;
use crate::flux::hub::Hub;
use crate::flux::protocol::*;
use crate::flux::value::Value;
use crate::managers::registry::AppRegistry;
use azure_core::managers::identity::peer_exe;
use azure_core::models::wire::{Reader, Writer};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};

static MANAGER_SOCKET: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Socket d'azure-manager a interroger sur l'identite des apps.
pub fn set_manager_socket(socket: &str) {
    let _ = MANAGER_SOCKET.set(socket.to_string());
}

fn manager_socket_default() -> String {
    MANAGER_SOCKET.get().cloned().unwrap_or_else(|| azure_core::security::registry::MANAGER_SOCKET.to_string())
}

/// Envois en attente par ecoute avant de la couper (voir `Hub`).
const LISTEN_QUEUE: usize = 4096;

struct Shared {
    hub: Mutex<Hub>,
    broker: Mutex<Broker>,
    registry: Mutex<AppRegistry>,
    /// Executable d'azure-manager, seul autorise a SET_ACCESS.
    manager_exe: Option<String>,
    /// Socket d'azure-manager a interroger sur l'identite des apps.
    manager_socket: String,
}

/// `manager_daemon` a cote de l'executable du daemon (meme dossier de
/// build ou d'installation).
pub fn default_manager_exe() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("manager_daemon").to_string_lossy().into_owned())
}

/// Bloque tant que le daemon tourne.
pub fn start_daemon_at(socket: &str, data_dir: &Path) -> Result<(), String> {
    start_daemon_with(socket, data_dir, default_manager_exe())
}

/// Comme `start_daemon_at`, en precisant l'executable d'azure-manager.
pub fn start_daemon_with(socket: &str, data_dir: &Path, manager_exe: Option<String>) -> Result<(), String> {
    start_daemon_for(socket, data_dir, manager_exe, None)
}

/// Comme `start_daemon_with`, avec le socket d'azure-manager de CE daemon
/// (sinon `set_manager_socket`, sinon celui par defaut) : plusieurs daemons
/// d'un meme processus (tests) peuvent ainsi avoir chacun leur manager.
pub fn start_daemon_for(socket: &str, data_dir: &Path, manager_exe: Option<String>, manager_socket: Option<String>) -> Result<(), String> {
    let manager_socket = manager_socket.unwrap_or_else(manager_socket_default);
    let shared = Arc::new(Shared { hub: Mutex::new(Hub::with_store(&data_dir.join("flux"))?), broker: Mutex::new(Broker::new()), registry: Mutex::new(AppRegistry::open(data_dir)?), manager_exe, manager_socket });
    let listener = azure_core::daemon::bind(socket)?;
    azure_core::daemon::serve(listener, move |stream| handle_connection(stream, Arc::clone(&shared)));
    Ok(())
}

fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    // Un thread qui a plante en le tenant ne bloque pas les autres.
    Ok(mutex.lock().unwrap_or_else(|e| e.into_inner()))
}

fn respond(stream: &mut UnixStream, result: Result<Vec<u8>, String>) -> bool {
    write_frame(stream, &response(result)).is_ok()
}

fn handle_connection(mut stream: UnixStream, shared: Arc<Shared>) {
    let mut app: Option<u32> = None;
    while let Ok(request) = read_frame(&mut stream) {
        let mut r = Reader::new(&request);
        let Ok(opcode) = r.u32() else { break };
        if opcode == LISTEN {
            // La connexion devient une ecoute, jusqu'a sa fermeture.
            return listen(stream, &shared, app, &mut r);
        }
        if opcode == SERVE {
            // La connexion sert une methode, jusqu'a sa fermeture.
            return serve(stream, &shared, app, &mut r);
        }
        let result = handle_request(opcode, &mut r, &mut app, &stream, &shared);
        if !respond(&mut stream, result) {
            break;
        }
    }
}

fn handle_request(opcode: u32, r: &mut Reader, app: &mut Option<u32>, stream: &UnixStream, shared: &Shared) -> Result<Vec<u8>, String> {
    if opcode == HELLO {
        let id = r.u32()?;
        r.finish()?;
        if app.is_some() {
            return Err("HELLO deja fait sur cette connexion".to_string());
        }
        // L'id du manager : seulement son executable, jamais lie au premier
        // venu (un manager de developpement l'aurait pris pour de bon, et le
        // vrai n'aurait plus jamais pu publier l'etat).
        if id == crate::MANAGER_APP_ID
            && let Some(manager) = &shared.manager_exe
        {
            let exe = peer_exe(stream)?;
            if exe != *manager {
                return Err(format!("L'app {id} est reservee a azure-manager ({manager}), pas a {exe}"));
            }
            *app = Some(id);
            return Ok(Vec::new());
        }
        // App connue d'azure-manager : il fait foi (meme empreinte). Sinon :
        // premier executable arrive.
        if azure_core::security::registry::verify(&shared.manager_socket, id, stream)?.is_none() {
            lock(&shared.registry)?.bind(id, &peer_exe(stream)?)?;
        }
        *app = Some(id);
        return Ok(Vec::new());
    }
    let app = app.ok_or("HELLO attendu avant toute requete")?;
    let out = Writer::new();
    Ok(match opcode {
        SHARE => {
            let name = r.str()?;
            let access = Access::read(r)?;
            let persist = r.u8()? != 0;
            r.finish()?;
            let (seq, state) = lock(&shared.hub)?.share_persistent(app, &name, access, persist)?;
            state.write(out.u64(seq)).finish()
        }
        PUBLISH => {
            let name = r.str()?;
            let changes = read_changes(r)?;
            r.finish()?;
            out.u64(lock(&shared.hub)?.publish(app, &name, &changes)?).finish()
        }
        CLOSE => {
            let name = r.str()?;
            r.finish()?;
            lock(&shared.hub)?.close(app, &name)?;
            out.finish()
        }
        LIST => {
            r.finish()?;
            let list = lock(&shared.hub)?.streams(app);
            list.iter().fold(out.u32(list.len() as u32), |w, s| w.u32(s.owner).str(&s.name).u64(s.seq).u8(s.public as u8)).finish()
        }
        CALL => {
            let owner = r.u32()?;
            let method = r.str()?;
            let timeout = std::time::Duration::from_millis(r.u32()? as u64).min(crate::call::MAX_TIMEOUT);
            let args = Value::read(r)?;
            r.finish()?;
            let started = std::time::Instant::now();
            // Resultat pris a part : le verrou est rendu avant de le lire.
            let started_call = lock(&shared.broker)?.start(app, owner, &method, &args);
            let (id, reply) = match started_call {
                Ok(ok) => ok,
                Err(e) => {
                    lock(&shared.broker)?.record(owner, &method, started.elapsed(), Err((false, e.clone())));
                    return Err(e);
                }
            };
            // Le verrou est relache pendant l'attente : les autres appels et
            // les reponses continuent.
            match reply.recv_timeout(timeout) {
                Ok(Ok(value)) => {
                    lock(&shared.broker)?.record(owner, &method, started.elapsed(), Ok(()));
                    value.write(out).finish()
                }
                Ok(Err(e)) => {
                    lock(&shared.broker)?.record(owner, &method, started.elapsed(), Err((false, e.clone())));
                    return Err(e);
                }
                Err(_) => {
                    let message = format!("'{method}' de l'app {owner} n'a pas repondu en {} ms", timeout.as_millis());
                    let mut broker = lock(&shared.broker)?;
                    broker.cancel(id);
                    broker.record(owner, &method, started.elapsed(), Err((true, message.clone())));
                    return Err(message);
                }
            }
        }
        SET_CALL_ACCESS => {
            let owner = r.u32()?;
            let method = r.str()?;
            let access = Access::read(r)?;
            r.finish()?;
            let exe = peer_exe(stream)?;
            if shared.manager_exe.as_deref() != Some(exe.as_str()) {
                return Err("SET_CALL_ACCESS reserve a azure-manager".to_string());
            }
            lock(&shared.broker)?.set_access(owner, &method, access)?;
            out.finish()
        }
        STATS => {
            r.finish()?;
            let exe = peer_exe(stream)?;
            if shared.manager_exe.as_deref() != Some(exe.as_str()) {
                return Err("STATS reserve a azure-manager".to_string());
            }
            let flux = lock(&shared.hub)?.stats();
            let calls = lock(&shared.broker)?.stats();
            let mut w = out.u32(flux.len() as u32);
            for (owner, name, changes, listeners, persist) in flux {
                w = w.u32(owner).str(&name).u64(changes).u32(listeners as u32).u8(persist as u8);
            }
            w = w.u32(calls.len() as u32);
            for (owner, method, s, served) in calls {
                w = w.u32(owner).str(&method).u64(s.calls).u64(s.errors).u64(s.timeouts).u64(s.total_ms).str(&s.last_error).u8(served as u8);
            }
            w.finish()
        }
        SET_ACCESS => {
            let owner = r.u32()?;
            let name = r.str()?;
            let access = Access::read(r)?;
            r.finish()?;
            let exe = peer_exe(stream)?;
            if shared.manager_exe.as_deref() != Some(exe.as_str()) {
                return Err("SET_ACCESS reserve a azure-manager".to_string());
            }
            lock(&shared.hub)?.set_access(owner, &name, access)?;
            out.finish()
        }
        other => return Err(format!("Opcode inconnu : {other}")),
    })
}

fn listen(mut stream: UnixStream, shared: &Shared, app: Option<u32>, r: &mut Reader) {
    let registered = (|| {
        let app = app.ok_or("HELLO attendu avant LISTEN")?;
        let owner = r.u32()?;
        let name = r.str()?;
        let filter = Filter::read(r)?;
        r.finish()?;
        let (tx, rx) = sync_channel(LISTEN_QUEUE);
        let id = lock(&shared.hub)?.listen(app, owner, &name, filter, tx)?;
        Ok::<_, String>((id, rx))
    })();
    let (id, rx) = match registered {
        Ok(ok) => ok,
        Err(message) => {
            respond(&mut stream, Err(message));
            return;
        }
    };
    if !respond(&mut stream, Ok(Vec::new())) {
        if let Ok(mut hub) = lock(&shared.hub) {
            hub.unlisten(id);
        }
        return;
    }

    // Le client ne parle plus : une lecture qui se termine = il est parti.
    // On retire alors l'ecoute, ce qui ferme le canal et arrete l'ecriture.
    if let Ok(mut reader) = stream.try_clone() {
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        std::thread::spawn(move || {
            let mut byte = [0u8; 1];
            let _ = std::io::Read::read(&mut reader, &mut byte);
            let _ = done_tx.send(());
        });
        while done_rx.try_recv().is_err() {
            match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(push) => {
                    if write_frame(&mut stream, &push).is_err() {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                // Coupee par le hub (trop lente, ou plus autorisee apres le
                // dernier envoi).
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }
    if let Ok(mut hub) = lock(&shared.hub) {
        hub.unlisten(id);
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

/// Connexion d'une app qui sert une methode : les demandes lui sont
/// envoyees (thread d'ecriture), ses reponses lues ici.
fn serve(mut stream: UnixStream, shared: &Shared, app: Option<u32>, r: &mut Reader) {
    let registered = (|| {
        let app = app.ok_or("HELLO attendu avant SERVE")?;
        let method = r.str()?;
        let access = Access::read(r)?;
        r.finish()?;
        let (tx, rx) = sync_channel(LISTEN_QUEUE);
        let server = lock(&shared.broker)?.serve(app, &method, access, tx)?;
        Ok::<_, String>((server, rx))
    })();
    let (server, rx) = match registered {
        Ok(ok) => ok,
        Err(message) => {
            respond(&mut stream, Err(message));
            return;
        }
    };
    if respond(&mut stream, Ok(Vec::new())) {
        if let Ok(mut writer) = stream.try_clone() {
            std::thread::spawn(move || {
                // Se termine quand le broker oublie ce serveur (canal ferme).
                while let Ok(push) = rx.recv() {
                    if write_frame(&mut writer, &push).is_err() {
                        let _ = writer.shutdown(std::net::Shutdown::Both);
                        break;
                    }
                }
            });
        }
        while let Ok(frame) = read_frame(&mut stream) {
            let mut r = Reader::new(&frame);
            let answer = (|| {
                let id = r.u64()?;
                let result = if r.u8()? == 1 { Ok(Value::read(&mut r)?) } else { Err(r.str()?) };
                Ok::<_, String>((id, result))
            })();
            match answer {
                Ok((id, result)) => {
                    if let Ok(mut broker) = lock(&shared.broker) {
                        broker.finish(server, id, result);
                    }
                }
                Err(_) => break,
            }
        }
    }
    if let Ok(mut broker) = lock(&shared.broker) {
        broker.server_gone(server);
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);
}
