// Protocole du daemon : chaque requete et chaque reponse est une trame
// `u32 taille` + contenu (voir `models::wire`). Contrairement a azure-rooter,
// chaque requete recoit une reponse : `u8 statut` (0 = ok, 1 = erreur avec
// un texte) puis les valeurs rendues.
use azure_core::models::storage_model::MAX_VALUE_LEN;
use std::io::Read;

pub use azure_core::models::frame::{check_status, error, response, write_frame, STATUS_ERROR, STATUS_OK};

pub const HELLO: u32 = 0;
pub const PUT: u32 = 1;
pub const GET: u32 = 2;
pub const DELETE: u32 = 3;
pub const KEYS: u32 = 4;
pub const SET_LOCATION: u32 = 5;
pub const LOCATION: u32 = 6;
pub const ADD_ACCOUNT: u32 = 10;
pub const REMOVE_ACCOUNT: u32 = 11;
pub const ACCOUNTS: u32 = 12;
pub const SHARE: u32 = 20;
pub const UNSHARE: u32 = 21;
pub const READ_SHARED: u32 = 22;
pub const WRITE_SHARED: u32 = 23;
pub const SHARED_LIST: u32 = 24;
/// Texte RsS + parametres + comptes -> un resultat par instruction.
pub const RSS: u32 = 30;

// Une valeur maximale + de quoi porter les autres champs.
const MAX_FRAME: usize = MAX_VALUE_LEN + 64 * 1024;

pub fn read_frame(stream: &mut impl Read) -> Result<Vec<u8>, String> {
    azure_core::models::frame::read_frame(stream, MAX_FRAME)
}
