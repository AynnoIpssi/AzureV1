// Cote daemon : les methodes servies (par app) et les appels en cours. Un
// appel = une demande envoyee a l'app qui sert la methode, et un canal ou
// sa reponse arrivera (ou une erreur si elle s'arrete entre-temps).
use crate::flux::protocol::*;
use crate::flux::value::Value;
use azure_core::models::wire::Writer;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender, SyncSender};

struct Serving {
    /// Connexion qui sert la methode (une nouvelle la remplace).
    server: u64,
    tx: SyncSender<Vec<u8>>,
    access: Access,
}

struct Pending {
    server: u64,
    reply: Sender<Result<Value, String>>,
}

/// Compteurs d'une methode (depuis le demarrage du daemon).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CallStats {
    pub calls: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub total_ms: u64,
    pub last_error: String,
}

#[derive(Default)]
pub struct Broker {
    stats: HashMap<(u32, String), CallStats>,
    servers: HashMap<(u32, String), Serving>,
    pending: HashMap<u64, Pending>,
    next_server: u64,
    next_call: u64,
}

impl Broker {
    pub fn new() -> Broker {
        Broker::default()
    }

    /// L'app `owner` sert `method` (remplace une connexion precedente).
    /// Retourne l'id de cette connexion.
    pub fn serve(&mut self, owner: u32, method: &str, access: Access, tx: SyncSender<Vec<u8>>) -> Result<u64, String> {
        check_name(method)?;
        self.next_server += 1;
        let server = self.next_server;
        if let Some(old) = self.servers.insert((owner, method.to_string()), Serving { server, tx, access }) {
            self.fail_server_calls(old.server, "la methode a ete reprise par une autre connexion");
        }
        Ok(server)
    }

    /// Qui peut appeler (azure-manager). Erreur si la methode n'est pas
    /// servie : son app recevra l'acces en la servant.
    pub fn set_access(&mut self, owner: u32, method: &str, access: Access) -> Result<(), String> {
        let serving = self.servers.get_mut(&(owner, method.to_string())).ok_or_else(|| format!("'{method}' de l'app {owner} pas servie"))?;
        serving.access = access;
        Ok(())
    }

    /// Envoie la demande ; la reponse arrivera sur le canal retourne.
    pub fn start(&mut self, caller: u32, owner: u32, method: &str, args: &Value) -> Result<(u64, Receiver<Result<Value, String>>), String> {
        let serving = self.servers.get(&(owner, method.to_string())).ok_or_else(|| format!("'{method}' de l'app {owner} n'est pas disponible (app arretee, ou methode pas servie)"))?;
        if !serving.access.allows(owner, caller) {
            return Err(format!("L'app {owner} ne permet pas a l'app {caller} d'appeler '{method}'"));
        }
        self.next_call += 1;
        let id = self.next_call;
        let request = args.write(Writer::new().u8(PUSH_REQUEST).u64(id).u32(caller)).finish();
        serving.tx.try_send(request).map_err(|_| format!("'{method}' de l'app {owner} est saturee"))?;
        let (reply, rx) = channel();
        self.pending.insert(id, Pending { server: serving.server, reply });
        Ok((id, rx))
    }

    /// Reponse de la connexion `server` a l'appel `id` (ignoree si ce
    /// n'est pas elle qui l'a recu, ou si l'appel a expire).
    pub fn finish(&mut self, server: u64, id: u64, result: Result<Value, String>) {
        if self.pending.get(&id).is_some_and(|p| p.server == server)
            && let Some(pending) = self.pending.remove(&id) {
                let _ = pending.reply.send(result);
            }
    }

    /// Appel abandonne (delai depasse).
    pub fn cancel(&mut self, id: u64) {
        self.pending.remove(&id);
    }

    /// La connexion `server` est fermee : ses methodes ne sont plus servies
    /// et ses appels en cours echouent.
    pub fn server_gone(&mut self, server: u64) {
        self.servers.retain(|_, s| s.server != server);
        self.fail_server_calls(server, "l'app s'est arretee pendant l'appel");
    }

    fn fail_server_calls(&mut self, server: u64, message: &str) {
        let ids: Vec<u64> = self.pending.iter().filter(|(_, p)| p.server == server).map(|(id, _)| *id).collect();
        for id in ids {
            if let Some(pending) = self.pending.remove(&id) {
                let _ = pending.reply.send(Err(message.to_string()));
            }
        }
    }

    /// Methodes servies (pour les tests et le suivi).
    pub fn served(&self) -> Vec<(u32, String)> {
        let mut list: Vec<(u32, String)> = self.servers.keys().cloned().collect();
        list.sort();
        list
    }

    /// Note le resultat d'un appel termine.
    pub fn record(&mut self, owner: u32, method: &str, elapsed: std::time::Duration, outcome: Result<(), (bool, String)>) {
        let stats = self.stats.entry((owner, method.to_string())).or_default();
        stats.calls += 1;
        stats.total_ms += elapsed.as_millis() as u64;
        if let Err((timeout, message)) = outcome {
            stats.errors += 1;
            if timeout {
                stats.timeouts += 1;
            }
            stats.last_error = message;
        }
    }

    /// Les methodes appelees ou servies, avec leurs compteurs, et si elles
    /// sont servies en ce moment.
    pub fn stats(&self) -> Vec<(u32, String, CallStats, bool)> {
        let mut keys: Vec<(u32, String)> = self.stats.keys().chain(self.servers.keys()).cloned().collect();
        keys.sort();
        keys.dedup();
        keys.into_iter().map(|k| {
            let served = self.servers.contains_key(&k);
            let stats = self.stats.get(&k).cloned().unwrap_or_default();
            (k.0, k.1, stats, served)
        }).collect()
    }

    pub fn pending_calls(&self) -> usize {
        self.pending.len()
    }
}
