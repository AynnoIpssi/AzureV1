// Cote daemon : tous les flux (etat + acces) et toutes les ecoutes. Chaque
// ecoute recoit ses envois par un canal borne ; son thread les ecrit sur
// le socket. Une ecoute trop lente (canal plein) est coupee : son client se
// reconnecte et repart d'un etat complet, sans bloquer les autres.
use crate::flux::change::{Change, Filter};
use crate::flux::protocol::*;
use crate::flux::value::Value;
use std::collections::HashMap;
use std::sync::mpsc::SyncSender;

struct Stream {
    access: Access,
    state: Value,
    seq: u64,
    /// Etat garde sur disque (voir `Hub::with_store`).
    persist: bool,
}

struct Listening {
    id: u64,
    app: u32,
    owner: u32,
    name: String,
    filter: Filter,
    tx: SyncSender<Vec<u8>>,
    /// Faux tant que le flux n'existe pas (ou a ete ferme) : l'ecoute
    /// attend qu'il soit partage.
    active: bool,
}

#[derive(Default)]
pub struct Hub {
    streams: HashMap<(u32, String), Stream>,
    listening: Vec<Listening>,
    next_id: u64,
    /// Dossier des flux persistants (un fichier par flux, chiffre).
    store: Option<std::path::PathBuf>,
    vault: Option<azure_core::security::vault::Vault>,
}

fn stream_file(dir: &std::path::Path, owner: u32, name: &str) -> std::path::PathBuf {
    // Le nom n'a que lettres, chiffres, `-`, `_` et `.` (voir `check_name`) :
    // sur pour un nom de fichier, sauf `.`/`..` seuls, impossibles ici
    // (prefixe par l'id).
    dir.join(format!("{owner}-{name}.flux"))
}

impl Hub {
    pub fn new() -> Hub {
        Hub::default()
    }

    /// Les flux marques persistants sont gardes dans `dir` et recharges
    /// ici (qui peut les ecouter : seulement leur app, jusqu'a ce qu'elle
    /// les repartage ou que azure-manager donne l'acces).
    pub fn with_store(dir: &std::path::Path) -> Result<Hub, String> {
        let vault = azure_core::security::vault::Vault::open(dir)?;
        let mut hub = Hub { store: Some(dir.to_path_buf()), vault: Some(vault.clone()), ..Hub::default() };
        for entry in std::fs::read_dir(dir).map_err(|e| format!("{} : {e}", dir.display()))?.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|x| x != "flux") {
                // (cle.bin, fichiers temporaires)
                continue;
            }
            let loaded = vault.read(&path, "flux").and_then(|data| data.ok_or_else(|| "absent".to_string())).and_then(|data| {
                let mut r = azure_core::models::wire::Reader::new(&data);
                let (owner, name, seq) = (r.u32()?, r.str()?, r.u64()?);
                let state = Value::read(&mut r)?;
                Ok((owner, name, seq, state))
            });
            match loaded {
                Ok((owner, name, seq, state)) => {
                    hub.streams.insert((owner, name), Stream { access: Access::Apps(Vec::new()), state, seq, persist: true });
                }
                Err(e) => eprintln!("azure-service : {} illisible ({e}), ignore", path.display()),
            }
        }
        Ok(hub)
    }

    fn save(&self, owner: u32, name: &str) {
        let (Some(dir), Some(stream)) = (&self.store, self.streams.get(&(owner, name.to_string()))) else { return };
        let file = stream_file(dir, owner, name);
        if !stream.persist {
            let _ = std::fs::remove_file(&file);
            return;
        }
        let data = stream.state.write(azure_core::models::wire::Writer::new().u32(owner).str(name).u64(stream.seq)).finish();
        if let Some(vault) = &self.vault
            && let Err(e) = vault.write(&file, "flux", &data) {
                eprintln!("azure-service : {e}");
            }
    }

    /// Comme `share`, en choisissant si l'etat est garde sur disque.
    pub fn share_persistent(&mut self, owner: u32, name: &str, access: Access, persist: bool) -> Result<(u64, Value), String> {
        let result = self.share(owner, name, access)?;
        if let Some(stream) = self.streams.get_mut(&(owner, name.to_string()))
            && stream.persist != persist {
                stream.persist = persist;
                self.save(owner, name);
            }
        Ok(result)
    }

    /// Nombre de modifications, d'ecoutes et persistance de chaque flux.
    pub fn stats(&self) -> Vec<(u32, String, u64, usize, bool)> {
        let mut out: Vec<_> = self.streams.iter().map(|((owner, name), s)| (*owner, name.clone(), s.seq, self.listeners(*owner, name), s.persist)).collect();
        out.sort();
        out
    }

    /// Cree le flux `name` de l'app `owner`, ou change qui peut l'ecouter.
    /// Retourne son numero et son etat (une app relancee reprend ou elle
    /// en etait).
    pub fn share(&mut self, owner: u32, name: &str, access: Access) -> Result<(u64, Value), String> {
        check_name(name)?;
        let stream = self.streams.entry((owner, name.to_string())).or_insert_with(|| Stream { access: access.clone(), state: Value::empty_map(), seq: 0, persist: false });
        stream.access = access;
        let (seq, state, access) = (stream.seq, stream.state.clone(), stream.access.clone());

        // Ecoutes en attente acceptees, ecoutes qui n'ont plus le droit coupees.
        self.listening.retain_mut(|l| {
            if l.owner != owner || l.name != name {
                return true;
            }
            if !access.allows(owner, l.app) {
                let _ = l.tx.try_send(push_denied(&format!("L'app {owner} ne partage plus '{name}' avec l'app {}", l.app)));
                return false;
            }
            if !l.active {
                l.active = true;
                return l.tx.try_send(push_snapshot(seq, &l.filter.extract(&state))).is_ok();
            }
            true
        });
        Ok((seq, state))
    }

    /// Change l'acces d'un flux deja partage (azure-manager). Erreur si le
    /// flux n'existe pas : son app recevra l'acces du manager en le
    /// partageant.
    pub fn set_access(&mut self, owner: u32, name: &str, access: Access) -> Result<(), String> {
        if !self.streams.contains_key(&(owner, name.to_string())) {
            return Err(format!("Flux '{name}' de l'app {owner} pas partage"));
        }
        self.share(owner, name, access).map(|_| ())
    }

    /// Applique `changes` (tout ou rien) et les envoie aux ecoutes
    /// concernees. Retourne le nouveau numero du flux.
    pub fn publish(&mut self, owner: u32, name: &str, changes: &[Change]) -> Result<u64, String> {
        let stream = self.streams.get_mut(&(owner, name.to_string())).ok_or_else(|| format!("Flux '{name}' pas partage (share d'abord)"))?;
        if changes.is_empty() {
            return Ok(stream.seq);
        }
        let mut next = stream.state.clone();
        for change in changes {
            change.apply(&mut next)?;
        }
        stream.state = next;
        stream.seq += 1;
        let seq = stream.seq;
        if stream.persist {
            self.save(owner, name);
        }

        self.listening.retain(|l| {
            if !l.active || l.owner != owner || l.name != name {
                return true;
            }
            let wanted: Vec<Change> = changes.iter().filter(|c| l.filter.wants(c)).cloned().collect();
            wanted.is_empty() || l.tx.try_send(push_update(seq, &wanted)).is_ok()
        });
        Ok(seq)
    }

    /// Supprime le flux et son etat. Ses ecoutes sont prevenues et attendent
    /// qu'il soit partage a nouveau.
    pub fn close(&mut self, owner: u32, name: &str) -> Result<(), String> {
        self.streams.remove(&(owner, name.to_string())).ok_or_else(|| format!("Flux '{name}' pas partage"))?;
        if let Some(dir) = &self.store {
            let _ = std::fs::remove_file(stream_file(dir, owner, name));
        }
        self.listening.retain_mut(|l| {
            if l.owner != owner || l.name != name || !l.active {
                return true;
            }
            l.active = false;
            l.tx.try_send(push_closed()).is_ok()
        });
        Ok(())
    }

    /// Ecoute le flux `name` de `owner` pour `app`. Flux existant : l'etat
    /// (filtre) est envoye tout de suite. Flux pas encore partage :
    /// l'ecoute attend. Refuse si `owner` ne le partage pas avec `app`.
    pub fn listen(&mut self, app: u32, owner: u32, name: &str, filter: Filter, tx: SyncSender<Vec<u8>>) -> Result<u64, String> {
        check_name(name)?;
        let active = match self.streams.get(&(owner, name.to_string())) {
            Some(stream) if !stream.access.allows(owner, app) => return Err(format!("L'app {owner} ne partage pas '{name}' avec l'app {app}")),
            Some(stream) => {
                tx.try_send(push_snapshot(stream.seq, &filter.extract(&stream.state))).map_err(|_| "Ecoute saturee".to_string())?;
                true
            }
            None => false,
        };
        self.next_id += 1;
        self.listening.push(Listening { id: self.next_id, app, owner, name: name.to_string(), filter, tx, active });
        Ok(self.next_id)
    }

    pub fn unlisten(&mut self, id: u64) {
        self.listening.retain(|l| l.id != id);
    }

    /// Les flux que `app` peut ecouter (les siens compris).
    pub fn streams(&self, app: u32) -> Vec<StreamInfo> {
        let mut list: Vec<StreamInfo> = self
            .streams
            .iter()
            .filter(|((owner, _), s)| s.access.allows(*owner, app))
            .map(|((owner, name), s)| StreamInfo { owner: *owner, name: name.clone(), seq: s.seq, public: s.access == Access::Public })
            .collect();
        list.sort_by(|a, b| (a.owner, &a.name).cmp(&(b.owner, &b.name)));
        list
    }

    /// Nombre d'ecoutes branchees sur un flux (actives ou en attente).
    pub fn listeners(&self, owner: u32, name: &str) -> usize {
        self.listening.iter().filter(|l| l.owner == owner && l.name == name).count()
    }
}
