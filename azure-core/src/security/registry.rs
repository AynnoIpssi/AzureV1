// Qui est l'app `id` ? Demande a azure-manager (opcode IDENTIFY), pour que
// azure-stockage et azure-service n'acceptent une app que si c'est bien
// elle (meme executable, ou meme empreinte). Le protocole est repris ici a
// l'identique : ces daemons ne dependent pas d'azure-manager.
use crate::managers::identity::{peer_exe, peer_fingerprint};
use crate::models::frame::{check_status, read_frame, write_frame};
use crate::models::wire::Writer;
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// Socket d'azure-manager par defaut.
pub static MANAGER_SOCKET: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| crate::paths::socket("manager"));
/// Ids attribues par azure-manager (en dessous : ids ecrits a la main).
pub const FIRST_MANAGED_ID: u32 = 1000;
const IDENTIFY: u32 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub name: String,
    pub exe: String,
    pub installed: bool,
    pub fingerprint: Option<[u8; 32]>,
    pub storage: bool,
}

/// `Ok(None)` : azure-manager ne connait pas cet id. `Err` : injoignable.
pub fn identify(socket: &str, id: u32) -> Result<Option<Identity>, String> {
    let mut stream = UnixStream::connect(socket).map_err(|e| format!("azure-manager injoignable : {e}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    write_frame(&mut stream, &Writer::new().u32(IDENTIFY).u32(id).finish())?;
    let response = read_frame(&mut stream, 1 << 20)?;
    let mut r = check_status(&response)?;
    if r.u8()? == 0 {
        return Ok(None);
    }
    let (name, exe, installed) = (r.str()?, r.str()?, r.u8()? != 0);
    let fp = r.bytes()?;
    let fingerprint = (fp.len() == 32).then(|| {
        let mut a = [0u8; 32];
        a.copy_from_slice(fp);
        a
    });
    Ok(Some(Identity { name, exe, installed, fingerprint, storage: r.u8()? != 0 }))
}

/// Le processus au bout de `stream` est-il l'app `identity` ? Installee :
/// meme empreinte. En developpement : meme executable ou meme empreinte.
pub fn matches(identity: &Identity, stream: &UnixStream) -> Result<bool, String> {
    let fp = peer_fingerprint(stream).ok();
    let same_binary = fp.is_some() && fp == identity.fingerprint;
    if identity.installed {
        return Ok(same_binary);
    }
    Ok(same_binary || peer_exe(stream)? == identity.exe)
}

/// Verifie l'app `id` aupres d'azure-manager. `Ok(Some(identite))` si elle
/// est connue et que c'est bien elle ; `Ok(None)` si le manager ne la
/// connait pas ou ne repond pas (le daemon applique alors sa propre regle :
/// premier executable arrive) ; `Err` si c'est un imposteur.
pub fn verify(socket: &str, id: u32, stream: &UnixStream) -> Result<Option<Identity>, String> {
    if id < FIRST_MANAGED_ID {
        return Ok(None);
    }
    match identify(socket, id) {
        Ok(Some(identity)) => {
            if matches(&identity, stream)? {
                Ok(Some(identity))
            } else {
                Err(format!("L'app {id} est '{}' ({}) : ce processus n'est pas elle", identity.name, identity.exe))
            }
        }
        Ok(None) | Err(_) => Ok(None),
    }
}
