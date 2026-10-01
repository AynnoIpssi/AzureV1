// Le "systeme de lien" entre rsC et rsH : fait correspondre les selecteurs
// d'une feuille de style rsC directement contre l'arbre d'AST .rsh (nom de
// balise, classe, id, ancetres), calcule la cascade (specificite + ordre
// source + !important) et produit un `ComputedStyle` par element - sans
// passer par l'ancien systeme `.style` a plat (classe seule, pas de balise
// ni d'id, pas de combinateurs).

use crate::compiler::rsh::mangers::parser::AstNode;
use crate::compiler::rsc::models::property::{ComputedStyle, Property};
use crate::compiler::rsc::models::rule::RscStylesheet;
use crate::compiler::rsc::models::selector::{Combinator, ComplexSelector, PseudoState, SimpleSelector, Specificity};
use crate::compiler::rsc::models::value::{Unit, Value};
use crate::layout::models::layout_props::{AlignItems, DisplayMode, FlexDirection, JustifyContent, Overflow, Track};
use crate::style::models::style::Style;
use crate::style::models::web_style::{LineHeight, TextAlign, WebStyle, WhiteSpace};
use crate::layout::models::css_box::{CssDisplay, CssTrack, Length};
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::{ColorStop, Fill, Shadow};

/// Les informations d'un element .rsh necessaires au filtrage par
/// selecteur : le nom de balise (equivalent CSS d'un nom d'element HTML,
/// tel que porte par les tokens .rsh - `<container.card#main>` -> balise
/// "container"), sa classe et son id.
#[derive(Debug, Clone)]
pub struct ElementInfo {
    pub tag: String,
    pub classes: Vec<String>,
    pub id: String,
}

impl ElementInfo {
    pub fn new(tag: &str, class: &str, id: &str) -> Self {
        ElementInfo {
            tag: tag.to_string(),
            // `class` peut porter plusieurs classes separees par des espaces
            // (`<text.note.warning>` en rsH).
            classes: class.split_whitespace().map(str::to_string).collect(),
            id: id.to_string(),
        }
    }
}

/// Nom de balise CSS pour un noeud d'AST rsH - c'est ce nom que les
/// selecteurs de type (`container { ... }`) comparent. `None` pour les
/// noeuds structurels (`RawText`, `If`/`While`/...) qui ne sont pas des
/// elements stylables.
pub fn tag_name(node: &AstNode) -> Option<&'static str> {
    match node {
        AstNode::Container { .. } => Some("container"),
        AstNode::Title { .. } => Some("title"),
        AstNode::Title1 { .. } => Some("title1"),
        AstNode::Title2 { .. } => Some("title2"),
        AstNode::Title3 { .. } => Some("title3"),
        AstNode::Text { .. } => Some("text"),
        AstNode::Button { .. } => Some("button"),
        AstNode::Image { .. } => Some("image"),
        AstNode::Video { .. } => Some("video"),
        AstNode::Textarea { .. } => Some("textarea"),
        _ => None,
    }
}

fn element_info(node: &AstNode) -> Option<ElementInfo> {
    let tag = tag_name(node)?;
    match node {
        AstNode::Container { class, id, .. }
        | AstNode::Title { class, id, .. }
        | AstNode::Title1 { class, id, .. }
        | AstNode::Title2 { class, id, .. }
        | AstNode::Title3 { class, id, .. }
        | AstNode::Text { class, id, .. }
        | AstNode::Button { class, id, .. }
        | AstNode::Image { class, id, .. }
        | AstNode::Video { class, id, .. }
        | AstNode::Textarea { class, id, .. } => Some(ElementInfo::new(tag, class, id)),
        _ => None,
    }
}

fn children_of(node: &AstNode) -> &[AstNode] {
    match node {
        AstNode::Container { children, .. }
        | AstNode::Title { children, .. }
        | AstNode::Title1 { children, .. }
        | AstNode::Title2 { children, .. }
        | AstNode::Title3 { children, .. }
        | AstNode::Text { children, .. }
        | AstNode::Button { children, .. }
        | AstNode::Image { children, .. }
        | AstNode::Video { children, .. }
        | AstNode::Textarea { children, .. }
        | AstNode::If { children, .. }
        | AstNode::ElseIf { children, .. }
        | AstNode::Else { children, .. }
        | AstNode::Match { children, .. }
        | AstNode::Arm { children, .. }
        | AstNode::While { children, .. }
        | AstNode::For { children, .. }
        | AstNode::ForEach { children, .. }
        | AstNode::Element { children, .. } => children,
        AstNode::RawText(_) => &[],
    }
}

fn simple_matches(sel: &crate::compiler::rsc::models::selector::SimpleSelector, el: &ElementInfo, pseudo: PseudoState) -> bool {
    sel.matches(&el.tag, &el.classes, &el.id, pseudo)
}

/// Est-ce que `selector` correspond a `element`, etant donne la chaine de
/// ses ancetres (ordonnee racine -> parent immediat) et la liste, dans
/// l'ordre du document, des freres qui le PRECEDENT au meme niveau (voir
/// `resolve_flat`) ? Remonte le selecteur de la feuille vers la racine, en
/// consommant ancetres/freres au fur et a mesure - meme algorithme que celui
/// utilise par les moteurs CSS (matching de droite a gauche).
///
/// `pseudo` decrit l'etat de `element` LUI-MEME (survole ? focalise ?) et
/// n'est applique qu'au maillon le plus a droite (le "sujet" du selecteur,
/// celui qui doit reellement matcher `element` - `button:hover` designe un
/// bouton survole, pas un descendant quelconque d'un ancetre survole) : les
/// ancetres/freres sont toujours matches avec `PseudoState::default()`, leur
/// propre etat de survol/focus n'etant pas suivi ici.
///
/// `around` : l'etat suppose des ancetres (voir `resolve_element_in`).
fn complex_matches(selector: &ComplexSelector, ancestors: &[ElementInfo], preceding: &[ElementInfo], element: &ElementInfo, pseudo: PseudoState, around: PseudoState) -> bool {
    let parts = &selector.parts;
    if parts.is_empty() {
        return false;
    }

    let last = parts.len() - 1;
    if !simple_matches(&parts[last].0, element, pseudo) {
        return false;
    }

    chain_matches(parts, last, ancestors, preceding, around)
}

// Est-ce que tout ce qui precede `parts[idx]` dans la chaine (les maillons
// `0..idx`, relies par leurs combinateurs) peut etre satisfait, sachant que
// `parts[idx]` vient de matcher un element dont le contexte est
// `(ancestors, preceding)` ? Recursif plutot qu'une simple boucle lineaire :
// un combinateur frere general (`~`) doit pouvoir essayer PLUSIEURS freres
// precedents (backtracking), pas juste le premier trouve - et par la meme
// occasion, `Descendant` en profite aussi pour redevenir correct dans les
// (rares) cas ou l'ancetre le plus proche qui matche ne mene lui-meme a
// aucune suite valide plus a gauche.
fn chain_matches(parts: &[(SimpleSelector, Option<Combinator>)], idx: usize, ancestors: &[ElementInfo], preceding: &[ElementInfo], around: PseudoState) -> bool {
    if idx == 0 {
        return true;
    }
    let combinator = parts[idx].1.expect("un combinateur relie chaque maillon sauf le premier");
    let target = &parts[idx - 1].0;

    match combinator {
        Combinator::Child => match ancestors.split_last() {
            Some((parent, rest)) if simple_matches(target, parent, around) => {
                // Au-dela d'un combinateur ancetre, on perd la trace des
                // freres du nouveau sujet (non suivis niveau par niveau ici) :
                // un combinateur frere plus a gauche dans la meme chaine
                // (`a + b > c`, lu de droite a gauche) ne trouvera donc
                // jamais de candidat - limite documentee, voir `Combinator`.
                chain_matches(parts, idx - 1, rest, &[], around)
            }
            _ => false,
        },
        Combinator::Descendant => {
            let mut rest = ancestors;
            while let Some((candidate, before)) = rest.split_last() {
                if simple_matches(target, candidate, around) && chain_matches(parts, idx - 1, before, &[], around) {
                    return true;
                }
                rest = before;
            }
            false
        }
        Combinator::AdjacentSibling => match preceding.split_last() {
            Some((sibling, before)) if simple_matches(target, sibling, PseudoState::default()) => {
                chain_matches(parts, idx - 1, ancestors, before, around)
            }
            _ => false,
        },
        Combinator::GeneralSibling => {
            let mut rest = preceding;
            while let Some((candidate, before)) = rest.split_last() {
                if simple_matches(target, candidate, PseudoState::default()) && chain_matches(parts, idx - 1, ancestors, before, around) {
                    return true;
                }
                rest = before;
            }
            false
        }
    }
}

/// Resout le `ComputedStyle` cascade d'un element sous l'etat `pseudo`
/// donne (`PseudoState::default()` pour son etat "de base", sans aucune
/// pseudo-classe active) : parcourt toutes les regles de `sheet`, retient
/// celles dont au moins un selecteur correspond, puis applique leurs
/// declarations dans l'ordre `(specificite, ordre source)` croissant - les
/// declarations `!important` sont appliquees dans une seconde passe, apres
/// toutes les declarations normales, pour toujours l'emporter comme le veut
/// la specification CSS. `preceding` : les freres deja rencontres AVANT
/// `element` au meme niveau, dans l'ordre du document - voir
/// `resolve_flat`, seul appelant qui sait construire cette liste - requis
/// pour les combinateurs freres (`+`/`~`, voir `Combinator`).
pub fn resolve_element(sheet: &RscStylesheet, ancestors: &[ElementInfo], preceding: &[ElementInfo], element: &ElementInfo, pseudo: PseudoState) -> ComputedStyle {
    resolve_element_in(sheet, ancestors, preceding, element, pseudo, PseudoState::default())
}

/// Comme `resolve_element`, les ancetres etant dans l'etat `around` : avec
/// `hover`, le style d'un element quand un ancetre est survole
/// (`.bloc:hover .poignee { opacity: 1 }`, voir `is_hover_group`).
pub fn resolve_element_in(sheet: &RscStylesheet, ancestors: &[ElementInfo], preceding: &[ElementInfo], element: &ElementInfo, pseudo: PseudoState, around: PseudoState) -> ComputedStyle {
    // Memorise (voir `clear_resolve_cache`) : une liste repete les memes
    // elements sous la meme lignee, et chaque element est resolu plusieurs
    // fois (normal, survol, focus, appui, decoration).
    if preceding.is_empty() || !has_sibling_rules(sheet) {
        use std::fmt::Write;
        let mut key = format!("{:x}|{pseudo:?}|{around:?}|", sheet as *const RscStylesheet as usize);
        // Un id ne change le style que si une regle le vise : les ids de
        // chaque ligne d'une liste (`#lancer-12`) restent hors de la cle,
        // sinon chaque ligne rate le cache.
        let id_of = |id: &str| if !id.is_empty() && id_used(sheet, id) { id.to_string() } else { String::new() };
        for a in ancestors {
            let _ = write!(key, "{}.{}#{}>", a.tag, a.classes.join("."), id_of(&a.id));
        }
        let _ = write!(key, "{}.{}#{}", element.tag, element.classes.join("."), id_of(&element.id));
        if let Some(hit) = RESOLVED.with(|c| c.borrow().get(&key).cloned()) {
            return hit;
        }
        let computed = resolve_uncached(sheet, ancestors, preceding, element, pseudo, around);
        RESOLVED.with(|c| c.borrow_mut().insert(key, computed.clone()));
        return computed;
    }
    resolve_uncached(sheet, ancestors, preceding, element, pseudo, around)
}

thread_local! {
    static RESOLVED: std::cell::RefCell<std::collections::HashMap<String, ComputedStyle>> = Default::default();
    static SIBLING_RULES: std::cell::Cell<Option<(usize, usize, bool)>> = const { std::cell::Cell::new(None) };
    static IDS_USED: std::cell::RefCell<Option<((usize, usize), std::collections::HashSet<String>)>> = const { std::cell::RefCell::new(None) };
}

/// Une regle de la feuille vise-t-elle l'id `id` ?
fn id_used(sheet: &RscStylesheet, id: &str) -> bool {
    let key = (sheet as *const RscStylesheet as usize, sheet.rules.len());
    IDS_USED.with(|c| {
        let mut c = c.borrow_mut();
        if c.as_ref().is_none_or(|(k, _)| *k != key) {
            let ids = sheet.rules.iter().flat_map(|r| &r.selectors).flat_map(|c| &c.parts).filter_map(|(s, _)| s.id.clone()).collect();
            *c = Some((key, ids));
        }
        c.as_ref().is_some_and(|(_, ids)| ids.contains(id))
    })
}

/// Oublie les styles memorises : a appeler quand on construit un nouvel
/// ecran (voir `interpreter::build_ui_with_context`).
pub fn clear_resolve_cache() {
    RESOLVED.with(|c| c.borrow_mut().clear());
    SIBLING_RULES.with(|s| s.set(None));
    IDS_USED.with(|c| *c.borrow_mut() = None);
}

/// La feuille a-t-elle des regles `a + b` / `a ~ b` ? Elles dependent des
/// freres precedents : on ne memorise alors que les elements sans freres.
pub fn has_sibling_rules(sheet: &RscStylesheet) -> bool {
    let id = (sheet as *const RscStylesheet as usize, sheet.rules.len());
    if let Some((p, n, v)) = SIBLING_RULES.with(|s| s.get())
        && (p, n) == id
    {
        return v;
    }
    let v = sheet.rules.iter().flat_map(|r| &r.selectors).flat_map(|c| &c.parts).any(|(_, comb)| matches!(comb, Some(Combinator::AdjacentSibling | Combinator::GeneralSibling)));
    SIBLING_RULES.with(|s| s.set(Some((id.0, id.1, v))));
    v
}

fn resolve_uncached(sheet: &RscStylesheet, ancestors: &[ElementInfo], preceding: &[ElementInfo], element: &ElementInfo, pseudo: PseudoState, around: PseudoState) -> ComputedStyle {
    struct Matched<'a> {
        specificity: Specificity,
        order: usize,
        decl: &'a crate::compiler::rsc::models::rule::Declaration,
    }

    let mut normal: Vec<Matched> = Vec::new();
    let mut important: Vec<Matched> = Vec::new();
    let mut order = 0usize;

    for rule in &sheet.rules {
        let best_specificity = rule
            .selectors
            .iter()
            .filter(|selector| complex_matches(selector, ancestors, preceding, element, pseudo, around))
            .map(ComplexSelector::specificity)
            .max();

        if let Some(specificity) = best_specificity {
            for decl in &rule.declarations {
                let matched = Matched { specificity, order, decl };
                if decl.important {
                    important.push(matched);
                } else {
                    normal.push(matched);
                }
                order += 1;
            }
        }
    }

    normal.sort_by(|a, b| a.specificity.cmp(&b.specificity).then(a.order.cmp(&b.order)));
    important.sort_by(|a, b| a.specificity.cmp(&b.specificity).then(a.order.cmp(&b.order)));

    let mut style = ComputedStyle::new();
    for matched in normal.into_iter().chain(important) {
        match Property::from_name(&matched.decl.name) {
            Some(property) => style.set(property, matched.decl.value.clone()),
            None => style.custom.push((matched.decl.name.clone(), format!("{:?}", matched.decl.value))),
        }
    }
    style
}

/// Est-ce que `element` est un « groupe de survol » : l'ancetre survole
/// d'un selecteur comme `.bloc:hover .poignee` (un maillon `:hover` qui
/// n'est pas le dernier). Ses descendants changent quand il est survole.
pub fn is_hover_group(sheet: &RscStylesheet, element: &ElementInfo) -> bool {
    let hover = PseudoState { hover: true, focus: false, active: false };
    sheet.rules.iter().flat_map(|rule| &rule.selectors).any(|selector| {
        let n = selector.parts.len();
        selector.parts[..n.saturating_sub(1)].iter().any(|(part, _)| part.pseudo_classes.iter().any(|p| p == "hover") && simple_matches(part, element, hover))
    })
}

/// Un noeud .rsh accompagne du `ComputedStyle` resolu par rsC - la
/// projection "stylee" de l'AST rsH, utile pour inspecter/tester la cascade
/// independamment du codegen.
#[derive(Debug, Clone)]
pub struct StyledNode {
    pub tag: String,
    pub class: String,
    pub id: String,
    pub style: ComputedStyle,
    pub children: Vec<StyledNode>,
}

/// Parcourt tout l'AST .rsh et resout le style de chaque element stylable
/// contre `sheet`. Les noeuds structurels (`If`/`While`/`RawText`/...) sont
/// transparents : leurs enfants stylables sont visites avec la meme chaine
/// d'ancetres ET la meme liste de freres precedents, exactement comme s'ils
/// n'existaient pas dans l'arbre CSS (un bouton enveloppe dans un `<if>`
/// reste un frere direct de ce qui l'entoure, voir `resolve_flat`).
pub fn resolve_tree(nodes: &[AstNode], sheet: &RscStylesheet) -> Vec<StyledNode> {
    resolve_siblings(nodes, &[], sheet)
}

fn resolve_siblings(nodes: &[AstNode], ancestors: &[ElementInfo], sheet: &RscStylesheet) -> Vec<StyledNode> {
    let mut out = Vec::new();
    let mut preceding: Vec<ElementInfo> = Vec::new();
    resolve_flat(nodes, ancestors, &mut preceding, sheet, &mut out);
    out
}

// Un seul niveau d'imbrication CSS (voir la doc de `resolve_tree` pour la
// transparence des noeuds structurels), en alimentant `preceding` au fur et
// a mesure : c'est la liste, dans l'ordre du document, des elements
// stylables deja rencontres a CE niveau - necessaire aux combinateurs
// freres (`+`/`~`), qui n'existaient pas avant (voir `chain_matches`).
fn resolve_flat(nodes: &[AstNode], ancestors: &[ElementInfo], preceding: &mut Vec<ElementInfo>, sheet: &RscStylesheet, out: &mut Vec<StyledNode>) {
    for node in nodes {
        match element_info(node) {
            Some(info) => {
                let style = resolve_element(sheet, ancestors, preceding, &info, PseudoState::default());
                let mut next_ancestors = ancestors.to_vec();
                next_ancestors.push(info.clone());
                let children = resolve_siblings(children_of(node), &next_ancestors, sheet);
                out.push(StyledNode {
                    tag: info.tag.clone(),
                    class: info.classes.first().cloned().unwrap_or_default(),
                    id: info.id.clone(),
                    style,
                    children,
                });
                preceding.push(info);
            }
            None => resolve_flat(children_of(node), ancestors, preceding, sheet, out),
        }
    }
}

/// Convertit un `ComputedStyle` (rsC) vers l'ancien `Style` (`style::models`)
/// pour rester compatible avec le codegen actuel : au-dela de
/// x/y/largeur/hauteur/marge/padding, `Style`/`LayoutProps` portent aussi
/// desormais `display`/flexbox/grid (voir
/// `layout::managers::layout_manager::resolve_children`) - ces champs ne
/// sont mappes ici que quand une regle rsC les definit reellement (`None`
/// sinon), pour laisser `LayoutProps::new` a ses valeurs par defaut (mode
/// `Block`) le reste du temps. `position` (`relative`/`absolute`/`fixed`)
/// reste hors perimetre : `left`/`top` restent les seules proprietes de
/// positionnement mappees (vers x/y).
pub fn to_legacy_style(computed: &ComputedStyle) -> Style {
    Style {
        x: number(computed.left.as_ref()),
        y: number(computed.top.as_ref()),
        width: number(computed.width.as_ref()),
        height: number(computed.height.as_ref()),
        margin: number(computed.margin.as_ref()),
        padding: number(computed.padding.as_ref()),
        radius: number(computed.border_radius.as_ref()),
        background: color_of(computed.background_color.as_ref().or(computed.background.as_ref())),
        color: color_of(computed.color.as_ref()),
        font_size: font_size_of(computed.font_size.as_ref()),
        font_weight: font_weight_of(computed.font_weight.as_ref()),
        display: display_of(computed.display.as_ref()),
        flex_direction: flex_direction_of(computed.flex_direction.as_ref()),
        flex_wrap: flex_wrap_of(computed.flex_wrap.as_ref()),
        justify_content: justify_content_of(computed.justify_content.as_ref()),
        align_items: align_items_of(computed.align_items.as_ref()),
        gap: number(computed.gap.as_ref()),
        flex_grow: number(computed.flex_grow.as_ref()),
        flex_shrink: number(computed.flex_shrink.as_ref()),
        flex_basis: number(computed.flex_basis.as_ref()),
        grid_template_columns: track_list_of(computed.grid_template_columns.as_ref()),
        grid_template_rows: track_list_of(computed.grid_template_rows.as_ref()),
        grid_column: usize_of(computed.grid_column.as_ref()),
        grid_column_span: usize_of(computed.grid_column_span.as_ref()),
        grid_row: usize_of(computed.grid_row.as_ref()),
        grid_row_span: usize_of(computed.grid_row_span.as_ref()),
        // `overflow-y` l'emporte sur `overflow` (seul l'axe vertical defile).
        overflow: overflow_of(computed.overflow_y.as_ref()).or_else(|| overflow_of(computed.overflow.as_ref())),
        fill: gradient_of(computed.background_image.as_ref()).or_else(|| gradient_of(computed.background.as_ref())),
        border_width: number(computed.border_width.as_ref()).or_else(|| border_part(computed.border.as_ref()).0),
        border_sides: [
            (&computed.border_top_width, &computed.border_top),
            (&computed.border_right_width, &computed.border_right),
            (&computed.border_bottom_width, &computed.border_bottom),
            (&computed.border_left_width, &computed.border_left),
        ]
        .map(|(width, shorthand)| number(width.as_ref()).or_else(|| border_part(shorthand.as_ref()).0)),
        // Une seule couleur pour les 4 cotes : `border-color`, sinon celle
        // de `border`, sinon celle du premier cote qui en donne une.
        border_color: color_of(computed.border_color.as_ref())
            .or_else(|| border_part(computed.border.as_ref()).1)
            .or_else(|| {
                [&computed.border_top, &computed.border_right, &computed.border_bottom, &computed.border_left]
                    .into_iter()
                    .find_map(|side| border_part(side.as_ref()).1)
            }),
        shadow: shadow_of(computed.box_shadow.as_ref()),
        opacity: opacity_of(computed.opacity.as_ref()),
        border_style: border_style_of(computed),
        background_image: background_image_of(computed),
        transition: transition_of(computed.transition.as_ref()),
        align_content: computed.align_content.as_ref().and_then(Value::as_keyword).and_then(|k| {
            use crate::layout::models::css_box::AlignContent;
            Some(match k {
                "flex-start" | "start" | "normal" => AlignContent::Start,
                "flex-end" | "end" => AlignContent::End,
                "center" => AlignContent::Center,
                "space-between" => AlignContent::SpaceBetween,
                "space-around" => AlignContent::SpaceAround,
                "space-evenly" => AlignContent::SpaceEvenly,
                "stretch" => AlignContent::Stretch,
                _ => return None,
            })
        }),
        overflow_x: overflow_of(computed.overflow_x.as_ref()).or_else(|| overflow_of(computed.overflow.as_ref())),
        web: web_of(computed),
    }
}

fn number(value: Option<&Value>) -> Option<f32> {
    value.and_then(Value::as_bare_number)
}

fn color_of(value: Option<&Value>) -> Option<Color> {
    value.and_then(Value::as_color)
}

fn font_weight_of(value: Option<&Value>) -> Option<f32> {
    value.and_then(|v| match v {
        Value::Number(n) => Some(*n),
        Value::Keyword(k) => match k.as_str() {
            "bold" => Some(700.0),
            "normal" => Some(400.0),
            "lighter" => Some(300.0),
            "bolder" => Some(800.0),
            _ => None,
        },
        _ => None,
    })
}

fn usize_of(value: Option<&Value>) -> Option<usize> {
    value.and_then(Value::as_bare_number).map(|n| n.max(1.0).round() as usize)
}

fn display_of(value: Option<&Value>) -> Option<DisplayMode> {
    value.and_then(Value::as_keyword).and_then(|k| match k {
        "block" => Some(DisplayMode::Block),
        "flex" => Some(DisplayMode::Flex),
        "grid" => Some(DisplayMode::Grid),
        _ => None,
    })
}

// `linear-gradient(angle | to <cote>, stops...)` / `radial-gradient([forme],
// stops...)`. Un arret est une couleur, eventuellement suivie de sa
// position en % ; les positions absentes sont reparties regulierement,
// comme en CSS.
fn gradient_of(value: Option<&Value>) -> Option<Fill> {
    let Some(Value::Function(name, args)) = value else { return None };
    let (angle, stop_args) = match name.as_str() {
        "linear-gradient" => match args.first().and_then(angle_of) {
            Some(angle) => (Some(angle), &args[1..]),
            None => (Some(180.0), &args[..]),
        },
        "radial-gradient" => match args.first() {
            Some(first) if first_color(first).is_none() => (None, &args[1..]),
            _ => (None, &args[..]),
        },
        _ => return None,
    };
    let mut stops: Vec<(Color, Option<f32>)> = Vec::new();
    for arg in stop_args {
        let color = first_color(arg)?;
        let position = match arg {
            Value::List(items) => items.iter().find_map(|v| match v {
                Value::Length(n, Unit::Percent) => Some(n / 100.0),
                _ => None,
            }),
            _ => None,
        };
        stops.push((color, position));
    }
    if stops.is_empty() {
        return None;
    }
    let last = stops.len().saturating_sub(1).max(1) as f32;
    let stops = stops
        .iter()
        .enumerate()
        .map(|(i, (color, position))| ColorStop { color: *color, position: position.unwrap_or(i as f32 / last) })
        .collect();
    Some(match angle {
        Some(angle) => Fill::Linear { angle, stops },
        None => Fill::Radial { stops },
    })
}

fn first_color(value: &Value) -> Option<Color> {
    match value {
        Value::Color(c) => Some(*c),
        Value::List(items) => items.iter().find_map(Value::as_color),
        _ => None,
    }
}

// `135deg`, `0.25turn`, `1.5rad`, ou `to right` / `to bottom right`...
fn angle_of(value: &Value) -> Option<f32> {
    match value {
        Value::Length(n, Unit::Unknown(unit)) => match unit.as_str() {
            "deg" => Some(*n),
            "turn" => Some(n * 360.0),
            "rad" => Some(n.to_degrees()),
            "grad" => Some(n * 0.9),
            _ => None,
        },
        Value::List(items) if items.first().and_then(Value::as_keyword) == Some("to") => {
            let sides: Vec<&str> = items[1..].iter().filter_map(Value::as_keyword).collect();
            let (mut x, mut y) = (0.0f32, 0.0f32);
            for side in &sides {
                match *side {
                    "top" => y = -1.0,
                    "bottom" => y = 1.0,
                    "left" => x = -1.0,
                    "right" => x = 1.0,
                    _ => return None,
                }
            }
            if x == 0.0 && y == 0.0 {
                return None;
            }
            // 0deg = vers le haut, 90deg = vers la droite.
            Some(x.atan2(-y).to_degrees().rem_euclid(360.0))
        }
        _ => None,
    }
}

// `border: 2px solid #fff` : (epaisseur, couleur), dans n'importe quel ordre.
fn border_part(value: Option<&Value>) -> (Option<f32>, Option<Color>) {
    let items: Vec<&Value> = match value {
        Some(Value::List(items)) => items.iter().collect(),
        Some(single) => vec![single],
        None => return (None, None),
    };
    let width = items.iter().find_map(|v| v.as_length_px());
    let color = items.iter().find_map(|v| v.as_color());
    let none = items.iter().any(|v| v.as_keyword() == Some("none"));
    if none { (Some(0.0), None) } else { (width, color) }
}

// `box-shadow: [inset] <x> <y> [flou] [etalement] <couleur>` ou `none`.
fn shadow_of(value: Option<&Value>) -> Option<Option<Shadow>> {
    let value = value?;
    if value.as_keyword() == Some("none") {
        return Some(None);
    }
    let items: Vec<&Value> = match value {
        Value::List(items) => items.iter().collect(),
        single => vec![single],
    };
    let lengths: Vec<f32> = items.iter().filter_map(|v| v.as_length_px()).collect();
    if lengths.len() < 2 {
        return None;
    }
    let color = items.iter().find_map(|v| v.as_color()).unwrap_or(Color::new(0, 0, 0, 128));
    Some(Some(Shadow {
        offset_x: lengths[0],
        offset_y: lengths[1],
        blur: lengths.get(2).copied().unwrap_or(0.0).max(0.0),
        spread: lengths.get(3).copied().unwrap_or(0.0),
        color,
        inset: items.iter().any(|v| v.as_keyword() == Some("inset")),
    }))
}

// `opacity: 0.5` ou `opacity: 50%`.
fn opacity_of(value: Option<&Value>) -> Option<f32> {
    match value? {
        Value::Number(n) => Some(n.clamp(0.0, 1.0)),
        Value::Length(n, Unit::Percent) => Some((n / 100.0).clamp(0.0, 1.0)),
        _ => None,
    }
}

// `background-image: url(fond.png)` (ou dans `background`), avec
// `background-size` (`cover`, `contain`, `auto`, px, %) et
// `background-position` (`center`, `top right`, `20% 80%`...).
fn background_image_of(computed: &ComputedStyle) -> Option<crate::ui::models::decoration::BackgroundImage> {
    use crate::ui::models::decoration::{BackgroundImage, BackgroundSize};
    fn url_in(v: &Value) -> Option<String> {
        match v {
            Value::Function(name, args) if name.eq_ignore_ascii_case("url") => args.iter().find_map(|a| match a {
                Value::Str(s) => Some(s.clone()),
                Value::Keyword(k) => Some(k.clone()),
                _ => None,
            }),
            Value::List(items) => items.iter().find_map(url_in),
            _ => None,
        }
    }
    let src = computed.background_image.as_ref().and_then(url_in).or_else(|| computed.background.as_ref().and_then(url_in))?;
    let words = |v: Option<&Value>| -> Vec<Value> {
        match v {
            Some(Value::List(items)) => items.clone(),
            Some(single) => vec![single.clone()],
            None => Vec::new(),
        }
    };
    let size_values = words(computed.background_size.as_ref());
    let size = match size_values.first().and_then(Value::as_keyword) {
        Some("cover") => BackgroundSize::Cover,
        Some("contain") => BackgroundSize::Contain,
        _ if size_values.is_empty() => BackgroundSize::Auto,
        _ => {
            let part = |v: Option<&Value>| match v {
                Some(Value::Length(n, Unit::Percent)) => (None, Some(*n)),
                Some(other) => (other.as_length_px(), None),
                None => (None, None),
            };
            let (wpx, wpc) = part(size_values.first());
            let (hpx, hpc) = part(size_values.get(1));
            if wpc.is_some() || hpc.is_some() {
                BackgroundSize::Percent(wpc, hpc)
            } else if wpx.is_some() || hpx.is_some() {
                BackgroundSize::Px(wpx, hpx)
            } else {
                BackgroundSize::Auto
            }
        }
    };
    // Mots-cles d'abord (chacun fixe son axe), puis `center` et les % sur
    // les axes qui restent, dans l'ordre (x puis y).
    let mut position: (Option<f32>, Option<f32>) = (None, None);
    let mut rest = Vec::new();
    for v in words(computed.background_position.as_ref()) {
        match v.as_keyword() {
            Some("left") => position.0 = Some(0.0),
            Some("right") => position.0 = Some(1.0),
            Some("top") => position.1 = Some(0.0),
            Some("bottom") => position.1 = Some(1.0),
            Some("center") => rest.push(0.5),
            _ => {
                if let Value::Length(n, Unit::Percent) = v {
                    rest.push(n / 100.0);
                }
            }
        }
    }
    let single_center = rest == [0.5] && position == (None, None);
    for value in rest {
        if position.0.is_none() {
            position.0 = Some(value);
        } else if position.1.is_none() {
            position.1 = Some(value);
        }
    }
    if single_center {
        position.1 = Some(0.5);
    }
    // Un seul axe donne : l'autre est centre (comme en CSS).
    let given = position.0.is_some() || position.1.is_some();
    let default = if given { 0.5 } else { 0.0 };
    let position = (position.0.unwrap_or(default), position.1.unwrap_or(default));
    Some(BackgroundImage { src, size, position })
}

// `border-style: dashed`, ou le style donne dans `border: 1px dashed #fff`.
fn border_style_of(computed: &ComputedStyle) -> Option<azure_engine::rendering::models::paint::BorderStyle> {
    use azure_engine::rendering::models::paint::BorderStyle;
    let keyword = |k: &str| -> Option<BorderStyle> {
        Some(match k {
            "solid" | "groove" | "ridge" | "inset" | "outset" => BorderStyle::Solid,
            "dashed" => BorderStyle::Dashed,
            "dotted" => BorderStyle::Dotted,
            "double" => BorderStyle::Double,
            "none" | "hidden" => BorderStyle::None,
            _ => return None,
        })
    };
    let from = |v: Option<&Value>| -> Option<BorderStyle> {
        match v? {
            Value::List(items) => items.iter().filter_map(Value::as_keyword).find_map(keyword),
            single => single.as_keyword().and_then(keyword),
        }
    };
    // `border: none` est deja traite (epaisseur 0) : ici, seulement un vrai style.
    from(computed.border_style.as_ref()).or_else(|| from(computed.border.as_ref()).filter(|s| *s != BorderStyle::None))
}

// `transition: background-color 0.2s ease 50ms`, `all .3s`, `none`. Une
// seule transition (la premiere) : elle vaut pour les couleurs de survol et
// de focus.
fn transition_of(value: Option<&Value>) -> Option<crate::ui::models::transition::Transition> {
    use crate::ui::models::transition::{Easing, Transition};
    let value = value?;
    if value.as_keyword() == Some("none") {
        return None;
    }
    let first = match value {
        Value::List(items) if items.iter().any(|v| matches!(v, Value::List(_))) => items.first()?.clone(),
        other => other.clone(),
    };
    let items: Vec<Value> = match first {
        Value::List(items) => items,
        single => vec![single],
    };
    let seconds = |v: &Value| match v {
        Value::Length(n, Unit::Unknown(u)) if u == "s" => Some(*n),
        Value::Length(n, Unit::Unknown(u)) if u == "ms" => Some(n / 1000.0),
        _ => None,
    };
    let times: Vec<f32> = items.iter().filter_map(seconds).collect();
    let easing = items
        .iter()
        .find_map(|v| match v {
            Value::Keyword(k) => Easing::from_css(k),
            Value::Function(name, args) if name == "cubic-bezier" && args.len() == 4 => {
                let n: Vec<f32> = args.iter().filter_map(Value::as_bare_number).collect();
                (n.len() == 4).then(|| Easing::Bezier(n[0], n[1], n[2], n[3]))
            }
            Value::Function(name, args) if name == "steps" => args.first().and_then(Value::as_bare_number).map(|n| Easing::Steps(n.max(1.0) as u32)),
            _ => None,
        })
        .unwrap_or(Easing::Bezier(0.25, 0.1, 0.25, 1.0));
    let duration = *times.first()?;
    Some(Transition { duration: duration.max(0.0), delay: times.get(1).copied().unwrap_or(0.0).max(0.0), easing })
}


// Taille de police en px : `rem`/`em` valent 16px (pas d'heritage de la
// taille du parent pour `em` a ce stade), `%` aussi par rapport a 16px.
fn font_size_of(value: Option<&Value>) -> Option<f32> {
    match value? {
        Value::Length(n, Unit::Px) | Value::Number(n) => Some(*n),
        Value::Length(n, Unit::Rem) | Value::Length(n, Unit::Em) => Some(n * 16.0),
        Value::Length(n, Unit::Percent) => Some(n * 16.0 / 100.0),
        _ => None,
    }
}

/// Les proprietes CSS "web" (unites reelles, cotes, texte) - voir
/// `WebStyle`. `em` se rapporte a la taille de police de l'element lui-meme
/// quand une regle la fixe, 16px sinon.
fn web_of(computed: &ComputedStyle) -> WebStyle {
    let em = font_size_of(computed.font_size.as_ref()).unwrap_or(16.0);
    let len = |v: Option<&Value>| v.and_then(|v| length_of(v, em));
    let sides = |short: Option<&Value>, longs: [Option<&Value>; 4]| {
        let mut out = expand_sides(short, em);
        for (slot, long) in out.iter_mut().zip(longs) {
            if let Some(l) = long.and_then(|v| length_of(v, em)) {
                *slot = Some(l);
            }
        }
        out
    };
    let gap = |v: Option<&Value>, index: usize| match v? {
        Value::List(items) => items.get(index).or(items.first()).and_then(|v| length_of(v, em)),
        single => length_of(single, em),
    };
    WebStyle {
        display: computed.display.as_ref().and_then(Value::as_keyword).and_then(|k| match k {
            "block" | "inline-block" | "inline" => Some(CssDisplay::Block),
            "flex" | "inline-flex" => Some(CssDisplay::Flex),
            "grid" | "inline-grid" => Some(CssDisplay::Grid),
            "none" => Some(CssDisplay::None),
            _ => None,
        }),
        width: len(computed.width.as_ref()),
        height: len(computed.height.as_ref()),
        min_width: len(computed.min_width.as_ref()),
        min_height: len(computed.min_height.as_ref()),
        max_width: len(computed.max_width.as_ref()),
        max_height: len(computed.max_height.as_ref()),
        margin: sides(computed.margin.as_ref(), [computed.margin_top.as_ref(), computed.margin_right.as_ref(), computed.margin_bottom.as_ref(), computed.margin_left.as_ref()]),
        padding: sides(computed.padding.as_ref(), [computed.padding_top.as_ref(), computed.padding_right.as_ref(), computed.padding_bottom.as_ref(), computed.padding_left.as_ref()]),
        border_box: computed.box_sizing.as_ref().and_then(Value::as_keyword).and_then(|k| match k {
            "border-box" => Some(true),
            "content-box" => Some(false),
            _ => None,
        }),
        row_gap: len(computed.row_gap.as_ref()).or_else(|| gap(computed.gap.as_ref(), 0)),
        column_gap: len(computed.column_gap.as_ref()).or_else(|| gap(computed.gap.as_ref(), 1)),
        flex_basis: len(computed.flex_basis.as_ref()),
        align_self: align_items_of(computed.align_self.as_ref()),
        grid_template_columns: css_tracks_of(computed.grid_template_columns.as_ref(), em),
        grid_template_rows: css_tracks_of(computed.grid_template_rows.as_ref(), em),
        fixed: computed.position.as_ref().and_then(Value::as_keyword).map(|k| k == "fixed"),
        position: computed.position.as_ref().and_then(Value::as_keyword).and_then(|k| {
            use crate::layout::models::css_box::CssPosition;
            match k {
                "relative" | "sticky" => Some(CssPosition::Relative),
                "absolute" => Some(CssPosition::Absolute),
                "static" | "fixed" => Some(CssPosition::Static),
                _ => None,
            }
        }),
        inset: [len(computed.top.as_ref()), len(computed.right.as_ref()), len(computed.bottom.as_ref()), len(computed.left.as_ref())],
        z_index: match computed.z_index.as_ref() {
            Some(Value::Number(n)) => Some(*n as i32),
            _ => None,
        },
        font_family: font_family_of(computed.font_family.as_ref()),
        line_height: computed.line_height.as_ref().and_then(|v| match v {
            Value::Number(n) => Some(LineHeight::Multiplier(*n)),
            Value::Keyword(k) if k == "normal" => Some(LineHeight::Normal),
            Value::Length(n, Unit::Percent) => Some(LineHeight::Multiplier(n / 100.0)),
            other => length_of(other, em).and_then(|l| match l {
                Length::Px(px) => Some(LineHeight::Px(px)),
                _ => None,
            }),
        }),
        text_align: computed.text_align.as_ref().and_then(Value::as_keyword).and_then(|k| match k {
            "left" | "start" | "justify" => Some(TextAlign::Left),
            "center" => Some(TextAlign::Center),
            "right" | "end" => Some(TextAlign::Right),
            _ => None,
        }),
        white_space: computed.white_space.as_ref().and_then(Value::as_keyword).and_then(|k| match k {
            "normal" | "pre-line" => Some(WhiteSpace::Normal),
            "nowrap" => Some(WhiteSpace::Nowrap),
            "pre" | "pre-wrap" => Some(WhiteSpace::Pre),
            _ => None,
        }),
        italic: computed.font_style.as_ref().and_then(Value::as_keyword).and_then(|k| match k {
            "italic" | "oblique" => Some(true),
            "normal" => Some(false),
            _ => None,
        }),
        letter_spacing: computed.letter_spacing.as_ref().and_then(|v| match v {
            Value::Keyword(k) if k == "normal" => Some(0.0),
            other => match length_of(other, em) {
                Some(Length::Px(px)) => Some(px),
                _ => None,
            },
        }),
        text_decoration: computed.text_decoration.as_ref().map(|v| {
            let words: Vec<&str> = match v {
                Value::List(items) => items.iter().filter_map(Value::as_keyword).collect(),
                single => single.as_keyword().into_iter().collect(),
            };
            crate::style::models::web_style::TextDecoration {
                underline: words.contains(&"underline"),
                line_through: words.contains(&"line-through"),
                overline: words.contains(&"overline"),
            }
        }),
        visible: computed.visibility.as_ref().and_then(Value::as_keyword).and_then(|k| match k {
            "hidden" | "collapse" => Some(false),
            "visible" => Some(true),
            _ => None,
        }),
        cursor: computed.cursor.as_ref().and_then(|v| match v {
            // `cursor: url(...), pointer` : la forme de repli.
            Value::List(items) => items.iter().filter_map(Value::as_keyword).find_map(crate::cursor::models::cursor_kind::CursorKind::from_css),
            single => single.as_keyword().and_then(crate::cursor::models::cursor_kind::CursorKind::from_css),
        }),
        // `scrollbar-color: #555 #111` : la poignee (`auto` : celle par defaut).
        scrollbar_color: computed.scrollbar_color.as_ref().and_then(|v| match v {
            Value::List(items) => items.first().and_then(Value::as_color),
            single => single.as_color(),
        }),
    }
}

// Une longueur CSS : `auto`, px, %, em/rem (convertis en px), ou un `0` nu.
fn length_of(value: &Value, em: f32) -> Option<Length> {
    match value {
        Value::Keyword(k) if k == "auto" || k == "none" => Some(Length::Auto),
        Value::Length(n, Unit::Px) => Some(Length::Px(*n)),
        Value::Length(n, Unit::Percent) => Some(Length::Percent(*n)),
        Value::Length(n, Unit::Em) => Some(Length::Px(n * em)),
        Value::Length(n, Unit::Rem) => Some(Length::Px(n * 16.0)),
        Value::Number(n) => Some(Length::Px(*n)),
        _ => None,
    }
}

// `margin: 8px` / `8px 16px` / `8px 16px 4px` / `8px 16px 4px 2px` : les
// quatre cotes dans l'ordre CSS (haut, droite, bas, gauche).
fn expand_sides(value: Option<&Value>, em: f32) -> [Option<Length>; 4] {
    let Some(value) = value else { return [None; 4] };
    let items: Vec<Length> = match value {
        Value::List(items) => items.iter().filter_map(|v| length_of(v, em)).collect(),
        single => length_of(single, em).into_iter().collect(),
    };
    match items.as_slice() {
        [a] => [Some(*a); 4],
        [v, h] => [Some(*v), Some(*h), Some(*v), Some(*h)],
        [t, h, b] => [Some(*t), Some(*h), Some(*b), Some(*h)],
        [t, r, b, l, ..] => [Some(*t), Some(*r), Some(*b), Some(*l)],
        [] => [None; 4],
    }
}

// `grid-template-columns: 240px 1fr`, `repeat(3, 1fr)`, `auto 1fr`...
fn css_tracks_of(value: Option<&Value>, em: f32) -> Option<Vec<CssTrack>> {
    fn push(value: &Value, em: f32, out: &mut Vec<CssTrack>) {
        match value {
            Value::Length(n, Unit::Fr) => out.push(CssTrack::Fr(*n)),
            Value::Function(name, args) if name == "repeat" => {
                let count = args.first().and_then(Value::as_bare_number).unwrap_or(1.0).max(1.0) as usize;
                for _ in 0..count {
                    for arg in &args[1..] {
                        push(arg, em, out);
                    }
                }
            }
            Value::List(items) => items.iter().for_each(|v| push(v, em, out)),
            other => match length_of(other, em) {
                Some(Length::Px(px)) => out.push(CssTrack::Px(px)),
                Some(Length::Percent(p)) => out.push(CssTrack::Percent(p)),
                Some(Length::Auto) => out.push(CssTrack::Auto),
                None => {}
            },
        }
    }
    let mut out = Vec::new();
    push(value?, em, &mut out);
    (!out.is_empty()).then_some(out)
}

// `font-family: "Sora", monospace` : la premiere famille, en minuscules.
fn font_family_of(value: Option<&Value>) -> Option<String> {
    let first = match value? {
        Value::List(items) => items.first()?.clone(),
        single => single.clone(),
    };
    match first {
        Value::Keyword(k) => Some(k),
        Value::Str(s) => Some(s.to_ascii_lowercase()),
        Value::List(items) => items.iter().find_map(|v| v.as_keyword().map(str::to_string)),
        _ => None,
    }
}

fn overflow_of(value: Option<&Value>) -> Option<Overflow> {
    value.and_then(Value::as_keyword).and_then(|k| match k {
        "visible" => Some(Overflow::Visible),
        "hidden" | "clip" => Some(Overflow::Hidden),
        "auto" => Some(Overflow::Auto),
        "scroll" => Some(Overflow::Scroll),
        _ => None,
    })
}

fn flex_direction_of(value: Option<&Value>) -> Option<FlexDirection> {
    value.and_then(Value::as_keyword).and_then(|k| match k {
        "row" => Some(FlexDirection::Row),
        "column" => Some(FlexDirection::Column),
        _ => None,
    })
}

fn flex_wrap_of(value: Option<&Value>) -> Option<bool> {
    value.and_then(Value::as_keyword).and_then(|k| match k {
        "wrap" => Some(true),
        "nowrap" => Some(false),
        _ => None,
    })
}

fn justify_content_of(value: Option<&Value>) -> Option<JustifyContent> {
    value.and_then(Value::as_keyword).and_then(|k| match k {
        "flex-start" | "start" => Some(JustifyContent::Start),
        "flex-end" | "end" => Some(JustifyContent::End),
        "center" => Some(JustifyContent::Center),
        "space-between" => Some(JustifyContent::SpaceBetween),
        "space-around" => Some(JustifyContent::SpaceAround),
        "space-evenly" => Some(JustifyContent::SpaceEvenly),
        _ => None,
    })
}

fn align_items_of(value: Option<&Value>) -> Option<AlignItems> {
    value.and_then(Value::as_keyword).and_then(|k| match k {
        "flex-start" | "start" => Some(AlignItems::Start),
        "flex-end" | "end" => Some(AlignItems::End),
        "center" => Some(AlignItems::Center),
        "stretch" => Some(AlignItems::Stretch),
        _ => None,
    })
}

// `Percent`/nombre nu -> `Track::Percent` (un nombre sans unite est une
// convention deja partagee par tout le reste de ce moteur de layout - voir
// `Value::as_bare_number`), `fr` -> `Track::Fr`. Tout le reste (mots-cles,
// couleurs...) est ignore plutot que de faire echouer toute la liste.
fn track_of(value: &Value) -> Option<Track> {
    match value {
        Value::Length(n, Unit::Fr) => Some(Track::Fr(*n)),
        Value::Length(n, Unit::Percent) => Some(Track::Percent(*n)),
        Value::Number(n) => Some(Track::Percent(*n)),
        _ => None,
    }
}

fn track_list_of(value: Option<&Value>) -> Option<Vec<Track>> {
    match value? {
        Value::List(items) => Some(items.iter().filter_map(track_of).collect()),
        single => track_of(single).map(|t| vec![t]),
    }
}
