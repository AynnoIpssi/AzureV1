// Enfermer une app (Landlock, Linux 5.13+, sans droits root) : une fois
// `apply` appele, le processus (et tout ce qu'il lance) ne peut plus
// toucher qu'aux chemins autorises, et ne peut plus ouvrir de connexion
// reseau TCP sauf permission. Definitif pour ce processus.
//
// Par defaut : lecture des dossiers systeme (/usr, /etc...), lecture et
// ecriture de /tmp, /dev et /run (sockets des daemons, Wayland). Le dossier
// personnel est interdit : les donnees et les cles d'Azure, les autres apps
// et leur configuration sont hors de portee. L'app ajoute son dossier et ce
// que son manifeste declare (`read`, `write`).
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const EXECUTE: u64 = 1 << 0;
const WRITE_FILE: u64 = 1 << 1;
const READ_FILE: u64 = 1 << 2;
const READ_DIR: u64 = 1 << 3;
const REFER: u64 = 1 << 13;
const TRUNCATE: u64 = 1 << 14;
const IOCTL_DEV: u64 = 1 << 15;
/// ABI 9 : se connecter a un socket unix nomme. Sans lui, une app enfermee
/// joindrait le bus D-Bus de session (/run/user/UID/bus) et ferait lancer
/// par systemd --user ce qu'elle veut, hors de l'enclos.
const RESOLVE_UNIX: u64 = 1 << 16;
const NET_BIND_TCP: u64 = 1 << 0;
const NET_CONNECT_TCP: u64 = 1 << 1;
/// ABI 6 : pas de connexion aux sockets unix abstraits hors de l'enclos, ni
/// de signal aux processus hors de l'enclos (une app enfermee ne peut pas
/// tuer un daemon).
const SCOPE_ABSTRACT_UNIX_SOCKET: u64 = 1 << 0;
const SCOPE_SIGNAL: u64 = 1 << 1;
const RULE_PATH_BENEATH: libc::c_int = 1;
const CREATE_RULESET_VERSION: u32 = 1;

#[repr(C)]
struct RulesetAttr {
    handled_access_fs: u64,
    handled_access_net: u64,
    scoped: u64,
}

#[repr(C, packed)]
struct PathBeneathAttr {
    allowed_access: u64,
    parent_fd: i32,
}

static SANDBOXED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Ce processus est-il enferme (par `Sandbox::apply`) ? Un processus
/// enferme ne doit pas lancer de daemon : il le serait avec lui.
pub fn is_sandboxed() -> bool {
    SANDBOXED.load(std::sync::atomic::Ordering::SeqCst)
}

/// Version de Landlock du noyau (0 = absent).
pub fn abi_version() -> i64 {
    // SAFETY : appel de detection, sans structure.
    let v = unsafe { libc::syscall(libc::SYS_landlock_create_ruleset, std::ptr::null::<RulesetAttr>(), 0usize, CREATE_RULESET_VERSION) };
    if v < 0 { 0 } else { v }
}

fn fs_rights(abi: i64) -> u64 {
    let mut all = (1u64 << 13) - 1;
    if abi >= 2 {
        all |= REFER;
    }
    if abi >= 3 {
        all |= TRUNCATE;
    }
    if abi >= 5 {
        all |= IOCTL_DEV;
    }
    if abi >= 9 {
        all |= RESOLVE_UNIX;
    }
    all
}

/// Ce qui a pu etre applique.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Enforcement {
    /// Fichiers et reseau.
    Full,
    /// Fichiers seulement (noyau trop ancien pour le reseau).
    FilesOnly,
    /// Rien : Landlock absent ou desactive.
    Unsupported,
}

#[derive(Clone, Debug, Default)]
pub struct Sandbox {
    read: Vec<PathBuf>,
    write: Vec<PathBuf>,
    /// Ou l'app peut se connecter a un socket unix nomme (ABI 9).
    sockets: Vec<PathBuf>,
    network: bool,
}

/// Une regle prete (voir `Sandbox::prepare`) : l'appliquer ne fait plus
/// que deux appels systeme, possibles entre `fork` et `exec`.
pub struct Prepared {
    ruleset: Option<libc::c_int>,
    pub enforcement: Enforcement,
}

impl Sandbox {
    /// Dossiers systeme en lecture (dont /run), /tmp /var/tmp /dev en
    /// ecriture, pas de reseau. Sockets unix : ceux des daemons d'Azure
    /// (`paths::runtime_dir`) et celui de Wayland, rien d'autre (ni bus de
    /// session, ni X11).
    pub fn system() -> Sandbox {
        let read = ["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc", "/opt", "/proc", "/sys", "/var/lib", "/nix", "/run"];
        let write = ["/tmp", "/var/tmp", "/dev"];
        let mut sockets = vec![crate::paths::runtime_dir()];
        if let Some(wayland) = wayland_socket() {
            sockets.push(wayland);
        }
        Sandbox { read: read.iter().map(PathBuf::from).collect(), write: write.iter().map(PathBuf::from).collect(), sockets, network: false }
    }

    /// Autorise la connexion aux sockets unix de `path` (un socket, ou un
    /// dossier et ce qu'il contient).
    pub fn socket(mut self, path: impl Into<PathBuf>) -> Sandbox {
        self.sockets.push(path.into());
        self
    }

    pub fn read(mut self, path: impl Into<PathBuf>) -> Sandbox {
        self.read.push(path.into());
        self
    }

    pub fn write(mut self, path: impl Into<PathBuf>) -> Sandbox {
        self.write.push(path.into());
        self
    }

    pub fn network(mut self, allowed: bool) -> Sandbox {
        self.network = allowed;
        self
    }

    pub fn allows_network(&self) -> bool {
        self.network
    }

    /// Construit la regle (ouvre les chemins) sans l'appliquer. Les chemins
    /// absents sont ignores.
    pub fn prepare(&self) -> Result<Prepared, String> {
        let abi = abi_version();
        if abi < 1 {
            return Ok(Prepared { ruleset: None, enforcement: Enforcement::Unsupported });
        }
        let all = fs_rights(abi);
        let net = if abi >= 4 && !self.network { NET_BIND_TCP | NET_CONNECT_TCP } else { 0 };
        let scoped = if abi >= 6 { SCOPE_ABSTRACT_UNIX_SOCKET | SCOPE_SIGNAL } else { 0 };
        let attr = RulesetAttr { handled_access_fs: all, handled_access_net: net, scoped };
        // Noyau d'avant ABI 6 : on lui passe la structure qu'il connait.
        let size = if abi >= 6 { std::mem::size_of::<RulesetAttr>() } else { 16 };
        // SAFETY : `attr` vit pendant l'appel et `size` ne depasse pas sa taille.
        let fd = unsafe { libc::syscall(libc::SYS_landlock_create_ruleset, &attr as *const RulesetAttr, size, 0u32) };
        if fd < 0 {
            return Err(format!("landlock : {}", std::io::Error::last_os_error()));
        }
        let fd = fd as libc::c_int;
        let read_rights = EXECUTE | READ_FILE | READ_DIR;
        let file_rights = EXECUTE | WRITE_FILE | READ_FILE | if abi >= 3 { TRUNCATE } else { 0 } | if abi >= 5 { IOCTL_DEV } else { 0 } | if abi >= 9 { RESOLVE_UNIX } else { 0 };
        // Ecrire sous /run ne donne pas le droit d'y joindre un socket.
        for (paths, rights) in [(&self.read, read_rights), (&self.write, all & !RESOLVE_UNIX), (&self.sockets, all & RESOLVE_UNIX)] {
            if rights == 0 {
                continue;
            }
            for path in paths {
                if let Err(e) = add_path(fd, path, rights, file_rights) {
                    // SAFETY : fermeture de notre propre descripteur.
                    unsafe { libc::close(fd) };
                    return Err(e);
                }
            }
        }
        let enforcement = if self.network || abi >= 4 { Enforcement::Full } else { Enforcement::FilesOnly };
        Ok(Prepared { ruleset: Some(fd), enforcement })
    }

    /// Enferme ce processus, definitivement.
    pub fn apply(&self) -> Result<Enforcement, String> {
        let prepared = self.prepare()?;
        prepared.enforce().map_err(|e| format!("landlock : {e}"))?;
        if prepared.enforcement != Enforcement::Unsupported {
            SANDBOXED.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        Ok(prepared.enforcement.clone())
    }
}

/// Le socket du compositeur Wayland ($WAYLAND_DISPLAY, relatif a
/// $XDG_RUNTIME_DIR s'il n'est pas absolu).
fn wayland_socket() -> Option<PathBuf> {
    let display = std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into());
    let display = PathBuf::from(display);
    if display.is_absolute() {
        return Some(display);
    }
    Some(PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?).join(display))
}

fn add_path(ruleset: libc::c_int, path: &Path, rights: u64, file_rights: u64) -> Result<(), String> {
    let Ok(c) = CString::new(path.as_os_str().as_bytes()) else { return Ok(()) };
    // SAFETY : chemin C valide, O_PATH n'ouvre pas le contenu.
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_PATH | libc::O_CLOEXEC) };
    if fd < 0 {
        return Ok(());
    }
    let is_dir = path.is_dir();
    let attr = PathBeneathAttr { allowed_access: if is_dir { rights } else { rights & file_rights }, parent_fd: fd };
    // SAFETY : `attr` vit pendant l'appel.
    let r = unsafe { libc::syscall(libc::SYS_landlock_add_rule, ruleset, RULE_PATH_BENEATH, &attr as *const PathBeneathAttr, 0u32) };
    let error = std::io::Error::last_os_error();
    // SAFETY : fermeture de notre propre descripteur.
    unsafe { libc::close(fd) };
    if r < 0 {
        return Err(format!("landlock : {} : {error}", path.display()));
    }
    Ok(())
}

impl Prepared {
    /// Applique la regle a ce processus (sans allocation : utilisable dans
    /// `CommandExt::pre_exec`).
    pub fn enforce(&self) -> std::io::Result<()> {
        let Some(fd) = self.ruleset else { return Ok(()) };
        // SAFETY : appels systeme simples ; le descripteur est le notre.
        unsafe {
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::syscall(libc::SYS_landlock_restrict_self, fd, 0u32) != 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

impl Drop for Prepared {
    fn drop(&mut self) {
        if let Some(fd) = self.ruleset {
            // SAFETY : fermeture de notre propre descripteur.
            unsafe { libc::close(fd) };
        }
    }
}
