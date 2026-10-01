use crate::compiler::services::condition::Context;
use crate::storage::models::stockage::Stockage;
use azure_stockage::rss::engine::{Login, RssResult};
use azure_stockage::rss::value::Value;
use std::sync::Arc;

/// La base RsS (le SQL d'Azure) d'une app, via le daemon de stockage :
///
/// ```text
/// let db = store.db();
/// db.run("CREATE TABLE IF NOT EXISTS notes (id INT PRIMARY KEY, titre TEXT, fait BOOL DEFAULT false)", &[])?;
/// db.run("INSERT INTO notes (titre) VALUES (?)", &params!["Courses"])?;
/// for note in db.query("SELECT id, titre FROM notes WHERE fait = ? ORDER BY id", &params![false])? {
///     let titre: String = note.get("titre")?;
/// }
/// let restantes: i64 = db.value("SELECT count(*) FROM notes WHERE fait = false", &[])?.unwrap_or(0);
/// db.transaction(|db| {
///     db.run("UPDATE notes SET fait = true WHERE id = ?", &params![1])?;
///     db.run("DELETE FROM notes WHERE fait = true", &[])
/// })?;
/// let partagees = store.db().login(10, "maman", "1234").query("SELECT * FROM @10.notes", &[])?;
/// ```
///
/// Les valeurs passent TOUJOURS par des `?` + `params![...]`, jamais collees
/// dans le texte : pas d'injection possible.
#[derive(Clone)]
pub struct Db {
    store: Stockage,
    logins: Vec<Login>,
}

impl Db {
    pub(crate) fn new(store: &Stockage) -> Db {
        Db { store: store.clone(), logins: Vec::new() }
    }

    /// Utilise ce compte de l'app `owner` pour ses tables protegees
    /// (`@owner.table`). Se chaine pour plusieurs apps.
    pub fn login(mut self, owner: u32, user: &str, password: &str) -> Db {
        self.logins.retain(|l| l.owner != owner);
        self.logins.push(Login { owner, user: user.to_string(), password: password.to_string() });
        self
    }

    fn exec(&self, sql: &str, params: &[Value]) -> Result<Vec<RssResult>, String> {
        self.store.with(|c| c.rss(sql, params, &self.logins))
    }

    fn last(&self, sql: &str, params: &[Value]) -> Result<RssResult, String> {
        self.exec(sql, params)?.pop().ok_or_else(|| "Aucune instruction RsS".to_string())
    }

    /// Une instruction qui modifie (INSERT, UPDATE, DELETE, CREATE...) :
    /// rend le nombre de lignes touchees.
    pub fn run(&self, sql: &str, params: &[Value]) -> Result<u64, String> {
        Ok(self.last(sql, params)?.affected)
    }

    /// Un SELECT (ou SHOW TABLES, DESCRIBE) : les lignes du resultat.
    pub fn query(&self, sql: &str, params: &[Value]) -> Result<Rows, String> {
        Ok(Rows::from(self.last(sql, params)?))
    }

    /// La premiere ligne d'un SELECT, `None` s'il n'y en a pas.
    pub fn first(&self, sql: &str, params: &[Value]) -> Result<Option<Row>, String> {
        Ok(self.query(sql, params)?.rows.into_iter().next())
    }

    /// La premiere colonne de la premiere ligne :
    /// `db.value::<i64>("SELECT count(*) FROM notes", &[])`.
    pub fn value<T: FromValue>(&self, sql: &str, params: &[Value]) -> Result<Option<T>, String> {
        match self.first(sql, params)? {
            Some(row) => row.values.into_iter().next().map(T::from_value).transpose(),
            None => Ok(None),
        }
    }

    /// Plusieurs instructions separees par `;` (sans parametres), par
    /// exemple le schema de l'app.
    pub fn script(&self, sql: &str) -> Result<(), String> {
        self.exec(sql, &[]).map(|_| ())
    }

    /// Execute un fichier `.rss` (voir `script`).
    pub fn run_file(&self, path: &str) -> Result<(), String> {
        let sql = std::fs::read_to_string(path).map_err(|e| format!("{path} : {e}"))?;
        self.script(&sql)
    }

    /// Tout ou rien : si `body` rend une erreur, rien de ce qu'il a fait ne
    /// reste. Les copies du meme `Stockage` partagent la connexion, donc la
    /// transaction : ne pas ecrire depuis une autre copie en meme temps.
    pub fn transaction<T>(&self, body: impl FnOnce(&Db) -> Result<T, String>) -> Result<T, String> {
        self.exec("BEGIN", &[])?;
        match body(self) {
            Ok(value) => {
                self.exec("COMMIT", &[])?;
                Ok(value)
            }
            Err(err) => {
                let _ = self.exec("ROLLBACK", &[]);
                Err(err)
            }
        }
    }
}

/// Les lignes d'un SELECT. Se parcourt avec `for row in rows`.
#[derive(Debug, Clone)]
pub struct Rows {
    pub columns: Vec<String>,
    rows: Vec<Row>,
}

impl From<RssResult> for Rows {
    fn from(result: RssResult) -> Rows {
        let columns = Arc::new(result.columns);
        let rows = result.rows.into_iter().map(|values| Row { columns: Arc::clone(&columns), values }).collect();
        Rows { columns: columns.to_vec(), rows }
    }
}

impl Rows {
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Row> {
        self.rows.iter()
    }

    /// Toute une colonne : `rows.column::<String>("titre")`.
    pub fn column<T: FromValue>(&self, name: &str) -> Result<Vec<T>, String> {
        self.rows.iter().map(|row| row.get(name)).collect()
    }
}

impl IntoIterator for Rows {
    type Item = Row;
    type IntoIter = std::vec::IntoIter<Row>;
    fn into_iter(self) -> Self::IntoIter {
        self.rows.into_iter()
    }
}

impl<'a> IntoIterator for &'a Rows {
    type Item = &'a Row;
    type IntoIter = std::slice::Iter<'a, Row>;
    fn into_iter(self) -> Self::IntoIter {
        self.rows.iter()
    }
}

/// Une ligne : `row.get::<String>("titre")`.
#[derive(Debug, Clone)]
pub struct Row {
    columns: Arc<Vec<String>>,
    values: Vec<Value>,
}

impl Row {
    pub fn value(&self, column: &str) -> Option<&Value> {
        self.columns.iter().position(|c| c == column).map(|i| &self.values[i])
    }

    pub fn get<T: FromValue>(&self, column: &str) -> Result<T, String> {
        let value = self.value(column).ok_or_else(|| format!("Pas de colonne '{column}' (colonnes : {})", self.columns.join(", ")))?;
        T::from_value(value.clone()).map_err(|e| format!("'{column}' : {e}"))
    }

    /// Comme `get`, avec `default` si la colonne manque, est NULL ou ne se
    /// convertit pas.
    pub fn get_or<T: FromValue>(&self, column: &str, default: T) -> T {
        self.get::<Option<T>>(column).ok().flatten().unwrap_or(default)
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }

    /// Un `Context` rsH avec les colonnes de cette ligne (en texte), pour
    /// les conditions : `<if.fait == "true">`.
    pub fn context(&self) -> Context {
        self.columns.iter().zip(&self.values).fold(Context::new(), |ctx, (c, v)| ctx.with_text(c, &v.to_string()))
    }
}

/// Conversion d'une valeur RsS vers un type Rust.
pub trait FromValue: Sized {
    fn from_value(value: Value) -> Result<Self, String>;
}

impl FromValue for Value {
    fn from_value(value: Value) -> Result<Value, String> {
        Ok(value)
    }
}

impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: Value) -> Result<Option<T>, String> {
        if value.is_null() { Ok(None) } else { T::from_value(value).map(Some) }
    }
}

impl FromValue for String {
    fn from_value(value: Value) -> Result<String, String> {
        match value {
            Value::Text(t) => Ok(t),
            other => Err(format!("{} n'est pas du TEXT", other.type_name())),
        }
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<bool, String> {
        match value {
            Value::Bool(b) => Ok(b),
            other => Err(format!("{} n'est pas un BOOL", other.type_name())),
        }
    }
}

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<f64, String> {
        match value {
            Value::Float(f) => Ok(f),
            Value::Int(n) => Ok(n as f64),
            other => Err(format!("{} n'est pas un nombre", other.type_name())),
        }
    }
}

impl FromValue for f32 {
    fn from_value(value: Value) -> Result<f32, String> {
        f64::from_value(value).map(|f| f as f32)
    }
}

macro_rules! from_int {
    ($($t:ty),*) => {$(
        impl FromValue for $t {
            fn from_value(value: Value) -> Result<$t, String> {
                match value {
                    Value::Int(n) => <$t>::try_from(n).map_err(|_| format!("{n} ne tient pas dans un {}", stringify!($t))),
                    other => Err(format!("{} n'est pas un INT", other.type_name())),
                }
            }
        }
    )*};
}

from_int!(i64, i32, i16, i8, u64, u32, u16, u8, usize, isize);

/// Les valeurs des `?` d'une requete : `&params![1, "texte", true, None::<i64>]`.
#[macro_export]
macro_rules! params {
    ($($value:expr),* $(,)?) => {
        [$($crate::storage::Value::from($value)),*]
    };
}
