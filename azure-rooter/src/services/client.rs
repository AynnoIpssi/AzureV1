// Cote app : parler au routeur. Chaque requete part en une seule ecriture ;
// opcodes et format : voir `models::request`.
use crate::models::request::*;
use azure_core::models::wire::Writer;
use std::io::Write;
use std::os::unix::net::UnixStream;

fn request(connection: &mut UnixStream, request: Writer) -> Result<(), String> {
    connection.write_all(&request.finish()).map_err(|e| e.to_string())
}

/// Se connecte au routeur sur le chemin par defaut (voir `crate::SOCKET_PATH`).
/// Voir `register_at` pour un autre chemin (tests d'integration notamment -
/// voir sa documentation pour pourquoi ne JAMAIS partager le chemin reel).
pub fn register(app_id: u32) -> Result<UnixStream, String> {
    register_at(&crate::SOCKET_PATH, app_id)
}

pub fn register_at(path: &str, app_id: u32) -> Result<UnixStream, String> {
    let mut connection = UnixStream::connect(path).map_err(|e| e.to_string())?;
    request(&mut connection, Writer::new().u32(REGISTER).u32(app_id))?;
    Ok(connection)
}

pub fn send(connection: &mut UnixStream, sender_id: u32, receiver_id: u32, content: &str) -> Result<(), String> {
    request(connection, Writer::new().u32(SEND).u32(sender_id).u32(receiver_id).str(content))
}

pub fn subscribe(connection: &mut UnixStream, app_id: u32, event_content: &str) -> Result<(), String> {
    request(connection, Writer::new().u32(SUBSCRIBE).u32(app_id).str(event_content))
}

pub fn publish(connection: &mut UnixStream, app_id: u32, event_content: &str, content: &str) -> Result<(), String> {
    request(connection, Writer::new().u32(PUBLISH).u32(app_id).str(event_content).str(content))
}

/// Attend le prochain message livre a cette connexion.
pub fn receive(connection: &mut UnixStream) -> Result<String, String> {
    String::from_utf8(read_bytes(connection)?).map_err(|e| e.to_string())
}

pub fn unsubscribe(connection: &mut UnixStream, app_id: u32, event_content: &str) -> Result<(), String> {
    request(connection, Writer::new().u32(UNSUBSCRIBE).u32(app_id).str(event_content))
}

pub fn unregister(connection: &mut UnixStream, app_id: u32) -> Result<(), String> {
    request(connection, Writer::new().u32(UNREGISTER).u32(app_id))
}

/// `follower_id` s'abonne a l'app `followed_id` : il recevra ses fenetres
/// `WindowScope::Followers` (voir `send_window`).
pub fn follow(connection: &mut UnixStream, follower_id: u32, followed_id: u32) -> Result<(), String> {
    request(connection, Writer::new().u32(FOLLOW).u32(follower_id).u32(followed_id))
}

pub fn unfollow(connection: &mut UnixStream, follower_id: u32, followed_id: u32) -> Result<(), String> {
    request(connection, Writer::new().u32(UNFOLLOW).u32(follower_id).u32(followed_id))
}

/// Envoie une fenetre deja encodee (`content`) aux destinataires de
/// `scope_code` (voir `azure_core::models::window_model::WindowScope::code`).
/// Cote destinataire, elle arrive comme un message normal (voir `receive`).
pub fn send_window(connection: &mut UnixStream, sender_id: u32, scope_code: u32, content: &str) -> Result<(), String> {
    request(connection, Writer::new().u32(SEND_WINDOW).u32(sender_id).u32(scope_code).str(content))
}
