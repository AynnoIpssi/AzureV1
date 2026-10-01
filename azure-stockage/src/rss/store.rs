// Ou et comment les tables RsS vivent sur le disque : dans l'espace prive
// de leur app (qui suit `set_location`), un fichier chiffre par table
// (`<HMAC>.rss`) + un catalogue des noms de tables.
//
// Un COMMIT peut toucher plusieurs fichiers : il passe par un journal pour
// etre tout-ou-rien, meme si le process est tue au milieu. Chaque nouveau
// fichier est d'abord ecrit a cote (`.rss-new`), puis la liste des
// changements est ecrite dans `rss-journal.bin`, puis tout est applique, puis
// le journal est efface. Au redemarrage, un journal present = un COMMIT
// interrompu apres sa decision : il est termine (voir `rss_recover`).
use crate::crypto::aead::{open, seal};
use crate::crypto::hmac::hmac_sha256;
use crate::managers::stockage::{hex, make_dir, read_optional, remove_optional, write_atomic, AzureStockage};
use crate::models::wire::{Reader, Writer};
use crate::rss::table::Table;
use std::collections::BTreeSet;
use std::path::PathBuf;

impl AzureStockage {
    fn rss_file(&self, owner: u32, label: &str) -> PathBuf {
        let id = hmac_sha256(&self.app_key_for(owner), label.as_bytes());
        self.private_dir(owner).join(format!("{}.rss", hex(&id)))
    }

    fn rss_aad(owner: u32, label: &str) -> Vec<u8> {
        [b"rss:".as_slice(), &owner.to_le_bytes(), label.as_bytes()].concat()
    }

    fn table_label(name: &str) -> String {
        format!("table:{name}")
    }

    /// Les noms des tables de `owner`, tries.
    pub(crate) fn rss_catalog(&self, owner: u32) -> Result<BTreeSet<String>, String> {
        let Some(data) = read_optional(&self.rss_file(owner, "catalog"))? else { return Ok(BTreeSet::new()) };
        let plain = open(&self.app_key_for(owner), &Self::rss_aad(owner, "catalog"), &data)?;
        let mut r = Reader::new(&plain);
        (0..r.u32()?).map(|_| r.str()).collect()
    }

    /// La table `name` de `owner` (depuis le cache, sinon le disque).
    pub(crate) fn rss_table(&mut self, owner: u32, name: &str) -> Result<Option<&Table>, String> {
        let key = (owner, name.to_string());
        if !self.rss_cache.contains_key(&key) {
            let label = Self::table_label(name);
            let Some(data) = read_optional(&self.rss_file(owner, &label))? else { return Ok(None) };
            let plain = open(&self.app_key_for(owner), &Self::rss_aad(owner, &label), &data)?;
            let table = Table::read(&mut Reader::new(&plain))?;
            if table.name != name {
                return Err(format!("Table '{name}' a la mauvaise place"));
            }
            self.rss_cache.insert(key.clone(), table);
        }
        Ok(self.rss_cache.get(&key))
    }

    /// Applique d'un coup les tables modifiees (`None` = supprimee).
    pub(crate) fn rss_commit(&mut self, changes: Vec<(u32, String, Option<Table>)>) -> Result<(), String> {
        if changes.is_empty() {
            return Ok(());
        }
        self.rss_write_journal(&changes)?;
        // A partir d'ici le COMMIT est decide : `rss_recover` le terminerait.
        self.rss_apply_journal()?;
        for (owner, name, table) in changes {
            match table {
                Some(mut table) => {
                    table.rebuild()?;
                    self.rss_cache.insert((owner, name), table);
                }
                None => {
                    self.rss_cache.remove(&(owner, name));
                }
            }
        }
        Ok(())
    }

    // Ecrit les nouveaux fichiers a cote (`.rss-new`), puis le journal.
    fn rss_write_journal(&self, changes: &[(u32, String, Option<Table>)]) -> Result<(), String> {
        // (fichier, nouveau contenu chiffre ou None = a supprimer)
        let mut files: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
        let mut owners = BTreeSet::new();
        for (owner, name, table) in changes {
            let label = Self::table_label(name);
            let data = match table {
                Some(table) => Some(seal(&self.app_key_for(*owner), &Self::rss_aad(*owner, &label), &table.write(Writer::new()).finish())?),
                None => None,
            };
            files.push((self.rss_file(*owner, &label), data));
            owners.insert(*owner);
        }
        for owner in owners {
            let mut catalog = self.rss_catalog(owner)?;
            for (o, name, table) in changes {
                if *o == owner {
                    if table.is_some() { catalog.insert(name.clone()) } else { catalog.remove(name) };
                }
            }
            let plain = catalog.iter().fold(Writer::new().u32(catalog.len() as u32), |w, n| w.str(n)).finish();
            files.push((self.rss_file(owner, "catalog"), Some(seal(&self.app_key_for(owner), &Self::rss_aad(owner, "catalog"), &plain)?)));
            make_dir(&self.private_dir(owner))?;
        }

        for (path, data) in &files {
            if let Some(data) = data {
                write_atomic(&path.with_extension("rss-new"), data)?;
            }
        }
        let journal = files.iter().fold(Writer::new().u32(files.len() as u32), |w, (path, data)| w.str(&path.to_string_lossy()).u8(data.is_some() as u8));
        let journal_path = self.root().join("rss-journal.bin");
        write_atomic(&journal_path, &seal(&self.derive_key(b"rss-journal"), b"rss-journal", &journal.finish())?)
    }

    fn rss_apply_journal(&self) -> Result<(), String> {
        let journal_path = self.root().join("rss-journal.bin");
        let Some(data) = read_optional(&journal_path)? else { return Ok(()) };
        let plain = open(&self.derive_key(b"rss-journal"), b"rss-journal", &data)?;
        let mut r = Reader::new(&plain);
        for _ in 0..r.u32()? {
            let path = PathBuf::from(r.str()?);
            if r.u8()? == 1 {
                let new = path.with_extension("rss-new");
                // Deja renomme par une tentative precedente : rien a faire.
                if new.exists() {
                    std::fs::rename(&new, &path).map_err(|e| format!("{} : {e}", path.display()))?;
                }
            } else {
                remove_optional(&path)?;
            }
        }
        remove_optional(&journal_path)?;
        Ok(())
    }

    /// Termine un COMMIT interrompu (appele a l'ouverture du stockage).
    pub(crate) fn rss_recover(&mut self) -> Result<(), String> {
        self.rss_apply_journal()
    }
}

// Crash simule au milieu d'un COMMIT (impossible a provoquer de l'exterieur).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rss::engine::{execute, Session};
    use crate::rss::value::Value;

    fn count(store: &mut AzureStockage) -> Vec<Vec<Value>> {
        execute(store, &mut Session::new(), 1, "SELECT count(*) FROM t", &[], &[]).unwrap().remove(0).rows
    }

    fn setup(name: &str) -> (std::path::PathBuf, Table) {
        let root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../target/tmp")).join(format!("rss-crash-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        let mut store = AzureStockage::open(&root).unwrap();
        execute(&mut store, &mut Session::new(), 1, "CREATE TABLE t (x INT); INSERT INTO t VALUES (1)", &[], &[]).unwrap();
        let mut table = store.rss_table(1, "t").unwrap().unwrap().clone();
        let row = table.prepare_row(vec![Some(Value::Int(2))]).unwrap();
        table.insert(row);
        table.version += 1;
        (root, table)
    }

    #[test]
    fn a_commit_killed_after_its_journal_is_finished_at_restart() {
        let (root, table) = setup("apres-journal");
        let store = AzureStockage::open(&root).unwrap();
        store.rss_write_journal(&[(1, "t".to_string(), Some(table))]).unwrap();
        drop(store); // le process meurt ici
        assert!(root.join("rss-journal.bin").exists());
        let mut store = AzureStockage::open(&root).unwrap();
        assert_eq!(count(&mut store), [[Value::Int(2)]]);
        assert!(!root.join("rss-journal.bin").exists());
    }

    #[test]
    fn a_commit_killed_before_its_journal_changes_nothing() {
        let (root, table) = setup("avant-journal");
        let store = AzureStockage::open(&root).unwrap();
        store.rss_write_journal(&[(1, "t".to_string(), Some(table))]).unwrap();
        // Le journal n'a jamais ete ecrit : seuls les `.rss-new` existent.
        std::fs::remove_file(root.join("rss-journal.bin")).unwrap();
        drop(store);
        let mut store = AzureStockage::open(&root).unwrap();
        assert_eq!(count(&mut store), [[Value::Int(1)]]);
    }
}
