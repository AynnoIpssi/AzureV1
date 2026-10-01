// Les regles du stockage Azure (voir le crate azure-stockage), partagees par
// le daemon et les apps :
// - chaque app a un espace PRIVE, chiffre, que seule elle peut lire ;
// - une app peut PARTAGER une donnee : publique (toute app la lit) ou
//   protegee (il faut un compte utilisateur + mot de passe, cree par l'app
//   proprietaire, avec un role).

/// Longueur maximale d'une cle ou d'un nom (octets UTF-8).
pub const MAX_KEY_LEN: usize = 256;

/// Longueur maximale d'une valeur stockee (16 Mo).
pub const MAX_VALUE_LEN: usize = 16 * 1024 * 1024;

/// Nom d'une donnee (cle privee, nom de partage, nom de compte). Jamais
/// vide, au plus `MAX_KEY_LEN` octets, sans caractere de controle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorageKey(String);

impl StorageKey {
    pub fn new(key: &str) -> Result<StorageKey, String> {
        if key.is_empty() {
            return Err("Storage key cannot be empty".to_string());
        }
        if key.len() > MAX_KEY_LEN {
            return Err(format!("Storage key is too long ({} > {MAX_KEY_LEN} bytes)", key.len()));
        }
        if key.chars().any(char::is_control) {
            return Err(format!("Storage key cannot contain control characters: {key:?}"));
        }
        Ok(StorageKey(key.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Verifie la taille d'une valeur avant de la stocker.
pub fn check_value(value: &[u8]) -> Result<(), String> {
    if value.len() > MAX_VALUE_LEN {
        return Err(format!("Value is too large ({} > {MAX_VALUE_LEN} bytes)", value.len()));
    }
    Ok(())
}

/// Role d'un compte sur les donnees protegees d'une app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Peut lire.
    Reader,
    /// Peut lire et modifier la valeur (pas la supprimer ni changer l'acces).
    Writer,
}

impl Role {
    pub fn code(&self) -> u32 {
        match self {
            Role::Reader => 0,
            Role::Writer => 1,
        }
    }

    pub fn from_code(code: u32) -> Option<Role> {
        match code {
            0 => Some(Role::Reader),
            1 => Some(Role::Writer),
            _ => None,
        }
    }

    pub fn can_read(&self) -> bool {
        true
    }

    pub fn can_write(&self) -> bool {
        *self == Role::Writer
    }
}

/// Qui peut lire une donnee partagee (l'app proprietaire peut toujours
/// tout faire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareAccess {
    /// Toute app peut la lire, sans compte. Seule l'app proprietaire la modifie.
    Public,
    /// Il faut un compte de l'app proprietaire : lecture pour `Reader`,
    /// lecture + ecriture pour `Writer`.
    Protected,
}

impl ShareAccess {
    pub fn code(&self) -> u32 {
        match self {
            ShareAccess::Public => 0,
            ShareAccess::Protected => 1,
        }
    }

    pub fn from_code(code: u32) -> Option<ShareAccess> {
        match code {
            0 => Some(ShareAccess::Public),
            1 => Some(ShareAccess::Protected),
            _ => None,
        }
    }
}
