// Qui est VRAIMENT au bout du socket : le noyau donne le pid du process
// connecte (SO_PEERCRED), impossible a falsifier par le client, et
// `/proc/<pid>/exe` donne son executable. Une app ne peut donc pas se faire
// passer pour une autre juste en annoncant un autre id : chaque daemon lie
// un id a l'executable qui l'a annonce en premier (azure-stockage
// `bind_app`, azure-service `AppRegistry`).
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;

pub fn peer_pid(stream: &UnixStream) -> Result<i32, String> {
    let mut cred = libc::ucred { pid: 0, uid: 0, gid: 0 };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY : `cred` et `len` vivent pendant tout l'appel, et `len` donne
    // la taille exacte de `cred`.
    let result = unsafe {
        libc::getsockopt(stream.as_raw_fd(), libc::SOL_SOCKET, libc::SO_PEERCRED, &mut cred as *mut libc::ucred as *mut libc::c_void, &mut len)
    };
    if result != 0 {
        return Err(format!("SO_PEERCRED : {}", std::io::Error::last_os_error()));
    }
    Ok(cred.pid)
}

/// Chemin de l'executable du process au bout de `stream`.
/// Le processus au bout de `stream` a-t-il renonce aux privileges
/// (`NoNewPrivs`) ? C'est le cas de toute app enfermee (Landlock l'exige),
/// et c'est irreversible. En cas de doute (`/proc` illisible) : oui.
/// (Un thread qui disparait entre la liste et la lecture compte comme un
/// doute : on prefere refuser.)
pub fn peer_restricted(stream: &UnixStream) -> bool {
    let Ok(pid) = peer_pid(stream) else { return true };
    // NoNewPrivs (comme Landlock) est par thread : un seul thread enferme
    // suffit (on ne sait pas lequel a ouvert la connexion).
    let Ok(tasks) = std::fs::read_dir(format!("/proc/{pid}/task")) else { return true };
    let mut seen = false;
    for task in tasks.flatten() {
        match std::fs::read_to_string(task.path().join("status")) {
            Ok(status) => {
                seen = true;
                if status.lines().any(|l| l.starts_with("NoNewPrivs:") && l[11..].trim() != "0") {
                    return true;
                }
            }
            Err(_) => return true,
        }
    }
    !seen
}

pub fn peer_exe(stream: &UnixStream) -> Result<String, String> {
    let pid = peer_pid(stream)?;
    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).map_err(|e| format!("/proc/{pid}/exe : {e}"))?;
    Ok(exe.to_string_lossy().into_owned())
}

/// Empreinte SHA-256 de l'executable qui tourne dans `pid` (le fichier
/// reellement execute, meme s'il a ete remplace ou deplace depuis).
pub fn fingerprint_pid(pid: i32) -> Result<[u8; 32], String> {
    fingerprint_file(std::path::Path::new(&format!("/proc/{pid}/exe")))
}

/// Empreinte de l'executable au bout de `stream`.
pub fn peer_fingerprint(stream: &UnixStream) -> Result<[u8; 32], String> {
    fingerprint_pid(peer_pid(stream)?)
}

/// Empreinte SHA-256 d'un fichier.
pub fn fingerprint_file(path: &std::path::Path) -> Result<[u8; 32], String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| format!("{} : {e}", path.display()))?;
    let mut hasher = crate::crypto::sha256::Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| format!("{} : {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finish())
}

/// Empreinte en hexadecimal.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
