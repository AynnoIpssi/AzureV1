// Protocole du socket du provider : trames u32 (taille) + contenu, comme
// azure-stockage. Requete = opcode u32 + arguments ; reponse = statut u8
// (0 ok, 1 erreur suivie du message) + valeurs.
use crate::models::service::{Restart, ServiceSpec, ServiceStatus, State};
use crate::models::wire::{Reader, Writer};
use std::io::Read;

pub use azure_core::models::frame::{check_status, error, response, write_frame, STATUS_ERROR, STATUS_OK};

pub const STATUS: u32 = 0;
pub const START: u32 = 1;
pub const STOP: u32 = 2;
pub const RESTART: u32 = 3;
pub const REGISTER: u32 = 4;
pub const UNREGISTER: u32 = 5;
/// Lance le service si besoin et attend qu'il soit pret (u32 = ms max).
pub const ENSURE: u32 = 6;
pub const SHUTDOWN: u32 = 7;
/// Fait connaitre un service sans le lancer (il le sera a la demande :
/// ENSURE, START). Meme contenu que REGISTER.
pub const DECLARE: u32 = 8;

const MAX_FRAME: usize = 1024 * 1024;

pub fn read_frame(stream: &mut impl Read) -> Result<Vec<u8>, String> {
    azure_core::models::frame::read_frame(stream, MAX_FRAME)
}

pub fn write_spec(mut w: Writer, spec: &ServiceSpec) -> Writer {
    w = w.str(&spec.name).str(&spec.command).u32(spec.args.len() as u32);
    for arg in &spec.args {
        w = w.str(arg);
    }
    w.str(spec.health.as_deref().unwrap_or("")).str(spec.restart.name()).u8(spec.autostart as u8)
}

pub fn read_spec(r: &mut Reader) -> Result<ServiceSpec, String> {
    let name = r.str()?;
    let command = r.str()?;
    let count = r.u32()?;
    let args = (0..count).map(|_| r.str()).collect::<Result<Vec<_>, _>>()?;
    let health = r.str()?;
    let restart = r.str()?;
    let restart = Restart::from_name(&restart).ok_or(format!("restart inconnu '{restart}'"))?;
    let autostart = r.u8()? != 0;
    Ok(ServiceSpec { name, command, args, health: (!health.is_empty()).then_some(health), restart, autostart })
}

pub fn write_status(w: Writer, status: &ServiceStatus) -> Writer {
    w.str(&status.name).u8(status.state.code()).u32(status.pid.unwrap_or(0)).u32(status.restarts).u64(status.uptime_secs).str(&status.message)
}

pub fn read_status(r: &mut Reader) -> Result<ServiceStatus, String> {
    let name = r.str()?;
    let state = State::from_code(r.u8()?).ok_or("Etat inconnu")?;
    let pid = r.u32()?;
    Ok(ServiceStatus { name, state, pid: (pid != 0).then_some(pid), restarts: r.u32()?, uptime_secs: r.u64()?, message: r.str()? })
}
