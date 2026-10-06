// Ce qu'une methode de service recoit et renvoie : une valeur en arbre
// (rien, booleen, nombre, texte, liste, table de champs). C'est la forme
// des valeurs qu'azure-service transporte entre les apps ; azure-service
// convertit dans les deux sens (`Value::from(valeur)`).
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum Valeur {
    #[default]
    Rien,
    Booleen(bool),
    Entier(i64),
    Decimal(f64),
    Texte(String),
    Liste(Vec<Valeur>),
    Table(BTreeMap<String, Valeur>),
}

impl Valeur {
    pub fn table<K: Into<String>>(champs: impl IntoIterator<Item = (K, Valeur)>) -> Valeur {
        Valeur::Table(champs.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn liste(elements: impl IntoIterator<Item = Valeur>) -> Valeur {
        Valeur::Liste(elements.into_iter().collect())
    }

    /// Le champ `nom` d'une table.
    pub fn champ(&self, nom: &str) -> Option<&Valeur> {
        match self {
            Valeur::Table(t) => t.get(nom),
            _ => None,
        }
    }

    pub fn en_texte(&self) -> Option<&str> {
        match self {
            Valeur::Texte(t) => Some(t),
            _ => None,
        }
    }

    pub fn en_entier(&self) -> Option<i64> {
        match self {
            Valeur::Entier(n) => Some(*n),
            Valeur::Decimal(f) if f.fract() == 0.0 => Some(*f as i64),
            _ => None,
        }
    }

    pub fn en_liste(&self) -> Option<&[Valeur]> {
        match self {
            Valeur::Liste(l) => Some(l),
            _ => None,
        }
    }

    // ---- Lire les arguments d'une methode ----

    /// Le texte du champ `nom`, obligatoire.
    pub fn texte(&self, nom: &str) -> Result<&str, String> {
        self.champ(nom).and_then(Valeur::en_texte).ok_or_else(|| format!("argument « {nom} » attendu (un texte)"))
    }

    /// Le texte du champ `nom`, ou `defaut` s'il est absent.
    pub fn texte_ou<'a>(&'a self, nom: &str, defaut: &'a str) -> &'a str {
        self.champ(nom).and_then(Valeur::en_texte).unwrap_or(defaut)
    }

    /// L'entier du champ `nom`, ou `defaut` s'il est absent.
    pub fn entier_ou(&self, nom: &str, defaut: i64) -> i64 {
        self.champ(nom).and_then(Valeur::en_entier).unwrap_or(defaut)
    }

    /// Des octets : le champ `base64`, sinon le champ `texte` (en UTF-8).
    pub fn octets(&self) -> Result<Vec<u8>, String> {
        if let Some(code) = self.champ("base64").and_then(Valeur::en_texte) {
            return crate::back::encodage::base64::base64_lire(code).ok_or_else(|| "argument « base64 » illisible".to_string());
        }
        self.texte("texte").map(|t| t.as_bytes().to_vec()).map_err(|_| "argument « texte » ou « base64 » attendu".to_string())
    }
}

impl From<bool> for Valeur {
    fn from(v: bool) -> Valeur {
        Valeur::Booleen(v)
    }
}

impl From<i64> for Valeur {
    fn from(v: i64) -> Valeur {
        Valeur::Entier(v)
    }
}

impl From<usize> for Valeur {
    fn from(v: usize) -> Valeur {
        Valeur::Entier(v as i64)
    }
}

impl From<f64> for Valeur {
    fn from(v: f64) -> Valeur {
        Valeur::Decimal(v)
    }
}

impl From<&str> for Valeur {
    fn from(v: &str) -> Valeur {
        Valeur::Texte(v.to_string())
    }
}

impl From<String> for Valeur {
    fn from(v: String) -> Valeur {
        Valeur::Texte(v)
    }
}

impl<T: Into<Valeur>> From<Vec<T>> for Valeur {
    fn from(v: Vec<T>) -> Valeur {
        Valeur::Liste(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Valeur>> From<Option<T>> for Valeur {
    fn from(v: Option<T>) -> Valeur {
        v.map_or(Valeur::Rien, Into::into)
    }
}
