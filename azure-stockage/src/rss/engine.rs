// Execute les instructions RsS pour une app, dans le daemon de stockage.
//
// Supporte :
// - CREATE TABLE [IF NOT EXISTS] (INT, FLOAT, TEXT, BOOL ; PRIMARY KEY,
//   UNIQUE, NOT NULL, DEFAULT), DROP TABLE [IF EXISTS] ;
// - CREATE [UNIQUE] INDEX nom ON table (colonne), DROP INDEX nom ;
// - INSERT INTO t [(cols)] VALUES (...), (...) ; UPDATE ... SET ... WHERE ;
//   DELETE FROM ... WHERE ;
// - SELECT [DISTINCT] ... FROM t [alias] [INNER|LEFT JOIN u ON ...]
//   WHERE ... GROUP BY ... HAVING ... ORDER BY ... [ASC|DESC] LIMIT n
//   OFFSET m ; COUNT/SUM/AVG/MIN/MAX (COUNT(*), COUNT(DISTINCT x)) ;
//   LOWER, UPPER, LENGTH, ABS, ROUND, COALESCE ; LIKE, IN, BETWEEN,
//   IS [NOT] NULL, || ;
// - BEGIN / COMMIT / ROLLBACK : tout ou rien, meme en cas de crash (voir
//   `store`). Sans BEGIN, chaque instruction est sa propre transaction ;
//   une instruction qui echoue ne laisse jamais de changement partiel ;
// - SHARE TABLE t PUBLIC | PROTECTED, UNSHARE TABLE t : les autres apps
//   ecrivent `@<app>.t` ; `Protected` demande un compte de l'app
//   proprietaire (Reader : SELECT ; Writer : aussi INSERT/UPDATE/DELETE) ;
// - SHOW TABLES, DESCRIBE t ;
// - parametres `?`, remplaces par des valeurs passees a part (jamais
//   colles dans le texte : pas d'injection).
use crate::managers::stockage::AzureStockage;
use crate::rss::ast::*;
use crate::rss::parser::parse;
use crate::rss::table::{IndexDef, Table};
use crate::rss::value::{Key, Value};
use azure_core::models::storage_model::{Role, ShareAccess};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Resultat d'une instruction : les lignes d'un SELECT (ou de SHOW /
/// DESCRIBE), ou le nombre de lignes touchees par une ecriture.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RssResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub affected: u64,
}

/// Compte a utiliser pour les tables protegees de l'app `owner`.
#[derive(Debug, Clone)]
pub struct Login {
    pub owner: u32,
    pub user: String,
    pub password: String,
}

type TableKey = (u32, String);

#[derive(Clone, Default)]
struct Tx {
    // Tables modifiees par la transaction (`None` = supprimee).
    tables: HashMap<TableKey, Option<Table>>,
    // Version de chaque table au moment ou la transaction l'a touchee
    // (`None` = n'existait pas) : verifiee au COMMIT.
    base: HashMap<TableKey, Option<u64>>,
}

/// L'etat RsS d'UNE connexion : la transaction ouverte par BEGIN, s'il y en a une.
#[derive(Default)]
pub struct Session {
    tx: Option<Tx>,
}

impl Session {
    pub fn new() -> Session {
        Session::default()
    }

    pub fn in_transaction(&self) -> bool {
        self.tx.is_some()
    }
}

/// Execute le texte `sql` (une ou plusieurs instructions separees par `;`)
/// au nom de l'app `app`. Rend un resultat par instruction ; s'arrete a la
/// premiere erreur.
pub fn execute(stockage: &mut AzureStockage, session: &mut Session, app: u32, sql: &str, params: &[Value], logins: &[Login]) -> Result<Vec<RssResult>, String> {
    let statements = parse(sql)?;
    let mut results = Vec::new();
    for statement in &statements {
        let result = match statement {
            Statement::Begin => {
                if session.tx.is_some() {
                    return Err("Une transaction est deja ouverte (BEGIN en double)".to_string());
                }
                session.tx = Some(Tx::default());
                RssResult::default()
            }
            Statement::Commit => {
                let tx = session.tx.take().ok_or("COMMIT sans BEGIN")?;
                commit(stockage, tx)?;
                RssResult::default()
            }
            Statement::Rollback => {
                session.tx.take().ok_or("ROLLBACK sans BEGIN")?;
                RssResult::default()
            }
            other => match &mut session.tx {
                Some(tx) => {
                    let backup = tx.clone();
                    let result = Exec { stockage, tx, app, params, logins }.run(other);
                    if result.is_err() {
                        *tx = backup;
                    }
                    result?
                }
                None => {
                    let mut tx = Tx::default();
                    let result = Exec { stockage, tx: &mut tx, app, params, logins }.run(other)?;
                    commit(stockage, tx)?;
                    result
                }
            },
        };
        results.push(result);
    }
    Ok(results)
}

fn commit(stockage: &mut AzureStockage, tx: Tx) -> Result<(), String> {
    let mut changes = Vec::new();
    for ((owner, name), table) in tx.tables {
        let base = tx.base.get(&(owner, name.clone())).copied().flatten();
        let current = stockage.rss_table(owner, &name)?.map(|t| t.version);
        if current != base {
            return Err(format!("Conflit : la table '{name}' a ete modifiee par une autre connexion pendant la transaction, recommencer"));
        }
        changes.push((owner, name, table.map(|mut t| {
            t.version = base.unwrap_or(0) + 1;
            t
        })));
    }
    stockage.rss_commit(changes)
}

struct Exec<'a> {
    stockage: &'a mut AzureStockage,
    tx: &'a mut Tx,
    app: u32,
    params: &'a [Value],
    logins: &'a [Login],
}

// Lignes en cours de SELECT : chaque colonne connait sa table (alias).
struct RowSet {
    cols: Vec<(String, String)>,
    rows: Vec<Vec<Value>>,
}

fn share_name(share: Option<ShareAccess>) -> &'static str {
    match share {
        None => "privee",
        Some(ShareAccess::Public) => "publique",
        Some(ShareAccess::Protected) => "protegee",
    }
}

impl Exec<'_> {
    fn run(&mut self, statement: &Statement) -> Result<RssResult, String> {
        match statement {
            Statement::Select(select) => self.select(select),
            Statement::Insert { table, columns, rows } => self.insert(table, columns.as_deref(), rows),
            Statement::Update { table, sets, filter } => self.update(table, sets, filter.as_ref()),
            Statement::Delete { table, filter } => self.delete(table, filter.as_ref()),
            Statement::CreateTable { name, columns, if_not_exists } => self.create_table(name, columns, *if_not_exists),
            Statement::DropTable { name, if_exists } => self.drop_table(name, *if_exists),
            Statement::CreateIndex { name, table, column, unique } => self.create_index(name, table, column, *unique),
            Statement::DropIndex { name } => self.drop_index(name),
            Statement::ShareTable { name, access } => self.set_share(name, Some(*access)),
            Statement::UnshareTable { name } => self.set_share(name, None),
            Statement::ShowTables => self.show_tables(),
            Statement::Describe { table } => self.describe(table),
            Statement::Begin | Statement::Commit | Statement::Rollback => unreachable!("gere par execute"),
        }
    }

    // ---- Acces aux tables ----

    fn table(&mut self, owner: u32, name: &str) -> Result<Option<&Table>, String> {
        let key = (owner, name.to_string());
        if self.tx.tables.contains_key(&key) {
            return Ok(self.tx.tables.get(&key).and_then(Option::as_ref));
        }
        self.stockage.rss_table(owner, name)
    }

    fn remember_base(&mut self, key: &TableKey) -> Result<(), String> {
        if !self.tx.base.contains_key(key) {
            let version = self.stockage.rss_table(key.0, &key.1)?.map(|t| t.version);
            self.tx.base.insert(key.clone(), version);
        }
        Ok(())
    }

    // Copie de travail modifiable (copiee a la premiere ecriture).
    fn table_mut(&mut self, owner: u32, name: &str) -> Result<&mut Table, String> {
        let key = (owner, name.to_string());
        self.remember_base(&key)?;
        if !self.tx.tables.contains_key(&key) {
            let table = self.stockage.rss_table(owner, name)?.cloned();
            self.tx.tables.insert(key.clone(), table);
        }
        self.tx.tables.get_mut(&key).and_then(Option::as_mut).ok_or_else(|| format!("Table '{name}' introuvable"))
    }

    /// Verifie que l'app a le droit de lire (ou d'ecrire) `tref`, et rend
    /// l'app proprietaire.
    fn access(&mut self, tref: &TableRef, write: bool) -> Result<u32, String> {
        let owner = tref.owner.unwrap_or(self.app);
        let app = self.app;
        let missing = || match tref.owner {
            Some(o) if o != app => format!("Table @{o}.{} introuvable ou non partagee", tref.name),
            _ => format!("Table '{}' introuvable", tref.name),
        };
        let share = match self.table(owner, &tref.name)? {
            Some(table) => table.share,
            None => return Err(missing()),
        };
        if owner == self.app {
            return Ok(owner);
        }
        let share = share.ok_or_else(missing)?;
        let role = match self.logins.iter().find(|l| l.owner == owner) {
            Some(login) => self.stockage.check_account(owner, &login.user, &login.password)?.ok_or_else(|| format!("Identifiants refuses pour l'app {owner}"))?,
            None if share == ShareAccess::Public => Role::Reader,
            None => return Err(format!("La table @{owner}.{} est protegee : il faut un compte de l'app {owner}", tref.name)),
        };
        if write && !role.can_write() {
            return Err(format!("Ecriture refusee sur @{owner}.{} (compte en lecture seule)", tref.name));
        }
        Ok(owner)
    }

    // Tables visibles de cette app (catalogue + transaction en cours).
    fn table_names(&mut self) -> Result<BTreeSet<String>, String> {
        let mut names = self.stockage.rss_catalog(self.app)?;
        for ((owner, name), table) in &self.tx.tables {
            if *owner == self.app {
                if table.is_some() { names.insert(name.clone()) } else { names.remove(name) };
            }
        }
        Ok(names)
    }

    // ---- Definition ----

    fn create_table(&mut self, name: &str, columns: &[ColumnDef], if_not_exists: bool) -> Result<RssResult, String> {
        if self.table(self.app, name)?.is_some() {
            return if if_not_exists { Ok(RssResult::default()) } else { Err(format!("La table '{name}' existe deja")) };
        }
        let columns = columns.iter().cloned().map(|mut c| {
            c.not_null |= c.primary;
            c
        });
        let table = Table::new(name, columns.collect())?;
        let key = (self.app, name.to_string());
        self.remember_base(&key)?;
        self.tx.tables.insert(key, Some(table));
        Ok(RssResult::default())
    }

    fn drop_table(&mut self, name: &str, if_exists: bool) -> Result<RssResult, String> {
        if self.table(self.app, name)?.is_none() {
            return if if_exists { Ok(RssResult::default()) } else { Err(format!("Table '{name}' introuvable")) };
        }
        let key = (self.app, name.to_string());
        self.remember_base(&key)?;
        self.tx.tables.insert(key, None);
        Ok(RssResult::default())
    }

    // La table de cette app qui porte l'index `index`, s'il existe.
    fn table_with_index(&mut self, index: &str) -> Result<Option<String>, String> {
        for name in self.table_names()? {
            if self.table(self.app, &name)?.is_some_and(|t| t.indexes.iter().any(|i| i.name == index)) {
                return Ok(Some(name));
            }
        }
        Ok(None)
    }

    fn create_index(&mut self, name: &str, table: &str, column: &str, unique: bool) -> Result<RssResult, String> {
        if self.table_with_index(name)?.is_some() {
            return Err(format!("L'index '{name}' existe deja"));
        }
        let t = self.table_mut(self.app, table)?;
        t.column_index(column)?;
        t.indexes.push(IndexDef { name: name.to_string(), column: column.to_string(), unique });
        t.rebuild()?;
        Ok(RssResult::default())
    }

    fn drop_index(&mut self, name: &str) -> Result<RssResult, String> {
        let table = self.table_with_index(name)?.ok_or_else(|| format!("Index '{name}' introuvable"))?;
        let t = self.table_mut(self.app, &table)?;
        t.indexes.retain(|i| i.name != name);
        t.rebuild()?;
        Ok(RssResult::default())
    }

    fn set_share(&mut self, name: &str, share: Option<ShareAccess>) -> Result<RssResult, String> {
        self.table_mut(self.app, name)?.share = share;
        Ok(RssResult::default())
    }

    fn show_tables(&mut self) -> Result<RssResult, String> {
        let mut rows = Vec::new();
        for name in self.table_names()? {
            let share = self.table(self.app, &name)?.and_then(|t| t.share);
            rows.push(vec![Value::Text(name), Value::Text(share_name(share).to_string())]);
        }
        Ok(RssResult { columns: vec!["table".into(), "partage".into()], rows, affected: 0 })
    }

    fn describe(&mut self, tref: &TableRef) -> Result<RssResult, String> {
        let owner = self.access(tref, false)?;
        let table = self.table(owner, &tref.name)?.ok_or("Table introuvable")?;
        let rows = table
            .columns
            .iter()
            .map(|c| {
                let mut constraints = Vec::new();
                if c.primary { constraints.push("PRIMARY KEY") }
                if c.unique { constraints.push("UNIQUE") }
                if c.not_null && !c.primary { constraints.push("NOT NULL") }
                vec![
                    Value::Text(c.name.clone()),
                    Value::Text(c.ty.name().to_string()),
                    Value::Text(constraints.join(" ")),
                    c.default.clone().unwrap_or(Value::Null),
                ]
            })
            .collect();
        Ok(RssResult { columns: vec!["colonne".into(), "type".into(), "contraintes".into(), "defaut".into()], rows, affected: 0 })
    }

    // ---- Ecriture ----

    fn insert(&mut self, tref: &TableRef, columns: Option<&[String]>, rows: &[Vec<Expr>]) -> Result<RssResult, String> {
        let owner = self.access(tref, true)?;
        let table = self.table(owner, &tref.name)?.ok_or("Table introuvable")?;
        let width = table.columns.len();
        let targets: Vec<usize> = match columns {
            Some(names) => names.iter().map(|n| table.column_index(n)).collect::<Result<_, _>>()?,
            None => (0..width).collect(),
        };
        let empty = RowRef { cols: &[], values: &[] };
        let mut prepared = Vec::new();
        for exprs in rows {
            if exprs.len() != targets.len() {
                return Err(format!("{} valeurs pour {} colonnes", exprs.len(), targets.len()));
            }
            let mut values = vec![None; width];
            for (expr, &col) in exprs.iter().zip(&targets) {
                values[col] = Some(eval(expr, &empty, None, self.params)?);
            }
            prepared.push(values);
        }
        let table = self.table_mut(owner, &tref.name)?;
        for values in prepared {
            let row = table.prepare_row(values)?;
            table.insert(row);
        }
        table.rebuild()?;
        Ok(RssResult { affected: rows.len() as u64, ..Default::default() })
    }

    fn update(&mut self, tref: &TableRef, sets: &[(String, Expr)], filter: Option<&Expr>) -> Result<RssResult, String> {
        let owner = self.access(tref, true)?;
        let params = self.params;
        let table = self.table_mut(owner, &tref.name)?;
        let targets: Vec<(usize, &Expr)> = sets.iter().map(|(c, e)| Ok((table.column_index(c)?, e))).collect::<Result<_, String>>()?;
        let cols = table_cols(table, &tref.name);
        let mut changes = Vec::new();
        for id in candidates(table, &tref.name, filter, params)? {
            let row = &table.rows[&id];
            let r = RowRef { cols: &cols, values: row };
            if !matches_filter(filter, &r, params)? {
                continue;
            }
            let mut new_row = row.clone();
            for (col, expr) in &targets {
                let def = &table.columns[*col];
                new_row[*col] = eval(expr, &r, None, params)?.coerce(def.ty).map_err(|e| format!("'{}.{}' : {e}", table.name, def.name))?;
            }
            changes.push((id, new_row));
        }
        let affected = changes.len() as u64;
        for (id, row) in changes {
            table.rows.insert(id, row);
        }
        table.rebuild()?;
        Ok(RssResult { affected, ..Default::default() })
    }

    fn delete(&mut self, tref: &TableRef, filter: Option<&Expr>) -> Result<RssResult, String> {
        let owner = self.access(tref, true)?;
        let params = self.params;
        let table = self.table_mut(owner, &tref.name)?;
        let cols = table_cols(table, &tref.name);
        let mut doomed = Vec::new();
        for id in candidates(table, &tref.name, filter, params)? {
            if matches_filter(filter, &RowRef { cols: &cols, values: &table.rows[&id] }, params)? {
                doomed.push(id);
            }
        }
        for id in &doomed {
            table.rows.remove(id);
        }
        table.rebuild()?;
        Ok(RssResult { affected: doomed.len() as u64, ..Default::default() })
    }

    // ---- Lecture ----

    fn source(&mut self, tref: &TableRef, alias: Option<&str>, hint: Option<&Expr>) -> Result<RowSet, String> {
        let owner = self.access(tref, false)?;
        let params = self.params;
        let alias = alias.unwrap_or(&tref.name).to_string();
        let table = self.table(owner, &tref.name)?.ok_or("Table introuvable")?;
        let ids = candidates(table, &alias, hint, params)?;
        Ok(RowSet { cols: table_cols(table, &alias), rows: ids.iter().map(|id| table.rows[id].clone()).collect() })
    }

    fn select(&mut self, s: &Select) -> Result<RssResult, String> {
        let params = self.params;
        let mut set = match &s.from {
            None => RowSet { cols: Vec::new(), rows: vec![Vec::new()] },
            Some(from) => {
                // L'index n'est utilise que sans jointure (le WHERE peut
                // viser une colonne d'une autre table).
                let hint = if from.joins.is_empty() { s.filter.as_ref() } else { None };
                let mut set = self.source(&from.table, from.alias.as_deref(), hint)?;
                for join in &from.joins {
                    let right = self.source(&join.table, join.alias.as_deref(), None)?;
                    set = join_sets(set, right, join, params)?;
                }
                set
            }
        };
        if let Some(filter) = &s.filter {
            let mut kept = Vec::new();
            for row in set.rows {
                if matches_filter(Some(filter), &RowRef { cols: &set.cols, values: &row }, params)? {
                    kept.push(row);
                }
            }
            set.rows = kept;
        }

        // Colonnes du resultat.
        let mut items: Vec<(Expr, String)> = Vec::new();
        for item in &s.items {
            match item {
                SelectItem::All => {
                    for (t, c) in &set.cols {
                        items.push((Expr::Column { table: Some(t.clone()), name: c.clone() }, c.clone()));
                    }
                }
                SelectItem::TableAll(t) => {
                    let before = items.len();
                    for (tt, c) in &set.cols {
                        if tt == t {
                            items.push((Expr::Column { table: Some(t.clone()), name: c.clone() }, c.clone()));
                        }
                    }
                    if items.len() == before {
                        return Err(format!("Table '{t}' absente du FROM"));
                    }
                }
                SelectItem::Expr(e, alias) => items.push((e.clone(), alias.clone().unwrap_or_else(|| e.label()))),
            }
        }
        let labels: Vec<String> = items.iter().map(|(_, l)| l.clone()).collect();

        let grouped = !s.group_by.is_empty()
            || s.having.is_some()
            || items.iter().any(|(e, _)| e.is_aggregate())
            || s.order_by.iter().any(|(e, _)| e.is_aggregate());

        // (ligne du resultat, cles de tri)
        let mut out: Vec<(Vec<Value>, Vec<Value>)> = Vec::new();
        let sort_keys = |row: &RowRef, group: Option<&[Vec<Value>]>, values: &[Value]| -> Result<Vec<Value>, String> {
            s.order_by
                .iter()
                .map(|(e, _)| match e {
                    Expr::Column { table: None, name } if labels.contains(name) => Ok(values[labels.iter().position(|l| l == name).unwrap()].clone()),
                    Expr::Literal(Value::Int(n)) => values.get((*n as usize).wrapping_sub(1)).cloned().ok_or_else(|| format!("ORDER BY {n} : pas de colonne {n}")),
                    e => eval(e, row, group, params),
                })
                .collect()
        };
        if grouped {
            let mut groups: BTreeMap<Vec<Key>, Vec<Vec<Value>>> = BTreeMap::new();
            if s.group_by.is_empty() {
                groups.insert(Vec::new(), std::mem::take(&mut set.rows));
            } else {
                for row in std::mem::take(&mut set.rows) {
                    let r = RowRef { cols: &set.cols, values: &row };
                    let key = s.group_by.iter().map(|e| eval(e, &r, None, params).map(Key)).collect::<Result<Vec<_>, _>>()?;
                    groups.entry(key).or_default().push(row);
                }
            }
            let nulls = vec![Value::Null; set.cols.len()];
            for rows in groups.values() {
                let first = rows.first().unwrap_or(&nulls);
                let r = RowRef { cols: &set.cols, values: first };
                if let Some(having) = &s.having
                    && eval(having, &r, Some(rows), params)?.truth()? != Some(true) {
                        continue;
                    }
                let values = items.iter().map(|(e, _)| eval(e, &r, Some(rows), params)).collect::<Result<Vec<_>, _>>()?;
                let keys = sort_keys(&r, Some(rows), &values)?;
                out.push((values, keys));
            }
        } else {
            for row in &set.rows {
                let r = RowRef { cols: &set.cols, values: row };
                let values = items.iter().map(|(e, _)| eval(e, &r, None, params)).collect::<Result<Vec<_>, _>>()?;
                let keys = sort_keys(&r, None, &values)?;
                out.push((values, keys));
            }
        }

        if !s.order_by.is_empty() {
            out.sort_by(|(_, a), (_, b)| {
                for ((x, y), (_, desc)) in a.iter().zip(b).zip(&s.order_by) {
                    let ord = x.total_cmp(y);
                    if ord != Ordering::Equal {
                        return if *desc { ord.reverse() } else { ord };
                    }
                }
                Ordering::Equal
            });
        }
        let mut rows: Vec<Vec<Value>> = out.into_iter().map(|(v, _)| v).collect();
        if s.distinct {
            let mut seen = BTreeSet::new();
            rows.retain(|row| seen.insert(row.iter().cloned().map(Key).collect::<Vec<_>>()));
        }
        let count = |e: &Option<Expr>, what: &str| -> Result<Option<usize>, String> {
            match e {
                None => Ok(None),
                Some(e) => match eval(e, &RowRef { cols: &[], values: &[] }, None, params)? {
                    Value::Int(n) if n >= 0 => Ok(Some(n as usize)),
                    other => Err(format!("{what} attend un entier positif, pas {other}")),
                },
            }
        };
        let offset = count(&s.offset, "OFFSET")?.unwrap_or(0);
        let limit = count(&s.limit, "LIMIT")?;
        let rows = rows.into_iter().skip(offset).take(limit.unwrap_or(usize::MAX)).collect();
        Ok(RssResult { columns: labels, rows, affected: 0 })
    }
}

fn table_cols(table: &Table, alias: &str) -> Vec<(String, String)> {
    table.columns.iter().map(|c| (alias.to_string(), c.name.clone())).collect()
}

// Les lignes a examiner : via un index si le filtre contient
// `colonne_indexee = constante` (au premier niveau des AND), sinon toutes.
fn candidates(table: &Table, alias: &str, filter: Option<&Expr>, params: &[Value]) -> Result<Vec<u64>, String> {
    fn conjuncts<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
        match e {
            Expr::Binary(BinaryOp::And, a, b) => {
                conjuncts(a, out);
                conjuncts(b, out);
            }
            other => out.push(other),
        }
    }
    let mut parts = Vec::new();
    if let Some(filter) = filter {
        conjuncts(filter, &mut parts);
    }
    let empty = RowRef { cols: &[], values: &[] };
    for part in parts {
        if let Expr::Binary(BinaryOp::Eq, a, b) = part {
            for (col, value) in [(a, b), (b, a)] {
                let Expr::Column { table: t, name } = &**col else { continue };
                if t.as_deref().is_some_and(|t| t != alias) || !matches!(**value, Expr::Literal(_) | Expr::Param(_)) {
                    continue;
                }
                let Ok(index) = table.column_index(name) else { continue };
                let value = eval(value, &empty, None, params)?;
                // Meme conversion qu'a l'ecriture : `id = 2.0` trouve 2.
                let Ok(value) = value.coerce(table.columns[index].ty) else { continue };
                if let Some(ids) = table.find(index, &value) {
                    return Ok(ids);
                }
            }
        }
    }
    Ok(table.rows.keys().copied().collect())
}

fn join_sets(left: RowSet, right: RowSet, join: &Join, params: &[Value]) -> Result<RowSet, String> {
    let cols: Vec<(String, String)> = left.cols.iter().chain(&right.cols).cloned().collect();
    let mut rows = Vec::new();
    for l in &left.rows {
        let mut matched = false;
        for r in &right.rows {
            let combined: Vec<Value> = l.iter().chain(r).cloned().collect();
            if eval(&join.on, &RowRef { cols: &cols, values: &combined }, None, params)?.truth()? == Some(true) {
                matched = true;
                rows.push(combined);
            }
        }
        if !matched && join.kind == JoinKind::Left {
            rows.push(l.iter().cloned().chain(std::iter::repeat_n(Value::Null, right.cols.len())).collect());
        }
    }
    Ok(RowSet { cols, rows })
}

// ---- Evaluation des expressions ----

struct RowRef<'a> {
    cols: &'a [(String, String)],
    values: &'a [Value],
}

fn matches_filter(filter: Option<&Expr>, row: &RowRef, params: &[Value]) -> Result<bool, String> {
    match filter {
        None => Ok(true),
        Some(f) => Ok(eval(f, row, None, params)?.truth()? == Some(true)),
    }
}

fn column(row: &RowRef, table: Option<&str>, name: &str) -> Result<Value, String> {
    let mut found = row.cols.iter().enumerate().filter(|(_, (t, c))| c == name && table.is_none_or(|x| x == t));
    match (found.next(), found.next()) {
        (Some((i, _)), None) => Ok(row.values[i].clone()),
        (None, _) => Err(match table {
            Some(t) => format!("Colonne '{t}.{name}' inconnue"),
            None => format!("Colonne '{name}' inconnue"),
        }),
        (Some(_), Some(_)) => Err(format!("Colonne '{name}' ambigue : preciser la table (ex. t.{name})")),
    }
}

fn like(text: &str, pattern: &str) -> bool {
    let (t, p): (Vec<char>, Vec<char>) = (text.to_lowercase().chars().collect(), pattern.to_lowercase().chars().collect());
    // Programmation dynamique : m[j] = pattern[..i] correspond a text[..j].
    let mut m = vec![false; t.len() + 1];
    m[0] = true;
    for &pc in &p {
        let mut next = vec![false; t.len() + 1];
        if pc == '%' {
            let mut any = false;
            for j in 0..=t.len() {
                any |= m[j];
                next[j] = any;
            }
        } else {
            for j in 1..=t.len() {
                next[j] = m[j - 1] && (pc == '_' || pc == t[j - 1]);
            }
        }
        m = next;
    }
    m[t.len()]
}

fn arithmetic(op: BinaryOp, a: Value, b: Value) -> Result<Value, String> {
    use Value::*;
    let overflow = || "Depassement de capacite d'un INT".to_string();
    Ok(match (a, b) {
        (Null, _) | (_, Null) => Null,
        (Int(x), Int(y)) => match op {
            BinaryOp::Add => Int(x.checked_add(y).ok_or_else(overflow)?),
            BinaryOp::Sub => Int(x.checked_sub(y).ok_or_else(overflow)?),
            BinaryOp::Mul => Int(x.checked_mul(y).ok_or_else(overflow)?),
            BinaryOp::Div if y == 0 => return Err("Division par zero".to_string()),
            BinaryOp::Div => Int(x / y),
            BinaryOp::Mod if y == 0 => return Err("Division par zero".to_string()),
            _ => Int(x % y),
        },
        (x @ (Int(_) | Float(_)), y @ (Int(_) | Float(_))) => {
            let f = |v: Value| if let Int(n) = v { n as f64 } else if let Float(f) = v { f } else { 0.0 };
            let (x, y) = (f(x), f(y));
            match op {
                BinaryOp::Add => Float(x + y),
                BinaryOp::Sub => Float(x - y),
                BinaryOp::Mul => Float(x * y),
                BinaryOp::Div if y == 0.0 => return Err("Division par zero".to_string()),
                BinaryOp::Div => Float(x / y),
                _ => Float(x % y),
            }
        }
        (x, y) => return Err(format!("Calcul impossible entre {} et {} (|| pour coller du texte)", x.type_name(), y.type_name())),
    })
}

fn eval(expr: &Expr, row: &RowRef, group: Option<&[Vec<Value>]>, params: &[Value]) -> Result<Value, String> {
    let ev = |e: &Expr| eval(e, row, group, params);
    Ok(match expr {
        Expr::Literal(v) => v.clone(),
        Expr::Param(i) => params.get(*i).cloned().ok_or_else(|| format!("Parametre ?{} manquant ({} fourni(s))", i + 1, params.len()))?,
        Expr::Column { table, name } => column(row, table.as_deref(), name)?,
        Expr::Not(e) => match ev(e)?.truth()? {
            Some(b) => Value::Bool(!b),
            None => Value::Null,
        },
        Expr::Neg(e) => match ev(e)? {
            Value::Int(n) => Value::Int(n.checked_neg().ok_or("Depassement de capacite d'un INT")?),
            Value::Float(f) => Value::Float(-f),
            Value::Null => Value::Null,
            other => return Err(format!("'-' impossible sur {}", other.type_name())),
        },
        Expr::Binary(op, a, b) => match op {
            BinaryOp::And => match (ev(a)?.truth()?, ev(b)?.truth()?) {
                (Some(false), _) | (_, Some(false)) => Value::Bool(false),
                (Some(true), Some(true)) => Value::Bool(true),
                _ => Value::Null,
            },
            BinaryOp::Or => match (ev(a)?.truth()?, ev(b)?.truth()?) {
                (Some(true), _) | (_, Some(true)) => Value::Bool(true),
                (Some(false), Some(false)) => Value::Bool(false),
                _ => Value::Null,
            },
            BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => match ev(a)?.compare(&ev(b)?)? {
                None => Value::Null,
                Some(ord) => Value::Bool(match op {
                    BinaryOp::Eq => ord == Ordering::Equal,
                    BinaryOp::NotEq => ord != Ordering::Equal,
                    BinaryOp::Lt => ord == Ordering::Less,
                    BinaryOp::LtEq => ord != Ordering::Greater,
                    BinaryOp::Gt => ord == Ordering::Greater,
                    _ => ord != Ordering::Less,
                }),
            },
            BinaryOp::Concat => match (ev(a)?, ev(b)?) {
                (Value::Null, _) | (_, Value::Null) => Value::Null,
                (x, y) => Value::Text(format!("{x}{y}")),
            },
            op => arithmetic(*op, ev(a)?, ev(b)?)?,
        },
        Expr::IsNull { expr, negated } => Value::Bool(ev(expr)?.is_null() != *negated),
        Expr::InList { expr, list, negated } => {
            let value = ev(expr)?;
            let mut result = Some(false);
            for item in list {
                match value.compare(&ev(item)?)? {
                    Some(Ordering::Equal) => {
                        result = Some(true);
                        break;
                    }
                    None => result = None,
                    _ => {}
                }
            }
            result.map_or(Value::Null, |b| Value::Bool(b != *negated))
        }
        Expr::Between { expr, low, high, negated } => {
            let value = ev(expr)?;
            match (value.compare(&ev(low)?)?, value.compare(&ev(high)?)?) {
                (Some(l), Some(h)) => Value::Bool((l != Ordering::Less && h != Ordering::Greater) != *negated),
                _ => Value::Null,
            }
        }
        Expr::Like { expr, pattern, negated } => match (ev(expr)?, ev(pattern)?) {
            (Value::Null, _) | (_, Value::Null) => Value::Null,
            (Value::Text(t), Value::Text(p)) => Value::Bool(like(&t, &p) != *negated),
            (x, _) => return Err(format!("LIKE s'applique a du TEXT, pas {}", x.type_name())),
        },
        Expr::Function { name, args, star, distinct } => function(name, args, *star, *distinct, row, group, params)?,
    })
}

fn function(name: &str, args: &[Expr], star: bool, distinct: bool, row: &RowRef, group: Option<&[Vec<Value>]>, params: &[Value]) -> Result<Value, String> {
    if AGGREGATES.contains(&name) {
        let rows = group.ok_or_else(|| format!("{}() n'est permis que dans SELECT, HAVING ou ORDER BY", name.to_uppercase()))?;
        if star {
            return if name == "count" { Ok(Value::Int(rows.len() as i64)) } else { Err(format!("{}(*) n'existe pas", name.to_uppercase())) };
        }
        let [arg] = args else { return Err(format!("{}() prend un argument", name.to_uppercase())) };
        let mut values = Vec::new();
        for r in rows {
            let v = eval(arg, &RowRef { cols: row.cols, values: r }, None, params)?;
            if !v.is_null() {
                values.push(v);
            }
        }
        if distinct {
            let mut seen = BTreeSet::new();
            values.retain(|v| seen.insert(Key(v.clone())));
        }
        return Ok(match name {
            "count" => Value::Int(values.len() as i64),
            "min" => values.into_iter().min_by(|a, b| a.total_cmp(b)).unwrap_or(Value::Null),
            "max" => values.into_iter().max_by(|a, b| a.total_cmp(b)).unwrap_or(Value::Null),
            _ if values.is_empty() => Value::Null,
            "sum" => values.into_iter().try_fold(Value::Int(0), |acc, v| arithmetic(BinaryOp::Add, acc, v))?,
            _ => {
                let n = values.len() as f64;
                match values.into_iter().try_fold(Value::Float(0.0), |acc, v| arithmetic(BinaryOp::Add, acc, v))? {
                    Value::Float(total) => Value::Float(total / n),
                    other => return Err(format!("AVG() impossible sur {}", other.type_name())),
                }
            }
        });
    }
    let values = args.iter().map(|a| eval(a, row, group, params)).collect::<Result<Vec<_>, _>>()?;
    let one = || match values.as_slice() {
        [v] => Ok(v.clone()),
        _ => Err(format!("{}() prend un argument", name.to_uppercase())),
    };
    Ok(match name {
        "lower" | "upper" | "length" => match one()? {
            Value::Null => Value::Null,
            Value::Text(t) => match name {
                "lower" => Value::Text(t.to_lowercase()),
                "upper" => Value::Text(t.to_uppercase()),
                _ => Value::Int(t.chars().count() as i64),
            },
            other => return Err(format!("{}() attend du TEXT, pas {}", name.to_uppercase(), other.type_name())),
        },
        "abs" => match one()? {
            Value::Int(n) => Value::Int(n.checked_abs().ok_or("Depassement de capacite d'un INT")?),
            Value::Float(f) => Value::Float(f.abs()),
            Value::Null => Value::Null,
            other => return Err(format!("ABS() attend un nombre, pas {}", other.type_name())),
        },
        "round" => {
            let digits = match values.get(1) {
                None => 0,
                Some(Value::Int(d)) => *d as i32,
                Some(other) => return Err(format!("ROUND(x, n) : n doit etre un INT, pas {}", other.type_name())),
            };
            match values.first() {
                Some(Value::Int(n)) => Value::Int(*n),
                Some(Value::Float(f)) => {
                    let scale = 10f64.powi(digits);
                    Value::Float((f * scale).round() / scale)
                }
                Some(Value::Null) => Value::Null,
                _ => return Err("ROUND() attend un nombre".to_string()),
            }
        }
        "coalesce" => values.into_iter().find(|v| !v.is_null()).unwrap_or(Value::Null),
        other => return Err(format!("Fonction inconnue : {other}()")),
    })
}

