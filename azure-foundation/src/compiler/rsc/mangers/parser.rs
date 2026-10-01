use std::fmt;

use crate::compiler::rsc::models::rule::{Declaration, RscStylesheet, Rule};
use crate::compiler::rsc::models::selector::{Combinator, ComplexSelector, SimpleSelector};
use crate::compiler::rsc::models::token::{Position, Spanned, TokenRsC};
use crate::compiler::rsc::models::value::{named_color, parse_hex_color, Unit, Value};

/// Une erreur de parsing rsC, rattachee a l'endroit du source ou elle a ete
/// detectee - meme forme que `compiler::mangers::parser::ParseError` cote
/// rsH, pour rester coherent entre les deux langages.
#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub pos: Position,
}

impl ParseError {
    fn new(kind: ParseErrorKind, pos: Position) -> Self {
        ParseError { kind, pos }
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseErrorKind {
    ExpectedSelector(Option<TokenRsC>),
    ExpectedOpenBrace(Option<TokenRsC>),
    ExpectedPseudoClassName(Option<TokenRsC>),
    EmptyClassName,
    EmptyIdName,
    ExpectedPropertyName(Option<TokenRsC>),
    ExpectedColon(Option<TokenRsC>),
    ExpectedValue(Option<TokenRsC>),
    InvalidColor(String),
    UnclosedBlock,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ligne {}, colonne {} : {}", self.pos.line, self.pos.column, self.kind)
    }
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorKind::ExpectedSelector(found) => write!(f, "un selecteur etait attendu, trouve {found:?}"),
            ParseErrorKind::ExpectedOpenBrace(found) => write!(f, "'{{' attendu, trouve {found:?}"),
            ParseErrorKind::ExpectedPseudoClassName(found) => {
                write!(f, "nom de pseudo-classe attendu apres ':', trouve {found:?}")
            }
            ParseErrorKind::EmptyClassName => write!(f, "nom de classe vide apres '.'"),
            ParseErrorKind::EmptyIdName => write!(f, "nom d'id vide apres '#'"),
            ParseErrorKind::ExpectedPropertyName(found) => write!(f, "nom de propriete attendu, trouve {found:?}"),
            ParseErrorKind::ExpectedColon(found) => {
                write!(f, "':' attendu apres le nom de propriete, trouve {found:?}")
            }
            ParseErrorKind::ExpectedValue(found) => write!(f, "une valeur etait attendue, trouve {found:?}"),
            ParseErrorKind::InvalidColor(raw) => write!(f, "couleur invalide: '{raw}'"),
            ParseErrorKind::UnclosedBlock => write!(f, "bloc '{{' jamais referme"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse une feuille de style rsC complete : une suite de blocs
/// `selecteur(s) { declaration; ... }`, avec vraie syntaxe de selecteurs
/// CSS (type, classe, id, universel, combinateurs, listes separees par des
/// virgules).
pub fn parse(tokens: Vec<Spanned<TokenRsC>>) -> Result<RscStylesheet, ParseError> {
    let mut i = 0;
    let mut sheet = RscStylesheet::default();

    skip_combinators(&tokens, &mut i);
    while i < tokens.len() {
        let selectors = parse_selector_list(&tokens, &mut i)?;
        expect_open_brace(&tokens, &mut i)?;
        let declarations = parse_declarations(&tokens, &mut i, &mut sheet.skipped)?;
        sheet.rules.push(Rule { selectors, declarations });
        skip_combinators(&tokens, &mut i);
    }

    Ok(sheet)
}

fn skip_combinators(tokens: &[Spanned<TokenRsC>], i: &mut usize) {
    while matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::Combinator)) {
        *i += 1;
    }
}

fn pos_at(tokens: &[Spanned<TokenRsC>], i: usize) -> Position {
    tokens
        .get(i)
        .map(|s| s.pos)
        .or_else(|| tokens.last().map(|s| s.pos))
        .unwrap_or_else(Position::start)
}

// -------------------- Selecteurs --------------------

fn parse_selector_list(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<Vec<ComplexSelector>, ParseError> {
    let mut list = vec![parse_complex_selector(tokens, i)?];
    loop {
        skip_combinators(tokens, i);
        if matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::Comma)) {
            *i += 1;
            skip_combinators(tokens, i);
            list.push(parse_complex_selector(tokens, i)?);
        } else {
            break;
        }
    }
    Ok(list)
}

// Une nouvelle occurrence de selecteur simple peut demarrer par un type,
// `*`, une classe, un id ou une pseudo-classe (selecteur "nu", implicitement
// universel comme `:hover { }`).
fn starts_simple_selector(token: Option<&TokenRsC>) -> bool {
    matches!(
        token,
        Some(TokenRsC::Ident(_))
            | Some(TokenRsC::Star)
            | Some(TokenRsC::ClassSel(_))
            | Some(TokenRsC::Hash(_))
            | Some(TokenRsC::Colon)
    )
}

fn parse_complex_selector(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<ComplexSelector, ParseError> {
    let first = parse_simple_selector(tokens, i)?;
    let mut parts = vec![(first, None)];

    loop {
        let explicit = match tokens.get(*i).map(|s| &s.value) {
            Some(TokenRsC::GreaterThan) => Some(Combinator::Child),
            Some(TokenRsC::Plus) => Some(Combinator::AdjacentSibling),
            Some(TokenRsC::Tilde) => Some(Combinator::GeneralSibling),
            _ => None,
        };

        if let Some(combinator) = explicit {
            *i += 1;
            skip_combinators(tokens, i);
            let simple = parse_simple_selector(tokens, i)?;
            parts.push((simple, Some(combinator)));
            continue;
        }

        if matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::Combinator)) {
            let save = *i;
            *i += 1;
            match tokens.get(*i).map(|s| &s.value) {
                Some(TokenRsC::GreaterThan) => {
                    *i += 1;
                    skip_combinators(tokens, i);
                    let simple = parse_simple_selector(tokens, i)?;
                    parts.push((simple, Some(Combinator::Child)));
                }
                Some(TokenRsC::Plus) => {
                    *i += 1;
                    skip_combinators(tokens, i);
                    let simple = parse_simple_selector(tokens, i)?;
                    parts.push((simple, Some(Combinator::AdjacentSibling)));
                }
                Some(TokenRsC::Tilde) => {
                    *i += 1;
                    skip_combinators(tokens, i);
                    let simple = parse_simple_selector(tokens, i)?;
                    parts.push((simple, Some(Combinator::GeneralSibling)));
                }
                other if starts_simple_selector(other) => {
                    // espace suivi d'un nouveau selecteur simple : combinateur
                    // descendant implicite (`container .card`).
                    let simple = parse_simple_selector(tokens, i)?;
                    parts.push((simple, Some(Combinator::Descendant)));
                }
                _ => {
                    // espace de fin (avant '{' ou ',') : pas un combinateur.
                    *i = save;
                    break;
                }
            }
            continue;
        }

        break;
    }

    Ok(ComplexSelector { parts })
}

fn parse_simple_selector(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<SimpleSelector, ParseError> {
    let start_pos = pos_at(tokens, *i);
    let mut sel = SimpleSelector::default();
    let mut any = false;

    match tokens.get(*i).map(|s| &s.value) {
        Some(TokenRsC::Star) => {
            sel.universal = true;
            *i += 1;
            any = true;
        }
        Some(TokenRsC::Ident(name)) => {
            sel.tag = Some(name.clone());
            *i += 1;
            any = true;
        }
        _ => {}
    }

    loop {
        match tokens.get(*i).map(|s| &s.value) {
            Some(TokenRsC::ClassSel(name)) => {
                if name.is_empty() {
                    return Err(ParseError::new(ParseErrorKind::EmptyClassName, pos_at(tokens, *i)));
                }
                sel.classes.push(name.clone());
                *i += 1;
                any = true;
            }
            Some(TokenRsC::Hash(name)) => {
                if name.is_empty() {
                    return Err(ParseError::new(ParseErrorKind::EmptyIdName, pos_at(tokens, *i)));
                }
                sel.id = Some(name.clone());
                *i += 1;
                any = true;
            }
            Some(TokenRsC::Colon) => {
                *i += 1;
                // tolere `::` (pseudo-element) en avalant un second ':'.
                if matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::Colon)) {
                    *i += 1;
                }
                match tokens.get(*i).map(|s| &s.value) {
                    Some(TokenRsC::Ident(name)) => {
                        sel.pseudo_classes.push(name.clone());
                        *i += 1;
                        if matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::OpenParen)) {
                            skip_balanced_parens(tokens, i);
                        }
                        any = true;
                    }
                    other => {
                        return Err(ParseError::new(
                            ParseErrorKind::ExpectedPseudoClassName(other.cloned()),
                            pos_at(tokens, *i),
                        ));
                    }
                }
            }
            _ => break,
        }
    }

    if !any {
        return Err(ParseError::new(
            ParseErrorKind::ExpectedSelector(tokens.get(*i).map(|s| s.value.clone())),
            start_pos,
        ));
    }

    Ok(sel)
}

// Avale un `( ... )` equilibre (arguments d'une pseudo-classe fonctionnelle
// comme `:nth-child(2)`) sans en interpreter le contenu - non evalue pour
// l'instant, voir `SimpleSelector::pseudo_classes`.
fn skip_balanced_parens(tokens: &[Spanned<TokenRsC>], i: &mut usize) {
    if !matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::OpenParen)) {
        return;
    }
    let mut depth = 0i32;
    loop {
        match tokens.get(*i).map(|s| &s.value) {
            Some(TokenRsC::OpenParen) => {
                depth += 1;
                *i += 1;
            }
            Some(TokenRsC::CloseParen) => {
                depth -= 1;
                *i += 1;
                if depth == 0 {
                    break;
                }
            }
            Some(_) => *i += 1,
            None => break,
        }
    }
}

fn expect_open_brace(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<(), ParseError> {
    skip_combinators(tokens, i);
    match tokens.get(*i).map(|s| &s.value) {
        Some(TokenRsC::OpenBrace) => {
            *i += 1;
            Ok(())
        }
        other => Err(ParseError::new(ParseErrorKind::ExpectedOpenBrace(other.cloned()), pos_at(tokens, *i))),
    }
}

// -------------------- Declarations --------------------

fn parse_declarations(tokens: &[Spanned<TokenRsC>], i: &mut usize, skipped: &mut Vec<String>) -> Result<Vec<Declaration>, ParseError> {
    let mut decls = Vec::new();
    skip_combinators(tokens, i);
    loop {
        match tokens.get(*i).map(|s| &s.value) {
            Some(TokenRsC::CloseBrace) => {
                *i += 1;
                break;
            }
            None => return Err(ParseError::new(ParseErrorKind::UnclosedBlock, pos_at(tokens, *i))),
            Some(TokenRsC::SemiColon) => {
                // ';;' ou bloc vide : declaration nulle, tout a fait valide en CSS.
                *i += 1;
                skip_combinators(tokens, i);
            }
            _ => {
                let start = *i;
                match parse_declaration(tokens, i) {
                    Ok(decl) => decls.push(decl),
                    Err(e) => {
                        // Declaration illisible : sautee jusqu'au `;` (ou a la
                        // fin du bloc), le reste de la feuille est garde.
                        skipped.push(format!("ligne {} : declaration ignoree ({:?})", tokens.get(start).map(|t| t.pos.line).unwrap_or(0), e.kind));
                        *i = start.max(*i);
                        while let Some(t) = tokens.get(*i).map(|s| &s.value) {
                            match t {
                                TokenRsC::SemiColon => {
                                    *i += 1;
                                    break;
                                }
                                TokenRsC::CloseBrace => break,
                                _ => *i += 1,
                            }
                        }
                    }
                }
                skip_combinators(tokens, i);
            }
        }
    }
    Ok(decls)
}

fn parse_declaration(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<Declaration, ParseError> {
    let name = match tokens.get(*i).map(|s| &s.value) {
        Some(TokenRsC::Ident(name)) => name.clone(),
        other => return Err(ParseError::new(ParseErrorKind::ExpectedPropertyName(other.cloned()), pos_at(tokens, *i))),
    };
    *i += 1;
    skip_combinators(tokens, i);

    match tokens.get(*i).map(|s| &s.value) {
        Some(TokenRsC::Colon) => *i += 1,
        other => return Err(ParseError::new(ParseErrorKind::ExpectedColon(other.cloned()), pos_at(tokens, *i))),
    }
    skip_combinators(tokens, i);

    let (value, important) = parse_declaration_value(tokens, i)?;

    skip_combinators(tokens, i);
    if matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::SemiColon)) {
        *i += 1;
    }

    Ok(Declaration { name, value, important })
}

fn is_value_terminator(tokens: &[Spanned<TokenRsC>], i: usize) -> bool {
    matches!(
        tokens.get(i).map(|s| &s.value),
        None | Some(TokenRsC::SemiColon) | Some(TokenRsC::CloseBrace) | Some(TokenRsC::Comma) | Some(TokenRsC::Bang) | Some(TokenRsC::CloseParen)
    )
}

// Une valeur de declaration est une liste de groupes separes par des
// virgules (`font-family: "A", sans-serif`), chaque groupe etant lui-meme
// une liste de composants separes par des espaces (`margin: 10px 20px`),
// suivie d'un `!important` optionnel.
fn parse_declaration_value(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<(Value, bool), ParseError> {
    let mut groups = vec![parse_value_group(tokens, i)?];
    loop {
        skip_combinators(tokens, i);
        if matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::Comma)) {
            *i += 1;
            skip_combinators(tokens, i);
            groups.push(parse_value_group(tokens, i)?);
        } else {
            break;
        }
    }

    let important = consume_important(tokens, i)?;

    let value = if groups.len() == 1 { groups.remove(0) } else { Value::List(groups) };
    Ok((value, important))
}

fn parse_value_group(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<Value, ParseError> {
    let mut items = vec![parse_single_value(tokens, i)?];
    loop {
        if matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::Combinator)) {
            let save = *i;
            *i += 1;
            if is_value_terminator(tokens, *i) {
                // espace de fin (avant ';', '}', ',' ou '!important') : pas
                // un separateur de composant.
                *i = save;
                break;
            }
            items.push(parse_single_value(tokens, i)?);
        } else {
            break;
        }
    }
    Ok(if items.len() == 1 { items.remove(0) } else { Value::List(items) })
}

fn consume_important(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<bool, ParseError> {
    skip_combinators(tokens, i);
    if matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::Bang)) {
        *i += 1;
        skip_combinators(tokens, i);
        return match tokens.get(*i).map(|s| &s.value) {
            Some(TokenRsC::Ident(name)) if name.eq_ignore_ascii_case("important") => {
                *i += 1;
                Ok(true)
            }
            other => Err(ParseError::new(ParseErrorKind::ExpectedValue(other.cloned()), pos_at(tokens, *i))),
        };
    }
    Ok(false)
}

fn parse_single_value(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<Value, ParseError> {
    match tokens.get(*i).map(|s| &s.value) {
        Some(TokenRsC::Ident(name)) if matches!(tokens.get(*i + 1).map(|s| &s.value), Some(TokenRsC::OpenParen)) => {
            let name = name.clone();
            let pos = pos_at(tokens, *i);
            *i += 2; // consomme l'ident et '('
            let args = parse_function_args(tokens, i)?;
            build_function_value(&name, &args, pos)
        }
        Some(TokenRsC::Ident(name)) => {
            let name = name.clone();
            *i += 1;
            Ok(match named_color(&name) {
                Some(color) => Value::Color(color),
                None => Value::Keyword(name.to_ascii_lowercase()),
            })
        }
        Some(TokenRsC::Hash(raw)) => {
            let raw = raw.clone();
            let pos = pos_at(tokens, *i);
            *i += 1;
            parse_hex_color(&raw)
                .map(Value::Color)
                .ok_or_else(|| ParseError::new(ParseErrorKind::InvalidColor(format!("#{raw}")), pos))
        }
        Some(TokenRsC::Number(n)) => {
            let n = *n;
            *i += 1;
            Ok(Value::Number(n))
        }
        Some(TokenRsC::Percentage(n)) => {
            let n = *n;
            *i += 1;
            Ok(Value::Length(n, Unit::Percent))
        }
        Some(TokenRsC::Dimension(n, unit)) => {
            let n = *n;
            let unit = Unit::parse(unit);
            *i += 1;
            Ok(Value::Length(n, unit))
        }
        Some(TokenRsC::StringLiteral(s)) => {
            let s = s.clone();
            *i += 1;
            Ok(Value::Str(s))
        }
        other => Err(ParseError::new(ParseErrorKind::ExpectedValue(other.cloned()), pos_at(tokens, *i))),
    }
}

// Arguments d'une fonction (`rgb(255, 0, 0)`, `linear-gradient(90deg, #fff
// 0%, #000 100%)`) : une liste d'arguments separes par des virgules, chacun
// pouvant etre fait de plusieurs mots (voir `parse_value_group`), jusqu'a la
// ')' fermante.
fn parse_function_args(tokens: &[Spanned<TokenRsC>], i: &mut usize) -> Result<Vec<Value>, ParseError> {
    skip_combinators(tokens, i);
    let mut args = Vec::new();
    if matches!(tokens.get(*i).map(|s| &s.value), Some(TokenRsC::CloseParen)) {
        *i += 1;
        return Ok(args);
    }
    loop {
        args.push(parse_value_group(tokens, i)?);
        skip_combinators(tokens, i);
        match tokens.get(*i).map(|s| &s.value) {
            Some(TokenRsC::Comma) => {
                *i += 1;
                skip_combinators(tokens, i);
            }
            Some(TokenRsC::CloseParen) => {
                *i += 1;
                break;
            }
            other => return Err(ParseError::new(ParseErrorKind::ExpectedValue(other.cloned()), pos_at(tokens, *i))),
        }
    }
    Ok(args)
}

// Traduit un appel de fonction en valeur typee. `rgb`/`rgba` deviennent
// directement une couleur ; toute autre fonction (`linear-gradient`,
// `radial-gradient`, `calc`...) est gardee avec ses arguments
// (`Value::Function`), interpretee par la propriete qui l'utilise.
fn build_function_value(name: &str, args: &[Value], pos: Position) -> Result<Value, ParseError> {
    match name.to_ascii_lowercase().as_str() {
        "rgb" | "rgba" => {
            let component = |v: &Value| -> Option<u8> {
                match v {
                    Value::Number(n) => Some(n.round().clamp(0.0, 255.0) as u8),
                    _ => None,
                }
            };
            let alpha = |v: &Value| -> Option<u8> {
                match v {
                    Value::Number(n) => Some((n.clamp(0.0, 1.0) * 255.0).round() as u8),
                    _ => None,
                }
            };
            if args.len() < 3 {
                return Err(ParseError::new(ParseErrorKind::InvalidColor(format!("{name}(...)")), pos));
            }
            let (r, g, b) = (component(&args[0]), component(&args[1]), component(&args[2]));
            let a = args.get(3).and_then(alpha).unwrap_or(255);
            match (r, g, b) {
                (Some(r), Some(g), Some(b)) => {
                    Ok(Value::Color(azure_engine::rendering::models::color::Color::new(r, g, b, a)))
                }
                _ => Err(ParseError::new(ParseErrorKind::InvalidColor(format!("{name}(...)")), pos)),
            }
        }
        other => Ok(Value::Function(other.to_string(), args.to_vec())),
    }
}
