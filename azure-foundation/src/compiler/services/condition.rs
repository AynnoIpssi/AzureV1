// Evaluateur d'expression booleenne pour les conditions .rsh (`if`/`elseif`/
// `while`, voir `interpreter::build_ui_with_context`) - UNIQUEMENT utilise
// cote interpreteur, jamais cote `codegen` (qui emet la condition brute
// comme du vrai code Rust, execute plus tard avec de vraies variables).
//
// `codegen` peut se permettre d'accepter n'importe quelle expression Rust
// (`has_next_notification()`, `count > 0 && !flag`...) parce qu'elle finit
// dans un programme compile, ou le compilateur Rust lui-meme l'evalue.
// L'interpreteur, lui, tourne AU MOMENT de la construction de l'arbre
// `UiNode`, sans compilateur disponible - il ne peut donc gerer qu'un
// sous-ensemble reduit, mais REEL, d'expressions (identifiants, litteraux,
// comparaisons, `!`/`&&`/`||`, parentheses), evaluees contre un `Context`
// explicite fourni par l'appelant. Tout ce qui sort de ce sous-ensemble
// (appel de methode, expression `for x in y`...) fait retourner `None` a
// `evaluate` plutot que de deviner - c'est a l'appelant de decider quoi
// faire d'une condition non evaluable (voir `build_if_chain`, qui la traite
// comme fausse : une branche non evaluable ne s'affiche pas, plutot que de
// toutes les afficher comme avant ce correctif).

use std::collections::{BTreeMap, HashMap};

/// Une valeur nommee disponible aux conditions .rsh evaluees par
/// l'interpreteur - fournie explicitement par l'appelant de `build_ui_with_context`,
/// jamais devinee. `List` sert a `for`/`foreach` (voir
/// `interpreter::build_for`), pas aux comparaisons. `Map` regroupe des
/// champs, lus avec un point : `app.nom` (dans une condition comme dans
/// `{{app.nom}}`, voir `interpolate`).
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionValue {
    Bool(bool),
    Number(f64),
    Text(String),
    List(Vec<ConditionValue>),
    Map(BTreeMap<String, ConditionValue>),
}

impl ConditionValue {
    /// `ConditionValue::map([("nom", ConditionValue::Text("notes".into()))])`
    pub fn map<K: Into<String>>(entries: impl IntoIterator<Item = (K, ConditionValue)>) -> ConditionValue {
        ConditionValue::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// Texte affiche par `{{...}}` : un nombre entier sans `.0`, une liste
    /// separee par des virgules, un objet vide.
    pub fn display(&self) -> String {
        match self {
            ConditionValue::Bool(b) => b.to_string(),
            ConditionValue::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", *n as i64),
            ConditionValue::Number(n) => n.to_string(),
            ConditionValue::Text(t) => t.clone(),
            ConditionValue::List(items) => items.iter().map(ConditionValue::display).collect::<Vec<_>>().join(", "),
            ConditionValue::Map(_) => String::new(),
        }
    }
}

impl From<&str> for ConditionValue {
    fn from(text: &str) -> ConditionValue {
        ConditionValue::Text(text.to_string())
    }
}

impl From<String> for ConditionValue {
    fn from(text: String) -> ConditionValue {
        ConditionValue::Text(text)
    }
}

impl From<bool> for ConditionValue {
    fn from(b: bool) -> ConditionValue {
        ConditionValue::Bool(b)
    }
}

impl From<f64> for ConditionValue {
    fn from(n: f64) -> ConditionValue {
        ConditionValue::Number(n)
    }
}

impl From<i64> for ConditionValue {
    fn from(n: i64) -> ConditionValue {
        ConditionValue::Number(n as f64)
    }
}

impl From<u32> for ConditionValue {
    fn from(n: u32) -> ConditionValue {
        ConditionValue::Number(n as f64)
    }
}

/// Les variables nommees visibles par `evaluate`/`build_ui_with_context` -
/// un `Context::new()` vide fait echouer tout identifiant (voir
/// `ConditionValue`), ce qui est le comportement par defaut de `build_ui`
/// (aucune condition ne peut alors etre vraie : les branches `if`/`elseif`
/// sans `else` ne s'affichent jamais, `else` s'affiche toujours).
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Partagees : une boucle `<for>` copie le contexte a chaque tour, ce
    /// qui ne doit copier que des pointeurs (pas toutes les listes de la
    /// page, sinon le cout grandit comme le carre du nombre d'elements).
    vars: HashMap<String, std::sync::Arc<ConditionValue>>,
    /// Les composants disponibles (voir `compiler::components`) ; sans :
    /// ceux fournis par Azure.
    pub(crate) library: Option<std::sync::Arc<crate::compiler::components::Library>>,
    /// Le contenu passe a un composant, rendu a la place de son `<slot/>`.
    pub(crate) slot: Option<std::sync::Arc<crate::compiler::components::Slot>>,
    /// Profondeur de composants imbriques (garde-fou contre la recursion).
    pub(crate) depth: u32,
}

impl Context {
    pub fn new() -> Context {
        Context::default()
    }

    /// Les composants de l'app (dossier `components/`), en plus de ceux
    /// d'Azure.
    pub fn with_library(mut self, library: std::sync::Arc<crate::compiler::components::Library>) -> Context {
        self.library = Some(library);
        self
    }

    pub fn with_bool(mut self, name: &str, value: bool) -> Context {
        self.vars.insert(name.to_string(), std::sync::Arc::new(ConditionValue::Bool(value)));
        self
    }

    pub fn with_number(mut self, name: &str, value: f64) -> Context {
        self.vars.insert(name.to_string(), std::sync::Arc::new(ConditionValue::Number(value)));
        self
    }

    pub fn with_text(mut self, name: &str, value: &str) -> Context {
        self.vars.insert(name.to_string(), std::sync::Arc::new(ConditionValue::Text(value.to_string())));
        self
    }

    pub fn with_list(mut self, name: &str, values: Vec<ConditionValue>) -> Context {
        self.vars.insert(name.to_string(), std::sync::Arc::new(ConditionValue::List(values)));
        self
    }

    /// N'importe quelle valeur (un objet, une liste d'objets...).
    pub fn with_value(mut self, name: &str, value: impl Into<ConditionValue>) -> Context {
        self.vars.insert(name.to_string(), std::sync::Arc::new(value.into()));
        self
    }

    /// Ajoute (ou remplace) les variables de `other`.
    pub fn merge(mut self, other: Context) -> Context {
        self.vars.extend(other.vars);
        self
    }

    /// `name` peut descendre dans les objets et les listes : `app.nom`,
    /// `apps.0.nom`.
    pub fn get(&self, name: &str) -> Option<&ConditionValue> {
        let mut parts = name.split('.');
        let first: &ConditionValue = self.vars.get(parts.next()?)?;
        parts.try_fold(first, |value, part| match value {
            ConditionValue::Map(map) => map.get(part),
            ConditionValue::List(items) => part.parse::<usize>().ok().and_then(|i| items.get(i)),
            _ => None,
        })
    }
}

/// Remplace chaque `{{nom}}` de `text` par la valeur de `nom` dans `ctx`
/// (voir `ConditionValue::display`) ; une variable absente donne un texte
/// vide. `{{ app.nom }}` : espaces permis. Un `{{` jamais ferme reste tel
/// quel.
pub fn interpolate(text: &str, ctx: &Context) -> String {
    if !text.contains("{{") {
        return text.to_string();
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start + 2..].find("}}") else { break };
        out.push_str(&rest[..start]);
        let name = rest[start + 2..start + 2 + len].trim();
        out.push_str(&ctx.get(name).map(ConditionValue::display).unwrap_or_default());
        rest = &rest[start + 2 + len + 2..];
    }
    out.push_str(rest);
    out
}

#[derive(Debug, Clone, PartialEq)]
enum CondToken {
    Ident(String),
    Number(f64),
    Str(String),
    True,
    False,
    Not,
    And,
    Or,
    Eq,
    Ne,
    Le,
    Ge,
    Lt,
    Gt,
    LParen,
    RParen,
    // Tout caractere hors de cette grammaire reduite (un '.', un ',',
    // un appel de methode...) - ne correspond jamais a une regle du
    // parseur, ce qui fait echouer `evaluate` proprement plutot que de mal
    // interpreter une expression trop riche pour ce sous-ensemble.
    Unknown,
}

fn tokenize_condition(expr: &str) -> Vec<CondToken> {
    let mut chars = expr.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '(' => {
                chars.next();
                tokens.push(CondToken::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(CondToken::RParen);
            }
            '!' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(CondToken::Ne);
                } else {
                    tokens.push(CondToken::Not);
                }
            }
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(CondToken::Eq);
                } else {
                    tokens.push(CondToken::Unknown);
                }
            }
            '&' => {
                chars.next();
                if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(CondToken::And);
                } else {
                    tokens.push(CondToken::Unknown);
                }
            }
            '|' => {
                chars.next();
                if chars.peek() == Some(&'|') {
                    chars.next();
                    tokens.push(CondToken::Or);
                } else {
                    tokens.push(CondToken::Unknown);
                }
            }
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(CondToken::Le);
                } else {
                    tokens.push(CondToken::Lt);
                }
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(CondToken::Ge);
                } else {
                    tokens.push(CondToken::Gt);
                }
            }
            '\'' | '"' => {
                let quote = c;
                chars.next();
                let mut s = String::new();
                for next in chars.by_ref() {
                    if next == quote {
                        break;
                    }
                    s.push(next);
                }
                tokens.push(CondToken::Str(s));
            }
            _ if c.is_ascii_digit() => {
                let mut n = String::new();
                while matches!(chars.peek(), Some(d) if d.is_ascii_digit() || *d == '.') {
                    n.push(chars.next().expect("peek just confirmed a char"));
                }
                match n.parse::<f64>() {
                    Ok(value) => tokens.push(CondToken::Number(value)),
                    Err(_) => tokens.push(CondToken::Unknown),
                }
            }
            _ if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();
                // `app.nom` : un point suivi d'une lettre, d'un chiffre ou de
                // `_` fait partie du nom (voir `Context::get`).
                loop {
                    let mut ahead = chars.clone();
                    let (next, after) = (ahead.next(), ahead.next());
                    let takes = match next {
                        Some(d) if d.is_alphanumeric() || d == '_' => true,
                        Some('.') => after.is_some_and(|d| d.is_alphanumeric() || d == '_'),
                        _ => false,
                    };
                    if !takes {
                        break;
                    }
                    ident.push(chars.next().expect("peek just confirmed a char"));
                }
                match ident.as_str() {
                    "true" => tokens.push(CondToken::True),
                    "false" => tokens.push(CondToken::False),
                    _ => tokens.push(CondToken::Ident(ident)),
                }
            }
            _ => {
                chars.next();
                tokens.push(CondToken::Unknown);
            }
        }
    }

    tokens
}

enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

fn compare(left: &ConditionValue, op: CmpOp, right: &ConditionValue) -> Option<bool> {
    use ConditionValue::*;
    match (left, right) {
        (Number(a), Number(b)) => Some(match op {
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
            CmpOp::Lt => a < b,
            CmpOp::Le => a <= b,
            CmpOp::Gt => a > b,
            CmpOp::Ge => a >= b,
        }),
        (Text(a), Text(b)) => Some(match op {
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
            CmpOp::Lt => a < b,
            CmpOp::Le => a <= b,
            CmpOp::Gt => a > b,
            CmpOp::Ge => a >= b,
        }),
        // Un texte qui est un nombre (un attribut `page="3"`) se compare
        // comme un nombre ; sinon, texte et nombre sont simplement differents.
        (Number(_), Text(t)) | (Text(t), Number(_)) => {
            let n = t.trim().replace(',', ".").parse::<f64>().ok();
            match (left, right, n) {
                (Number(a), _, Some(b)) => compare(&Number(*a), op, &Number(b)),
                (_, Number(b), Some(a)) => compare(&Number(a), op, &Number(*b)),
                _ => match op {
                    CmpOp::Eq => Some(false),
                    CmpOp::Ne => Some(true),
                    _ => None,
                },
            }
        }
        (Bool(a), Bool(b)) => match op {
            CmpOp::Eq => Some(a == b),
            CmpOp::Ne => Some(a != b),
            // "vrai < faux" n'a pas de sens defini ici - plutot que de
            // choisir un ordre arbitraire, on refuse.
            _ => None,
        },
        // Comparer des types differents (ou une `List`, jamais comparable)
        // n'a pas de sens defini - fail-closed plutot que deviner.
        _ => None,
    }
}

struct Parser<'a> {
    tokens: &'a [CondToken],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&CondToken> {
        self.tokens.get(self.pos)
    }

    fn parse_or(&mut self, ctx: &Context) -> Option<bool> {
        let mut left = self.parse_and(ctx)?;
        while matches!(self.peek(), Some(CondToken::Or)) {
            self.pos += 1;
            let right = self.parse_and(ctx)?;
            left = left || right;
        }
        Some(left)
    }

    fn parse_and(&mut self, ctx: &Context) -> Option<bool> {
        let mut left = self.parse_unary(ctx)?;
        while matches!(self.peek(), Some(CondToken::And)) {
            self.pos += 1;
            let right = self.parse_unary(ctx)?;
            left = left && right;
        }
        Some(left)
    }

    fn parse_unary(&mut self, ctx: &Context) -> Option<bool> {
        if matches!(self.peek(), Some(CondToken::Not)) {
            self.pos += 1;
            return self.parse_unary(ctx).map(|b| !b);
        }
        self.parse_comparison(ctx)
    }

    fn parse_comparison(&mut self, ctx: &Context) -> Option<bool> {
        let left = self.parse_atom(ctx)?;

        let cmp = match self.peek() {
            Some(CondToken::Eq) => Some(CmpOp::Eq),
            Some(CondToken::Ne) => Some(CmpOp::Ne),
            Some(CondToken::Le) => Some(CmpOp::Le),
            Some(CondToken::Ge) => Some(CmpOp::Ge),
            Some(CondToken::Lt) => Some(CmpOp::Lt),
            Some(CondToken::Gt) => Some(CmpOp::Gt),
            _ => None,
        };

        match cmp {
            Some(op) => {
                self.pos += 1;
                let right = self.parse_atom(ctx)?;
                compare(&left, op, &right)
            }
            // Pas d'operateur de comparaison : un "drapeau" nu, valide
            // uniquement si c'est deja un booleen - `<if.count>` (un nombre
            // seul, sans comparaison) est intentionnellement refuse plutot
            // que de deviner ce que "vrai" voudrait dire pour un nombre.
            None => match left {
                ConditionValue::Bool(b) => Some(b),
                _ => None,
            },
        }
    }

    fn parse_atom(&mut self, ctx: &Context) -> Option<ConditionValue> {
        match self.peek()?.clone() {
            CondToken::True => {
                self.pos += 1;
                Some(ConditionValue::Bool(true))
            }
            CondToken::False => {
                self.pos += 1;
                Some(ConditionValue::Bool(false))
            }
            CondToken::Number(n) => {
                self.pos += 1;
                Some(ConditionValue::Number(n))
            }
            CondToken::Str(s) => {
                self.pos += 1;
                Some(ConditionValue::Text(s))
            }
            CondToken::Ident(name) => {
                self.pos += 1;
                ctx.get(&name).cloned()
            }
            CondToken::LParen => {
                self.pos += 1;
                let inner = self.parse_or(ctx)?;
                match self.peek() {
                    Some(CondToken::RParen) => {
                        self.pos += 1;
                        Some(ConditionValue::Bool(inner))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// Evalue `expr` (une condition `.rsh` brute, voir `AstNode::If::condition`)
/// contre `ctx`. `Some(bool)` si `expr` tient entierement dans le
/// sous-ensemble supporte (voir la doc du module) ; `None` si elle utilise
/// quoi que ce soit hors de ce sous-ensemble (appel de methode, `in`,
/// identifiant absent de `ctx`...) - a l'appelant de decider quoi faire d'un
/// `None` (voir `interpreter::build_if_chain`, qui le traite comme faux).
pub fn evaluate(expr: &str, ctx: &Context) -> Option<bool> {
    let tokens = tokenize_condition(expr);
    let mut parser = Parser { tokens: &tokens, pos: 0 };
    let value = parser.parse_or(ctx)?;
    if parser.pos != parser.tokens.len() {
        // Jetons restants non consommes (ex: le `(`/`)` d'un appel de
        // methode apres un identifiant deja lu comme atome complet) :
        // l'expression deborde du sous-ensemble supporte.
        return None;
    }
    Some(value)
}

/// Evalue une expression "valeur" .rsh (un identifiant, ou un litteral
/// booleen/numerique/texte) contre `ctx` - le sous-ensemble utilise par le
/// SUJET d'un `match` (voir `AstNode::Match::condition`) et par le motif de
/// chacun de ses `arm` (voir `AstNode::Arm::pattern`), par opposition a
/// `evaluate` qui evalue une expression BOOLEENNE complete (comparaisons,
/// `&&`/`||`, parentheses...). `None` si `expr` n'est pas un atome simple
/// valide dans ce sous-ensemble (voir `Parser::parse_atom`) - un identifiant
/// absent de `ctx`, ou une expression plus riche (comparaison, appel de
/// methode...), par exemple.
pub fn evaluate_value(expr: &str, ctx: &Context) -> Option<ConditionValue> {
    let tokens = tokenize_condition(expr);
    let mut parser = Parser { tokens: &tokens, pos: 0 };
    let value = parser.parse_atom(ctx)?;
    if parser.pos != parser.tokens.len() {
        // Jetons restants non consommes : l'expression deborde d'un simple
        // atome (ex: une comparaison), hors du sous-ensemble supporte ici.
        return None;
    }
    Some(value)
}

/// `true` si le sujet d'un `match` (voir `evaluate_value`, `subject` est
/// `None` quand le sujet lui-meme n'a pas pu etre evalue) satisfait le motif
/// brut `pattern` d'un `arm`. `"_"` matche INCONDITIONNELLEMENT, meme quand
/// `subject` est `None` - le "catch-all" de Rust, qui n'a par definition rien
/// a comparer. Sinon `pattern` peut lister plusieurs valeurs litterales
/// separees par `|` (motif "ou", comme en Rust : `arm.1 | 2`), chacune
/// evaluee independamment par `evaluate_value` contre un `Context` VIDE - un
/// motif est toujours un litteral autonome, jamais une variable liee au
/// `Context` de l'appelant - et comparee au sujet par egalite stricte.
/// `false` si `subject` est `None` et que le motif n'est pas `_` : une valeur
/// inconnue ne matche aucun litteral, fail-closed comme le reste de ce
/// module (voir `evaluate`).
pub fn matches_pattern(pattern: &str, subject: Option<&ConditionValue>) -> bool {
    if pattern.trim() == "_" {
        return true;
    }
    let Some(subject) = subject else { return false };
    let empty = Context::new();
    pattern.split('|').filter_map(|part| evaluate_value(part.trim(), &empty)).any(|value| &value == subject)
}
