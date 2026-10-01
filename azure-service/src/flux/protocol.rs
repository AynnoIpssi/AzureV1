// Protocole du flux sur le socket d'azure-service. Trames u32 (taille) +
// contenu, comme les autres daemons.
//
// Requete = opcode u32 + arguments ; reponse = statut u8 (0 ok, 1 erreur +
// message) + valeurs. Apres un LISTEN accepte, la connexion ne sert plus
// qu'a recevoir des envois du daemon : type u8 (PUSH_*) + contenu.
use crate::flux::change::Change;
use crate::flux::value::Value;
use azure_core::models::wire::{Reader, Writer};
use std::io::Read;

pub use azure_core::models::frame::{check_status, error, response, write_frame, STATUS_ERROR, STATUS_OK};

pub const HELLO: u32 = 0;
/// Nom, acces, persistant (u8).
pub const SHARE: u32 = 1;
pub const PUBLISH: u32 = 2;
pub const CLOSE: u32 = 3;
pub const LIST: u32 = 4;
pub const LISTEN: u32 = 5;
/// Reserve a azure-manager : change qui peut ecouter le flux d'une autre
/// app (tableau de bord).
pub const SET_ACCESS: u32 = 6;
/// Appels entre apps (voir `crate::call`) : la connexion devient celle qui
/// sert une methode.
pub const SERVE: u32 = 7;
/// Appelle une methode d'une autre app et attend la reponse.
pub const CALL: u32 = 8;
/// Reserve a azure-manager : qui peut appeler une methode.
pub const SET_CALL_ACCESS: u32 = 9;
/// Reserve a azure-manager : compteurs des flux et des appels.
pub const STATS: u32 = 10;

pub const PUSH_SNAPSHOT: u8 = 0;
pub const PUSH_UPDATE: u8 = 1;
pub const PUSH_CLOSED: u8 = 2;
pub const PUSH_DENIED: u8 = 3;
/// Vers une app qui sert une methode : une demande a traiter.
pub const PUSH_REQUEST: u8 = 4;

pub const MAX_FRAME: usize = 16 * 1024 * 1024;

/// Qui peut ecouter un flux (son app proprietaire le peut toujours).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Access {
    Public,
    Apps(Vec<u32>),
}

impl Access {
    pub fn allows(&self, owner: u32, app: u32) -> bool {
        app == owner
            || match self {
                Access::Public => true,
                Access::Apps(apps) => apps.contains(&app),
            }
    }

    pub fn write(&self, w: Writer) -> Writer {
        match self {
            Access::Public => w.u8(0),
            Access::Apps(apps) => apps.iter().fold(w.u8(1).u32(apps.len() as u32), |w, app| w.u32(*app)),
        }
    }

    pub fn read(r: &mut Reader) -> Result<Access, String> {
        match r.u8()? {
            0 => Ok(Access::Public),
            1 => {
                let count = r.u32()?.min(65536);
                Ok(Access::Apps((0..count).map(|_| r.u32()).collect::<Result<_, _>>()?))
            }
            other => Err(format!("Acces inconnu : {other}")),
        }
    }
}

/// Un flux qu'une app a le droit d'ecouter (voir `Flux::streams`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamInfo {
    pub owner: u32,
    pub name: String,
    /// Numero de la derniere modification.
    pub seq: u64,
    pub public: bool,
}

/// Nom d'un flux : lettres, chiffres, `-`, `_`, `.`, 128 caracteres max.
pub fn check_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty() && name.len() <= 128 && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if ok { Ok(()) } else { Err(format!("Nom de flux invalide : '{name}' (lettres, chiffres, - _ .)")) }
}

pub fn read_frame(stream: &mut impl Read) -> Result<Vec<u8>, String> {
    azure_core::models::frame::read_frame(stream, MAX_FRAME)
}

pub fn write_changes(w: Writer, changes: &[Change]) -> Writer {
    changes.iter().fold(w.u32(changes.len() as u32), |w, c| c.write(w))
}

pub fn read_changes(r: &mut Reader) -> Result<Vec<Change>, String> {
    let count = r.u32()?;
    let mut changes = Vec::new();
    for _ in 0..count {
        changes.push(Change::read(r)?);
    }
    Ok(changes)
}

pub fn push_snapshot(seq: u64, state: &Value) -> Vec<u8> {
    state.write(Writer::new().u8(PUSH_SNAPSHOT).u64(seq)).finish()
}

pub fn push_update(seq: u64, changes: &[Change]) -> Vec<u8> {
    write_changes(Writer::new().u8(PUSH_UPDATE).u64(seq), changes).finish()
}

pub fn push_closed() -> Vec<u8> {
    Writer::new().u8(PUSH_CLOSED).finish()
}

pub fn push_denied(message: &str) -> Vec<u8> {
    Writer::new().u8(PUSH_DENIED).str(message).finish()
}

/// Compteurs d'un flux (voir `STATS`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluxStats {
    pub owner: u32,
    pub name: String,
    pub changes: u64,
    pub listeners: u32,
    pub persist: bool,
}

/// Compteurs d'une methode (voir `STATS`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodStats {
    pub owner: u32,
    pub method: String,
    pub calls: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub total_ms: u64,
    pub last_error: String,
    pub served: bool,
}
