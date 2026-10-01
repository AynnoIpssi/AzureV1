// L'arbre d'un programme RsS, produit par `parser` et execute par `engine`.
use crate::rss::value::{DataType, Value};
use azure_core::models::storage_model::ShareAccess;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    CreateTable { name: String, columns: Vec<ColumnDef>, if_not_exists: bool },
    DropTable { name: String, if_exists: bool },
    CreateIndex { name: String, table: String, column: String, unique: bool },
    DropIndex { name: String },
    Insert { table: TableRef, columns: Option<Vec<String>>, rows: Vec<Vec<Expr>> },
    Select(Box<Select>),
    Update { table: TableRef, sets: Vec<(String, Expr)>, filter: Option<Expr> },
    Delete { table: TableRef, filter: Option<Expr> },
    Begin,
    Commit,
    Rollback,
    ShareTable { name: String, access: ShareAccess },
    UnshareTable { name: String },
    ShowTables,
    Describe { table: TableRef },
}

/// `notes` (table de l'app qui parle) ou `@10.notes` (table partagee par
/// l'app 10).
#[derive(Debug, Clone, PartialEq)]
pub struct TableRef {
    pub owner: Option<u32>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: String,
    pub ty: DataType,
    pub primary: bool,
    pub unique: bool,
    pub not_null: bool,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    pub distinct: bool,
    pub items: Vec<SelectItem>,
    pub from: Option<From>,
    pub filter: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<(Expr, bool)>,
    pub limit: Option<Expr>,
    pub offset: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectItem {
    /// `*`
    All,
    /// `n.*`
    TableAll(String),
    Expr(Expr, Option<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct From {
    pub table: TableRef,
    pub alias: Option<String>,
    pub joins: Vec<Join>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoinKind {
    Inner,
    Left,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Join {
    pub kind: JoinKind,
    pub table: TableRef,
    pub alias: Option<String>,
    pub on: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Concat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Value),
    /// `?` numero n (a partir de 0) dans tout le texte.
    Param(usize),
    Column { table: Option<String>, name: String },
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    IsNull { expr: Box<Expr>, negated: bool },
    InList { expr: Box<Expr>, list: Vec<Expr>, negated: bool },
    Between { expr: Box<Expr>, low: Box<Expr>, high: Box<Expr>, negated: bool },
    Like { expr: Box<Expr>, pattern: Box<Expr>, negated: bool },
    /// `COUNT(*)` : `star`. `COUNT(DISTINCT x)` : `distinct`.
    Function { name: String, args: Vec<Expr>, star: bool, distinct: bool },
}

pub const AGGREGATES: [&str; 5] = ["count", "sum", "avg", "min", "max"];

impl Expr {
    pub fn is_aggregate(&self) -> bool {
        match self {
            Expr::Function { name, args, .. } => AGGREGATES.contains(&name.as_str()) || args.iter().any(Expr::is_aggregate),
            Expr::Not(e) | Expr::Neg(e) => e.is_aggregate(),
            Expr::Binary(_, a, b) => a.is_aggregate() || b.is_aggregate(),
            Expr::IsNull { expr, .. } => expr.is_aggregate(),
            Expr::InList { expr, list, .. } => expr.is_aggregate() || list.iter().any(Expr::is_aggregate),
            Expr::Between { expr, low, high, .. } => expr.is_aggregate() || low.is_aggregate() || high.is_aggregate(),
            Expr::Like { expr, pattern, .. } => expr.is_aggregate() || pattern.is_aggregate(),
            Expr::Literal(_) | Expr::Param(_) | Expr::Column { .. } => false,
        }
    }

    /// Nom de colonne du resultat quand il n'y a pas d'alias.
    pub fn label(&self) -> String {
        match self {
            Expr::Column { name, .. } => name.clone(),
            Expr::Function { name, args, star, .. } => {
                let inner = if *star { "*".to_string() } else { args.iter().map(Expr::label).collect::<Vec<_>>().join(", ") };
                format!("{name}({inner})")
            }
            Expr::Literal(v) => v.to_string(),
            _ => "?column?".to_string(),
        }
    }
}
