// Protocole du socket du routeur. Contrairement aux autres daemons, les
// requetes ne sont pas encadrees et n'ont pas de reponse : opcode u32 puis
// ses champs, au format de `azure_core::models::wire` (entiers u32
// little-endian, textes precedes de leur taille u32). Ce que le routeur
// livre a une app est une trame `u32 taille` + contenu.
//
//   REGISTER     app                        l'app ecoute sur cette connexion
//   SEND         expediteur, destinataire, message
//   SUBSCRIBE    app, evenement
//   PUBLISH      app, evenement, message    a tous les abonnes de l'evenement
//   UNSUBSCRIBE  app, evenement
//   UNREGISTER   app                        l'app se deconnecte
//   FOLLOW       abonne, app suivie         une app s'abonne a une AUTRE app
//   UNFOLLOW     abonne, app suivie
//   SEND_WINDOW  expediteur, scope, fenetre aux abonnes de l'expediteur ou a
//                                           toutes les apps (voir WindowScope)
use std::io::Read;

pub const REGISTER: u32 = 0;
pub const SEND: u32 = 1;
pub const SUBSCRIBE: u32 = 2;
pub const PUBLISH: u32 = 3;
pub const UNSUBSCRIBE: u32 = 4;
pub const UNREGISTER: u32 = 5;
pub const FOLLOW: u32 = 6;
pub const UNFOLLOW: u32 = 7;
pub const SEND_WINDOW: u32 = 8;

/// Taille maximale d'un message (au-dela : connexion coupee, plutot que
/// d'allouer ce qu'un client annonce).
pub const MAX_MESSAGE: usize = 16 * 1024 * 1024;

pub fn read_u32(stream: &mut impl Read) -> Result<u32, String> {
    let mut buffer = [0u8; 4];
    stream.read_exact(&mut buffer).map_err(|e| e.to_string())?;
    Ok(u32::from_le_bytes(buffer))
}

/// Taille u32 puis contenu (borne a `MAX_MESSAGE`).
pub fn read_bytes(stream: &mut impl Read) -> Result<Vec<u8>, String> {
    azure_core::models::frame::read_frame(stream, MAX_MESSAGE)
}

pub fn read_str(stream: &mut impl Read) -> Result<String, String> {
    String::from_utf8(read_bytes(stream)?).map_err(|e| e.to_string())
}
