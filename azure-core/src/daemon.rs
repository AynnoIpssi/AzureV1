// Ce que tous les daemons font pour ouvrir leur socket et accepter leurs
// clients : dossier prive, socket d'un daemon arrete retire, plafond de
// connexions, un thread par connexion.
use crate::security::limits::ConnectionGate;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;

/// Ouvre le socket d'un daemon. Le dossier est cree en 700 (et refuse s'il
/// n'est pas a nous, voir `paths::prepare_socket_dir`). Un fichier laisse
/// par un daemon arrete (crash, `kill -9` : Rust ne le retire jamais) est
/// supprime ; un daemon qui repond encore garde le sien, et on refuse.
pub fn bind(socket: &str) -> Result<UnixListener, String> {
    crate::paths::prepare_socket_dir(socket)?;
    if UnixStream::connect(socket).is_ok() {
        return Err(format!("{socket} : un daemon y repond deja"));
    }
    let _ = std::fs::remove_file(socket);
    UnixListener::bind(socket).map_err(|e| format!("{socket} : {e}"))
}

/// Accepte les clients pour toujours : chaque connexion admise
/// (`ConnectionGate`) est confiee a `handler` dans son propre thread ; au-dela
/// du plafond, elle est fermee aussitot.
pub fn serve(listener: UnixListener, handler: impl Fn(UnixStream) + Send + Sync + 'static) {
    let handler = Arc::new(handler);
    let gate = ConnectionGate::default();
    for stream in listener.incoming().flatten() {
        let Some(permit) = gate.admit(&stream) else { continue };
        let handler = Arc::clone(&handler);
        std::thread::spawn(move || {
            let _permit = permit;
            handler(stream)
        });
    }
}
