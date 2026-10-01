// Isolation par espaces de noms (namespaces), en plus de Landlock : ce que
// Landlock ne sait pas faire sur un noyau ancien (ABI < 6 : signaux, ABI < 9 :
// sockets nommes), le lanceur le fait ici, entre `fork` et `exec` :
//
// - espace utilisateur (sans privilege : l'uid reste le meme) ;
// - espace de montage : le dossier de session ($XDG_RUNTIME_DIR, avec le bus
//   D-Bus, PipeWire...) est remplace par un dossier vide ou ne reviennent que
//   le socket Wayland et le dossier des daemons d'Azure ; X11
//   (/tmp/.X11-unix) et le bus systeme (/run/dbus) sont masques ;
// - espace de processus : l'app est le processus 1 de son espace, elle ne
//   voit ni ne peut viser aucun autre processus (donc aucun signal) ;
// - espace reseau (si l'app n'a pas le reseau) : ni TCP, ni sockets unix
//   abstraits.
//
// Si le systeme interdit les espaces de noms sans privilege, rien n'est
// fait (Landlock reste applique par l'app) : `enter` ne l'empeche pas de
// demarrer.
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub struct Isolation {
    network: bool,
    uid_map: Vec<u8>,
    gid_map: Vec<u8>,
    /// Dossier de session a remplacer par un dossier vide.
    runtime: Option<CString>,
    /// Ce qui revient dans le dossier vide : (chemin, est un dossier).
    keep: Vec<(CString, bool)>,
    /// Dossiers masques par un dossier vide.
    hide: Vec<CString>,
}

fn cstring(path: &Path) -> Option<CString> {
    CString::new(path.as_os_str().as_bytes()).ok()
}

impl Isolation {
    /// Tout est prepare ici (allocations comprises) : `enter` ne fait plus
    /// que des appels systeme.
    pub fn for_app(network: bool) -> Isolation {
        // SAFETY : getuid / getgid n'echouent jamais.
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()).map(PathBuf::from).filter(|d| d.is_dir());
        let mut keep = Vec::new();
        if let Some(runtime) = &runtime {
            let display = PathBuf::from(std::env::var_os("WAYLAND_DISPLAY").unwrap_or_else(|| "wayland-0".into()));
            let wayland = if display.is_absolute() { display } else { runtime.join(display) };
            let azure = crate::paths::runtime_dir();
            for (path, dir) in [(wayland, false), (azure, true)] {
                // Seulement ce qui est DANS le dossier de session (le reste
                // n'est pas masque), a un seul niveau (pas de dossiers a creer).
                if path.parent() == Some(runtime.as_path()) && path.exists() {
                    keep.extend(cstring(&path).map(|c| (c, dir)));
                }
            }
        }
        let hide = ["/tmp/.X11-unix", "/run/dbus"].iter().map(Path::new).filter(|p| p.is_dir()).filter_map(cstring).collect();
        Isolation {
            network,
            uid_map: format!("{uid} {uid} 1\n").into_bytes(),
            gid_map: format!("{gid} {gid} 1\n").into_bytes(),
            runtime: runtime.as_deref().and_then(cstring),
            keep,
            hide,
        }
    }

    /// A appeler dans `CommandExt::pre_exec` (processus fils, avant `exec`).
    /// Le processus lance devient le processus 1 d'un nouvel espace ; le fils
    /// du lanceur attend sa fin et sort avec le meme code.
    pub fn enter(&self) -> std::io::Result<()> {
        let mut flags = libc::CLONE_NEWUSER | libc::CLONE_NEWNS | libc::CLONE_NEWPID;
        if !self.network {
            flags |= libc::CLONE_NEWNET;
        }
        // SAFETY : appels systeme simples, chaines C preparees d'avance.
        unsafe {
            if libc::unshare(flags) != 0 {
                // Espaces de noms interdits ici : Landlock seul.
                return Ok(());
            }
            write_file(c"/proc/self/setgroups", b"deny")?;
            write_file(c"/proc/self/uid_map", &self.uid_map)?;
            write_file(c"/proc/self/gid_map", &self.gid_map)?;
            check(libc::mount(std::ptr::null(), c"/".as_ptr(), std::ptr::null(), libc::MS_REC | libc::MS_PRIVATE, std::ptr::null()))?;

            if let Some(runtime) = &self.runtime {
                // Garder une prise sur ce qui doit revenir, avant de le masquer.
                let mut fds = [-1; 2];
                for (i, (path, _)) in self.keep.iter().enumerate().take(2) {
                    fds[i] = libc::open(path.as_ptr(), libc::O_PATH | libc::O_CLOEXEC);
                }
                check(libc::mount(c"tmpfs".as_ptr(), runtime.as_ptr(), c"tmpfs".as_ptr(), libc::MS_NOSUID | libc::MS_NODEV, c"mode=0700".as_ptr().cast()))?;
                for (i, (path, dir)) in self.keep.iter().enumerate().take(2) {
                    if fds[i] < 0 {
                        continue;
                    }
                    if *dir {
                        libc::mkdir(path.as_ptr(), 0o700);
                    } else {
                        let f = libc::open(path.as_ptr(), libc::O_CREAT | libc::O_WRONLY | libc::O_CLOEXEC, 0o600);
                        if f >= 0 {
                            libc::close(f);
                        }
                    }
                    let mut source = [0u8; 32];
                    fd_path(fds[i], &mut source);
                    let r = libc::mount(source.as_ptr().cast(), path.as_ptr(), std::ptr::null(), libc::MS_BIND, std::ptr::null());
                    libc::close(fds[i]);
                    check(r)?;
                }
            }
            for dir in &self.hide {
                check(libc::mount(c"tmpfs".as_ptr(), dir.as_ptr(), c"tmpfs".as_ptr(), libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC, c"mode=0755".as_ptr().cast()))?;
            }

            // Le nouvel espace de processus ne vaut que pour les enfants.
            let pid = libc::fork();
            if pid < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if pid > 0 {
                // Rester en attente sans rien garder d'ouvert (en particulier
                // le tube par lequel `Command::spawn` attend l'exec).
                libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32);
                // `kill` de ce processus (le seul pid que voit le lanceur)
                // arrete l'app.
                crate::security::termination::forward_to(pid);
                let mut status = 0;
                while libc::waitpid(pid, &mut status, 0) < 0 {
                    if *libc::__errno_location() != libc::EINTR {
                        libc::_exit(1);
                    }
                }
                let code = if libc::WIFEXITED(status) { libc::WEXITSTATUS(status) } else { 128 + libc::WTERMSIG(status) };
                libc::_exit(code);
            }
            // Si le processus qui attend disparait (SIGKILL), l'app avec.
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
            // Processus 1 : son propre /proc (les autres processus n'y sont pas).
            libc::mount(c"proc".as_ptr(), c"/proc".as_ptr(), c"proc".as_ptr(), libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC, std::ptr::null());
        }
        Ok(())
    }
}

fn check(r: libc::c_int) -> std::io::Result<()> {
    if r != 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
}

/// Ecrit `data` dans `path` (sans allocation).
unsafe fn write_file(path: &std::ffi::CStr, data: &[u8]) -> std::io::Result<()> {
    // SAFETY : chemin C valide, tampon valide de `data.len()` octets.
    unsafe {
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_CLOEXEC);
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let n = libc::write(fd, data.as_ptr().cast(), data.len());
        let err = std::io::Error::last_os_error();
        libc::close(fd);
        if n != data.len() as isize {
            return Err(err);
        }
    }
    Ok(())
}

/// « /proc/self/fd/<fd> » dans `out`, termine par un zero (sans allocation).
fn fd_path(fd: libc::c_int, out: &mut [u8; 32]) {
    let prefix = b"/proc/self/fd/";
    out[..prefix.len()].copy_from_slice(prefix);
    let mut digits = [0u8; 12];
    let (mut n, mut len) = (fd.max(0) as u32, 0);
    loop {
        digits[len] = b'0' + (n % 10) as u8;
        len += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for i in 0..len {
        out[prefix.len() + i] = digits[len - 1 - i];
    }
    out[prefix.len() + len] = 0;
}
