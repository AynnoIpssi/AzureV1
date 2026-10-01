use crate::storage::models::stockage::Stockage;
use crate::storage::models::store_value::ToStore;
use azure_core::models::storage_model::{Role, ShareAccess};

/// Un partage en construction, en une ligne ou en accordeon :
///
/// ```text
/// store.share("recette").value("crepes").public().save()?;
/// ```
///
/// ```text
/// store.share("journal")
///     .value("cher journal")
///     .protected()
///     .account("maman", "1234").reader()
///     .account("papa", "abcd").writer()
///     .save()?;
/// ```
///
/// `.reader()` / `.writer()` s'appliquent au DERNIER `.account(...)`
/// (`Reader` par defaut), comme `.name()` apres une route. Sans `.value()`,
/// la valeur deja partagee est gardee (pour changer seulement l'acces ou
/// ajouter des comptes). Acces par defaut : `protected()`, donc un oubli ne
/// rend jamais une donnee publique.
pub struct Share<'a> {
    store: &'a Stockage,
    name: String,
    value: Option<Vec<u8>>,
    access: ShareAccess,
    accounts: Vec<(String, String, Role)>,
}

impl<'a> Share<'a> {
    pub(crate) fn new(store: &'a Stockage, name: &str) -> Share<'a> {
        Share { store, name: name.to_string(), value: None, access: ShareAccess::Protected, accounts: Vec::new() }
    }

    pub fn value(mut self, value: impl ToStore) -> Share<'a> {
        self.value = Some(value.to_store());
        self
    }

    /// Toute app peut lire, sans compte.
    pub fn public(mut self) -> Share<'a> {
        self.access = ShareAccess::Public;
        self
    }

    /// Il faut un compte de cette app (voir `account`).
    pub fn protected(mut self) -> Share<'a> {
        self.access = ShareAccess::Protected;
        self
    }

    /// Cree (ou remplace) le compte `user` de cette app au `save`. Les
    /// comptes appartiennent a l'app, pas a ce seul partage : ils ouvrent
    /// toutes ses donnees protegees.
    pub fn account(mut self, user: &str, password: &str) -> Share<'a> {
        self.accounts.push((user.to_string(), password.to_string(), Role::Reader));
        self
    }

    fn last_role(mut self, role: Role) -> Share<'a> {
        if let Some(account) = self.accounts.last_mut() {
            account.2 = role;
        }
        self
    }

    /// Le dernier compte peut lire.
    pub fn reader(self) -> Share<'a> {
        self.last_role(Role::Reader)
    }

    /// Le dernier compte peut lire et modifier la valeur.
    pub fn writer(self) -> Share<'a> {
        self.last_role(Role::Writer)
    }

    /// Envoie tout au daemon : les comptes d'abord, puis le partage.
    pub fn save(self) -> Result<(), String> {
        let value = match self.value {
            Some(value) => value,
            None => {
                let owner = self.store.app_id();
                self.store
                    .with(|c| c.read_shared(owner, &self.name, None))
                    .map_err(|e| format!("'{}' n'est pas encore partage : donner une .value() ({e})", self.name))?
            }
        };
        for (user, password, role) in &self.accounts {
            self.store.add_account(user, password, *role)?;
        }
        self.store.with(|c| c.share(&self.name, &value, self.access))
    }
}
