// Transforme les jetons RsS en instructions (`ast::Statement`). Descente
// recursive ; priorite des operateurs, de la plus faible a la plus forte :
// OR, AND, NOT, comparaisons (= <> < <= > >= IS IN BETWEEN LIKE),
// + - ||, * / %, - unaire.
use crate::rss::ast::*;
use crate::rss::lexer::{tokenize, Tok, Token};
use crate::rss::value::{DataType, Value};
use azure_core::models::storage_model::ShareAccess;

pub fn parse(sql: &str) -> Result<Vec<Statement>, String> {
    let mut parser = Parser { tokens: tokenize(sql)?, pos: 0, params: 0 };
    let mut statements = Vec::new();
    loop {
        while parser.eat_sym(";") {}
        if parser.at_end() {
            break;
        }
        statements.push(parser.statement()?);
        if !parser.at_end() && !parser.eat_sym(";") {
            return Err(parser.error("';' attendu entre deux instructions"));
        }
    }
    Ok(statements)
}

// Mots qui ne peuvent pas servir d'alias sans AS (sinon `FROM notes WHERE`
// prendrait WHERE pour un alias).
const RESERVED: [&str; 27] = [
    "select", "from", "where", "group", "order", "by", "having", "limit", "offset", "join", "inner", "left", "on", "and", "or",
    "not", "as", "insert", "update", "delete", "set", "values", "into", "create", "drop", "union", "is",
];

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    params: usize,
}

impl Parser {
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos).map(|t| &t.tok)
    }

    fn error(&self, msg: &str) -> String {
        match self.tokens.get(self.pos) {
            Some(t) => format!("RsS ligne {}, colonne {} : {msg}", t.line, t.column),
            None => format!("RsS (fin du texte) : {msg}"),
        }
    }

    fn is_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Tok::Word { text, quoted: false }) if text == kw)
    }

    fn is_kw_at(&self, offset: usize, kw: &str) -> bool {
        matches!(self.tokens.get(self.pos + offset).map(|t| &t.tok), Some(Tok::Word { text, quoted: false }) if text == kw)
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        let found = self.is_kw(kw);
        if found {
            self.pos += 1;
        }
        found
    }

    fn expect_kw(&mut self, kw: &str) -> Result<(), String> {
        if self.eat_kw(kw) { Ok(()) } else { Err(self.error(&format!("{} attendu", kw.to_uppercase()))) }
    }

    fn eat_sym(&mut self, sym: &str) -> bool {
        let found = matches!(self.peek(), Some(Tok::Sym(s)) if *s == sym);
        if found {
            self.pos += 1;
        }
        found
    }

    fn expect_sym(&mut self, sym: &str) -> Result<(), String> {
        if self.eat_sym(sym) { Ok(()) } else { Err(self.error(&format!("'{sym}' attendu"))) }
    }

    fn name(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(Tok::Word { text, .. }) => {
                let text = text.clone();
                self.pos += 1;
                Ok(text)
            }
            _ => Err(self.error("nom attendu")),
        }
    }

    // Alias optionnel : `AS x`, ou un mot non reserve.
    fn alias(&mut self) -> Result<Option<String>, String> {
        if self.eat_kw("as") {
            return self.name().map(Some);
        }
        match self.peek() {
            Some(Tok::Word { text, quoted }) if *quoted || !RESERVED.contains(&text.as_str()) => self.name().map(Some),
            _ => Ok(None),
        }
    }

    fn table_ref(&mut self) -> Result<TableRef, String> {
        if self.eat_sym("@") {
            let owner = match self.peek() {
                Some(Tok::Int(n)) if *n >= 0 && *n <= u32::MAX as i64 => *n as u32,
                _ => return Err(self.error("numero d'app attendu apres '@' (ex. @10.notes)")),
            };
            self.pos += 1;
            self.expect_sym(".")?;
            return Ok(TableRef { owner: Some(owner), name: self.name()? });
        }
        Ok(TableRef { owner: None, name: self.name()? })
    }

    fn statement(&mut self) -> Result<Statement, String> {
        if self.eat_kw("select") {
            return Ok(Statement::Select(Box::new(self.select()?)));
        }
        if self.eat_kw("insert") {
            return self.insert();
        }
        if self.eat_kw("update") {
            return self.update();
        }
        if self.eat_kw("delete") {
            self.expect_kw("from")?;
            let table = self.table_ref()?;
            let filter = if self.eat_kw("where") { Some(self.expr()?) } else { None };
            return Ok(Statement::Delete { table, filter });
        }
        if self.eat_kw("create") {
            let unique = self.eat_kw("unique");
            if self.eat_kw("index") {
                let name = self.name()?;
                self.expect_kw("on")?;
                let table = self.name()?;
                self.expect_sym("(")?;
                let column = self.name()?;
                self.expect_sym(")")?;
                return Ok(Statement::CreateIndex { name, table, column, unique });
            }
            if unique {
                return Err(self.error("INDEX attendu apres CREATE UNIQUE"));
            }
            self.expect_kw("table")?;
            return self.create_table();
        }
        if self.eat_kw("drop") {
            if self.eat_kw("index") {
                return Ok(Statement::DropIndex { name: self.name()? });
            }
            self.expect_kw("table")?;
            let if_exists = self.eat_kw("if") && { self.expect_kw("exists")?; true };
            return Ok(Statement::DropTable { name: self.name()?, if_exists });
        }
        if self.eat_kw("begin") {
            self.eat_kw("transaction");
            return Ok(Statement::Begin);
        }
        if self.eat_kw("commit") {
            return Ok(Statement::Commit);
        }
        if self.eat_kw("rollback") {
            return Ok(Statement::Rollback);
        }
        if self.eat_kw("share") {
            self.expect_kw("table")?;
            let name = self.name()?;
            let access = if self.eat_kw("public") {
                ShareAccess::Public
            } else if self.eat_kw("protected") {
                ShareAccess::Protected
            } else {
                return Err(self.error("PUBLIC ou PROTECTED attendu"));
            };
            return Ok(Statement::ShareTable { name, access });
        }
        if self.eat_kw("unshare") {
            self.expect_kw("table")?;
            return Ok(Statement::UnshareTable { name: self.name()? });
        }
        if self.eat_kw("show") {
            self.expect_kw("tables")?;
            return Ok(Statement::ShowTables);
        }
        if self.eat_kw("describe") {
            return Ok(Statement::Describe { table: self.table_ref()? });
        }
        Err(self.error("instruction inconnue (SELECT, INSERT, UPDATE, DELETE, CREATE, DROP, BEGIN, COMMIT, ROLLBACK, SHARE, SHOW, DESCRIBE)"))
    }

    fn create_table(&mut self) -> Result<Statement, String> {
        let if_not_exists = self.eat_kw("if") && {
            self.expect_kw("not")?;
            self.expect_kw("exists")?;
            true
        };
        let name = self.name()?;
        self.expect_sym("(")?;
        let mut columns = Vec::new();
        loop {
            let col_name = self.name()?;
            let type_name = self.name()?;
            let ty = DataType::from_name(&type_name).ok_or_else(|| self.error(&format!("type inconnu '{type_name}' (INT, FLOAT, TEXT, BOOL, BLOB, ANY)")))?;
            // VARCHAR(255) : la taille est acceptee et ignoree.
            if self.eat_sym("(") {
                while !self.eat_sym(")") {
                    if self.at_end() {
                        return Err(self.error("')' attendu"));
                    }
                    self.pos += 1;
                }
            }
            let mut col = ColumnDef { name: col_name, ty, primary: false, unique: false, not_null: false, default: None };
            loop {
                if self.eat_kw("primary") {
                    self.expect_kw("key")?;
                    col.primary = true;
                } else if self.eat_kw("unique") {
                    col.unique = true;
                } else if self.eat_kw("not") {
                    self.expect_kw("null")?;
                    col.not_null = true;
                } else if self.eat_kw("null") {
                } else if self.eat_kw("default") {
                    col.default = Some(self.literal()?);
                } else {
                    break;
                }
            }
            columns.push(col);
            if !self.eat_sym(",") {
                break;
            }
        }
        self.expect_sym(")")?;
        Ok(Statement::CreateTable { name, columns, if_not_exists })
    }

    fn literal(&mut self) -> Result<Value, String> {
        let negative = self.eat_sym("-");
        let value = match self.peek().cloned() {
            Some(Tok::Int(n)) => Value::Int(if negative { -n } else { n }),
            Some(Tok::Float(f)) => Value::Float(if negative { -f } else { f }),
            Some(Tok::Str(s)) if !negative => Value::Text(s),
            Some(Tok::Blob(b)) if !negative => Value::Blob(b),
            Some(Tok::Word { text, quoted: false }) if !negative && matches!(text.as_str(), "true" | "false" | "null") => match text.as_str() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => Value::Null,
            },
            _ => return Err(self.error("valeur attendue (nombre, 'texte', TRUE, FALSE, NULL)")),
        };
        self.pos += 1;
        Ok(value)
    }

    fn insert(&mut self) -> Result<Statement, String> {
        self.expect_kw("into")?;
        let table = self.table_ref()?;
        let columns = if self.eat_sym("(") {
            let mut names = vec![self.name()?];
            while self.eat_sym(",") {
                names.push(self.name()?);
            }
            self.expect_sym(")")?;
            Some(names)
        } else {
            None
        };
        self.expect_kw("values")?;
        let mut rows = Vec::new();
        loop {
            self.expect_sym("(")?;
            let mut row = vec![self.expr()?];
            while self.eat_sym(",") {
                row.push(self.expr()?);
            }
            self.expect_sym(")")?;
            rows.push(row);
            if !self.eat_sym(",") {
                break;
            }
        }
        Ok(Statement::Insert { table, columns, rows })
    }

    fn update(&mut self) -> Result<Statement, String> {
        let table = self.table_ref()?;
        self.expect_kw("set")?;
        let mut sets = Vec::new();
        loop {
            let column = self.name()?;
            self.expect_sym("=")?;
            sets.push((column, self.expr()?));
            if !self.eat_sym(",") {
                break;
            }
        }
        let filter = if self.eat_kw("where") { Some(self.expr()?) } else { None };
        Ok(Statement::Update { table, sets, filter })
    }

    fn select(&mut self) -> Result<Select, String> {
        let distinct = self.eat_kw("distinct");
        let mut items = Vec::new();
        loop {
            if self.eat_sym("*") {
                items.push(SelectItem::All);
            } else if matches!(self.peek(), Some(Tok::Word { .. }))
                && matches!(self.tokens.get(self.pos + 1).map(|t| &t.tok), Some(Tok::Sym(".")))
                && matches!(self.tokens.get(self.pos + 2).map(|t| &t.tok), Some(Tok::Sym("*")))
            {
                let table = self.name()?;
                self.pos += 2;
                items.push(SelectItem::TableAll(table));
            } else {
                let expr = self.expr()?;
                items.push(SelectItem::Expr(expr, self.alias()?));
            }
            if !self.eat_sym(",") {
                break;
            }
        }
        let from = if self.eat_kw("from") {
            let table = self.table_ref()?;
            let alias = self.alias()?;
            let mut joins = Vec::new();
            loop {
                let kind = if self.eat_kw("join") {
                    JoinKind::Inner
                } else if self.is_kw("inner") && self.is_kw_at(1, "join") {
                    self.pos += 2;
                    JoinKind::Inner
                } else if self.eat_kw("left") {
                    self.eat_kw("outer");
                    self.expect_kw("join")?;
                    JoinKind::Left
                } else {
                    break;
                };
                let table = self.table_ref()?;
                let alias = self.alias()?;
                self.expect_kw("on")?;
                joins.push(Join { kind, table, alias, on: self.expr()? });
            }
            Some(From { table, alias, joins })
        } else {
            None
        };
        let filter = if self.eat_kw("where") { Some(self.expr()?) } else { None };
        let mut group_by = Vec::new();
        if self.eat_kw("group") {
            self.expect_kw("by")?;
            loop {
                group_by.push(self.expr()?);
                if !self.eat_sym(",") {
                    break;
                }
            }
        }
        let having = if self.eat_kw("having") { Some(self.expr()?) } else { None };
        let mut order_by = Vec::new();
        if self.eat_kw("order") {
            self.expect_kw("by")?;
            loop {
                let expr = self.expr()?;
                let desc = if self.eat_kw("desc") {
                    true
                } else {
                    // `ASC` (facultatif) : lu, et c'est l'ordre par defaut.
                    self.eat_kw("asc");
                    false
                };
                order_by.push((expr, desc));
                if !self.eat_sym(",") {
                    break;
                }
            }
        }
        let limit = if self.eat_kw("limit") { Some(self.expr()?) } else { None };
        let offset = if self.eat_kw("offset") { Some(self.expr()?) } else { None };
        Ok(Select { distinct, items, from, filter, group_by, having, order_by, limit, offset })
    }

    // ---- Expressions ----

    fn expr(&mut self) -> Result<Expr, String> {
        self.or()
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut left = self.and()?;
        while self.eat_kw("or") {
            left = Expr::Binary(BinaryOp::Or, Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut left = self.not()?;
        while self.eat_kw("and") {
            left = Expr::Binary(BinaryOp::And, Box::new(left), Box::new(self.not()?));
        }
        Ok(left)
    }

    fn not(&mut self) -> Result<Expr, String> {
        if self.eat_kw("not") {
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.comparison()
    }

    fn comparison(&mut self) -> Result<Expr, String> {
        let left = self.additive()?;
        let op = match self.peek() {
            Some(Tok::Sym("=")) => Some(BinaryOp::Eq),
            Some(Tok::Sym("<>")) | Some(Tok::Sym("!=")) => Some(BinaryOp::NotEq),
            Some(Tok::Sym("<")) => Some(BinaryOp::Lt),
            Some(Tok::Sym("<=")) => Some(BinaryOp::LtEq),
            Some(Tok::Sym(">")) => Some(BinaryOp::Gt),
            Some(Tok::Sym(">=")) => Some(BinaryOp::GtEq),
            _ => None,
        };
        if let Some(op) = op {
            self.pos += 1;
            return Ok(Expr::Binary(op, Box::new(left), Box::new(self.additive()?)));
        }
        if self.eat_kw("is") {
            let negated = self.eat_kw("not");
            self.expect_kw("null")?;
            return Ok(Expr::IsNull { expr: Box::new(left), negated });
        }
        let negated = self.is_kw("not") && (self.is_kw_at(1, "in") || self.is_kw_at(1, "between") || self.is_kw_at(1, "like"));
        if negated {
            self.pos += 1;
        }
        if self.eat_kw("in") {
            self.expect_sym("(")?;
            let mut list = vec![self.expr()?];
            while self.eat_sym(",") {
                list.push(self.expr()?);
            }
            self.expect_sym(")")?;
            return Ok(Expr::InList { expr: Box::new(left), list, negated });
        }
        if self.eat_kw("between") {
            let low = self.additive()?;
            self.expect_kw("and")?;
            let high = self.additive()?;
            return Ok(Expr::Between { expr: Box::new(left), low: Box::new(low), high: Box::new(high), negated });
        }
        if self.eat_kw("like") {
            return Ok(Expr::Like { expr: Box::new(left), pattern: Box::new(self.additive()?), negated });
        }
        Ok(left)
    }

    fn additive(&mut self) -> Result<Expr, String> {
        let mut left = self.multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Sym("+")) => BinaryOp::Add,
                Some(Tok::Sym("-")) => BinaryOp::Sub,
                Some(Tok::Sym("||")) => BinaryOp::Concat,
                _ => return Ok(left),
            };
            self.pos += 1;
            left = Expr::Binary(op, Box::new(left), Box::new(self.multiplicative()?));
        }
    }

    fn multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Sym("*")) => BinaryOp::Mul,
                Some(Tok::Sym("/")) => BinaryOp::Div,
                Some(Tok::Sym("%")) => BinaryOp::Mod,
                _ => return Ok(left),
            };
            self.pos += 1;
            left = Expr::Binary(op, Box::new(left), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat_sym("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat_sym("+") {
            return self.unary();
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let tok = self.peek().cloned().ok_or_else(|| self.error("expression attendue"))?;
        match tok {
            Tok::Int(n) => {
                self.pos += 1;
                Ok(Expr::Literal(Value::Int(n)))
            }
            Tok::Float(f) => {
                self.pos += 1;
                Ok(Expr::Literal(Value::Float(f)))
            }
            Tok::Str(s) => {
                self.pos += 1;
                Ok(Expr::Literal(Value::Text(s)))
            }
            Tok::Blob(b) => {
                self.pos += 1;
                Ok(Expr::Literal(Value::Blob(b)))
            }
            Tok::Param => {
                self.pos += 1;
                self.params += 1;
                Ok(Expr::Param(self.params - 1))
            }
            Tok::Sym("(") => {
                self.pos += 1;
                let expr = self.expr()?;
                self.expect_sym(")")?;
                Ok(expr)
            }
            Tok::Word { text, quoted } => {
                self.pos += 1;
                if !quoted {
                    match text.as_str() {
                        "true" => return Ok(Expr::Literal(Value::Bool(true))),
                        "false" => return Ok(Expr::Literal(Value::Bool(false))),
                        "null" => return Ok(Expr::Literal(Value::Null)),
                        _ => {}
                    }
                    if self.eat_sym("(") {
                        return self.function(text);
                    }
                }
                if self.eat_sym(".") {
                    return Ok(Expr::Column { table: Some(text), name: self.name()? });
                }
                Ok(Expr::Column { table: None, name: text })
            }
            _ => Err(self.error("expression attendue")),
        }
    }

    fn function(&mut self, name: String) -> Result<Expr, String> {
        if self.eat_sym("*") {
            self.expect_sym(")")?;
            return Ok(Expr::Function { name, args: Vec::new(), star: true, distinct: false });
        }
        let distinct = self.eat_kw("distinct");
        let mut args = Vec::new();
        if !self.eat_sym(")") {
            loop {
                args.push(self.expr()?);
                if !self.eat_sym(",") {
                    break;
                }
            }
            self.expect_sym(")")?;
        }
        Ok(Expr::Function { name, args, star: false, distinct })
    }
}
