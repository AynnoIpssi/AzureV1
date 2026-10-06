// Valeurs et types de RsS (RustSql, le SQL d'Azure).
use crate::models::wire::{Reader, Writer};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Int,
    Float,
    Text,
    Bool,
    /// Octets bruts (`x'CAFE'`).
    Blob,
    /// N'importe quelle valeur, rangee telle quelle (colonne sans type).
    Any,
}

impl DataType {
    pub fn name(&self) -> &'static str {
        match self {
            DataType::Int => "INT",
            DataType::Float => "FLOAT",
            DataType::Text => "TEXT",
            DataType::Bool => "BOOL",
            DataType::Blob => "BLOB",
            DataType::Any => "ANY",
        }
    }

    pub fn code(&self) -> u8 {
        match self {
            DataType::Int => 0,
            DataType::Float => 1,
            DataType::Text => 2,
            DataType::Bool => 3,
            DataType::Blob => 4,
            DataType::Any => 5,
        }
    }

    pub fn from_code(code: u8) -> Option<DataType> {
        [DataType::Int, DataType::Float, DataType::Text, DataType::Bool, DataType::Blob, DataType::Any].into_iter().find(|t| t.code() == code)
    }

    /// Le type d'un nom SQL (`INTEGER`, `VARCHAR`...), `None` si inconnu.
    pub fn from_name(name: &str) -> Option<DataType> {
        match name {
            "int" | "integer" | "bigint" | "smallint" => Some(DataType::Int),
            "float" | "real" | "double" | "decimal" | "numeric" => Some(DataType::Float),
            "text" | "varchar" | "char" | "string" => Some(DataType::Text),
            "bool" | "boolean" => Some(DataType::Bool),
            "blob" | "bytea" | "binary" => Some(DataType::Blob),
            "any" => Some(DataType::Any),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Blob(Vec<u8>),
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Value { Value::Blob(v) }
}
impl From<i64> for Value {
    fn from(v: i64) -> Value { Value::Int(v) }
}
impl From<i32> for Value {
    fn from(v: i32) -> Value { Value::Int(v as i64) }
}
impl From<u32> for Value {
    fn from(v: u32) -> Value { Value::Int(v as i64) }
}
impl From<usize> for Value {
    fn from(v: usize) -> Value { Value::Int(v as i64) }
}
impl From<f64> for Value {
    fn from(v: f64) -> Value { Value::Float(v) }
}
impl From<bool> for Value {
    fn from(v: bool) -> Value { Value::Bool(v) }
}
impl From<&str> for Value {
    fn from(v: &str) -> Value { Value::Text(v.to_string()) }
}
impl From<String> for Value {
    fn from(v: String) -> Value { Value::Text(v) }
}
impl From<&String> for Value {
    fn from(v: &String) -> Value { Value::Text(v.clone()) }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Value { v.map_or(Value::Null, Into::into) }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "NULL"),
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Text(v) => write!(f, "{v}"),
            Value::Bool(v) => write!(f, "{v}"),
            Value::Blob(v) => {
                write!(f, "x'")?;
                v.iter().try_for_each(|b| write!(f, "{b:02X}"))?;
                write!(f, "'")
            }
        }
    }
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "NULL",
            Value::Int(_) => "INT",
            Value::Float(_) => "FLOAT",
            Value::Text(_) => "TEXT",
            Value::Bool(_) => "BOOL",
            Value::Blob(_) => "BLOB",
        }
    }

    /// Vrai / faux / inconnu (NULL), pour WHERE, AND, OR, NOT.
    pub fn truth(&self) -> Result<Option<bool>, String> {
        match self {
            Value::Null => Ok(None),
            Value::Bool(b) => Ok(Some(*b)),
            Value::Int(n) => Ok(Some(*n != 0)),
            other => Err(format!("Une condition attend un BOOL, pas {}", other.type_name())),
        }
    }

    /// Comparaison SQL : `None` si l'un des deux est NULL. Erreur entre
    /// types incompatibles (TEXT contre INT...).
    pub fn compare(&self, other: &Value) -> Result<Option<Ordering>, String> {
        Ok(match (self, other) {
            (Value::Null, _) | (_, Value::Null) => None,
            (Value::Int(a), Value::Int(b)) => Some(a.cmp(b)),
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Text(a), Value::Text(b)) => Some(a.cmp(b)),
            (Value::Bool(a), Value::Bool(b)) => Some(a.cmp(b)),
            (Value::Blob(a), Value::Blob(b)) => Some(a.cmp(b)),
            (a, b) => return Err(format!("Impossible de comparer {} et {}", a.type_name(), b.type_name())),
        })
    }

    /// Ordre TOTAL pour ORDER BY, GROUP BY et les index : NULL d'abord,
    /// puis nombres, textes, booleens.
    pub fn total_cmp(&self, other: &Value) -> Ordering {
        fn rank(v: &Value) -> u8 {
            match v {
                Value::Null => 0,
                Value::Int(_) | Value::Float(_) => 1,
                Value::Text(_) => 2,
                Value::Bool(_) => 3,
                Value::Blob(_) => 4,
            }
        }
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Int(a), Value::Float(b)) => (*a as f64).total_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.total_cmp(&(*b as f64)),
            (Value::Float(a), Value::Float(b)) => a.total_cmp(b),
            (Value::Text(a), Value::Text(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Blob(a), Value::Blob(b)) => a.cmp(b),
            (a, b) => rank(a).cmp(&rank(b)),
        }
    }

    /// Convertit pour ranger dans une colonne de type `ty` (INT -> FLOAT
    /// accepte, FLOAT entier -> INT accepte, le reste refuse).
    pub fn coerce(self, ty: DataType) -> Result<Value, String> {
        match (self, ty) {
            (Value::Null, _) => Ok(Value::Null),
            (v, DataType::Any) => Ok(v),
            (v @ Value::Blob(_), DataType::Blob) => Ok(v),
            (v @ Value::Int(_), DataType::Int) => Ok(v),
            (Value::Int(n), DataType::Float) => Ok(Value::Float(n as f64)),
            (v @ Value::Float(_), DataType::Float) => Ok(v),
            (Value::Float(f), DataType::Int) if f.fract() == 0.0 && f.abs() < 9.0e15 => Ok(Value::Int(f as i64)),
            (v @ Value::Text(_), DataType::Text) => Ok(v),
            (v @ Value::Bool(_), DataType::Bool) => Ok(v),
            (v, ty) => Err(format!("La valeur {v} ({}) ne va pas dans une colonne {}", v.type_name(), ty.name())),
        }
    }

    pub fn write(&self, w: Writer) -> Writer {
        match self {
            Value::Null => w.u8(0),
            Value::Int(v) => w.u8(1).u64(*v as u64),
            Value::Float(v) => w.u8(2).u64(v.to_bits()),
            Value::Text(v) => w.u8(3).str(v),
            Value::Bool(v) => w.u8(4).u8(*v as u8),
            Value::Blob(v) => w.u8(5).bytes(v),
        }
    }

    pub fn read(r: &mut Reader) -> Result<Value, String> {
        Ok(match r.u8()? {
            0 => Value::Null,
            1 => Value::Int(r.u64()? as i64),
            2 => Value::Float(f64::from_bits(r.u64()?)),
            3 => Value::Text(r.str()?),
            4 => Value::Bool(r.u8()? != 0),
            5 => Value::Blob(r.bytes()?.to_vec()),
            tag => return Err(format!("Valeur RsS inconnue ({tag})")),
        })
    }
}

/// `Value` avec un ordre total (voir `total_cmp`), pour les cles de
/// `BTreeMap` (index, GROUP BY).
#[derive(Debug, Clone)]
pub struct Key(pub Value);

impl PartialEq for Key {
    fn eq(&self, other: &Key) -> bool {
        self.0.total_cmp(&other.0) == Ordering::Equal
    }
}
impl Eq for Key {}
impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Key) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Key {
    fn cmp(&self, other: &Key) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}
