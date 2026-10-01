// Cote app : servir une methode, ou en appeler une.
//
// ```text
// // App boutique : repond aux demandes de prix.
// let serveur = flux.serve("prix", Access::Apps(vec![CAISSE]), |demande| {
//     let id = demande.args.get("produit").and_then(Value::as_i64).ok_or("produit attendu")?;
//     Ok(Value::from(prix_de(id)))
// })?;                                   // repond tant que `serveur` existe
//
// // App caisse : demande et attend la reponse.
// let prix = flux.call(BOUTIQUE, "prix", Value::map([("produit", 42.into())]), DEFAULT_TIMEOUT)?;
// ```
use crate::flux::client::{hello, request, CallError, Flux};
use crate::flux::protocol::*;
use crate::flux::value::Value;
use azure_core::models::wire::{Reader, Writer};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Delai d'un appel sans precision.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

const RETRY: Duration = Duration::from_millis(300);

/// Une demande recue par l'app qui sert la methode.
#[derive(Clone, Debug, PartialEq)]
pub struct CallRequest {
    /// Id de l'app qui appelle (verifie par le daemon).
    pub caller: u32,
    pub args: Value,
}

type Handler = dyn Fn(&CallRequest) -> Result<Value, String> + Send + Sync;

/// Une methode servie. Elle ne l'est plus quand cet objet disparait.
pub struct Server {
    stop: Arc<AtomicBool>,
    current: Arc<Mutex<Option<UnixStream>>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(slot) = self.current.lock()
            && let Some(stream) = slot.as_ref() {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
    }
}

fn open_server(socket: &str, app: u32, method: &str, access: &Access) -> Result<UnixStream, CallError> {
    let mut stream = hello(socket, app)?;
    request(&mut stream, access.write(Writer::new().u32(SERVE).str(method)).finish())?;
    Ok(stream)
}

/// Une connexion deja ouverte est-elle encore vivante ? (Le daemon a pu
/// redemarrer depuis : on se reconnecte AVANT d'envoyer, pour ne jamais
/// rejouer un appel deja parti.)
fn alive(stream: &UnixStream) -> bool {
    use std::os::unix::io::AsRawFd;
    let mut byte = [0u8; 1];
    // SAFETY : lecture d'un octet au plus dans `byte`, sans le consommer.
    let n = unsafe { libc::recv(stream.as_raw_fd(), byte.as_mut_ptr() as *mut libc::c_void, 1, libc::MSG_PEEK | libc::MSG_DONTWAIT) };
    // Rien a lire (-1, EAGAIN) = vivante ; 0 = fermee par le daemon.
    n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock
}

impl Flux {
    /// Sert la methode `method` : `handler` recoit chaque demande (dans un
    /// thread a part, plusieurs a la fois) et retourne la reponse ou une
    /// erreur, transmise telle quelle a l'appelant. `access` : qui peut
    /// appeler (le manager peut le changer ensuite). Reconnexion
    /// automatique si le daemon redemarre.
    pub fn serve(&self, method: &str, access: Access, handler: impl Fn(&CallRequest) -> Result<Value, String> + Send + Sync + 'static) -> Result<Server, String> {
        let first = open_server(self.socket(), self.app_id(), method, &access)?;
        let stop = Arc::new(AtomicBool::new(false));
        let current = Arc::new(Mutex::new(first.try_clone().ok()));
        let handler: Arc<Handler> = Arc::new(handler);
        {
            let (stop, current) = (Arc::clone(&stop), Arc::clone(&current));
            let (socket, app, method) = (self.socket().to_string(), self.app_id(), method.to_string());
            std::thread::spawn(move || {
                let mut connection = Some(first);
                while !stop.load(Ordering::SeqCst) {
                    if let Some(stream) = connection.take() {
                        serve_requests(stream, &handler);
                    }
                    if stop.load(Ordering::SeqCst) {
                        return;
                    }
                    std::thread::sleep(RETRY);
                    match open_server(&socket, app, &method, &access) {
                        Ok(stream) => {
                            if let Ok(mut slot) = current.lock() {
                                *slot = stream.try_clone().ok();
                            }
                            connection = Some(stream);
                        }
                        // Refus definitif (nom invalide...) : on arrete.
                        Err(CallError::Remote(_)) => return,
                        Err(CallError::Io(_)) => {}
                    }
                }
            });
        }
        Ok(Server { stop, current })
    }

    /// Appelle `method` de l'app `owner` et attend la reponse (au plus
    /// `timeout`, borne a 5 min). L'erreur de l'app appelee est retournee
    /// telle quelle.
    pub fn call(&self, owner: u32, method: &str, args: impl Into<Value>, timeout: Duration) -> Result<Value, String> {
        let timeout = timeout.min(crate::call::MAX_TIMEOUT);
        let payload = args.into().write(Writer::new().u32(CALL).u32(owner).str(method).u32(timeout.as_millis() as u32)).finish();
        // Connexion reservee aux appels ; si un autre thread s'en sert deja,
        // une connexion de plus pour ne pas l'attendre.
        let response = match self.caller.try_lock() {
            Ok(mut slot) => {
                if !slot.as_ref().is_some_and(alive) {
                    *slot = Some(hello(self.socket(), self.app_id())?);
                }
                let result = request(slot.as_mut().expect("connecte juste au-dessus"), payload);
                if matches!(result, Err(CallError::Io(_))) {
                    *slot = None;
                }
                result?
            }
            Err(_) => request(&mut hello(self.socket(), self.app_id())?, payload)?,
        };
        Value::read(&mut check_status(&response)?)
    }
}

/// Lit les demandes jusqu'a la fermeture de la connexion ; chaque demande
/// est traitee dans son propre thread.
fn serve_requests(stream: UnixStream, handler: &Arc<Handler>) {
    let Ok(writer) = stream.try_clone() else { return };
    let writer = Arc::new(Mutex::new(writer));
    let mut reader = stream;
    while let Ok(frame) = read_frame(&mut reader) {
        let mut r = Reader::new(&frame);
        let Ok(PUSH_REQUEST) = r.u8() else { continue };
        let (Ok(id), Ok(caller), Ok(args)) = (r.u64(), r.u32(), Value::read(&mut r)) else { continue };
        let (handler, writer) = (Arc::clone(handler), Arc::clone(&writer));
        std::thread::spawn(move || {
            let request = CallRequest { caller, args };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(&request))).unwrap_or_else(|_| Err("l'app a plante en repondant".to_string()));
            let answer = match result {
                Ok(value) => value.write(Writer::new().u64(id).u8(1)),
                Err(message) => Writer::new().u64(id).u8(0).str(&message),
            };
            if let Ok(mut writer) = writer.lock() {
                let _ = write_frame(&mut *writer, &answer.finish());
            }
        });
    }
}
