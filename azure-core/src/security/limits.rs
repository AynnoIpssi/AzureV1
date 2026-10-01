// Plafond de connexions simultanees d'un daemon : chaque connexion coute un
// thread, et une app pourrait en ouvrir des milliers. Au-dela du plafond
// (par processus, et en tout), la connexion est fermee aussitot.
use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

/// Connexions simultanees par processus client.
pub const PER_PROCESS: usize = 64;
/// Connexions simultanees en tout.
pub const TOTAL: usize = 1024;

#[derive(Clone)]
pub struct ConnectionGate {
    inner: Arc<Mutex<Counts>>,
    per_process: usize,
    total: usize,
}

#[derive(Default)]
struct Counts {
    total: usize,
    by_pid: HashMap<i32, usize>,
}

/// Une place prise ; rendue quand la connexion se termine (`Drop`).
pub struct Permit {
    gate: ConnectionGate,
    pid: i32,
}

impl Default for ConnectionGate {
    fn default() -> ConnectionGate {
        ConnectionGate::new(PER_PROCESS, TOTAL)
    }
}

impl ConnectionGate {
    pub fn new(per_process: usize, total: usize) -> ConnectionGate {
        ConnectionGate { inner: Arc::new(Mutex::new(Counts::default())), per_process, total }
    }

    /// Une place pour `stream`, ou `None` si le plafond est atteint.
    pub fn admit(&self, stream: &UnixStream) -> Option<Permit> {
        let pid = crate::managers::identity::peer_pid(stream).unwrap_or(-1);
        let mut counts = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mine = counts.by_pid.get(&pid).copied().unwrap_or(0);
        if counts.total >= self.total || mine >= self.per_process {
            return None;
        }
        counts.total += 1;
        counts.by_pid.insert(pid, mine + 1);
        Some(Permit { gate: self.clone(), pid })
    }

    pub fn open_connections(&self) -> usize {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).total
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut counts = self.gate.inner.lock().unwrap_or_else(|e| e.into_inner());
        counts.total = counts.total.saturating_sub(1);
        if let Some(n) = counts.by_pid.get_mut(&self.pid) {
            *n -= 1;
            if *n == 0 {
                counts.by_pid.remove(&self.pid);
            }
        }
    }
}
