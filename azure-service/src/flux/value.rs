// Les valeurs d'un flux : un arbre, comme du JSON. L'etat partage d'un flux
// est toujours un `Map` a la racine.
use azure_core::models::wire::{Reader, Writer};
use std::collections::BTreeMap;
use std::fmt;

/// Profondeur maximale d'une valeur recue (une valeur trop imbriquee
/// ferait deborder la pile du daemon).
pub const MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

impl Value {
    pub fn empty_map() -> Value {
        Value::Map(BTreeMap::new())
    }

    /// `Value::map([("nom", "pomme".into()), ("prix", 2.into())])`
    pub fn map<K: Into<String>>(entries: impl IntoIterator<Item = (K, Value)>) -> Value {
        Value::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn list(items: impl IntoIterator<Item = Value>) -> Value {
        Value::List(items.into_iter().collect())
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Un entier compte aussi comme nombre.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Map(map) => Some(map),
            _ => None,
        }
    }

    /// Valeur a `path` (`"panier.items.0.nom"`), `None` si absente ou si le
    /// chemin est invalide.
    pub fn get(&self, path: &str) -> Option<&Value> {
        let path = crate::flux::path::Path::parse(path).ok()?;
        path.segments().iter().try_fold(self, |value, segment| child(value, segment))
    }

    pub fn write(&self, w: Writer) -> Writer {
        match self {
            Value::Null => w.u8(0),
            Value::Bool(b) => w.u8(1).u8(*b as u8),
            Value::Int(i) => w.u8(2).u64(*i as u64),
            Value::Float(f) => w.u8(3).u64(f.to_bits()),
            Value::Text(s) => w.u8(4).str(s),
            Value::List(items) => items.iter().fold(w.u8(5).u32(items.len() as u32), |w, item| item.write(w)),
            Value::Map(map) => map.iter().fold(w.u8(6).u32(map.len() as u32), |w, (k, v)| v.write(w.str(k))),
        }
    }

    pub fn read(r: &mut Reader) -> Result<Value, String> {
        read_at(r, 0)
    }
}

pub(crate) fn child<'a>(value: &'a Value, segment: &str) -> Option<&'a Value> {
    match value {
        Value::Map(map) => map.get(segment),
        Value::List(items) => segment.parse::<usize>().ok().and_then(|i| items.get(i)),
        _ => None,
    }
}

fn read_at(r: &mut Reader, depth: usize) -> Result<Value, String> {
    if depth > MAX_DEPTH {
        return Err(format!("Valeur trop imbriquee (plus de {MAX_DEPTH} niveaux)"));
    }
    Ok(match r.u8()? {
        0 => Value::Null,
        1 => Value::Bool(r.u8()? != 0),
        2 => Value::Int(r.u64()? as i64),
        3 => Value::Float(f64::from_bits(r.u64()?)),
        4 => Value::Text(r.str()?),
        5 => {
            let count = r.u32()?;
            let mut items = Vec::new();
            for _ in 0..count {
                items.push(read_at(r, depth + 1)?);
            }
            Value::List(items)
        }
        6 => {
            let count = r.u32()?;
            let mut map = BTreeMap::new();
            for _ in 0..count {
                let key = r.str()?;
                map.insert(key, read_at(r, depth + 1)?);
            }
            Value::Map(map)
        }
        other => return Err(format!("Type de valeur inconnu : {other}")),
    })
}

/// Ecrit comme du JSON (pour afficher, deboguer, ou passer a rsH).
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Text(s) => write!(f, "{s:?}"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Value::Map(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k:?}: {v}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Value {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Value {
        Value::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Value {
        Value::Int(i as i64)
    }
}

impl From<u32> for Value {
    fn from(i: u32) -> Value {
        Value::Int(i as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Value {
        Value::Float(f)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::Text(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::Text(s)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(items: Vec<T>) -> Value {
        Value::List(items.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Value {
        value.map(Into::into).unwrap_or(Value::Null)
    }
}

// Les valeurs des services de la librairie (azure-libraire, `service`) :
// la meme forme, dans les deux sens.
impl From<azure_libraire::service::Valeur> for Value {
    fn from(v: azure_libraire::service::Valeur) -> Value {
        use azure_libraire::service::Valeur;
        match v {
            Valeur::Rien => Value::Null,
            Valeur::Booleen(b) => Value::Bool(b),
            Valeur::Entier(n) => Value::Int(n),
            Valeur::Decimal(f) => Value::Float(f),
            Valeur::Texte(t) => Value::Text(t),
            Valeur::Liste(l) => Value::List(l.into_iter().map(Value::from).collect()),
            Valeur::Table(t) => Value::Map(t.into_iter().map(|(k, v)| (k, Value::from(v))).collect()),
        }
    }
}

impl From<&Value> for azure_libraire::service::Valeur {
    fn from(v: &Value) -> azure_libraire::service::Valeur {
        use azure_libraire::service::Valeur;
        match v {
            Value::Null => Valeur::Rien,
            Value::Bool(b) => Valeur::Booleen(*b),
            Value::Int(n) => Valeur::Entier(*n),
            Value::Float(f) => Valeur::Decimal(*f),
            Value::Text(t) => Valeur::Texte(t.clone()),
            Value::List(l) => Valeur::Liste(l.iter().map(Valeur::from).collect()),
            Value::Map(t) => Valeur::Table(t.iter().map(|(k, v)| (k.clone(), Valeur::from(v))).collect()),
        }
    }
}
