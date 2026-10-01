use crate::storage::models::stockage::Stockage;
use crate::storage::models::store_value::{FromStore, ToStore};
use azure_stockage::managers::stockage::{Credentials, SharedInfo};

/// Les donnees partagees par l'app `owner`, vues depuis cette app :
///
/// ```text
/// let recette: String = store.from(10).get("recette")?;
/// let journal: String = store.from(10).login("maman", "1234").get("journal")?;
/// store.from(10).login("papa", "abcd").set("journal", "nouveau")?;
/// ```
pub struct Remote<'a> {
    store: &'a Stockage,
    owner: u32,
    login: Option<(String, String)>,
}

impl<'a> Remote<'a> {
    pub(crate) fn new(store: &'a Stockage, owner: u32) -> Remote<'a> {
        Remote { store, owner, login: None }
    }

    /// Se presente avec un compte de l'app `owner` (donnees protegees).
    pub fn login(mut self, user: &str, password: &str) -> Remote<'a> {
        self.login = Some((user.to_string(), password.to_string()));
        self
    }

    fn credentials(&self) -> Option<Credentials<'_>> {
        self.login.as_ref().map(|(user, password)| Credentials { user, password })
    }

    /// Erreur si le partage n'existe pas ou si l'acces est refuse.
    pub fn get<T: FromStore>(&self, name: &str) -> Result<T, String> {
        let bytes = self.store.with(|c| c.read_shared(self.owner, name, self.credentials()))?;
        T::from_store(&bytes)
    }

    /// Comme `get`, mais `default` en cas d'erreur (affichee sur stderr).
    pub fn get_or<T: FromStore>(&self, name: &str, default: T) -> T {
        self.get(name).unwrap_or_else(|err| {
            eprintln!("Stockage: '{name}' de l'app {} : {err}", self.owner);
            default
        })
    }

    /// Modifie la valeur (il faut etre l'app `owner` ou un compte `Writer`).
    pub fn set(&self, name: &str, value: impl ToStore) -> Result<(), String> {
        self.store.with(|c| c.write_shared(self.owner, name, &value.to_store(), self.credentials()))
    }

    /// Ce que l'app `owner` partage (noms + acces).
    pub fn list(&self) -> Result<Vec<SharedInfo>, String> {
        self.store.with(|c| c.shared_list(self.owner))
    }
}
