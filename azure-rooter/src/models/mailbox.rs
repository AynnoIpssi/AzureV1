// Les messages envoyes a une app qui ne tourne pas : gardes (sur disque si
// un dossier est donne) et livres des qu'elle s'enregistre aupres du
// routeur. Au plus `MAX_PER_APP` par app (les plus anciens partent), et
// jamais plus de `MAX_AGE` (7 jours).
use azure_core::models::wire::{Reader, Writer};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_PER_APP: usize = 500;
pub const MAX_AGE_SECS: u64 = 7 * 24 * 3600;

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(Default)]
pub struct Mailbox {
    file: Option<PathBuf>,
    /// Chiffrement du fichier (voir `azure_core::security::vault`).
    vault: Option<azure_core::security::vault::Vault>,
    pending: BTreeMap<u32, VecDeque<(u64, String)>>,
}

impl Mailbox {
    /// En memoire seulement (perdue si le routeur s'arrete).
    pub fn in_memory() -> Mailbox {
        Mailbox::default()
    }

    /// Gardee dans `dir/mailbox.bin`.
    pub fn open(dir: &Path) -> Result<Mailbox, String> {
        let vault = azure_core::security::vault::Vault::open(dir)?;
        let file = dir.join("mailbox.bin");
        let mut mailbox = Mailbox { file: Some(file.clone()), vault: Some(vault.clone()), pending: BTreeMap::new() };
        let data = vault.read(&file, "mailbox").unwrap_or_else(|e| {
            eprintln!("routeur : {e}, boite aux lettres videe");
            None
        });
        if let Some(data) = data {
            let mut r = Reader::new(&data);
            let mut read = || -> Result<(), String> {
                for _ in 0..r.u32()? {
                    let app = r.u32()?;
                    let queue = mailbox.pending.entry(app).or_default();
                    for _ in 0..r.u32()? {
                        queue.push_back((r.u64()?, r.str()?));
                    }
                }
                Ok(())
            };
            // Fichier abime : on repart d'une boite vide plutot que de refuser de demarrer.
            if let Err(e) = read() {
                eprintln!("routeur : {} illisible ({e}), boite aux lettres videe", file.display());
                mailbox.pending.clear();
            }
        }
        mailbox.expire();
        Ok(mailbox)
    }

    fn expire(&mut self) {
        let limit = now().saturating_sub(MAX_AGE_SECS);
        for queue in self.pending.values_mut() {
            queue.retain(|(t, _)| *t >= limit);
        }
        self.pending.retain(|_, q| !q.is_empty());
    }

    fn save(&self) {
        let Some(file) = &self.file else { return };
        let mut w = Writer::new().u32(self.pending.len() as u32);
        for (app, queue) in &self.pending {
            w = w.u32(*app).u32(queue.len() as u32);
            for (t, content) in queue {
                w = w.u64(*t).str(content);
            }
        }
        if let Some(vault) = &self.vault
            && let Err(e) = vault.write(file, "mailbox", &w.finish()) {
                eprintln!("routeur : {e}");
            }
    }

    /// Garde `content` pour l'app `app`.
    pub fn push(&mut self, app: u32, content: String) {
        self.expire();
        let queue = self.pending.entry(app).or_default();
        queue.push_back((now(), content));
        while queue.len() > MAX_PER_APP {
            queue.pop_front();
        }
        self.save();
    }

    /// Les messages en attente pour `app`, dans l'ordre d'envoi (retires).
    pub fn take(&mut self, app: u32) -> Vec<String> {
        self.expire();
        let messages: Vec<String> = self.pending.remove(&app).map(|q| q.into_iter().map(|(_, c)| c).collect()).unwrap_or_default();
        if !messages.is_empty() {
            self.save();
        }
        messages
    }

    pub fn count(&self, app: u32) -> usize {
        self.pending.get(&app).map(VecDeque::len).unwrap_or(0)
    }
}
