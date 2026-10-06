use crate::storage::models::stockage::Stockage;
use azure_stockage::rss::engine::RssResult;
use azure_stockage::rss::value::Value;
pub use azure_stockage::services::client::{AdminApp, AdminTable};

/// L'acces admin aux donnees de TOUTES les apps, reserve a Azure Data (le
/// daemon de stockage refuse les autres, voir SECURITE.md) :
///
/// ```text
/// let admin = store.admin();
/// for app in admin.apps()? {
///     for table in admin.schema(app.id)? { ... }
/// }
/// let lignes = admin.rss(app.id, "SELECT * FROM notes LIMIT 50", &[])?;
/// ```
#[derive(Clone)]
pub struct Admin {
    store: Stockage,
}

impl Admin {
    pub(crate) fn new(store: &Stockage) -> Admin {
        Admin { store: store.clone() }
    }

    /// Les apps qui ont un espace de stockage.
    pub fn apps(&self) -> Result<Vec<AdminApp>, String> {
        self.store.with(|c| c.admin_apps())
    }

    /// Les tables RsS de l'app `owner` (colonnes, index, nombre de lignes).
    pub fn schema(&self, owner: u32) -> Result<Vec<AdminTable>, String> {
        self.store.with(|c| c.admin_schema(owner))
    }

    /// Du RsS au nom de l'app `owner` : un resultat par instruction.
    pub fn rss(&self, owner: u32, sql: &str, params: &[Value]) -> Result<Vec<RssResult>, String> {
        self.store.with(|c| c.admin_rss(owner, sql, params))
    }

    /// Les cles privees de l'app `owner` et la taille de leur valeur.
    pub fn keys(&self, owner: u32) -> Result<Vec<(String, u64)>, String> {
        self.store.with(|c| c.admin_keys(owner))
    }

    pub fn get(&self, owner: u32, key: &str) -> Result<Option<Vec<u8>>, String> {
        self.store.with(|c| c.admin_get(owner, key))
    }
}
