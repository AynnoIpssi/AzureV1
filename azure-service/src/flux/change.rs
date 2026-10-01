// Ce qui circule dans un flux : des modifications de l'etat partage
// (`Set`, `Delete`, `Push`) et des evenements libres (`Event`), qui ne
// changent pas l'etat.
use crate::flux::path::Path;
use crate::flux::value::Value;
use azure_core::models::wire::{Reader, Writer};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// Remplace (ou cree) la valeur a ce chemin. Les objets manquants sur
    /// le chemin sont crees. `""` remplace tout l'etat (un objet).
    Set(String, Value),
    /// Retire la valeur (une case de liste decale les suivantes). Absente :
    /// rien a faire.
    Delete(String),
    /// Ajoute a la fin de la liste a ce chemin (creee si absente).
    Push(String, Value),
    /// Evenement nomme, avec un contenu.
    Event(String, Value),
}

impl Change {
    pub fn set(path: &str, value: impl Into<Value>) -> Change {
        Change::Set(path.to_string(), value.into())
    }

    pub fn delete(path: &str) -> Change {
        Change::Delete(path.to_string())
    }

    pub fn push(path: &str, value: impl Into<Value>) -> Change {
        Change::Push(path.to_string(), value.into())
    }

    pub fn event(name: &str, value: impl Into<Value>) -> Change {
        Change::Event(name.to_string(), value.into())
    }

    /// Chemin modifie, `None` pour un evenement.
    pub fn path(&self) -> Option<&str> {
        match self {
            Change::Set(path, _) | Change::Delete(path) | Change::Push(path, _) => Some(path),
            Change::Event(..) => None,
        }
    }

    /// Verifie la forme (chemin, nom d'evenement) sans rien appliquer.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Change::Set(path, value) => {
                let path = Path::parse(path)?;
                if path.is_root() && !matches!(value, Value::Map(_)) {
                    return Err("L'etat d'un flux est un objet : Set(\"\", ...) attend un objet".to_string());
                }
                Ok(())
            }
            Change::Delete(path) | Change::Push(path, _) => Path::parse(path).map(|_| ()),
            Change::Event(name, _) => {
                if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
                    Err(format!("Nom d'evenement invalide : '{name}'"))
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Applique la modification a `state` (un evenement ne change rien).
    pub fn apply(&self, state: &mut Value) -> Result<(), String> {
        self.validate()?;
        match self {
            Change::Set(path, value) => {
                let path = Path::parse(path)?;
                match path.segments().split_last() {
                    None => *state = value.clone(),
                    Some((last, parents)) => {
                        let parent = walk_creating(state, parents, &path)?;
                        put(parent, last, value.clone(), &path)?;
                    }
                }
            }
            Change::Delete(path) => {
                let path = Path::parse(path)?;
                match path.segments().split_last() {
                    None => *state = Value::empty_map(),
                    Some((last, parents)) => {
                        let Some(parent) = walk(state, parents) else { return Ok(()) };
                        match parent {
                            Value::Map(map) => {
                                map.remove(last);
                            }
                            Value::List(items) => {
                                if let Some(i) = last.parse::<usize>().ok().filter(|i| *i < items.len()) {
                                    items.remove(i);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Change::Push(path, value) => {
                let path = Path::parse(path)?;
                let target = walk_creating(state, path.segments(), &path)?;
                match target {
                    Value::List(items) => items.push(value.clone()),
                    Value::Null => *target = Value::List(vec![value.clone()]),
                    _ => return Err(format!("'{}' n'est pas une liste", path.as_string())),
                }
            }
            Change::Event(..) => {}
        }
        Ok(())
    }

    pub fn write(&self, w: Writer) -> Writer {
        match self {
            Change::Set(path, value) => value.write(w.u8(0).str(path)),
            Change::Delete(path) => w.u8(1).str(path),
            Change::Push(path, value) => value.write(w.u8(2).str(path)),
            Change::Event(name, value) => value.write(w.u8(3).str(name)),
        }
    }

    pub fn read(r: &mut Reader) -> Result<Change, String> {
        Ok(match r.u8()? {
            0 => Change::Set(r.str()?, Value::read(r)?),
            1 => Change::Delete(r.str()?),
            2 => Change::Push(r.str()?, Value::read(r)?),
            3 => Change::Event(r.str()?, Value::read(r)?),
            other => return Err(format!("Modification inconnue : {other}")),
        })
    }
}

/// Descend dans `state` sans rien creer.
fn walk<'a>(state: &'a mut Value, segments: &[String]) -> Option<&'a mut Value> {
    segments.iter().try_fold(state, |value, segment| match value {
        Value::Map(map) => map.get_mut(segment),
        Value::List(items) => segment.parse::<usize>().ok().and_then(|i| items.get_mut(i)),
        _ => None,
    })
}

/// Descend dans `state` en creant les objets manquants (`Null` compris).
fn walk_creating<'a>(state: &'a mut Value, segments: &[String], full: &Path) -> Result<&'a mut Value, String> {
    let mut value = state;
    for segment in segments {
        if value.is_null() {
            *value = Value::Map(BTreeMap::new());
        }
        value = match value {
            Value::Map(map) => map.entry(segment.clone()).or_insert(Value::Null),
            Value::List(items) => {
                let len = items.len();
                segment.parse::<usize>().ok().and_then(|i| items.get_mut(i)).ok_or_else(|| format!("'{}' : pas de case {segment} dans une liste de {len}", full.as_string()))?
            }
            _ => return Err(format!("'{}' : '{segment}' est sous une valeur qui n'est ni un objet ni une liste", full.as_string())),
        };
    }
    Ok(value)
}

fn put(parent: &mut Value, key: &str, value: Value, full: &Path) -> Result<(), String> {
    if parent.is_null() {
        *parent = Value::Map(BTreeMap::new());
    }
    match parent {
        Value::Map(map) => {
            map.insert(key.to_string(), value);
            Ok(())
        }
        Value::List(items) => match key.parse::<usize>() {
            Ok(i) if i < items.len() => {
                items[i] = value;
                Ok(())
            }
            // Juste apres la derniere case : ajoute.
            Ok(i) if i == items.len() => {
                items.push(value);
                Ok(())
            }
            _ => Err(format!("'{}' : pas de case {key} dans une liste de {}", full.as_string(), items.len())),
        },
        _ => Err(format!("'{}' : la valeur parente n'est ni un objet ni une liste", full.as_string())),
    }
}

/// Ce qu'une ecoute veut recevoir. Aucun chemin = tout l'etat ; aucun
/// evenement = tous les evenements.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filter {
    pub paths: Vec<Path>,
    pub events: Vec<String>,
}

impl Filter {
    pub fn wants(&self, change: &Change) -> bool {
        match change {
            Change::Event(name, _) => self.events.is_empty() || self.events.iter().any(|e| e == "*" || e == name),
            other => {
                let Ok(path) = Path::parse(other.path().unwrap_or("")) else { return false };
                self.paths.is_empty() || self.paths.iter().any(|pattern| pattern.related(&path))
            }
        }
    }

    /// La partie de l'etat que l'ecoute a demandee (tout, sans chemin).
    /// Sous une liste, la liste entiere est gardee.
    pub fn extract(&self, state: &Value) -> Value {
        if self.paths.is_empty() {
            return state.clone();
        }
        let mut out = Value::empty_map();
        for pattern in &self.paths {
            pick(state, pattern.segments(), &mut Vec::new(), &mut out);
        }
        out
    }

    pub fn write(&self, w: Writer) -> Writer {
        let w = self.paths.iter().fold(w.u32(self.paths.len() as u32), |w, p| w.str(&p.as_string()));
        self.events.iter().fold(w.u32(self.events.len() as u32), |w, e| w.str(e))
    }

    pub fn read(r: &mut Reader) -> Result<Filter, String> {
        let count = r.u32()?.min(1024);
        let paths = (0..count).map(|_| Path::pattern(&r.str()?)).collect::<Result<Vec<_>, _>>()?;
        let count = r.u32()?.min(1024);
        let events = (0..count).map(|_| r.str()).collect::<Result<Vec<_>, _>>()?;
        Ok(Filter { paths, events })
    }
}

/// Copie dans `out`, au meme chemin, ce que `pattern` designe dans `value`.
fn pick(value: &Value, pattern: &[String], at: &mut Vec<String>, out: &mut Value) {
    let Some((first, rest)) = pattern.split_first() else {
        let mut target = out;
        for key in at.iter() {
            if !matches!(target, Value::Map(_)) {
                *target = Value::empty_map();
            }
            target = match target {
                Value::Map(map) => map.entry(key.clone()).or_insert(Value::Null),
                _ => return,
            };
        }
        *target = value.clone();
        return;
    };
    match value {
        Value::Map(map) => {
            let keys: Vec<&String> = if first == "*" { map.keys().collect() } else { map.get_key_value(first).map(|(k, _)| k).into_iter().collect() };
            for key in keys {
                at.push(key.clone());
                pick(&map[key], rest, at, out);
                at.pop();
            }
        }
        // Une liste est copiee entiere (des cases isolees n'auraient plus
        // leur numero).
        Value::List(_) => pick(value, &[], at, out),
        _ => {}
    }
}
