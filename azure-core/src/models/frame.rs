// Trames echangees avec les daemons (stockage, provider, service, manager) :
// `u32 taille` (little-endian) + contenu. Requete = opcode u32 + arguments
// (voir `wire`) ; reponse = statut u8 (0 ok, 1 erreur suivie du message)
// puis les valeurs rendues.
use crate::models::wire::{Reader, Writer};
use std::io::{Read, Write};

pub const STATUS_OK: u8 = 0;
pub const STATUS_ERROR: u8 = 1;

pub fn write_frame(stream: &mut impl Write, payload: &[u8]) -> Result<(), String> {
    stream.write_all(&(payload.len() as u32).to_le_bytes()).and_then(|_| stream.write_all(payload)).map_err(|e| e.to_string())
}

/// Lit une trame ; refuse d'allouer plus de `max` octets (la connexion est
/// alors a fermer : la suite du flux n'a plus de sens).
pub fn read_frame(stream: &mut impl Read, max: usize) -> Result<Vec<u8>, String> {
    let mut size = [0u8; 4];
    stream.read_exact(&mut size).map_err(|e| e.to_string())?;
    let size = u32::from_le_bytes(size) as usize;
    if size > max {
        return Err(format!("Trame trop grande ({size} octets)"));
    }
    let mut payload = vec![0u8; size];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
    Ok(payload)
}

/// Reponse d'erreur.
pub fn error(message: &str) -> Vec<u8> {
    Writer::new().u8(STATUS_ERROR).str(message).finish()
}

/// Reponse a une requete : statut puis valeurs, ou l'erreur.
pub fn response(result: Result<Vec<u8>, String>) -> Vec<u8> {
    match result {
        Ok(values) => [vec![STATUS_OK], values].concat(),
        Err(message) => error(&message),
    }
}

/// Lit le statut d'une reponse : `Ok(reader)` positionne sur les valeurs,
/// ou l'erreur renvoyee par le daemon.
pub fn check_status(payload: &[u8]) -> Result<Reader<'_>, String> {
    let mut reader = Reader::new(payload);
    match reader.u8()? {
        STATUS_OK => Ok(reader),
        _ => Err(reader.str()?),
    }
}
