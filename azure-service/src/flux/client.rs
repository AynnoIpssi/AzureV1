// Cote app : partager un flux, ou en ecouter un.
//
// ```text
// // App B : partage son panier en temps reel.
// let flux = Flux::connect(APP_B)?;
// let mut panier = flux.share("panier").to(&[APP_A]).open()?;   // ou .public()
// panier.set("total", 42)?;
// panier.push("items", Value::map([("nom", "pomme".into()), ("prix", 2.into())]))?;
// panier.emit("paye", "carte")?;                                 // evenement libre
// panier.batch(|b| { b.set("total", 0); b.delete("items"); })?;  // tout ou rien
//
// // App A : recoit l'etat actuel puis chaque modification.
// let mut ecoute = Flux::connect(APP_A)?.listen(APP_B, "panier")
//     .path("total")                                   // seulement ce qui l'interesse
//     .on("total", |total| println!("total = {total}"))
//     .on_event("paye", |mode| println!("paye par {mode}"))
//     .start()?;
// loop { ecoute.wait(Duration::from_secs(1)); }        // ou ecoute.poll() dans une boucle d'app
// ```
use crate::flux::change::{Change, Filter};
use crate::flux::path::Path;
use crate::flux::protocol::*;
use crate::flux::value::Value;
use azure_core::models::wire::{Reader, Writer};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Delai entre deux tentatives de reconnexion d'une ecoute.
const RETRY: Duration = Duration::from_millis(300);

pub(crate) enum CallError {
    /// Connexion perdue : on peut se reconnecter et reessayer.
    Io(String),
    /// Le daemon a refuse (erreur definitive).
    Remote(String),
}

impl From<CallError> for String {
    fn from(error: CallError) -> String {
        match error {
            CallError::Io(e) => format!("azure-service injoignable : {e}"),
            CallError::Remote(e) => e,
        }
    }
}

pub(crate) fn hello(socket: &str, app: u32) -> Result<UnixStream, CallError> {
    let mut stream = UnixStream::connect(socket).map_err(|e| CallError::Io(format!("{socket} : {e}")))?;
    request(&mut stream, Writer::new().u32(HELLO).u32(app).finish())?;
    Ok(stream)
}

pub(crate) fn request(stream: &mut UnixStream, payload: Vec<u8>) -> Result<Vec<u8>, CallError> {
    write_frame(stream, &payload).map_err(CallError::Io)?;
    let response = read_frame(stream).map_err(CallError::Io)?;
    check_status(&response).map_err(CallError::Remote)?;
    Ok(response)
}

/// Connexion d'une app a azure-service. Se clone librement (les copies
/// partagent la connexion).
#[derive(Clone)]
pub struct Flux {
    stream: Arc<Mutex<UnixStream>>,
    app: u32,
    socket: String,
    // Connexion reservee aux appels (voir `crate::call`) : un appel attend
    // sa reponse sans bloquer les flux.
    pub(crate) caller: Arc<Mutex<Option<UnixStream>>>,
}

impl Flux {
    /// Se connecte au daemon par defaut, lance par azure-provider s'il ne
    /// tourne pas (un daemon lance a la main suffit aussi).
    pub fn connect(app: u32) -> Result<Flux, String> {
        let ensured = azure_provider::Provider::ensure("service");
        Flux::connect_at(&crate::SOCKET_PATH, app).map_err(|e| match ensured {
            Err(provider) => format!("{e} (azure-provider : {provider})"),
            Ok(()) => e,
        })
    }

    pub fn connect_at(socket: &str, app: u32) -> Result<Flux, String> {
        Ok(Flux { stream: Arc::new(Mutex::new(hello(socket, app)?)), app, socket: socket.to_string(), caller: Arc::new(Mutex::new(None)) })
    }

    pub fn app_id(&self) -> u32 {
        self.app
    }

    pub(crate) fn socket(&self) -> &str {
        &self.socket
    }

    fn send_request(&self, payload: Vec<u8>) -> Result<Vec<u8>, CallError> {
        let mut stream = self.stream.lock().unwrap_or_else(|e| e.into_inner());
        request(&mut stream, payload)
    }

    fn reconnect(&self) -> Result<(), CallError> {
        let fresh = hello(&self.socket, self.app)?;
        *self.stream.lock().unwrap_or_else(|e| e.into_inner()) = fresh;
        Ok(())
    }

    /// Partage un flux de cette app. Personne d'autre ne l'ecoute tant que
    /// `public()` ou `to(...)` n'est pas donne.
    pub fn share(&self, name: &str) -> ShareBuilder {
        ShareBuilder { flux: self.clone(), name: name.to_string(), access: Access::Apps(Vec::new()), persist: false }
    }

    /// Ecoute le flux `name` de l'app `owner`.
    pub fn listen(&self, owner: u32, name: &str) -> ListenerBuilder {
        ListenerBuilder { socket: self.socket.clone(), app: self.app, owner, name: name.to_string(), filter: Filter::default(), handlers: Vec::new(), errors: Vec::new() }
    }

    /// Change qui peut ecouter le flux `name` de l'app `owner`. Reserve a
    /// azure-manager (refuse pour toute autre app).
    pub fn set_access(&self, owner: u32, name: &str, access: &Access) -> Result<(), String> {
        self.send_request(access.write(Writer::new().u32(SET_ACCESS).u32(owner).str(name)).finish())?;
        Ok(())
    }

    /// Qui peut appeler la methode `method` de l'app `owner`. Reserve a
    /// azure-manager.
    pub fn set_call_access(&self, owner: u32, method: &str, access: &Access) -> Result<(), String> {
        self.send_request(access.write(Writer::new().u32(SET_CALL_ACCESS).u32(owner).str(method)).finish())?;
        Ok(())
    }

    /// Compteurs des flux et des appels. Reserve a azure-manager.
    pub fn stats(&self) -> Result<(Vec<FluxStats>, Vec<MethodStats>), String> {
        let response = self.send_request(Writer::new().u32(STATS).finish())?;
        let mut r = check_status(&response)?;
        let mut flux = Vec::new();
        for _ in 0..r.u32()? {
            flux.push(FluxStats { owner: r.u32()?, name: r.str()?, changes: r.u64()?, listeners: r.u32()?, persist: r.u8()? != 0 });
        }
        let mut methods = Vec::new();
        for _ in 0..r.u32()? {
            methods.push(MethodStats { owner: r.u32()?, method: r.str()?, calls: r.u64()?, errors: r.u64()?, timeouts: r.u64()?, total_ms: r.u64()?, last_error: r.str()?, served: r.u8()? != 0 });
        }
        Ok((flux, methods))
    }

    /// Les flux que cette app peut ecouter (les siens compris).
    pub fn streams(&self) -> Result<Vec<StreamInfo>, String> {
        let response = self.send_request(Writer::new().u32(LIST).finish())?;
        let mut r = check_status(&response)?;
        let count = r.u32()?;
        (0..count).map(|_| Ok(StreamInfo { owner: r.u32()?, name: r.str()?, seq: r.u64()?, public: r.u8()? != 0 })).collect()
    }
}

pub struct ShareBuilder {
    flux: Flux,
    name: String,
    access: Access,
    persist: bool,
}

impl ShareBuilder {
    /// Toute app peut ecouter.
    pub fn public(mut self) -> ShareBuilder {
        self.access = Access::Public;
        self
    }

    /// Seulement ces apps (en plus de celle qui partage).
    pub fn to(mut self, apps: &[u32]) -> ShareBuilder {
        self.access = Access::Apps(apps.to_vec());
        self
    }

    /// L'etat est garde sur disque par azure-service : il survit a son
    /// redemarrage (et a celui de la machine).
    pub fn persist(mut self, persist: bool) -> ShareBuilder {
        self.persist = persist;
        self
    }

    /// Cree le flux, ou le reprend avec son etat s'il existe deja (app
    /// relancee, ou flux persistant).
    pub fn open(self) -> Result<Shared, String> {
        let (seq, state) = share_call(&self.flux, &self.name, &self.access, self.persist)?;
        Ok(Shared { flux: self.flux, name: self.name, access: self.access, state, seq, persist: self.persist })
    }
}

fn share_call(flux: &Flux, name: &str, access: &Access, persist: bool) -> Result<(u64, Value), CallError> {
    let response = flux.send_request(access.write(Writer::new().u32(SHARE).str(name)).u8(persist as u8).finish())?;
    let mut r = check_status(&response).map_err(CallError::Remote)?;
    let seq = r.u64().map_err(CallError::Remote)?;
    Ok((seq, Value::read(&mut r).map_err(CallError::Remote)?))
}

/// Un flux partage par cette app. Garde une copie de l'etat : si le daemon
/// redemarre (il perd tout), la prochaine modification le lui renvoie.
pub struct Shared {
    flux: Flux,
    name: String,
    access: Access,
    state: Value,
    seq: u64,
    persist: bool,
}

impl Shared {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// L'etat actuel (tel que les ecoutes le voient).
    pub fn state(&self) -> &Value {
        &self.state
    }

    pub fn get(&self, path: &str) -> Option<&Value> {
        self.state.get(path)
    }

    /// Numero de la derniere modification.
    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn set(&mut self, path: &str, value: impl Into<Value>) -> Result<u64, String> {
        self.apply(vec![Change::set(path, value)])
    }

    pub fn delete(&mut self, path: &str) -> Result<u64, String> {
        self.apply(vec![Change::delete(path)])
    }

    pub fn push(&mut self, path: &str, value: impl Into<Value>) -> Result<u64, String> {
        self.apply(vec![Change::push(path, value)])
    }

    /// Evenement libre : envoye aux ecoutes, ne change pas l'etat.
    pub fn emit(&mut self, event: &str, value: impl Into<Value>) -> Result<u64, String> {
        self.apply(vec![Change::event(event, value)])
    }

    /// Plusieurs modifications d'un coup : toutes appliquees ou aucune, et
    /// recues ensemble par les ecoutes.
    pub fn batch(&mut self, build: impl FnOnce(&mut Batch)) -> Result<u64, String> {
        let mut batch = Batch::default();
        build(&mut batch);
        self.apply(batch.changes)
    }

    pub fn apply(&mut self, changes: Vec<Change>) -> Result<u64, String> {
        // Verifie d'abord sur la copie locale : une modification invalide
        // n'est meme pas envoyee.
        let mut next = self.state.clone();
        for change in &changes {
            change.apply(&mut next)?;
        }
        let payload = write_changes(Writer::new().u32(PUBLISH).str(&self.name), &changes).finish();
        let response = match self.flux.send_request(payload.clone()) {
            Err(CallError::Io(_)) => {
                // Daemon relance (ou connexion coupee) : on se reconnecte, on
                // repartage, et on lui rend l'etat s'il l'a perdu.
                self.flux.reconnect()?;
                let (seq, state) = share_call(&self.flux, &self.name, &self.access, self.persist)?;
                if seq == 0 && state != self.state {
                    self.flux.send_request(write_changes(Writer::new().u32(PUBLISH).str(&self.name), &[Change::Set(String::new(), self.state.clone())]).finish())?;
                }
                self.flux.send_request(payload)?
            }
            other => other?,
        };
        self.seq = check_status(&response)?.u64()?;
        self.state = next;
        Ok(self.seq)
    }

    /// Change qui peut ecouter. Les ecoutes qui n'ont plus le droit sont
    /// coupees (elles recoivent `FluxEvent::Denied`).
    pub fn share_with(&mut self, access: Access) -> Result<(), String> {
        share_call(&self.flux, &self.name, &access, self.persist)?;
        self.access = access;
        Ok(())
    }

    /// Supprime le flux et son etat ; les ecoutes recoivent
    /// `FluxEvent::Closed` et attendent qu'il soit partage a nouveau.
    pub fn close(self) -> Result<(), String> {
        self.flux.send_request(Writer::new().u32(CLOSE).str(&self.name).finish())?;
        Ok(())
    }
}

/// Modifications groupees (voir `Shared::batch`).
#[derive(Default)]
pub struct Batch {
    changes: Vec<Change>,
}

impl Batch {
    pub fn set(&mut self, path: &str, value: impl Into<Value>) -> &mut Batch {
        self.changes.push(Change::set(path, value));
        self
    }

    pub fn delete(&mut self, path: &str) -> &mut Batch {
        self.changes.push(Change::delete(path));
        self
    }

    pub fn push(&mut self, path: &str, value: impl Into<Value>) -> &mut Batch {
        self.changes.push(Change::push(path, value));
        self
    }

    pub fn emit(&mut self, event: &str, value: impl Into<Value>) -> &mut Batch {
        self.changes.push(Change::event(event, value));
        self
    }
}

// ---- Ecoute ----

/// Ce que recoit une ecoute (voir `Listener::poll`).
#[derive(Clone, Debug, PartialEq)]
pub enum FluxEvent {
    /// Etat complet recu (au branchement, ou apres une reconnexion) : la
    /// copie locale vient d'etre remplacee.
    Snapshot { seq: u64 },
    /// Modifications (deja appliquees a la copie locale).
    Update { seq: u64, changes: Vec<Change> },
    /// Le flux a ete ferme par son app ; l'ecoute attend qu'il revienne.
    Closed,
    /// L'app ne partage pas (ou plus) ce flux avec nous : ecoute terminee.
    Denied(String),
    /// Connexion perdue : reconnexion automatique en cours.
    Disconnected,
}

impl FluxEvent {
    /// Vrai si l'evenement touche `path` : un etat complet, ou une
    /// modification de `path`, d'un parent ou d'un enfant.
    pub fn touches(&self, path: &str) -> bool {
        let Ok(pattern) = Path::pattern(path) else { return false };
        match self {
            FluxEvent::Snapshot { .. } => true,
            FluxEvent::Update { changes, .. } => changes.iter().any(|c| c.path().and_then(|p| Path::parse(p).ok()).is_some_and(|p| pattern.related(&p))),
            _ => false,
        }
    }

    /// Les evenements libres `name` de cette mise a jour.
    pub fn events<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
        let changes: &[Change] = match self {
            FluxEvent::Update { changes, .. } => changes,
            _ => &[],
        };
        changes.iter().filter_map(move |c| match c {
            Change::Event(event, value) if event == name => Some(value),
            _ => None,
        })
    }
}

enum Handler {
    Path(String, Box<dyn FnMut(&Value) + Send>),
    Event(String, Box<dyn FnMut(&Value) + Send>),
}

pub struct ListenerBuilder {
    socket: String,
    app: u32,
    owner: u32,
    name: String,
    filter: Filter,
    handlers: Vec<Handler>,
    errors: Vec<String>,
}

impl ListenerBuilder {
    /// Ne recevoir que cette partie de l'etat (`"panier.items.*.prix"`).
    /// Plusieurs `path` s'additionnent ; aucun = tout l'etat.
    pub fn path(mut self, pattern: &str) -> ListenerBuilder {
        match Path::pattern(pattern) {
            Ok(path) => self.filter.paths.push(path),
            // Signalee par `start`.
            Err(e) => self.errors.push(e),
        }
        self
    }

    /// Ne recevoir que ces evenements libres. Aucun = tous.
    pub fn event(mut self, name: &str) -> ListenerBuilder {
        self.filter.events.push(name.to_string());
        self
    }

    /// Appele avec la valeur a `path` (`Null` si absente) a chaque fois
    /// qu'elle change, et une premiere fois a la reception de l'etat. Lance
    /// depuis `poll` / `wait`, dans le thread de l'app.
    pub fn on(mut self, path: &str, handler: impl FnMut(&Value) + Send + 'static) -> ListenerBuilder {
        self.handlers.push(Handler::Path(path.to_string(), Box::new(handler)));
        self
    }

    /// Appele avec le contenu de chaque evenement libre `name`.
    pub fn on_event(mut self, name: &str, handler: impl FnMut(&Value) + Send + 'static) -> ListenerBuilder {
        self.handlers.push(Handler::Event(name.to_string(), Box::new(handler)));
        self
    }

    /// Branche l'ecoute. Erreur tout de suite si le daemon est injoignable
    /// ou si l'app `owner` ne partage pas ce flux avec nous. Un flux pas
    /// encore partage est attendu.
    pub fn start(self) -> Result<Listener, String> {
        if let Some(error) = self.errors.first() {
            return Err(error.clone());
        }
        for handler in &self.handlers {
            if let Handler::Path(path, _) = handler {
                Path::parse(path)?;
            }
        }
        let first = open_listen(&self.socket, self.app, self.owner, &self.name, &self.filter)?;
        let (tx, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let current = Arc::new(Mutex::new(first.try_clone().ok()));
        {
            let (stop, current) = (Arc::clone(&stop), Arc::clone(&current));
            let (socket, app, owner, name, filter) = (self.socket.clone(), self.app, self.owner, self.name.clone(), self.filter.clone());
            std::thread::spawn(move || listen_loop(first, tx, stop, current, &socket, app, owner, &name, &filter));
        }
        Ok(Listener { rx, state: Value::empty_map(), seq: 0, connected: true, finished: false, handlers: self.handlers, stop, current })
    }
}

fn open_listen(socket: &str, app: u32, owner: u32, name: &str, filter: &Filter) -> Result<UnixStream, CallError> {
    let mut stream = hello(socket, app)?;
    request(&mut stream, filter.write(Writer::new().u32(LISTEN).u32(owner).str(name)).finish())?;
    Ok(stream)
}

enum Received {
    Snapshot(u64, Value),
    Update(u64, Vec<Change>),
    Closed,
    Denied(String),
    Disconnected,
}

fn decode(frame: &[u8]) -> Result<Received, String> {
    let mut r = Reader::new(frame);
    Ok(match r.u8()? {
        PUSH_SNAPSHOT => Received::Snapshot(r.u64()?, Value::read(&mut r)?),
        PUSH_UPDATE => Received::Update(r.u64()?, read_changes(&mut r)?),
        PUSH_CLOSED => Received::Closed,
        PUSH_DENIED => Received::Denied(r.str()?),
        other => return Err(format!("Envoi inconnu : {other}")),
    })
}

#[allow(clippy::too_many_arguments)]
fn listen_loop(first: UnixStream, tx: Sender<Received>, stop: Arc<AtomicBool>, current: Arc<Mutex<Option<UnixStream>>>, socket: &str, app: u32, owner: u32, name: &str, filter: &Filter) {
    let mut stream = Some(first);
    while !stop.load(Ordering::SeqCst) {
        if let Some(mut connection) = stream.take() {
            while let Ok(frame) = read_frame(&mut connection) {
                let Ok(received) = decode(&frame) else { break };
                let denied = matches!(received, Received::Denied(_));
                if tx.send(received).is_err() || denied {
                    return;
                }
            }
            if stop.load(Ordering::SeqCst) || tx.send(Received::Disconnected).is_err() {
                return;
            }
        }
        std::thread::sleep(RETRY);
        match open_listen(socket, app, owner, name, filter) {
            Ok(connection) => {
                if let Ok(mut slot) = current.lock() {
                    *slot = connection.try_clone().ok();
                }
                stream = Some(connection);
            }
            Err(CallError::Remote(message)) => {
                let _ = tx.send(Received::Denied(message));
                return;
            }
            Err(CallError::Io(_)) => {}
        }
    }
}

/// Une ecoute branchee. Garde une copie de la partie ecoutee de l'etat,
/// mise a jour par `poll` / `wait`.
pub struct Listener {
    rx: Receiver<Received>,
    state: Value,
    seq: u64,
    connected: bool,
    finished: bool,
    handlers: Vec<Handler>,
    stop: Arc<AtomicBool>,
    current: Arc<Mutex<Option<UnixStream>>>,
}

impl Listener {
    /// Copie locale de ce qui est ecoute.
    pub fn state(&self) -> &Value {
        &self.state
    }

    pub fn get(&self, path: &str) -> Option<&Value> {
        self.state.get(path)
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Vrai une fois l'ecoute refusee (`FluxEvent::Denied`).
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Traite tout ce qui est arrive, sans attendre : copie locale mise a
    /// jour, rappels `on` / `on_event` lances. Retourne les evenements.
    pub fn poll(&mut self) -> Vec<FluxEvent> {
        let mut events = Vec::new();
        while let Ok(received) = self.rx.try_recv() {
            events.push(self.handle(received));
        }
        events
    }

    /// Comme `poll`, en attendant au plus `timeout` le premier evenement.
    pub fn wait(&mut self, timeout: Duration) -> Vec<FluxEvent> {
        match self.rx.recv_timeout(timeout) {
            Ok(received) => {
                let mut events = vec![self.handle(received)];
                events.extend(self.poll());
                events
            }
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => Vec::new(),
        }
    }

    fn handle(&mut self, received: Received) -> FluxEvent {
        let event = match received {
            Received::Snapshot(seq, state) => {
                self.state = state;
                self.seq = seq;
                self.connected = true;
                FluxEvent::Snapshot { seq }
            }
            Received::Update(seq, changes) => {
                for change in &changes {
                    // La copie locale ne contient que la partie ecoutee : une
                    // modification qui ne s'y applique pas n'y a pas sa place.
                    let _ = change.apply(&mut self.state);
                }
                self.seq = seq;
                FluxEvent::Update { seq, changes }
            }
            Received::Closed => {
                self.state = Value::empty_map();
                FluxEvent::Closed
            }
            Received::Denied(message) => {
                self.finished = true;
                self.connected = false;
                FluxEvent::Denied(message)
            }
            Received::Disconnected => {
                self.connected = false;
                FluxEvent::Disconnected
            }
        };
        for handler in &mut self.handlers {
            match handler {
                Handler::Path(path, call) if event.touches(path) => call(self.state.get(path).unwrap_or(&Value::Null)),
                Handler::Event(name, call) => event.events(name).for_each(call),
                _ => {}
            }
        }
        event
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(slot) = self.current.lock()
            && let Some(stream) = slot.as_ref() {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
    }
}
