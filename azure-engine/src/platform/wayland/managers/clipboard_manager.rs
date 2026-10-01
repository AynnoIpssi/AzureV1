// Presse-papiers du systeme (protocole `wl_data_device`) : copier depuis
// une app Azure et coller ailleurs, et l'inverse.
//
// Copier : on cree une `wl_data_source` qui propose du texte, et on la
// declare selection du siege. Quand une app colle, le compositeur nous
// envoie `send(type, fd)` : on ecrit le texte dans ce fd.
//
// Coller : le compositeur annonce chaque selection par une `wl_data_offer`
// (evenement `data_offer`, puis ses types, puis `selection`). Pour la lire,
// on lui passe l'extremite d'ecriture d'un tube (`receive`) et on lit
// l'autre extremite jusqu'a la fin.
use crate::platform::wayland::models::clipboard::Clipboard;
use crate::platform::wayland::models::connection::WaylandConnection;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::time::{Duration, Instant};

/// Types proposes pour du texte, du plus precis au plus vague.
pub const TEXT_MIMES: [&str; 5] = ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain", "TEXT", "STRING"];

/// Au-dela, une app qui ne finit pas d'envoyer sa selection est abandonnee :
/// la fenetre ne doit pas rester figee.
const PASTE_TIMEOUT: Duration = Duration::from_secs(2);

// Opcodes des requetes.
const MANAGER_CREATE_DATA_SOURCE: u32 = 0;
const MANAGER_GET_DATA_DEVICE: u32 = 1;
const SOURCE_OFFER: u32 = 0;
const SOURCE_DESTROY: u32 = 1;
const DEVICE_SET_SELECTION: u32 = 1;
const OFFER_RECEIVE: u32 = 1;
const OFFER_DESTROY: u32 = 2;

// Opcodes des evenements.
const SOURCE_SEND: u16 = 1;
const SOURCE_CANCELLED: u16 = 2;
const DEVICE_DATA_OFFER: u16 = 0;
const DEVICE_ENTER: u16 = 1;
const DEVICE_LEAVE: u16 = 2;
const DEVICE_DROP: u16 = 4;
const DEVICE_SELECTION: u16 = 5;
const OFFER_OFFER: u16 = 0;

fn message(object: u32, opcode: u32, args: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(8 + args.len());
    msg.extend_from_slice(&object.to_le_bytes());
    msg.extend_from_slice(&((((8 + args.len()) as u32) << 16) | opcode).to_le_bytes());
    msg.extend_from_slice(args);
    msg
}

fn string_arg(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&((s.len() + 1) as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}

fn u32_at(args: &[u8], offset: usize) -> Option<u32> {
    args.get(offset..offset + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn read_string(args: &[u8], offset: usize) -> Option<String> {
    let len = u32_at(args, offset)? as usize;
    let bytes = args.get(offset + 4..offset + 4 + len.saturating_sub(1))?;
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// Lie le `wl_data_device` du siege (`manager_id` : le
/// `wl_data_device_manager` deja lie).
pub fn get_data_device(connection: &mut WaylandConnection, manager_id: u32, seat_id: u32, new_id: u32) -> Result<u32, String> {
    let mut args = new_id.to_le_bytes().to_vec();
    args.extend_from_slice(&seat_id.to_le_bytes());
    connection.send(&message(manager_id, MANAGER_GET_DATA_DEVICE, &args))?;
    Ok(new_id)
}

/// Rend la selection du systeme a nous : `text` sera envoye a qui colle.
/// `serial` : celui du dernier evenement clavier ou souris (le compositeur
/// refuse une selection qui ne suit pas une action de l'utilisateur).
pub fn set_selection(connection: &mut WaylandConnection, clipboard: &mut Clipboard, text: &str, serial: u32, new_id: u32) -> Result<(), String> {
    let Some(device_id) = clipboard.device_id else { return Ok(()) };
    if let Some(old) = clipboard.source_id.take() {
        connection.send(&message(old, SOURCE_DESTROY, &[]))?;
    }
    connection.send(&message(clipboard.manager_id, MANAGER_CREATE_DATA_SOURCE, &new_id.to_le_bytes()))?;
    for mime in TEXT_MIMES {
        connection.send(&message(new_id, SOURCE_OFFER, &string_arg(mime)))?;
    }
    let mut args = new_id.to_le_bytes().to_vec();
    args.extend_from_slice(&serial.to_le_bytes());
    connection.send(&message(device_id, DEVICE_SET_SELECTION, &args))?;
    clipboard.source_id = Some(new_id);
    clipboard.text = text.to_string();
    Ok(())
}

/// Le texte de la selection du systeme, `None` si elle est vide ou n'est
/// pas du texte.
pub fn selection_text(connection: &mut WaylandConnection, clipboard: &Clipboard) -> Option<String> {
    // La selection est a nous : inutile (et bloquant) de se l'envoyer.
    if clipboard.source_id.is_some() {
        return Some(clipboard.text.clone());
    }
    let offer = clipboard.selection?;
    let mimes = clipboard.offers.get(&offer)?;
    let mime = TEXT_MIMES.iter().find(|m| mimes.iter().any(|o| o == *m))?;

    let mut fds = [0 as RawFd; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return None;
    }
    let (read_fd, write_fd) = (fds[0], fds[1]);
    let sent = connection.send_with_fd(&message(offer, OFFER_RECEIVE, &string_arg(mime)), write_fd);
    // Le compositeur a sa copie : la notre fermee, la lecture voit la fin
    // quand l'autre app a fini d'ecrire.
    unsafe { libc::close(write_fd) };
    let mut pipe = unsafe { std::fs::File::from_raw_fd(read_fd) };
    sent.ok()?;

    let start = Instant::now();
    let mut data = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let left = PASTE_TIMEOUT.saturating_sub(start.elapsed());
        if left.is_zero() {
            return None;
        }
        let mut pfd = libc::pollfd { fd: read_fd, events: libc::POLLIN, revents: 0 };
        if unsafe { libc::poll(&mut pfd, 1, left.as_millis() as i32) } <= 0 {
            return None;
        }
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
    Some(String::from_utf8_lossy(&data).into_owned())
}

/// Traite un evenement du presse-papiers ; `false` s'il ne le concerne pas.
pub fn handle_event(connection: &mut WaylandConnection, clipboard: &mut Clipboard, object_id: u32, opcode: u16, args: &[u8]) -> bool {
    if Some(object_id) == clipboard.source_id {
        match opcode {
            SOURCE_SEND => {
                // Le fd arrive avec le message (voir `WaylandConnection::take_fd`).
                if let Some(fd) = connection.take_fd() {
                    let text = clipboard.text.clone();
                    // Ecrit a part : une app qui lit lentement ne bloque pas
                    // la fenetre.
                    std::thread::spawn(move || {
                        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
                        let _ = file.write_all(text.as_bytes());
                    });
                }
            }
            SOURCE_CANCELLED => {
                // Une autre app a pris la selection.
                let _ = connection.send(&message(object_id, SOURCE_DESTROY, &[]));
                clipboard.source_id = None;
            }
            _ => {}
        }
        return true;
    }
    if Some(object_id) == clipboard.device_id {
        match opcode {
            DEVICE_DATA_OFFER => {
                if let Some(id) = u32_at(args, 0) {
                    clipboard.offers.insert(id, Vec::new());
                }
            }
            DEVICE_SELECTION => {
                let new = u32_at(args, 0).filter(|&id| id != 0);
                if let Some(old) = clipboard.selection
                    && Some(old) != new
                {
                    destroy_offer(connection, clipboard, old);
                }
                clipboard.selection = new;
            }
            DEVICE_ENTER => {
                // serial, surface, x, y, offre
                clipboard.drag = u32_at(args, 16).filter(|&id| id != 0);
            }
            DEVICE_LEAVE | DEVICE_DROP => {
                if let Some(drag) = clipboard.drag.take()
                    && Some(drag) != clipboard.selection
                {
                    destroy_offer(connection, clipboard, drag);
                }
            }
            _ => {}
        }
        return true;
    }
    if let Some(mimes) = clipboard.offers.get_mut(&object_id) {
        if opcode == OFFER_OFFER
            && let Some(mime) = read_string(args, 0)
        {
            mimes.push(mime);
        }
        return true;
    }
    false
}

fn destroy_offer(connection: &mut WaylandConnection, clipboard: &mut Clipboard, offer: u32) {
    if clipboard.offers.remove(&offer).is_some() {
        let _ = connection.send(&message(offer, OFFER_DESTROY, &[]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_chaines_sont_terminees_et_alignees() {
        assert_eq!(string_arg("abc"), [4, 0, 0, 0, b'a', b'b', b'c', 0]);
        assert_eq!(string_arg("abcd").len(), 4 + 8);
        assert_eq!(read_string(&string_arg("text/plain"), 0).as_deref(), Some("text/plain"));
    }

    #[test]
    fn un_message_porte_sa_taille_et_son_opcode() {
        let m = message(7, 1, &[1, 2, 3, 4]);
        assert_eq!(u32_at(&m, 0), Some(7));
        assert_eq!(u32_at(&m, 4), Some((12 << 16) | 1));
    }
}
