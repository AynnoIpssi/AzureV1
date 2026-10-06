// Une table RsS : son schema, ses lignes, ses index. Les index (cle
// primaire, colonnes UNIQUE, CREATE INDEX) sont des `BTreeMap` valeur ->
// lignes, reconstruits en memoire (jamais ecrits sur le disque) : ils
// accelerent `WHERE colonne = valeur` et verifient l'unicite.
use crate::models::wire::{Reader, Writer};
use crate::rss::ast::ColumnDef;
use crate::rss::value::{DataType, Key, Value};
use azure_core::models::storage_model::ShareAccess;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq)]
pub struct IndexDef {
    pub name: String,
    pub column: String,
    pub unique: bool,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    /// Lignes par numero interne (ordre d'insertion).
    pub rows: BTreeMap<u64, Vec<Value>>,
    pub next_id: u64,
    pub indexes: Vec<IndexDef>,
    /// `Some` si l'app proprietaire partage cette table.
    pub share: Option<ShareAccess>,
    /// Augmente a chaque COMMIT : detecte deux transactions concurrentes.
    pub version: u64,
    lookup: HashMap<usize, BTreeMap<Key, Vec<u64>>>,
}

impl Table {
    pub fn new(name: &str, columns: Vec<ColumnDef>) -> Result<Table, String> {
        if columns.iter().filter(|c| c.primary).count() > 1 {
            return Err(format!("Table '{name}' : une seule colonne PRIMARY KEY"));
        }
        for (i, col) in columns.iter().enumerate() {
            if columns[..i].iter().any(|c| c.name == col.name) {
                return Err(format!("Table '{name}' : colonne '{}' en double", col.name));
            }
            if let Some(default) = &col.default {
                default.clone().coerce(col.ty).map_err(|e| format!("DEFAULT de '{}' : {e}", col.name))?;
            }
        }
        let mut table = Table {
            name: name.to_string(),
            columns,
            rows: BTreeMap::new(),
            next_id: 1,
            indexes: Vec::new(),
            share: None,
            version: 0,
            lookup: HashMap::new(),
        };
        table.rebuild()?;
        Ok(table)
    }

    /// Le rang de la colonne `name` : son nom exact, sinon la seule
    /// colonne qui porte ce nom a la casse pres (`nom` trouve `Nom`).
    pub fn column_index(&self, name: &str) -> Result<usize, String> {
        let exact = self.columns.iter().position(|c| c.name == name);
        let mut close = self.columns.iter().enumerate().filter(|(_, c)| c.name.eq_ignore_ascii_case(name)).map(|(i, _)| i);
        exact.or_else(|| close.next().filter(|_| close.next().is_none())).ok_or_else(|| format!("Colonne '{name}' inconnue dans '{}'", self.name))
    }

    fn is_unique(&self, col: usize) -> bool {
        let def = &self.columns[col];
        def.primary || def.unique || self.indexes.iter().any(|i| i.unique && i.column == def.name)
    }

    fn is_indexed(&self, col: usize) -> bool {
        let def = &self.columns[col];
        def.primary || def.unique || self.indexes.iter().any(|i| i.column == def.name)
    }

    /// Reconstruit les index et verifie les contraintes (NOT NULL, UNIQUE).
    /// Appele apres chaque modification : une erreur annule l'instruction.
    pub fn rebuild(&mut self) -> Result<(), String> {
        self.lookup.clear();
        for col in 0..self.columns.len() {
            let def = &self.columns[col];
            let not_null = def.not_null || def.primary;
            if !not_null && !self.is_indexed(col) {
                continue;
            }
            let mut map: BTreeMap<Key, Vec<u64>> = BTreeMap::new();
            for (id, row) in &self.rows {
                let value = &row[col];
                if value.is_null() {
                    if not_null {
                        return Err(format!("'{}.{}' ne peut pas etre NULL", self.name, def.name));
                    }
                    continue;
                }
                map.entry(Key(value.clone())).or_default().push(*id);
            }
            if self.is_unique(col)
                && let Some((key, _)) = map.iter().find(|(_, ids)| ids.len() > 1) {
                    return Err(format!("'{}.{}' doit etre unique : {} est en double", self.name, def.name, key.0));
                }
            if self.is_indexed(col) {
                self.lookup.insert(col, map);
            }
        }
        Ok(())
    }

    /// Les lignes ou `column == value`, via l'index ; `None` si la colonne
    /// n'est pas indexee.
    pub fn find(&self, col: usize, value: &Value) -> Option<Vec<u64>> {
        let map = self.lookup.get(&col)?;
        Some(map.get(&Key(value.clone())).cloned().unwrap_or_default())
    }

    /// Complete une ligne (DEFAULT, cle primaire INT automatique) et la
    /// convertit aux types des colonnes.
    pub fn prepare_row(&self, values: Vec<Option<Value>>) -> Result<Vec<Value>, String> {
        self.prepare_row_after(values, &mut None)
    }

    /// Comme `prepare_row`, pour les lignes d'une meme instruction, chacune
    /// inseree avant de preparer la suivante : `max` garde la plus grande
    /// cle primaire INT (lue une seule fois dans la table, puis tenue a
    /// jour), au lieu de relire toute la table a chaque ligne - sans quoi
    /// inserer n lignes coute n * n.
    pub fn prepare_row_after(&self, mut values: Vec<Option<Value>>, max: &mut Option<i64>) -> Result<Vec<Value>, String> {
        let mut row = Vec::with_capacity(self.columns.len());
        for (i, col) in self.columns.iter().enumerate() {
            let value = match values[i].take() {
                Some(v) => v,
                None => col.default.clone().unwrap_or(Value::Null),
            };
            let auto = col.primary && col.ty == DataType::Int;
            let value = if value.is_null() && auto {
                // Cle primaire INT absente : la suivante apres la plus grande.
                let plus_grande = *max.get_or_insert_with(|| self.rows.values().filter_map(|r| if let Value::Int(n) = r[i] { Some(n) } else { None }).max().unwrap_or(0));
                Value::Int(plus_grande + 1)
            } else {
                value
            };
            let value = value.coerce(col.ty).map_err(|e| format!("'{}.{}' : {e}", self.name, col.name))?;
            if auto
                && let (Some(plus_grande), Value::Int(n)) = (max.as_mut(), &value)
            {
                *plus_grande = (*plus_grande).max(*n);
            }
            row.push(value);
        }
        Ok(row)
    }

    pub fn insert(&mut self, row: Vec<Value>) {
        self.rows.insert(self.next_id, row);
        self.next_id += 1;
    }

    pub fn write(&self, w: Writer) -> Writer {
        let mut w = w.str(&self.name).u32(self.columns.len() as u32);
        for c in &self.columns {
            let flags = c.primary as u8 | (c.unique as u8) << 1 | (c.not_null as u8) << 2;
            w = w.str(&c.name).u8(c.ty.code()).u8(flags);
            w = match &c.default {
                Some(v) => v.write(w.u8(1)),
                None => w.u8(0),
            };
        }
        w = w.u64(self.next_id).u32(self.indexes.len() as u32);
        for i in &self.indexes {
            w = w.str(&i.name).str(&i.column).u8(i.unique as u8);
        }
        w = w.u32(self.share.map_or(0, |a| a.code() + 1)).u64(self.version).u64(self.rows.len() as u64);
        for (id, row) in &self.rows {
            w = w.u64(*id);
            for v in row {
                w = v.write(w);
            }
        }
        w
    }

    pub fn read(r: &mut Reader) -> Result<Table, String> {
        let name = r.str()?;
        let mut columns = Vec::new();
        for _ in 0..r.u32()? {
            let col_name = r.str()?;
            let ty = DataType::from_code(r.u8()?).ok_or("Type de colonne inconnu")?;
            let flags = r.u8()?;
            let default = if r.u8()? == 1 { Some(Value::read(r)?) } else { None };
            columns.push(ColumnDef { name: col_name, ty, primary: flags & 1 != 0, unique: flags & 2 != 0, not_null: flags & 4 != 0, default });
        }
        let next_id = r.u64()?;
        let mut indexes = Vec::new();
        for _ in 0..r.u32()? {
            indexes.push(IndexDef { name: r.str()?, column: r.str()?, unique: r.u8()? != 0 });
        }
        let share = match r.u32()? {
            0 => None,
            code => Some(ShareAccess::from_code(code - 1).ok_or("Acces inconnu")?),
        };
        let version = r.u64()?;
        let mut rows = BTreeMap::new();
        for _ in 0..r.u64()? {
            let id = r.u64()?;
            let row = (0..columns.len()).map(|_| Value::read(r)).collect::<Result<Vec<_>, _>>()?;
            rows.insert(id, row);
        }
        let mut table = Table { name, columns, rows, next_id, indexes, share, version, lookup: HashMap::new() };
        table.rebuild()?;
        Ok(table)
    }
}
