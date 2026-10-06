// L'objet qui gere TOUT le stockage Azure. Seul le daemon (voir `daemon.rs`)
// l'utilise : les apps passent par lui, jamais directement par le disque.
//
// Sur le disque (`root`, dossier 0700) :
//   master.key               cle maitresse (32 octets aleatoires, 0600)
//   apps.bin                 quelle app (id) correspond a quel executable
//   locations.bin            apps dont l'espace prive est ailleurs (voir
//                            `set_location`)
//   private/<app>/<cle>.bin  donnees privees d'une app (et ses tables RsS,
//                            `<table>.rss`) - ou `<emplacement>/<app>/...`
//   shared/<partage>.bin     donnees partagees
//   accounts/<app>.bin       comptes crees par une app
// Tout est chiffre et authentifie (ChaCha20-Poly1305). Chaque app a sa
// propre cle, derivee de la cle maitresse ; les noms de fichiers sont des
// HMAC, donc meme les noms des cles ne se lisent pas sur le disque.
use crate::crypto::aead::{open, seal};
use crate::crypto::hmac::hmac_sha256;
use crate::crypto::random::random_bytes;
use crate::models::account::Account;
use crate::models::wire::{Reader, Writer};
use azure_core::models::storage_model::{check_value, Role, ShareAccess, StorageKey};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

/// Identifiants presentes pour lire / ecrire une donnee partagee protegee.
#[derive(Debug, Clone, Copy)]
pub struct Credentials<'a> {
    pub user: &'a str,
    pub password: &'a str,
}

/// Une donnee partagee, telle que la voient les autres apps dans une liste.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedInfo {
    pub name: String,
    pub access: ShareAccess,
}

struct SharedEntry {
    owner: u32,
    name: String,
    access: ShareAccess,
    value: Vec<u8>,
}

pub struct AzureStockage {
    root: PathBuf,
    master: [u8; 32],
    // Apps dont l'espace prive n'est pas sous `root/private` (voir
    // `set_location`).
    locations: HashMap<u32, PathBuf>,
    // Tables RsS deja lues, par (app, nom) : voir `rss::store`.
    pub(crate) rss_cache: HashMap<(u32, String), crate::rss::table::Table>,
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn io(context: &Path, err: std::io::Error) -> String {
    format!("{} : {err}", context.display())
}

pub(crate) fn make_dir(path: &Path) -> Result<(), String> {
    fs::DirBuilder::new().recursive(true).mode(0o700).create(path).map_err(|e| io(path, e))
}

// Ecriture atomique : un fichier temporaire, puis `rename`. Un crash au
// milieu laisse l'ancienne version intacte, jamais un fichier a moitie ecrit.
pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp).map_err(|e| io(&tmp, e))?;
    file.write_all(data).and_then(|_| file.sync_all()).map_err(|e| io(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| io(path, e))
}

pub(crate) fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(data) => Ok(Some(data)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io(path, e)),
    }
}

pub(crate) fn remove_optional(path: &Path) -> Result<bool, String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(io(path, e)),
    }
}

impl AzureStockage {
    /// Ouvre (ou cree au premier lancement) le stockage dans `root`.
    pub fn open(root: &Path) -> Result<AzureStockage, String> {
        for dir in ["", "private", "shared", "accounts"] {
            make_dir(&root.join(dir))?;
        }
        let key_path = root.join("master.key");
        let master = match read_optional(&key_path)? {
            Some(data) => data.try_into().map_err(|_| format!("{} : cle maitresse invalide", key_path.display()))?,
            None => {
                let mut key = [0u8; 32];
                random_bytes(&mut key)?;
                write_atomic(&key_path, &key)?;
                key
            }
        };
        let mut stockage = AzureStockage { root: root.to_path_buf(), master, locations: HashMap::new(), rss_cache: HashMap::new() };
        stockage.locations = stockage.load_locations()?;
        stockage.rss_recover()?;
        Ok(stockage)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn derive_key(&self, label: &[u8]) -> [u8; 32] {
        self.derive(label)
    }

    // ---- Emplacement des donnees ----

    fn load_locations(&self) -> Result<HashMap<u32, PathBuf>, String> {
        let Some(data) = read_optional(&self.root.join("locations.bin"))? else { return Ok(HashMap::new()) };
        let plain = open(&self.derive(b"locations"), b"locations", &data)?;
        let mut reader = Reader::new(&plain);
        (0..reader.u32()?).map(|_| Ok((reader.u32()?, PathBuf::from(reader.str()?)))).collect()
    }

    fn save_locations(&self) -> Result<(), String> {
        let mut writer = Writer::new().u32(self.locations.len() as u32);
        for (app, path) in &self.locations {
            writer = writer.u32(*app).str(&path.to_string_lossy());
        }
        write_atomic(&self.root.join("locations.bin"), &seal(&self.derive(b"locations"), b"locations", &writer.finish())?)
    }

    /// Ou vit l'espace prive de `app` : `None` = a l'endroit par defaut
    /// (dans `root`).
    pub fn location(&self, app: u32) -> Option<&Path> {
        self.locations.get(&app).map(PathBuf::as_path)
    }

    /// Deplace l'espace prive de `app` (cles ET tables RsS) dans le dossier
    /// `place` (chemin absolu, cree si besoin), par exemple un disque
    /// externe ; `None` le ramene a l'endroit par defaut. Les donnees deja
    /// la sont deplacees, toujours chiffrees. Refuse si la destination
    /// contient deja un espace pour cette app.
    pub fn set_location(&mut self, app: u32, place: Option<&Path>) -> Result<(), String> {
        if let Some(place) = place
            && !place.is_absolute() {
                return Err(format!("L'emplacement doit etre un chemin absolu : {}", place.display()));
            }
        let old_dir = self.private_dir(app);
        let new_base = place.map_or_else(|| self.root.join("private"), Path::to_path_buf);
        let new_dir = new_base.join(old_dir.file_name().ok_or("Dossier prive invalide")?);
        if new_dir == old_dir {
            return Ok(());
        }
        if new_dir.exists() {
            return Err(format!("{} existe deja : deplacement refuse pour ne rien ecraser", new_dir.display()));
        }
        make_dir(&new_base)?;
        if old_dir.exists() {
            move_dir(&old_dir, &new_dir)?;
        }
        match place {
            Some(place) => self.locations.insert(app, place.to_path_buf()),
            None => self.locations.remove(&app),
        };
        self.save_locations()
    }

    fn derive(&self, label: &[u8]) -> [u8; 32] {
        hmac_sha256(&self.master, label)
    }

    fn app_key(&self, app: u32) -> [u8; 32] {
        self.derive(&[b"private-app:".as_slice(), &app.to_le_bytes()].concat())
    }

    fn shared_key(&self) -> [u8; 32] {
        self.derive(b"shared")
    }

    // ---- Identite des apps ----

    /// Associe l'app `app` a l'executable `exe` la PREMIERE fois qu'elle se
    /// presente ; ensuite, seul ce meme executable peut se presenter comme
    /// `app` (sinon n'importe quel programme pourrait lire ses donnees en
    /// annoncant son id).
    pub fn bind_app(&self, app: u32, exe: &str) -> Result<(), String> {
        let path = self.root.join("apps.bin");
        let key = self.derive(b"registry");
        let mut apps = self.bound_apps()?;
        match apps.iter().find(|(id, _)| *id == app) {
            Some((_, known)) if known == exe => Ok(()),
            Some((_, known)) => Err(format!("L'app {app} appartient a l'executable {known}, pas a {exe}")),
            None => {
                apps.push((app, exe.to_string()));
                let mut writer = Writer::new().u32(apps.len() as u32);
                for (id, path) in &apps {
                    writer = writer.u32(*id).str(path);
                }
                write_atomic(&path, &seal(&key, b"apps", &writer.finish())?)
            }
        }
    }

    /// Les apps qui se sont presentees sans azure-manager : (id, executable).
    pub fn bound_apps(&self) -> Result<Vec<(u32, String)>, String> {
        let mut apps = Vec::new();
        if let Some(data) = read_optional(&self.root.join("apps.bin"))? {
            let plain = open(&self.derive(b"registry"), b"apps", &data)?;
            let mut reader = Reader::new(&plain);
            for _ in 0..reader.u32()? {
                apps.push((reader.u32()?, reader.str()?));
            }
        }
        Ok(apps)
    }

    // ---- Stockage prive ----

    pub(crate) fn app_key_for(&self, app: u32) -> [u8; 32] {
        self.app_key(app)
    }

    pub(crate) fn private_dir(&self, app: u32) -> PathBuf {
        let base = self.locations.get(&app).cloned().unwrap_or_else(|| self.root.join("private"));
        base.join(&hex(&hmac_sha256(&self.app_key(app), b"dir"))[..32])
    }

    fn private_path(&self, app: u32, key: &StorageKey) -> PathBuf {
        let id = hmac_sha256(&self.app_key(app), &[b"key:".as_slice(), key.as_str().as_bytes()].concat());
        self.private_dir(app).join(format!("{}.bin", hex(&id)))
    }

    fn private_aad(app: u32) -> Vec<u8> {
        [b"private:".as_slice(), &app.to_le_bytes()].concat()
    }

    // Dechiffre un fichier prive et verifie qu'il est bien a sa place (le
    // nom de la cle est dans le contenu chiffre) : un fichier copie ou
    // deplace sous le nom d'une autre cle est refuse.
    fn open_private(&self, app: u32, path: &Path) -> Result<Option<(String, Vec<u8>)>, String> {
        let Some(data) = read_optional(path)? else { return Ok(None) };
        let plain = open(&self.app_key(app), &Self::private_aad(app), &data)?;
        let mut reader = Reader::new(&plain);
        let key = reader.str()?;
        let value = reader.bytes()?.to_vec();
        reader.finish()?;
        if self.private_path(app, &StorageKey::new(&key)?) != path {
            return Err("Donnee privee a la mauvaise place".to_string());
        }
        Ok(Some((key, value)))
    }

    /// Enregistre `value` sous `key` dans l'espace prive de `app` (remplace
    /// l'ancienne valeur).
    pub fn put(&self, app: u32, key: &str, value: &[u8]) -> Result<(), String> {
        let key = StorageKey::new(key)?;
        check_value(value)?;
        make_dir(&self.private_dir(app))?;
        let plain = Writer::new().str(key.as_str()).bytes(value).finish();
        write_atomic(&self.private_path(app, &key), &seal(&self.app_key(app), &Self::private_aad(app), &plain)?)
    }

    pub fn get(&self, app: u32, key: &str) -> Result<Option<Vec<u8>>, String> {
        let key = StorageKey::new(key)?;
        Ok(self.open_private(app, &self.private_path(app, &key))?.map(|(_, value)| value))
    }

    /// `true` si la cle existait.
    pub fn delete(&self, app: u32, key: &str) -> Result<bool, String> {
        remove_optional(&self.private_path(app, &StorageKey::new(key)?))
    }

    /// Les cles de l'espace prive de `app`, triees.
    pub fn keys(&self, app: u32) -> Result<Vec<String>, String> {
        let dir = self.private_dir(app);
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io(&dir, e)),
        };
        let mut keys = Vec::new();
        for entry in entries {
            let path = entry.map_err(|e| io(&dir, e))?.path();
            if path.extension().is_some_and(|ext| ext == "bin") {
                keys.extend(self.open_private(app, &path)?.map(|(key, _)| key));
            }
        }
        keys.sort();
        Ok(keys)
    }

    // ---- Comptes ----

    fn accounts_path(&self, owner: u32) -> PathBuf {
        let id = hmac_sha256(&self.shared_key(), &[b"accounts:".as_slice(), &owner.to_le_bytes()].concat());
        self.root.join("accounts").join(format!("{}.bin", hex(&id)))
    }

    fn accounts_aad(owner: u32) -> Vec<u8> {
        [b"accounts:".as_slice(), &owner.to_le_bytes()].concat()
    }

    /// Le role du compte `user` de l'app `owner` si le mot de passe est bon.
    pub(crate) fn check_account(&self, owner: u32, user: &str, password: &str) -> Result<Option<Role>, String> {
        Ok(self.load_accounts(owner)?.iter().find(|a| a.user == user && a.check_password(password)).map(|a| a.role))
    }

    fn load_accounts(&self, owner: u32) -> Result<Vec<Account>, String> {
        let Some(data) = read_optional(&self.accounts_path(owner))? else { return Ok(Vec::new()) };
        let plain = open(&self.shared_key(), &Self::accounts_aad(owner), &data)?;
        let mut reader = Reader::new(&plain);
        (0..reader.u32()?).map(|_| Account::read(&mut reader)).collect()
    }

    fn save_accounts(&self, owner: u32, accounts: &[Account]) -> Result<(), String> {
        let mut writer = Writer::new().u32(accounts.len() as u32);
        for account in accounts {
            writer = account.write(writer);
        }
        write_atomic(&self.accounts_path(owner), &seal(&self.shared_key(), &Self::accounts_aad(owner), &writer.finish())?)
    }

    /// Cree (ou remplace) le compte `user` de l'app `owner`, qui donnera
    /// acces a ses donnees partagees `Protected` selon `role`.
    pub fn add_account(&self, owner: u32, user: &str, password: &str, role: Role) -> Result<(), String> {
        let user = StorageKey::new(user)?;
        let mut accounts = self.load_accounts(owner)?;
        accounts.retain(|a| a.user != user.as_str());
        accounts.push(Account::new(user.as_str(), password, role)?);
        self.save_accounts(owner, &accounts)
    }

    pub fn remove_account(&self, owner: u32, user: &str) -> Result<bool, String> {
        let mut accounts = self.load_accounts(owner)?;
        let before = accounts.len();
        accounts.retain(|a| a.user != user);
        if accounts.len() == before {
            return Ok(false);
        }
        self.save_accounts(owner, &accounts)?;
        Ok(true)
    }

    /// Les comptes de `owner` (nom + role, jamais les mots de passe).
    pub fn accounts(&self, owner: u32) -> Result<Vec<(String, Role)>, String> {
        Ok(self.load_accounts(owner)?.into_iter().map(|a| (a.user, a.role)).collect())
    }

    // ---- Stockage partage ----

    fn shared_path(&self, owner: u32, name: &StorageKey) -> PathBuf {
        let id = hmac_sha256(&self.shared_key(), &[&owner.to_le_bytes(), name.as_str().as_bytes()].concat());
        self.root.join("shared").join(format!("{}.bin", hex(&id)))
    }

    fn load_shared(&self, owner: u32, name: &StorageKey) -> Result<Option<SharedEntry>, String> {
        let Some(data) = read_optional(&self.shared_path(owner, name))? else { return Ok(None) };
        let entry = decode_shared(&open(&self.shared_key(), b"shared", &data)?)?;
        // Fichier deplace / copie sous un autre nom : refuse.
        if entry.owner != owner || entry.name != name.as_str() {
            return Err("Donnee partagee a la mauvaise place".to_string());
        }
        Ok(Some(entry))
    }

    fn save_shared(&self, entry: &SharedEntry) -> Result<(), String> {
        let plain = Writer::new().u32(entry.owner).str(&entry.name).u32(entry.access.code()).bytes(&entry.value).finish();
        write_atomic(&self.shared_path(entry.owner, &StorageKey::new(&entry.name)?), &seal(&self.shared_key(), b"shared", &plain)?)
    }

    /// L'app `owner` partage `value` sous `name` (cree ou remplace, valeur
    /// ET acces).
    pub fn share(&self, owner: u32, name: &str, value: &[u8], access: ShareAccess) -> Result<(), String> {
        let name = StorageKey::new(name)?;
        check_value(value)?;
        self.save_shared(&SharedEntry { owner, name: name.as_str().to_string(), access, value: value.to_vec() })
    }

    /// Retire un partage de `owner`. `true` s'il existait.
    pub fn unshare(&self, owner: u32, name: &str) -> Result<bool, String> {
        remove_optional(&self.shared_path(owner, &StorageKey::new(name)?))
    }

    // Le role accorde a `app` sur `entry`, `None` = aucun acces. L'app
    // proprietaire a toujours tous les droits.
    fn role_for(&self, app: u32, entry: &SharedEntry, credentials: Option<Credentials>) -> Result<Option<Role>, String> {
        if app == entry.owner {
            return Ok(Some(Role::Writer));
        }
        if let Some(creds) = credentials {
            let accounts = self.load_accounts(entry.owner)?;
            return Ok(accounts.iter().find(|a| a.user == creds.user && a.check_password(creds.password)).map(|a| a.role));
        }
        Ok((entry.access == ShareAccess::Public).then_some(Role::Reader))
    }

    /// L'app `app` lit la donnee partagee `name` de `owner`. Une donnee
    /// `Protected` demande un compte de `owner` ; une donnee `Public` se lit
    /// sans (des identifiants faux restent refuses).
    pub fn read_shared(&self, app: u32, owner: u32, name: &str, credentials: Option<Credentials>) -> Result<Vec<u8>, String> {
        let name = StorageKey::new(name)?;
        let entry = self.load_shared(owner, &name)?.ok_or_else(|| format!("Aucun partage '{}' pour l'app {owner}", name.as_str()))?;
        match self.role_for(app, &entry, credentials)? {
            Some(role) if role.can_read() => Ok(entry.value),
            _ => Err(format!("Acces refuse a '{}' de l'app {owner}", name.as_str())),
        }
    }

    /// L'app `app` modifie la VALEUR d'une donnee partagee de `owner` : il
    /// faut etre `owner` ou avoir un compte `Writer`.
    pub fn write_shared(&self, app: u32, owner: u32, name: &str, value: &[u8], credentials: Option<Credentials>) -> Result<(), String> {
        let name = StorageKey::new(name)?;
        check_value(value)?;
        let mut entry = self.load_shared(owner, &name)?.ok_or_else(|| format!("Aucun partage '{}' pour l'app {owner}", name.as_str()))?;
        match self.role_for(app, &entry, credentials)? {
            Some(role) if role.can_write() => {
                entry.value = value.to_vec();
                self.save_shared(&entry)
            }
            _ => Err(format!("Ecriture refusee sur '{}' de l'app {owner}", name.as_str())),
        }
    }

    /// Les donnees partagees par `owner` (noms + acces, sans les valeurs).
    pub fn shared_list(&self, owner: u32) -> Result<Vec<SharedInfo>, String> {
        let dir = self.root.join("shared");
        let mut list = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|e| io(&dir, e))? {
            let path = entry.map_err(|e| io(&dir, e))?.path();
            if path.extension().is_some_and(|ext| ext == "bin") {
                let data = fs::read(&path).map_err(|e| io(&path, e))?;
                let shared = decode_shared(&open(&self.shared_key(), b"shared", &data)?)?;
                if shared.owner == owner && self.shared_path(owner, &StorageKey::new(&shared.name)?) == path {
                    list.push(SharedInfo { name: shared.name, access: shared.access });
                }
            }
        }
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }
}

// `rename`, ou copie + suppression quand la destination est sur un autre
// disque (`rename` refuse alors de traverser).
fn move_dir(from: &Path, to: &Path) -> Result<(), String> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_dir(from, to)?;
    fs::remove_dir_all(from).map_err(|e| io(from, e))
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    make_dir(to)?;
    for entry in fs::read_dir(from).map_err(|e| io(from, e))? {
        let path = entry.map_err(|e| io(from, e))?.path();
        let target = to.join(path.file_name().ok_or("Nom de fichier invalide")?);
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            let data = fs::read(&path).map_err(|e| io(&path, e))?;
            write_atomic(&target, &data)?;
        }
    }
    Ok(())
}

fn decode_shared(plain: &[u8]) -> Result<SharedEntry, String> {
    let mut reader = Reader::new(plain);
    let owner = reader.u32()?;
    let name = reader.str()?;
    let access = ShareAccess::from_code(reader.u32()?).ok_or("Acces inconnu")?;
    let value = reader.bytes()?.to_vec();
    reader.finish()?;
    Ok(SharedEntry { owner, name, access, value })
}
