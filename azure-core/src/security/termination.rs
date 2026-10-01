// Arreter une app avec `kill` (SIGTERM), SIGINT (Ctrl+C) ou SIGHUP.
//
// Isolee (voir `isolation`), une app est le processus 1 de son espace de
// processus : le noyau ne lui livre alors un signal que si elle a un
// gestionnaire (sauf SIGKILL). Sans lui, `kill` n'a aucun effet.
//
// - `install` : dans l'app. Le premier signal demande un arret propre (la
//   fenetre se ferme comme par son bouton, `on_close` compris ; voir
//   `requested`) ; faute de quoi l'app sort d'elle-meme apres `GRACE`, ou
//   tout de suite au second signal.
// - `forward_to` : dans un lanceur qui attend l'app (processus 1 d'un autre
//   espace) : les signaux recus lui sont transmis.
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Duration;

/// Le temps laisse a l'app pour se fermer proprement.
pub const GRACE: Duration = Duration::from_secs(3);

/// Les signaux qui demandent l'arret.
pub const SIGNALS: [libc::c_int; 4] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP, libc::SIGQUIT];

/// Le signal recu (0 : aucun).
static RECEIVED: AtomicI32 = AtomicI32::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// Le processus a qui transmettre (voir `forward_to`).
static TARGET: AtomicI32 = AtomicI32::new(0);

extern "C" fn on_signal(signal: libc::c_int) {
    // Second signal : on n'attend plus.
    if RECEIVED.swap(signal, Ordering::SeqCst) != 0 {
        // SAFETY : _exit est sur dans un gestionnaire de signal.
        unsafe { libc::_exit(128 + signal) };
    }
}

extern "C" fn on_forward(signal: libc::c_int) {
    let target = TARGET.load(Ordering::SeqCst);
    if target > 0 {
        // SAFETY : kill est sur dans un gestionnaire de signal.
        unsafe { libc::kill(target, signal) };
    }
}

/// Pose `handler` pour `SIGNALS`. `restart` : les lectures en cours
/// reprennent apres le signal (SA_RESTART ; `poll` revient quand meme avec
/// EINTR). Que des appels systeme.
fn set_handlers(handler: extern "C" fn(libc::c_int), restart: bool) {
    // SAFETY : structure initialisee a zero puis remplie ; gestionnaire
    // `extern "C"` valide pour toute la vie du processus.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = handler as usize;
        if restart {
            action.sa_flags = libc::SA_RESTART;
        }
        libc::sigemptyset(&mut action.sa_mask);
        for signal in SIGNALS {
            libc::sigaction(signal, &action, std::ptr::null_mut());
        }
    }
}

/// Dans l'app, une fois (les appels suivants ne font rien).
pub fn install() {
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    set_handlers(on_signal, true);
    // Une app sans fenetre (ou bloquee) ne regarde jamais `requested` :
    // elle sort d'elle-meme apres `GRACE`.
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_millis(100));
        let signal = RECEIVED.load(Ordering::SeqCst);
        if signal != 0 {
            std::thread::sleep(GRACE);
            std::process::exit(128 + signal);
        }
    });
}

/// Un arret a-t-il ete demande (voir `install`) ?
pub fn requested() -> bool {
    RECEIVED.load(Ordering::SeqCst) != 0
}

/// Transmet a `pid` les signaux d'arret recus par ce processus. Que des
/// appels systeme : possible entre `fork` et `exec`.
pub fn forward_to(pid: libc::pid_t) {
    TARGET.store(pid, Ordering::SeqCst);
    set_handlers(on_forward, false);
}
