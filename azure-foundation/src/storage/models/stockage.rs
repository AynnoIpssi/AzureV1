use crate::compiler::services::condition::Context;
use crate::storage::models::db::Db;
use crate::storage::models::remote::Remote;
use crate::storage::models::share::Share;
use crate::storage::models::store_value::{FromStore, ToStore};
use azure_core::models::storage_model::Role;
use azure_stockage::services::client::StockageClient;
use std::sync::{Arc, Mutex};

/// Le stockage d'UNE app, via le daemon azure-stockage. Se clone librement
/// (toutes les copies partagent la meme connexion) : une copie pour la
/// fenetre (`AzureWindow::stockage`), une autre capturee dans une route...
#[derive(Clone)]
pub struct Stockage {
    client: Arc<Mutex<StockageClient>>,
    // Prefixe des cles privees (voir `scope`), vide = aucun.
    prefix: String,
}

impl Stockage {
    /// Se connecte au daemon en tant qu'app `app_id`. Le daemon est lance
    /// par azure-provider s'il ne tourne pas (voir `crate::provider`).
    pub fn connect(app_id: u32) -> Result<Stockage, String> {
        crate::provider::with_service("stockage", || StockageClient::connect(app_id)).map(Stockage::from_client)
    }

    /// Comme `connect`, sur un autre socket (tests).
    pub fn connect_at(socket: &str, app_id: u32) -> Result<Stockage, String> {
        Ok(Stockage::from_client(StockageClient::connect_at(socket, app_id)?))
    }

    pub fn from_client(client: StockageClient) -> Stockage {
        Stockage { client: Arc::new(Mutex::new(client)), prefix: String::new() }
    }

    pub(crate) fn with<T>(&self, action: impl FnOnce(&mut StockageClient) -> Result<T, String>) -> Result<T, String> {
        let mut client = self.client.lock().unwrap_or_else(|e| e.into_inner());
        action(&mut client)
    }

    pub fn app_id(&self) -> u32 {
        self.with(|c| Ok(c.app_id())).unwrap_or(0)
    }

    fn full_key(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }

    // ---- Donnees privees ----

    /// Range `value` sous `key` (remplace l'ancienne valeur).
    pub fn set(&self, key: &str, value: impl ToStore) -> Result<(), String> {
        let key = self.full_key(key);
        self.with(|c| c.put(&key, &value.to_store()))
    }

    /// La valeur de `key`, `None` si elle n'existe pas.
    pub fn get<T: FromStore>(&self, key: &str) -> Result<Option<T>, String> {
        let key = self.full_key(key);
        self.with(|c| c.get(&key))?.map(|bytes| T::from_store(&bytes)).transpose()
    }

    /// La valeur de `key`, ou `default` si elle n'existe pas OU ne se lit
    /// pas (l'erreur est affichee sur stderr) : pratique dans un `on_click`.
    pub fn get_or<T: FromStore>(&self, key: &str, default: T) -> T {
        match self.get(key) {
            Ok(value) => value.unwrap_or(default),
            Err(err) => {
                eprintln!("Stockage: '{key}' : {err}");
                default
            }
        }
    }

    pub fn has(&self, key: &str) -> Result<bool, String> {
        Ok(self.get::<Vec<u8>>(key)?.is_some())
    }

    /// Supprime `key` ; `true` si elle existait.
    pub fn forget(&self, key: &str) -> Result<bool, String> {
        let key = self.full_key(key);
        self.with(|c| c.delete(&key))
    }

    /// Lit `key` (ou part de `default`), applique `change`, range et rend
    /// le resultat : `store.update("clics", 0, |n: i64| n + 1)`.
    pub fn update<T: FromStore + ToStore>(&self, key: &str, default: T, change: impl FnOnce(T) -> T) -> Result<T, String> {
        let value = change(self.get(key)?.unwrap_or(default));
        self.set(key, &value)?;
        Ok(value)
    }

    /// La valeur de `key` ; si elle n'existe pas, `compute` la calcule, et
    /// elle est rangee pour la prochaine fois.
    pub fn remember<T: FromStore + ToStore>(&self, key: &str, compute: impl FnOnce() -> T) -> Result<T, String> {
        if let Some(value) = self.get(key)? {
            return Ok(value);
        }
        let value = compute();
        self.set(key, &value)?;
        Ok(value)
    }

    /// Les cles de cet espace (sans le prefixe de `scope`), triees.
    pub fn keys(&self) -> Result<Vec<String>, String> {
        let keys = self.with(|c| c.keys())?;
        Ok(keys.into_iter().filter_map(|k| k.strip_prefix(&self.prefix).map(str::to_string)).collect())
    }

    /// Supprime toutes les cles de cet espace (seulement celles du `scope`
    /// si c'en est un). Rend le nombre de cles supprimees.
    pub fn clear(&self) -> Result<usize, String> {
        let keys = self.keys()?;
        for key in &keys {
            self.forget(key)?;
        }
        Ok(keys.len())
    }

    /// Un sous-espace : `store.scope("reglages").set("theme", "sombre")`
    /// range la cle `reglages.theme`. Les scopes s'imbriquent.
    pub fn scope(&self, name: &str) -> Stockage {
        Stockage { client: Arc::clone(&self.client), prefix: format!("{}{name}.", self.prefix) }
    }

    /// Un `Context` rsH avec ces cles en texte, pour les conditions :
    /// `build_ui_with_context(&ast, &style, &store.context(&["theme"]))`
    /// puis `<if.theme == "sombre">` dans le rsH. Une cle absente ou non
    /// textuelle n'y est pas.
    pub fn context(&self, keys: &[&str]) -> Context {
        keys.iter().fold(Context::new(), |ctx, key| match self.get::<String>(key) {
            Ok(Some(value)) => ctx.with_text(key, &value),
            _ => ctx,
        })
    }

    // ---- RsS ----

    /// La base RsS (SQL) de cette app, sur la meme connexion (voir `Db`).
    pub fn db(&self) -> Db {
        Db::new(self)
    }

    /// L'acces admin aux donnees de toutes les apps (Azure Data seulement,
    /// voir `Admin`).
    pub fn admin(&self) -> crate::storage::models::admin::Admin {
        crate::storage::models::admin::Admin::new(self)
    }

    // ---- Emplacement ----

    /// Range les donnees privees de cette app (cles et tables RsS) dans le
    /// dossier `place` de la machine, par exemple `/mnt/disque/notes`. Ce
    /// qui existe deja y est deplace, toujours chiffre.
    pub fn move_to(&self, place: &str) -> Result<(), String> {
        self.with(|c| c.set_location(Some(place)))
    }

    /// Ramene les donnees a l'endroit par defaut du daemon.
    pub fn reset_location(&self) -> Result<(), String> {
        self.with(|c| c.set_location(None))
    }

    /// Le dossier choisi avec `move_to`, `None` = endroit par defaut.
    pub fn location(&self) -> Result<Option<String>, String> {
        self.with(|c| c.location())
    }

    // ---- Partage ----

    /// Commence le partage de `name` (voir `Share`) ; rien n'est envoye
    /// avant `.save()`.
    pub fn share(&self, name: &str) -> Share<'_> {
        Share::new(self, name)
    }

    /// Retire un partage ; `true` s'il existait.
    pub fn unshare(&self, name: &str) -> Result<bool, String> {
        self.with(|c| c.unshare(name))
    }

    /// Les donnees partagees par une AUTRE app `owner` (voir `Remote`).
    pub fn from(&self, owner: u32) -> Remote<'_> {
        Remote::new(self, owner)
    }

    /// Les donnees partagees par CETTE app, vues comme une autre les voit.
    pub fn shared(&self) -> Remote<'_> {
        Remote::new(self, self.app_id())
    }

    // ---- Comptes ----

    /// Cree ou remplace un compte de cette app (voir `ShareAccess::Protected`).
    pub fn add_account(&self, user: &str, password: &str, role: Role) -> Result<(), String> {
        self.with(|c| c.add_account(user, password, role))
    }

    pub fn remove_account(&self, user: &str) -> Result<bool, String> {
        self.with(|c| c.remove_account(user))
    }

    /// Les comptes de cette app (nom + role, jamais les mots de passe).
    pub fn accounts(&self) -> Result<Vec<(String, Role)>, String> {
        self.with(|c| c.accounts())
    }
}
