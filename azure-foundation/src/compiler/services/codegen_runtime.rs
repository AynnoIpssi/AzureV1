// Ce que le codegen ne peut pas figer en Rust : les composants (`<card>`,
// `<select>`...) et l'interpolation `{{...}}` dependent des donnees de la
// page (`Context`), connues seulement a l'execution. Pour ces parties, le code
// genere garde l'arbre rsH et la feuille rsC sous forme de litteraux Rust, et
// appelle l'interpreteur au moment de construire l'ecran : le resultat est
// exactement celui de l'interpreteur (voir `tests/codegen_dynamic.rs`).
use crate::compiler::rsc::models::rule::RscStylesheet;
use crate::compiler::rsc::models::selector::{Combinator, SimpleSelector};
use crate::compiler::rsc::models::value::{Unit, Value};
use crate::compiler::rsc::services::link::ElementInfo;
use crate::compiler::rsh::mangers::parser::AstNode;

/// `true` si ce noeud (ou un descendant) a besoin des donnees de la page :
/// un composant, ou un `{{...}}` dans un texte, une classe, un id, une source.
pub fn is_dynamic(node: &AstNode) -> bool {
    let interpolated = |s: &str| s.contains("{{");
    match node {
        AstNode::Element { .. } => true,
        AstNode::RawText(text) => interpolated(text),
        AstNode::Container { class, id, children }
        | AstNode::Title { class, id, children }
        | AstNode::Title1 { class, id, children }
        | AstNode::Title2 { class, id, children }
        | AstNode::Title3 { class, id, children }
        | AstNode::Text { class, id, children }
        | AstNode::Button { class, id, children }
        | AstNode::Textarea { class, id, children } => interpolated(class) || interpolated(id) || children.iter().any(is_dynamic),
        AstNode::Image { class, id, src, children } | AstNode::Video { class, id, src, children } => {
            interpolated(class) || interpolated(id) || interpolated(src) || children.iter().any(is_dynamic)
        }
        AstNode::If { children, .. }
        | AstNode::ElseIf { children, .. }
        | AstNode::Else { children, .. }
        | AstNode::Match { children, .. }
        | AstNode::Arm { children, .. }
        | AstNode::While { children, .. }
        | AstNode::For { children, .. }
        | AstNode::ForEach { children, .. } => children.iter().any(is_dynamic),
    }
}

/// Faut-il construire CE noeud a l'execution ? Un conteneur statique reste
/// genere en Rust meme si certains de ses enfants sont dynamiques (ils sont
/// traites un par un) ; un texte, un bouton, un composant ou une condition
/// qui touche aux donnees, oui.
pub fn needs_runtime(node: &AstNode) -> bool {
    match node {
        AstNode::Container { class, id, .. } => class.contains("{{") || id.contains("{{"),
        _ => is_dynamic(node),
    }
}

const AST: &str = "azure_foundation::compiler::rsh::mangers::parser::AstNode";

fn s(value: &str) -> String {
    format!("{value:?}.to_string()")
}

fn children(nodes: &[AstNode]) -> String {
    format!("vec![{}]", nodes.iter().map(ast_literal).collect::<Vec<_>>().join(", "))
}

/// L'arbre rsH en litteral Rust.
pub fn ast_literal(node: &AstNode) -> String {
    let element = |variant: &str, class: &str, id: &str, kids: &[AstNode]| format!("{AST}::{variant} {{ class: {}, id: {}, children: {} }}", s(class), s(id), children(kids));
    let control = |variant: &str, field: &str, value: &str, kids: &[AstNode]| format!("{AST}::{variant} {{ {field}: {}, children: {} }}", s(value), children(kids));
    match node {
        AstNode::Container { class, id, children: k } => element("Container", class, id, k),
        AstNode::Title { class, id, children: k } => element("Title", class, id, k),
        AstNode::Title1 { class, id, children: k } => element("Title1", class, id, k),
        AstNode::Title2 { class, id, children: k } => element("Title2", class, id, k),
        AstNode::Title3 { class, id, children: k } => element("Title3", class, id, k),
        AstNode::Text { class, id, children: k } => element("Text", class, id, k),
        AstNode::Button { class, id, children: k } => element("Button", class, id, k),
        AstNode::Textarea { class, id, children: k } => element("Textarea", class, id, k),
        AstNode::Image { class, id, src, children: k } => format!("{AST}::Image {{ class: {}, id: {}, src: {}, children: {} }}", s(class), s(id), s(src), children(k)),
        AstNode::Video { class, id, src, children: k } => format!("{AST}::Video {{ class: {}, id: {}, src: {}, children: {} }}", s(class), s(id), s(src), children(k)),
        AstNode::RawText(text) => format!("{AST}::RawText({})", s(text)),
        AstNode::Element { tag, class, id, attrs, children: k } => {
            let attrs: Vec<String> = attrs.iter().map(|(a, b)| format!("({}, {})", s(a), s(b))).collect();
            format!("{AST}::Element {{ tag: {}, class: {}, id: {}, attrs: vec![{}], children: {} }}", s(tag), s(class), s(id), attrs.join(", "), children(k))
        }
        AstNode::If { condition, children: k } => control("If", "condition", condition, k),
        AstNode::ElseIf { condition, children: k } => control("ElseIf", "condition", condition, k),
        AstNode::Else { condition, children: k } => control("Else", "condition", condition, k),
        AstNode::Match { condition, children: k } => control("Match", "condition", condition, k),
        AstNode::Arm { pattern, children: k } => control("Arm", "pattern", pattern, k),
        AstNode::While { condition, children: k } => control("While", "condition", condition, k),
        AstNode::For { condition, children: k } => control("For", "condition", condition, k),
        AstNode::ForEach { condition, children: k } => control("ForEach", "condition", condition, k),
    }
}

pub fn element_info_literal(info: &ElementInfo) -> String {
    let classes: Vec<String> = info.classes.iter().map(|c| s(c)).collect();
    format!(
        "azure_foundation::compiler::rsc::services::link::ElementInfo {{ tag: {}, classes: vec![{}], id: {} }}",
        s(&info.tag),
        classes.join(", "),
        s(&info.id)
    )
}

fn value_literal(value: &Value) -> String {
    let path = "azure_foundation::compiler::rsc::models::value";
    match value {
        Value::Length(n, unit) => {
            let unit = match unit {
                Unit::Px => "Px".to_string(),
                Unit::Percent => "Percent".to_string(),
                Unit::Em => "Em".to_string(),
                Unit::Rem => "Rem".to_string(),
                Unit::Vw => "Vw".to_string(),
                Unit::Vh => "Vh".to_string(),
                Unit::Fr => "Fr".to_string(),
                Unit::Unknown(u) => format!("Unknown({})", s(u)),
            };
            format!("{path}::Value::Length({n:?}f32, {path}::Unit::{unit})")
        }
        Value::Number(n) => format!("{path}::Value::Number({n:?}f32)"),
        Value::Color(c) => format!("{path}::Value::Color(azure_engine::rendering::models::color::Color::new({}, {}, {}, {}))", c.r, c.g, c.b, c.a),
        Value::Keyword(k) => format!("{path}::Value::Keyword({})", s(k)),
        Value::Str(v) => format!("{path}::Value::Str({})", s(v)),
        Value::List(items) => format!("{path}::Value::List(vec![{}])", items.iter().map(value_literal).collect::<Vec<_>>().join(", ")),
        Value::Function(name, args) => format!("{path}::Value::Function({}, vec![{}])", s(name), args.iter().map(value_literal).collect::<Vec<_>>().join(", ")),
    }
}

fn simple_literal(sel: &SimpleSelector) -> String {
    let opt = |v: &Option<String>| v.as_ref().map(|v| format!("Some({})", s(v))).unwrap_or_else(|| "None".to_string());
    let list = |v: &[String]| format!("vec![{}]", v.iter().map(|x| s(x)).collect::<Vec<_>>().join(", "));
    format!(
        "azure_foundation::compiler::rsc::models::selector::SimpleSelector {{ universal: {}, tag: {}, id: {}, classes: {}, pseudo_classes: {} }}",
        sel.universal,
        opt(&sel.tag),
        opt(&sel.id),
        list(&sel.classes),
        list(&sel.pseudo_classes)
    )
}

/// La feuille rsC en litteral Rust (construite une fois, a la premiere
/// utilisation, par le code genere).
pub fn sheet_literal(sheet: &RscStylesheet) -> String {
    let m = "azure_foundation::compiler::rsc::models";
    let rules: Vec<String> = sheet
        .rules
        .iter()
        .map(|rule| {
            let selectors: Vec<String> = rule
                .selectors
                .iter()
                .map(|sel| {
                    let parts: Vec<String> = sel
                        .parts
                        .iter()
                        .map(|(simple, comb)| {
                            let comb = match comb {
                                None => "None".to_string(),
                                Some(Combinator::Descendant) => format!("Some({m}::selector::Combinator::Descendant)"),
                                Some(Combinator::Child) => format!("Some({m}::selector::Combinator::Child)"),
                                Some(Combinator::AdjacentSibling) => format!("Some({m}::selector::Combinator::AdjacentSibling)"),
                                Some(Combinator::GeneralSibling) => format!("Some({m}::selector::Combinator::GeneralSibling)"),
                            };
                            format!("({}, {comb})", simple_literal(simple))
                        })
                        .collect();
                    format!("{m}::selector::ComplexSelector {{ parts: vec![{}] }}", parts.join(", "))
                })
                .collect();
            let declarations: Vec<String> = rule
                .declarations
                .iter()
                .map(|d| format!("{m}::rule::Declaration {{ name: {}, value: {}, important: {} }}", s(&d.name), value_literal(&d.value), d.important))
                .collect();
            format!("{m}::rule::Rule {{ selectors: vec![{}], declarations: vec![{}] }}", selectors.join(", "), declarations.join(", "))
        })
        .collect();
    format!("{m}::rule::RscStylesheet {{ rules: vec![{}], skipped: Vec::new() }}", rules.join(",\n        "))
}
