// Mettre une fenetre au premier plan (protocole `xdg-activation-v1`).
//
// Une app ne peut pas se mettre devant les autres d'elle-meme : le
// compositeur l'interdit (vol de focus). L'app ou l'utilisateur vient
// d'agir demande un jeton lie a cette action (le serial du clic), le passe a
// l'app a montrer, qui s'active avec : `activate(jeton, sa surface)`.
use crate::platform::wayland::models::connection::WaylandConnection;

// Requetes de `xdg_activation_v1`.
const GET_ACTIVATION_TOKEN: u32 = 1;
const ACTIVATE: u32 = 2;
// Requetes de `xdg_activation_token_v1`.
const TOKEN_SET_SERIAL: u32 = 0;
const TOKEN_SET_SURFACE: u32 = 2;
const TOKEN_COMMIT: u32 = 3;
const TOKEN_DESTROY: u32 = 4;
/// Evenement `xdg_activation_token_v1::done(jeton)`.
pub const TOKEN_DONE: u16 = 0;

pub fn message(object: u32, opcode: u32, args: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(8 + args.len());
    msg.extend_from_slice(&object.to_le_bytes());
    msg.extend_from_slice(&((((8 + args.len()) as u32) << 16) | opcode).to_le_bytes());
    msg.extend_from_slice(args);
    msg
}

pub fn string_arg(s: &str) -> Vec<u8> {
    let mut out = ((s.len() + 1) as u32).to_le_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out.push(0);
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}

/// Le jeton d'un evenement `done`.
pub fn read_token(args: &[u8]) -> Option<String> {
    let len = u32::from_le_bytes(args.get(0..4)?.try_into().ok()?) as usize;
    let bytes = args.get(4..4 + len.saturating_sub(1))?;
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// Demande un jeton pour `surface`, lie a l'action `serial` sur `seat`. Le
/// jeton arrive plus tard (evenement `done` sur `token_id`).
pub fn request_token(connection: &mut WaylandConnection, activation_id: u32, token_id: u32, serial: u32, seat_id: u32, surface_id: u32) -> Result<(), String> {
    connection.send(&message(activation_id, GET_ACTIVATION_TOKEN, &token_id.to_le_bytes()))?;
    let mut args = serial.to_le_bytes().to_vec();
    args.extend_from_slice(&seat_id.to_le_bytes());
    connection.send(&message(token_id, TOKEN_SET_SERIAL, &args))?;
    connection.send(&message(token_id, TOKEN_SET_SURFACE, &surface_id.to_le_bytes()))?;
    connection.send(&message(token_id, TOKEN_COMMIT, &[]))
}

pub fn destroy_token(connection: &mut WaylandConnection, token_id: u32) -> Result<(), String> {
    connection.send(&message(token_id, TOKEN_DESTROY, &[]))
}

/// Met `surface` au premier plan avec `token`.
pub fn activate(connection: &mut WaylandConnection, activation_id: u32, token: &str, surface_id: u32) -> Result<(), String> {
    let mut args = string_arg(token);
    args.extend_from_slice(&surface_id.to_le_bytes());
    connection.send(&message(activation_id, ACTIVATE, &args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jeton_aller_retour() {
        assert_eq!(read_token(&string_arg("abc-123")).as_deref(), Some("abc-123"));
        let m = message(9, ACTIVATE, &[0; 8]);
        assert_eq!(u32::from_le_bytes(m[4..8].try_into().unwrap()), (16 << 16) | 2);
    }
}
