use crate::compiler::rsh::mangers::parser::AstNode;
use crate::compiler::rsc::models::rule::RscStylesheet;
use crate::compiler::rsc::models::selector::PseudoState;
use crate::compiler::rsc::models::value::Value;
use crate::compiler::rsc::services::link::{self, ElementInfo};
use crate::compiler::services::interpreter::skip_blank_text;
use crate::layout::models::layout_props::{AlignItems, DisplayMode, FlexDirection, JustifyContent, Track};
use crate::style::models::style::Style;
use crate::ui::models::decoration::Decoration;
use crate::compiler::services::web;
use crate::layout::models::css_box::{CssBox, CssTrack, Length, Sides};
use crate::style::models::web_style::{TextAlign, WhiteSpace};
use azure_engine::rendering::models::paint::{ColorStop, Fill};
use crate::style::models::stylesheet::Stylesheet;
use azure_engine::rendering::models::color::Color;

/// D'ou vient le style applique a chaque element : l'ancien systeme
/// `.style` (une table de classes a plat, sans notion de balise ni d'id),
/// ou le nouveau moteur de selecteurs rsC (`compiler::rsc::services::link`), qui
/// matche reellement par balise/classe/id/ancetres avec cascade CSS.
/// Public : construit aussi bien par `generate`/`generate_with_rsc` que par
/// `services::interpreter::build_ui`, ou directement par un appelant
/// externe (voir `examples/run_window_demo.rs`).
pub enum StyleSource<'a> {
    Legacy(&'a Stylesheet),
    Rsc(&'a RscStylesheet),
}

// Styles de base par type d'element : ce que chaque balise .rsh recoit tant
// qu'aucun composant de style (classe) ne vient le surcharger. Une classe
// resolue via la Stylesheet est fusionnee par-dessus (voir `Style::merge`).
pub fn base_layout() -> Style {
    Style {
        x: Some(0.0),
        y: Some(0.0),
        width: Some(100.0),
        height: Some(100.0),
        margin: Some(0.0),
        padding: Some(0.0),
        ..Style::default()
    }
}

// Transparent par defaut, comme en CSS : un conteneur sans `background`
// laisse voir celui de son parent (degrade compris). La fenetre peint deja
// son propre fond derriere l'arbre entier.
pub fn base_container_style() -> Style {
    Style { background: Some(Color::new(0, 0, 0, 0)), ..base_layout() }
}

pub fn base_button_style() -> Style {
    Style { background: Some(Color::new(46, 43, 39, 255)), ..base_layout() }
}

pub fn base_media_style() -> Style {
    base_layout()
}

pub fn base_text_style(font_size: f32, weight: f32) -> Style {
    Style {
        color: Some(Color::new(255, 255, 255, 255)),
        font_size: Some(font_size),
        font_weight: Some(weight),
        ..base_layout()
    }
}

/// Style de depart d'un texte : avec rsC, celui de la balise (les titres
/// fixent leur taille, `text` herite tout de son parent) ; avec l'ancien
/// systeme `.style`, des valeurs fixes (pas d'heritage).
pub fn label_base_style(tag: &str, font_size: f32, weight: f32, source: &StyleSource) -> Style {
    match source {
        StyleSource::Rsc(_) => base_style_for(tag),
        StyleSource::Legacy(_) => base_text_style(font_size, weight),
    }
}

// Un titre fixe sa taille et sa graisse (comme `h1` dans un navigateur),
// mais herite sa couleur.
fn base_title_style(font_size: f32, weight: f32) -> Style {
    Style { font_size: Some(font_size), font_weight: Some(weight), ..base_layout() }
}

pub fn base_textarea_style() -> Style {
    Style {
        background: Some(Color::new(22, 21, 20, 255)),
        color: Some(Color::new(255, 255, 255, 255)),
        ..base_layout()
    }
}

// Applique le style de l'element par-dessus son style de base, quelle que
// soit la source : l'ancien systeme ne regarde que `class`, le nouveau
// moteur rsC matche par balise/classe/id, l'historique d'ancetres ET les
// freres deja emis au meme niveau (`preceding`, voir
// `link::resolve_element` - requis par les combinateurs `+`/`~`).
pub fn resolve(tag: &str, class: &str, id: &str, base: Style, source: &StyleSource, ancestors: &[ElementInfo], preceding: &[ElementInfo]) -> Style {
    match source {
        StyleSource::Legacy(stylesheet) => stylesheet.resolve(class, &base),
        StyleSource::Rsc(stylesheet) => resolve_rsc(tag, class, id, base, stylesheet, ancestors, preceding),
    }
}

// Cascade rsC d'un element, avec l'heritage CSS : une propriete de texte
// (couleur, police, taille, graisse, interligne, alignement, retour a la
// ligne) qu'aucune regle ne fixe pour cet element prend la valeur de son
// parent. Ordre de priorite, comme en CSS : regle de l'auteur > valeur par
// defaut de la balise (taille des titres...) > valeur heritee.
fn resolve_rsc(tag: &str, class: &str, id: &str, base: Style, stylesheet: &RscStylesheet, ancestors: &[ElementInfo], preceding: &[ElementInfo]) -> Style {
    let (inherited, author) = cascade(tag, class, id, stylesheet, ancestors, preceding);
    inherited.merge(&base).merge(&author)
}

// Ce que la feuille donne a un element (herite du parent, regles de
// l'auteur), memorise pendant la construction d'un ecran : les lignes d'une
// liste ont toutes la meme lignee, et l'heritage recalcule sinon chaque
// ancetre pour chaque element (des centaines de regles a chaque fois).
thread_local! {
    static CASCADE: std::cell::RefCell<std::collections::HashMap<String, (Style, Style)>> = Default::default();
}

/// Oublie les styles memorises (debut de la construction d'un ecran).
pub fn clear_cascade_cache() {
    CASCADE.with(|c| c.borrow_mut().clear());
    link::clear_resolve_cache();
}

fn cascade(tag: &str, class: &str, id: &str, stylesheet: &RscStylesheet, ancestors: &[ElementInfo], preceding: &[ElementInfo]) -> (Style, Style) {
    let compute = || {
        let element = ElementInfo::new(tag, class, id);
        let computed = link::resolve_element(stylesheet, ancestors, preceding, &element, PseudoState::default());
        (inherited_from_parent(stylesheet, ancestors), link::to_legacy_style(&computed))
    };
    if !preceding.is_empty() && link::has_sibling_rules(stylesheet) {
        return compute();
    }
    use std::fmt::Write;
    let mut key = format!("{:x}|", stylesheet as *const RscStylesheet as usize);
    for a in ancestors {
        let _ = write!(key, "{}.{}#{}>", a.tag, a.classes.join("."), a.id);
    }
    let _ = write!(key, "{tag}.{}#{id}", class.split_whitespace().collect::<Vec<_>>().join("."));
    if let Some(hit) = CASCADE.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let value = compute();
    CASCADE.with(|c| c.borrow_mut().insert(key, value.clone()));
    value
}

// Les proprietes heritables du parent (le dernier de `ancestors`), lui-meme
// resolu avec son propre heritage. Les freres precedents d'un ancetre ne sont
// pas connus ici : une regle `+`/`~` qui viserait cet ancetre n'est pas prise
// en compte pour l'heritage.
fn inherited_from_parent(stylesheet: &RscStylesheet, ancestors: &[ElementInfo]) -> Style {
    let Some((parent, rest)) = ancestors.split_last() else { return Style::default() };
    let class = parent.classes.join(" ");
    let parent_style = resolve_rsc(&parent.tag, &class, &parent.id, base_style_for(&parent.tag), stylesheet, rest, &[]);
    Style {
        color: parent_style.color,
        font_size: parent_style.font_size,
        font_weight: parent_style.font_weight,
        web: parent_style.web.inherited(),
        ..Style::default()
    }
}

/// Style par defaut d'une balise (l'equivalent de la feuille du navigateur).
pub fn base_style_for(tag: &str) -> Style {
    match tag {
        "container" => base_container_style(),
        "button" => base_button_style(),
        "image" | "video" => base_media_style(),
        "textarea" => base_textarea_style(),
        "title" => base_title_style(24.0, 700.0),
        "title1" => base_title_style(32.0, 700.0),
        "title2" => base_title_style(24.0, 600.0),
        "title3" => base_title_style(18.0, 600.0),
        _ => base_layout(),
    }
}

/// Resout `background-color`/`background` d'un element SOUS un pseudo-etat
/// donne (`:hover`/`:focus`) plutot que son etat de base - `None` si la
/// source ne supporte pas les pseudo-classes (l'ancien systeme `.style`,
/// une simple table de classes a plat sans notion de selecteur CSS) ou si
/// aucune regle ne matche sous cet etat. Utilise pour donner a
/// `Button`/`TextArea` une couleur de survol/focus reellement pilotee par
/// une feuille de style rsC (`button:hover { background-color: ...; }`),
/// EN PLUS de l'indicateur visuel generique deja dessine par
/// `ui::services::draw_ui` (independant de toute feuille de style, voir
/// `HOVER_BUTTON_COLOR`/`FOCUS_COLOR`).
pub fn resolve_pseudo_background(tag: &str, class: &str, id: &str, source: &StyleSource, ancestors: &[ElementInfo], preceding: &[ElementInfo], pseudo: PseudoState) -> Option<Color> {
    match source {
        StyleSource::Legacy(_) => None,
        StyleSource::Rsc(stylesheet) => {
            let element = ElementInfo::new(tag, class, id);
            let computed = link::resolve_element(stylesheet, ancestors, preceding, &element, pseudo);
            computed
                .background_color
                .as_ref()
                .or(computed.background.as_ref())
                .and_then(Value::as_color)
        }
    }
}

/// Couleur du TEXTE sous un pseudo-etat (`button:hover { color: ... }`).
pub fn resolve_pseudo_color(tag: &str, class: &str, id: &str, source: &StyleSource, ancestors: &[ElementInfo], preceding: &[ElementInfo], pseudo: PseudoState) -> Option<Color> {
    match source {
        StyleSource::Legacy(_) => None,
        StyleSource::Rsc(stylesheet) => {
            let element = ElementInfo::new(tag, class, id);
            link::resolve_element(stylesheet, ancestors, preceding, &element, pseudo).color.as_ref().and_then(Value::as_color)
        }
    }
}

/// Comme `resolve_pseudo_background`, pour un degrade (`background:
/// linear-gradient(...)` sous `:hover`/`:focus`) - `None` s'il n'y en a pas
/// ou s'il est identique a celui de l'etat de base.
#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
pub fn resolve_pseudo_fill(tag: &str, class: &str, id: &str, base: &Style, source: &StyleSource, ancestors: &[ElementInfo], preceding: &[ElementInfo], pseudo: PseudoState) -> Option<Fill> {
    match source {
        StyleSource::Legacy(_) => None,
        StyleSource::Rsc(stylesheet) => {
            let element = ElementInfo::new(tag, class, id);
            let computed = link::resolve_element(stylesheet, ancestors, preceding, &element, pseudo);
            link::to_legacy_style(&computed).fill.filter(|f| Some(f) != base.fill.as_ref())
        }
    }
}

// Chemin complet (pas juste le nom du type) : le code genere vit dans un
// AUTRE crate (une app, voir `examples/run_window_demo.rs`) qui depend
// d'azure_foundation comme n'importe quelle dependance externe - il ne
// beneficie d'aucun `use` local vers ces enums, contrairement a ce fichier.
const LAYOUT_PROPS_PATH: &str = "azure_foundation::layout::models::layout_props";

fn display_mode_literal(v: DisplayMode) -> String {
    let name = match v {
        DisplayMode::Block => "Block",
        DisplayMode::Flex => "Flex",
        DisplayMode::Grid => "Grid",
    };
    format!("{LAYOUT_PROPS_PATH}::DisplayMode::{name}")
}

fn flex_direction_literal(v: FlexDirection) -> String {
    let name = match v {
        FlexDirection::Row => "Row",
        FlexDirection::Column => "Column",
    };
    format!("{LAYOUT_PROPS_PATH}::FlexDirection::{name}")
}

fn justify_content_literal(v: JustifyContent) -> String {
    let name = match v {
        JustifyContent::Start => "Start",
        JustifyContent::End => "End",
        JustifyContent::Center => "Center",
        JustifyContent::SpaceBetween => "SpaceBetween",
        JustifyContent::SpaceAround => "SpaceAround",
        JustifyContent::SpaceEvenly => "SpaceEvenly",
    };
    format!("{LAYOUT_PROPS_PATH}::JustifyContent::{name}")
}

fn align_items_literal(v: AlignItems) -> String {
    let name = match v {
        AlignItems::Start => "Start",
        AlignItems::End => "End",
        AlignItems::Center => "Center",
        AlignItems::Stretch => "Stretch",
    };
    format!("{LAYOUT_PROPS_PATH}::AlignItems::{name}")
}

fn track_list_literal(tracks: &[Track]) -> String {
    let items: Vec<String> = tracks
        .iter()
        .map(|t| match t {
            Track::Percent(p) => format!("{LAYOUT_PROPS_PATH}::Track::Percent({p:.1})"),
            Track::Fr(f) => format!("{LAYOUT_PROPS_PATH}::Track::Fr({f:.1})"),
        })
        .collect();
    format!("vec![{}]", items.join(", "))
}

/// Litteral Rust d'un `LayoutProps` complet. Reste un simple appel
/// `LayoutProps::new(...)` tant qu'aucune regle rsC ne definit de propriete
/// flex/grid (le cas courant - zero churn sur le code genere existant) ;
/// sinon une expression bloc qui construit puis ne modifie QUE les champs
/// effectivement definis, laissant les autres a leur defaut (`DisplayMode::Block`
/// et consorts, voir `LayoutProps::new`).
fn layout_literal(style: &Style) -> String {
    let base = format!(
        "LayoutProps::new({:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1})",
        style.x.unwrap_or(0.0),
        style.y.unwrap_or(0.0),
        style.width.unwrap_or(100.0),
        style.height.unwrap_or(100.0),
        style.margin.unwrap_or(0.0),
        style.padding.unwrap_or(0.0),
    );

    let mut extra: Vec<String> = Vec::new();
    if let Some(v) = style.display {
        extra.push(format!("layout.display = {};", display_mode_literal(v)));
    }
    if let Some(v) = style.flex_direction {
        extra.push(format!("layout.flex_direction = {};", flex_direction_literal(v)));
    }
    if let Some(v) = style.flex_wrap {
        extra.push(format!("layout.flex_wrap = {v};"));
    }
    if let Some(v) = style.justify_content {
        extra.push(format!("layout.justify_content = {};", justify_content_literal(v)));
    }
    if let Some(v) = style.align_items {
        extra.push(format!("layout.align_items = {};", align_items_literal(v)));
    }
    if let Some(v) = style.gap {
        extra.push(format!("layout.gap = {v:.1};"));
    }
    if let Some(v) = style.flex_grow {
        extra.push(format!("layout.flex_grow = {v:.1};"));
    }
    if let Some(v) = style.flex_shrink {
        extra.push(format!("layout.flex_shrink = {v:.1};"));
    }
    if let Some(v) = style.flex_basis {
        extra.push(format!("layout.flex_basis = Some({v:.1});"));
    }
    if let Some(ref tracks) = style.grid_template_columns {
        extra.push(format!("layout.grid_template_columns = {};", track_list_literal(tracks)));
    }
    if let Some(ref tracks) = style.grid_template_rows {
        extra.push(format!("layout.grid_template_rows = {};", track_list_literal(tracks)));
    }
    if let Some(v) = style.grid_column {
        extra.push(format!("layout.grid_column = Some({v});"));
    }
    if let Some(v) = style.grid_column_span {
        extra.push(format!("layout.grid_column_span = {v};"));
    }
    if let Some(v) = style.grid_row {
        extra.push(format!("layout.grid_row = Some({v});"));
    }
    if let Some(v) = style.grid_row_span {
        extra.push(format!("layout.grid_row_span = {v};"));
    }
    if let Some(v) = style.overflow {
        extra.push(format!("layout.overflow = {LAYOUT_PROPS_PATH}::Overflow::{v:?};"));
    }
    if let Some(v) = style.overflow_x {
        extra.push(format!("layout.overflow_x = {LAYOUT_PROPS_PATH}::Overflow::{v:?};"));
    }

    if extra.is_empty() {
        base
    } else {
        format!("{{ let mut layout = {base}; {} layout }}", extra.join(" "))
    }
}

// `Decoration` (degrade, coins arrondis, bordure, ombre, opacite) du nœud
// qui vient d'etre pousse dans `var` - rien n'est emis si elle est vide, pour
// ne pas alourdir le code genere. `label` : un texte n'a pas de fond par
// defaut, sa couleur `background` passe par la decoration (voir
// `interpreter::build_label_text`, meme regle).
/// La `Decoration` d'un element, partagee par l'interpreteur et le codegen :
/// celle de son style, plus - pour un texte - sa couleur de fond (un texte
/// n'a pas de fond par defaut, elle ne passe que par la decoration), et -
/// pour un bouton ou une zone de saisie - ses degrades sous `:hover`/`:focus`.
pub fn decoration_for(tag: &str, class: &str, id: &str, style: &Style, source: &StyleSource, ancestors: &[ElementInfo], preceding: &[ElementInfo]) -> Decoration {
    let mut decoration = Decoration::from_style(style);
    decoration.anchor = id.to_string();
    // Pour l'inspecteur (F12) : qui est cet element, quelles regles le visent.
    let rules = match source {
        StyleSource::Rsc(sheet) => link::matched_rules_cached(sheet, ancestors, preceding, &ElementInfo::new(tag, class, id)),
        StyleSource::Legacy(_) => Default::default(),
    };
    decoration.inspect = Some(std::sync::Arc::new(crate::inspector::NodeInfo { tag: tag.to_string(), class: class.to_string(), id: id.to_string(), rules }));
    match tag {
        "button" | "textarea" => {
            decoration.hover_fill = resolve_pseudo_fill(tag, class, id, style, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: false });
            if tag == "button" {
                decoration.active_fill = resolve_pseudo_fill(tag, class, id, style, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: true });
            }
            if tag == "textarea" {
                decoration.focus_fill = resolve_pseudo_fill(tag, class, id, style, source, ancestors, preceding, PseudoState { hover: false, focus: true, active: false });
            }
        }
        "container" | "image" | "video" => {}
        _ => {
            if decoration.fill.is_none() {
                decoration.fill = style.background.map(Fill::Solid);
            }
        }
    }
    // Survol d'un ancetre : `.bloc:hover .poignee { opacity: 1 }`.
    if let StyleSource::Rsc(sheet) = source {
        let element = ElementInfo::new(tag, class, id);
        decoration.hover_group = link::is_hover_group(sheet, &element);
        let hover = PseudoState { hover: true, focus: false, active: false };
        let computed = link::resolve_element_in(sheet, ancestors, preceding, &element, PseudoState::default(), hover);
        decoration.group_hover_opacity = link::to_legacy_style(&computed).opacity.map(|o| o.clamp(0.0, 1.0)).filter(|o| *o != decoration.opacity);
    }
    decoration
}

fn emit_decoration(kind: &str, mut decoration: Decoration, out: &mut String, level: usize, var: &str) {
    // L'inspecteur ne passe pas dans le code genere.
    decoration.inspect = None;
    if decoration == Decoration::default() {
        return;
    }
    let paint = "azure_engine::rendering::models::paint";
    let stops = |stops: &[ColorStop]| {
        let items: Vec<String> = stops
            .iter()
            .map(|s| format!("{paint}::ColorStop {{ color: {}, position: {:.3} }}", color_literal(Some(s.color)), s.position))
            .collect();
        format!("vec![{}]", items.join(", "))
    };
    let fill_literal = |fill: &Option<Fill>| match fill {
        None => "None".to_string(),
        Some(Fill::Solid(c)) => format!("Some({paint}::Fill::Solid({}))", color_literal(Some(*c))),
        Some(Fill::Linear { angle, stops: s }) => format!("Some({paint}::Fill::Linear {{ angle: {angle:.2}, stops: {} }})", stops(s)),
        Some(Fill::Radial { stops: s }) => format!("Some({paint}::Fill::Radial {{ stops: {} }})", stops(s)),
    };
    let fill = fill_literal(&decoration.fill);
    let hover_fill = fill_literal(&decoration.hover_fill);
    let active_fill = fill_literal(&decoration.active_fill);
    let focus_fill = fill_literal(&decoration.focus_fill);
    let shadow = match decoration.shadow {
        None => "None".to_string(),
        Some(sh) => format!(
            "Some({paint}::Shadow {{ offset_x: {:.1}, offset_y: {:.1}, blur: {:.1}, spread: {:.1}, color: {}, inset: {} }})",
            sh.offset_x, sh.offset_y, sh.blur, sh.spread, color_literal(Some(sh.color)), sh.inset
        ),
    };
    let background_image = match &decoration.background_image {
        Some(bg) => {
            let path = "azure_foundation::ui::models::decoration::BackgroundSize";
            let opt = |v: Option<f32>| v.map(|v| format!("Some({v:.2})")).unwrap_or_else(|| "None".to_string());
            let size = match bg.size {
                crate::ui::models::decoration::BackgroundSize::Auto => format!("{path}::Auto"),
                crate::ui::models::decoration::BackgroundSize::Cover => format!("{path}::Cover"),
                crate::ui::models::decoration::BackgroundSize::Contain => format!("{path}::Contain"),
                crate::ui::models::decoration::BackgroundSize::Px(w, h) => format!("{path}::Px({}, {})", opt(w), opt(h)),
                crate::ui::models::decoration::BackgroundSize::Percent(w, h) => format!("{path}::Percent({}, {})", opt(w), opt(h)),
            };
            format!(
                "Some(azure_foundation::ui::models::decoration::BackgroundImage {{ src: {:?}.to_string(), size: {size}, position: ({:.3}, {:.3}) }})",
                bg.src, bg.position.0, bg.position.1
            )
        }
        None => "None".to_string(),
    };
    let cursor = match decoration.cursor {
        Some(k) => format!("Some(azure_foundation::cursor::models::cursor_kind::CursorKind::{k:?})"),
        None => "None".to_string(),
    };
    let scrollbar_color = color_literal_opt(decoration.scrollbar_color);
    let transition = match decoration.transition {
        Some(t) => {
            let easing = match t.easing {
                crate::ui::models::transition::Easing::Linear => "Linear".to_string(),
                crate::ui::models::transition::Easing::Bezier(a, b, c, d) => format!("Bezier({a:.3}, {b:.3}, {c:.3}, {d:.3})"),
                crate::ui::models::transition::Easing::Steps(n) => format!("Steps({n})"),
            };
            format!(
                "Some(azure_foundation::ui::models::transition::Transition {{ duration: {:.3}, delay: {:.3}, easing: azure_foundation::ui::models::transition::Easing::{easing} }})",
                t.duration, t.delay
            )
        }
        None => "None".to_string(),
    };
    out.push_str(&format!(
        "{}if let Some(UiNode::{kind}(node)) = {var}.last_mut() {{ node.decoration = azure_foundation::ui::models::decoration::Decoration {{ fill: {fill}, radius: {:.1}, border: {}, border_color: {}, shadow: {shadow}, opacity: {:.3}, hover_fill: {hover_fill}, active_fill: {active_fill}, focus_fill: {focus_fill}, tooltip: {:?}.to_string(), background_image: {background_image}, anchor: {:?}.to_string(), visible: {}, cursor: {cursor}, border_style: {paint}::BorderStyle::{:?}, transition: {transition}, scrollbar_color: {scrollbar_color}, hover_group: {}, group_hover_opacity: {:?}, ..Default::default() }}; }}\n",
        indent(level),
        decoration.radius,
        border_literal(decoration.border),
        color_literal(Some(decoration.border_color)),
        decoration.opacity,
        decoration.tooltip,
        decoration.anchor,
        decoration.visible,
        decoration.border_style,
        decoration.hover_group,
        decoration.group_hover_opacity,
    ));
}

fn border_literal(b: azure_engine::rendering::models::paint::BorderWidths) -> String {
    format!(
        "azure_engine::rendering::models::paint::BorderWidths {{ top: {:.2}, right: {:.2}, bottom: {:.2}, left: {:.2} }}",
        b.top, b.right, b.bottom, b.left
    )
}

// Texte d'un widget web a emettre avec sa mise en forme (voir `emit_web`).
struct WebText<'a> {
    text: &'a str,
    font_size: f32,
    // `Some` pour un bouton : sa couleur de texte au survol, puis a l'appui.
    button_hover: Option<(Option<Color>, Option<Color>)>,
}

// Modele web du nœud qui vient d'etre pousse (seulement avec rsC) : sa boite
// CSS, et pour un texte ou un bouton sa mise en forme et son texte aux
// espaces regroupes - les memes valeurs que l'interpreteur (voir
// `services::web`).
fn emit_web(kind: &str, style: &Style, source: &StyleSource, text: Option<WebText>, out: &mut String, level: usize, var: &str) {
    if !matches!(source, StyleSource::Rsc(_)) {
        return;
    }
    let mut body = format!("node.layout.css = Some(Box::new({})); ", css_box_literal(&web::css_box(style)));
    if let Some(t) = text {
        let mut text_style = web::text_style(style, t.font_size);
        let white_space = if t.button_hover.is_some() { WhiteSpace::Nowrap } else { text_style.white_space };
        if t.button_hover.is_some() && style.web.text_align.is_none() {
            text_style.align = TextAlign::Center;
        }
        body.push_str(&format!(
            "node.text = {:?}.to_string(); node.text_style = Some(azure_foundation::ui::models::text_style::TextStyle::new(azure_foundation::ui::services::fonts::font_for({:?}), {:.2}, azure_foundation::style::models::web_style::TextAlign::{:?}, azure_foundation::style::models::web_style::WhiteSpace::{:?})); ",
            web::collapse_whitespace(t.text, white_space),
            style.web.font_family.as_deref(),
            text_style.line_height,
            text_style.align,
            text_style.white_space,
        ));
        if text_style.options.italic || text_style.options.letter_spacing != 0.0 || text_style.decoration.any() {
            let d = text_style.decoration;
            body.push_str(&format!(
                "if let Some(style) = node.text_style.take() {{ node.text_style = Some(style.with_options({}, {:.2}, azure_foundation::style::models::web_style::TextDecoration {{ underline: {}, line_through: {}, overline: {} }})); }} ",
                text_style.options.italic, text_style.options.letter_spacing, d.underline, d.line_through, d.overline
            ));
        }
        if let Some((hover, active)) = t.button_hover {
            body.push_str(&format!("node.hover_text_color = {}; node.active_text_color = {}; ", color_literal_opt(hover), color_literal_opt(active)));
        }
    }
    out.push_str(&format!("{}if let Some(UiNode::{kind}(node)) = {var}.last_mut() {{ {body}}}\n", indent(level)));
}

fn length_literal(l: Length) -> String {
    let path = "azure_foundation::layout::models::css_box::Length";
    match l {
        Length::Auto => format!("{path}::Auto"),
        Length::Px(v) => format!("{path}::Px({v:.2})"),
        Length::Percent(v) => format!("{path}::Percent({v:.2})"),
    }
}

fn sides_literal(s: &Sides) -> String {
    format!(
        "azure_foundation::layout::models::css_box::Sides {{ top: {}, right: {}, bottom: {}, left: {} }}",
        length_literal(s.top),
        length_literal(s.right),
        length_literal(s.bottom),
        length_literal(s.left)
    )
}

fn css_tracks_literal(tracks: &[CssTrack]) -> String {
    let path = "azure_foundation::layout::models::css_box::CssTrack";
    let items: Vec<String> = tracks
        .iter()
        .map(|t| match t {
            CssTrack::Px(v) => format!("{path}::Px({v:.2})"),
            CssTrack::Percent(v) => format!("{path}::Percent({v:.2})"),
            CssTrack::Fr(v) => format!("{path}::Fr({v:.2})"),
            CssTrack::Auto => format!("{path}::Auto"),
        })
        .collect();
    format!("vec![{}]", items.join(", "))
}

// Une `CssBox` construite champ par champ (ses caches internes ne sont pas
// accessibles depuis une autre crate).
fn css_box_literal(c: &CssBox) -> String {
    let opt = |v: Option<usize>| match v { Some(v) => format!("Some({v})"), None => "None".to_string() };
    let align_self = match c.align_self { Some(a) => format!("Some({})", align_items_literal(a)), None => "None".to_string() };
    format!(
        "{{ let mut css = azure_foundation::layout::models::css_box::CssBox::default(); \
css.display = azure_foundation::layout::models::css_box::CssDisplay::{:?}; css.width = {}; css.height = {}; css.min_width = {}; css.min_height = {}; css.max_width = {}; css.max_height = {}; \
css.margin = {}; css.padding = {}; css.border = {}; css.border_box = {}; css.flex_direction = {}; css.flex_wrap = {}; css.justify_content = {}; css.align_items = {}; \
css.row_gap = {}; css.column_gap = {}; css.grid_template_columns = {}; css.grid_template_rows = {}; css.flex_grow = {:.2}; css.flex_shrink = {:.2}; css.flex_basis = {}; \
css.align_self = {}; css.grid_column = {}; css.grid_column_span = {}; css.grid_row = {}; css.grid_row_span = {}; \
css.fixed = {}; css.position = azure_foundation::layout::models::css_box::CssPosition::{:?}; css.inset = {}; css.z_index = {}; \
css.align_content = azure_foundation::layout::models::css_box::AlignContent::{:?}; css }}",
        c.display,
        length_literal(c.width),
        length_literal(c.height),
        length_literal(c.min_width),
        length_literal(c.min_height),
        length_literal(c.max_width),
        length_literal(c.max_height),
        sides_literal(&c.margin),
        sides_literal(&c.padding),
        border_literal(c.border),
        c.border_box,
        flex_direction_literal(c.flex_direction),
        c.flex_wrap,
        justify_content_literal(c.justify_content),
        align_items_literal(c.align_items),
        length_literal(c.row_gap),
        length_literal(c.column_gap),
        css_tracks_literal(&c.grid_template_columns),
        css_tracks_literal(&c.grid_template_rows),
        c.flex_grow,
        c.flex_shrink,
        length_literal(c.flex_basis),
        align_self,
        opt(c.grid_column),
        c.grid_column_span,
        opt(c.grid_row),
        c.grid_row_span,
        c.fixed,
        c.position,
        sides_literal(&c.inset),
        c.z_index,
        c.align_content,
    )
}

fn color_literal(color: Option<Color>) -> String {
    let c = color.unwrap_or(Color::new(255, 255, 255, 255));
    format!("Color::new({}, {}, {}, {})", c.r, c.g, c.b, c.a)
}

// Litteral Rust d'un `Option<Color>` - contrairement a `color_literal`, qui
// retombe toujours sur une couleur par defaut, `None` reste `None` : c'est
// ce qu'attendent `Button::hover_color`/`TextArea::hover_background`/
// `focus_background` (voir `resolve_pseudo_background`) pour distinguer
// "aucune regle `:hover`/`:focus` ne s'applique" de "elle s'applique avec
// cette couleur".
fn color_literal_opt(color: Option<Color>) -> String {
    match color {
        Some(c) => format!("Some({})", color_literal(Some(c))),
        None => "None".to_string(),
    }
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

/// Genere le code source Rust (un module complet) qui construit l'arbre de
/// `UiNode` correspondant a l'AST donne. `stylesheet` fournit les composants
/// de style reutilisables (les classes) references par `.nom` sur les
/// balises .rsh ; leurs valeurs sont resolues ici, a la generation, et
/// emises comme des `LayoutProps`/`Color` litteraux dans le code produit.
pub fn generate(nodes: &[AstNode], stylesheet: &Stylesheet) -> String {
    generate_internal(nodes, &StyleSource::Legacy(stylesheet))
}

/// Meme generation que `generate`, mais le style de chaque element est
/// resolu par le moteur de selecteurs rsC plutot que par l'ancienne table
/// de classes a plat : c'est le systeme de lien entre rsC et rsH - la
/// feuille de style s'applique directement sur l'arbre .rsh via le nom de
/// balise, la classe, l'id et les ancetres de chaque element, avec une
/// vraie cascade CSS (specificite, ordre source, `!important`).
pub fn generate_with_rsc(nodes: &[AstNode], stylesheet: &RscStylesheet) -> String {
    generate_internal(nodes, &StyleSource::Rsc(stylesheet))
}

fn generate_internal(nodes: &[AstNode], source: &StyleSource) -> String {
    let mut out = String::new();

    out.push_str("// Fichier genere automatiquement a partir d'un .rsh - ne pas modifier a la main.\n");
    // Tous les types possibles sont importes ; une page n'en utilise qu'une partie.
    out.push_str("#[allow(unused_imports)]\nuse azure_foundation::ui::models::container::Container;\n");
    out.push_str("#[allow(unused_imports)]\nuse azure_foundation::ui::models::label::Label;\n");
    out.push_str("#[allow(unused_imports)]\nuse azure_foundation::ui::models::button::Button;\n");
    out.push_str("#[allow(unused_imports)]\nuse azure_foundation::ui::models::image::Image;\n");
    out.push_str("#[allow(unused_imports)]\nuse azure_foundation::ui::models::video::Video;\n");
    out.push_str("#[allow(unused_imports)]\nuse azure_foundation::ui::models::textarea::TextArea;\n");
    out.push_str("#[allow(unused_imports)]\nuse azure_foundation::ui::models::ui_node::UiNode;\n");
    out.push_str("use azure_foundation::layout::models::layout_props::LayoutProps;\n");
    out.push_str("use azure_engine::rendering::models::color::Color;\n");
    out.push('\n');
    out.push_str("pub fn build_ui() -> Vec<UiNode> {\n");
    out.push_str("    build_ui_with(&azure_foundation::compiler::services::condition::Context::new())\n");
    out.push_str("}\n\n");
    out.push_str("/// Avec les donnees de la page (composants, `{{...}}`).\n");
    out.push_str("#[allow(unused_variables, clippy::vec_init_then_push)]\n");
    out.push_str("pub fn build_ui_with(ctx: &azure_foundation::compiler::services::condition::Context) -> Vec<UiNode> {\n");
    out.push_str("    let mut nodes: Vec<UiNode> = Vec::new();\n");

    emit_siblings(nodes, &mut out, 1, "nodes", source, &[], &mut Vec::new());

    out.push_str("    nodes\n");
    out.push_str("}\n");

    // Parties dynamiques : la feuille rsC, construite une fois.
    if let StyleSource::Rsc(sheet) = source
        && nodes.iter().any(crate::compiler::services::codegen_runtime::is_dynamic) {
            out.push_str("\n#[allow(dead_code)]\nfn sheet() -> &'static azure_foundation::compiler::rsc::models::rule::RscStylesheet {\n");
            out.push_str("    static SHEET: std::sync::OnceLock<azure_foundation::compiler::rsc::models::rule::RscStylesheet> = std::sync::OnceLock::new();\n");
            out.push_str(&format!("    SHEET.get_or_init(|| {})\n", crate::compiler::services::codegen_runtime::sheet_literal(sheet)));
            out.push_str("}\n");
        }

    out
}

// Noeuds (un seul, ou une chaine if/elseif/else) construits a l'execution
// par l'interpreteur (voir `codegen_runtime`).
fn emit_runtime(nodes: &[AstNode], out: &mut String, level: usize, var: &str, ancestors: &[ElementInfo], preceding: &mut Vec<ElementInfo>) {
    use crate::compiler::services::codegen_runtime::{ast_literal, element_info_literal};
    let ancestors: Vec<String> = ancestors.iter().map(element_info_literal).collect();
    let before: Vec<String> = preceding.iter().map(element_info_literal).collect();
    let literals: Vec<String> = nodes.iter().map(ast_literal).collect();
    out.push_str(&format!(
        "{}{var}.extend(azure_foundation::compiler::services::interpreter::build_nodes(&[{}], &azure_foundation::compiler::services::codegen::StyleSource::Rsc(sheet()), &[{}], &mut vec![{}], ctx));\n",
        indent(level),
        literals.join(", "),
        ancestors.join(", "),
        before.join(", ")
    ));
    // Les elements de base produits sont connus ; ceux d'un composant non
    // (un selecteur `+`/`~` qui suit un composant ne le voit pas).
    for node in nodes {
        if let Some(tag) = crate::compiler::rsc::services::link::tag_name(node) {
            let (class, id) = match node {
                AstNode::Container { class, id, .. }
                | AstNode::Title { class, id, .. }
                | AstNode::Title1 { class, id, .. }
                | AstNode::Title2 { class, id, .. }
                | AstNode::Title3 { class, id, .. }
                | AstNode::Text { class, id, .. }
                | AstNode::Button { class, id, .. }
                | AstNode::Image { class, id, .. }
                | AstNode::Video { class, id, .. }
                | AstNode::Textarea { class, id, .. } => (class.as_str(), id.as_str()),
                _ => ("", ""),
            };
            if !class.contains("{{") && !id.contains("{{") {
                preceding.push(ElementInfo::new(tag, class, id));
            }
        }
    }
}

// Emet les instructions qui remplissent le Vec<UiNode> nomme `var`, une
// entree par noeud. Les chaines If/ElseIf/Else consecutives sont regroupees
// en un seul if/else-if/else Rust plutot que plusieurs `if` independants.
// `ancestors` porte la chaine balise/classe/id des elements parents deja
// traverses - necessaire pour que le moteur rsC evalue les selecteurs
// descendant/enfant (`StyleSource::Legacy` l'ignore). `preceding` : meme
// principe pour les combinateurs freres (`+`/`~`) - la liste, dans l'ordre
// du document, des elements deja emis a CE niveau (voir
// `link::resolve_flat`, dont ceci est l'equivalent cote codegen) ;
// accumulee au fil de l'appel, jamais reinitialisee pour un noeud
// structurel (If/While/For/...) - seul un nouveau niveau d'imbrication
// reelle (les enfants d'un `Container`, voir `emit_node`) en demarre une
// nouvelle.
fn emit_siblings(nodes: &[AstNode], out: &mut String, level: usize, var: &str, source: &StyleSource, ancestors: &[ElementInfo], preceding: &mut Vec<ElementInfo>) {
    let mut idx = 0;
    let rsc = matches!(source, StyleSource::Rsc(_));
    while idx < nodes.len() {
        match &nodes[idx] {
            AstNode::If { .. } => {
                // Chaine if/elseif/else : dynamique en bloc (conditions sur
                // les donnees de la page), ou generee en Rust.
                let mut end = idx + 1;
                while matches!(nodes.get(end), Some(AstNode::ElseIf { .. } | AstNode::Else { .. })) {
                    end += 1;
                }
                if rsc && nodes[idx..end].iter().any(crate::compiler::services::codegen_runtime::needs_runtime) {
                    emit_runtime(&nodes[idx..end], out, level, var, ancestors, preceding);
                    idx = end;
                } else {
                    idx = emit_if_chain(nodes, idx, out, level, var, source, ancestors, preceding);
                }
            }
            node if rsc && crate::compiler::services::codegen_runtime::needs_runtime(node) => {
                emit_runtime(std::slice::from_ref(node), out, level, var, ancestors, preceding);
                idx += 1;
            }
            // ElseIf/Else rencontre sans If juste avant : anomalie de template.
            // On ne perd pas le contenu, mais on le signale clairement.
            AstNode::ElseIf { condition, children } => {
                out.push_str(&format!(
                    "{}// ATTENTION: elseif sans if precedent, traite comme un if independant\n",
                    indent(level)
                ));
                out.push_str(&format!("{}if {} {{\n", indent(level), condition));
                emit_siblings(children, out, level + 1, var, source, ancestors, preceding);
                out.push_str(&format!("{}}}\n", indent(level)));
                idx += 1;
            }
            AstNode::Else { children, .. } => {
                out.push_str(&format!(
                    "{}// ATTENTION: else sans if precedent, bloc toujours execute\n",
                    indent(level)
                ));
                emit_siblings(children, out, level, var, source, ancestors, preceding);
                idx += 1;
            }
            _ => {
                emit_node(&nodes[idx], out, level, var, source, ancestors, preceding);
                idx += 1;
            }
        }
    }
}

// Suppose que nodes[start] est un If. Consomme aussi les ElseIf et le Else
// (optionnel) qui suivent immediatement pour former une seule chaine
// if / else if / else. Retourne l'index juste apres la chaine consommee.
#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
fn emit_if_chain(
    nodes: &[AstNode],
    start: usize,
    out: &mut String,
    level: usize,
    var: &str,
    source: &StyleSource,
    ancestors: &[ElementInfo],
    preceding: &mut Vec<ElementInfo>,
) -> usize {
    let mut idx = start;

    let (condition, children) = match &nodes[idx] {
        AstNode::If { condition, children } => (condition, children),
        _ => unreachable!("emit_if_chain doit demarrer sur un If"),
    };
    out.push_str(&format!("{}if {} {{\n", indent(level), condition));
    emit_siblings(children, out, level + 1, var, source, ancestors, preceding);
    out.push_str(&format!("{}}}", indent(level)));
    idx += 1;

    while let Some(AstNode::ElseIf { condition, children }) = nodes.get(skip_blank_text(nodes, idx)) {
        idx = skip_blank_text(nodes, idx);
        out.push_str(&format!(" else if {} {{\n", condition));
        emit_siblings(children, out, level + 1, var, source, ancestors, preceding);
        out.push_str(&format!("{}}}", indent(level)));
        idx += 1;
    }

    if let Some(AstNode::Else { children, .. }) = nodes.get(skip_blank_text(nodes, idx)) {
        idx = skip_blank_text(nodes, idx);
        out.push_str(" else {\n");
        emit_siblings(children, out, level + 1, var, source, ancestors, preceding);
        out.push_str(&format!("{}}}", indent(level)));
        idx += 1;
    }

    out.push('\n');
    idx
}

fn emit_node(node: &AstNode, out: &mut String, level: usize, var: &str, source: &StyleSource, ancestors: &[ElementInfo], preceding: &mut Vec<ElementInfo>) {
    match node {
        AstNode::Container { class, id, children } => {
            let style = resolve("container", class, id, base_container_style(), source, ancestors, preceding);
            out.push_str(&format!(
                "{}{}.push(UiNode::Container(Container::new(\n",
                indent(level),
                var
            ));
            out.push_str(&format!("{}{},\n", indent(level + 1), layout_literal(&style)));
            out.push_str(&format!("{}{},\n", indent(level + 1), color_literal(style.background)));
            out.push_str(&format!("{}{{\n", indent(level + 1)));
            out.push_str(&format!(
                "{}let mut children: Vec<UiNode> = Vec::new();\n",
                indent(level + 2)
            ));
            let mut next_ancestors = ancestors.to_vec();
            next_ancestors.push(ElementInfo::new("container", class, id));
            emit_siblings(children, out, level + 2, "children", source, &next_ancestors, &mut Vec::new());
            out.push_str(&format!("{}children\n", indent(level + 2)));
            out.push_str(&format!("{}}},\n", indent(level + 1)));
            out.push_str(&format!("{})));\n", indent(level)));
            emit_decoration("Container", decoration_for("container", class, id, &style, source, ancestors, preceding), out, level, var);
            emit_web("Container", &style, source, None, out, level, var);
            preceding.push(ElementInfo::new("container", class, id));
        }
        AstNode::Title { class, id, children } => emit_label("title", class, id, children, out, level, var, 24.0, 700.0, source, ancestors, preceding),
        AstNode::Title1 { class, id, children } => emit_label("title1", class, id, children, out, level, var, 32.0, 700.0, source, ancestors, preceding),
        AstNode::Title2 { class, id, children } => emit_label("title2", class, id, children, out, level, var, 24.0, 600.0, source, ancestors, preceding),
        AstNode::Title3 { class, id, children } => emit_label("title3", class, id, children, out, level, var, 18.0, 600.0, source, ancestors, preceding),
        AstNode::Text { class, id, children } => emit_label("text", class, id, children, out, level, var, 14.0, 400.0, source, ancestors, preceding),
        // Comme l'interpreteur : le blanc entre deux balises n'est pas un texte.
        AstNode::RawText(text) => {
            if !text.trim().is_empty() {
                emit_label_text("text", "", "", text, out, level, var, 14.0, 400.0, source, ancestors, preceding);
            }
        }
        AstNode::Button { class, id, children } => {
            let text = extract_text(children);
            let style = resolve("button", class, id, base_button_style(), source, ancestors, preceding);
            // `button:hover { background-color: ...; }` (voir
            // `resolve_pseudo_background`) : `None` si la feuille n'a
            // aucune regle sous cet etat, auquel cas `Button::hover_color`
            // reste `None` et le rendu retombe sur `color` (voir
            // `ui::services::draw_ui::draw_button`). Le `.filter` retire le
            // cas ou l'etat survole retombe simplement sur la MEME regle de
            // base (aucune regle `:hover` reelle, juste la cascade normale
            // qui continue de s'appliquer) - sinon `hover_color` "existerait"
            // toujours, identique a `color`, des qu'une seule regle non
            // conditionnelle s'applique.
            let hover = resolve_pseudo_background("button", class, id, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: false })
                .filter(|c| Some(*c) != style.background);
            let active = resolve_pseudo_background("button", class, id, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: true })
                .filter(|c| Some(*c) != hover.or(style.background));
            out.push_str(&format!(
                "{}{}.push(UiNode::Button({{ let mut node = Button::new({}, {}, false, {:?}.to_string()); node.hover_color = {}; node.active_color = {}; node.text_color = {}; node.id = {:?}.to_string(); node.font_size = {:.1}; node.font_weight = {:.1}; node }}));\n",
                indent(level),
                var,
                layout_literal(&style),
                color_literal(style.background),
                text,
                color_literal_opt(hover),
                color_literal_opt(active),
                color_literal_opt(style.color),
                id,
                style.font_size.unwrap_or(16.0),
                style.font_weight.unwrap_or(500.0),
            ));
            emit_decoration("Button", decoration_for("button", class, id, &style, source, ancestors, preceding), out, level, var);
            let hover_text = resolve_pseudo_color("button", class, id, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: false }).filter(|c| Some(*c) != style.color);
            let active_text = resolve_pseudo_color("button", class, id, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: true }).filter(|c| Some(*c) != hover_text.or(style.color));
            emit_web("Button", &style, source, Some(WebText { text: &text, font_size: style.font_size.unwrap_or(16.0), button_hover: Some((hover_text, active_text)) }), out, level, var);
            preceding.push(ElementInfo::new("button", class, id));
        }
        AstNode::Image { class, id, src, .. } => {
            let style = resolve("image", class, id, base_media_style(), source, ancestors, preceding);
            out.push_str(&format!(
                "{}{}.push(UiNode::Image(Image::new({}, {:?}.to_string())));\n",
                indent(level),
                var,
                layout_literal(&style),
                src
            ));
            emit_decoration("Image", decoration_for("image", class, id, &style, source, ancestors, preceding), out, level, var);
            emit_web("Image", &style, source, None, out, level, var);
            preceding.push(ElementInfo::new("image", class, id));
        }
        AstNode::Video { class, id, src, .. } => {
            let style = resolve("video", class, id, base_media_style(), source, ancestors, preceding);
            out.push_str(&format!(
                "{}{}.push(UiNode::Video(Video::new({}, {:?}.to_string())));\n",
                indent(level),
                var,
                layout_literal(&style),
                src
            ));
            preceding.push(ElementInfo::new("video", class, id));
        }
        AstNode::Textarea { class, id, children } => {
            // Le texte initial est un placeholder statique ici (le codegen
            // emet du texte a compiler, pas un etat vivant) : la saisie
            // reelle n'existe que cote interpreteur/fenetre, voir
            // `services::interpreter` et `ui::services::interact`.
            let text = extract_text(children);
            let style = resolve("textarea", class, id, base_textarea_style(), source, ancestors, preceding);
            let hover = resolve_pseudo_background("textarea", class, id, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: false })
                .filter(|c| Some(*c) != style.background);
            let focus = resolve_pseudo_background("textarea", class, id, source, ancestors, preceding, PseudoState { hover: false, focus: true, active: false })
                .filter(|c| Some(*c) != style.background);
            out.push_str(&format!(
                "{}{}.push(UiNode::TextArea({{ let mut node = TextArea::new({}, {}, {}, {:?}.to_string()); node.hover_background = {}; node.focus_background = {}; node }}));\n",
                indent(level),
                var,
                layout_literal(&style),
                color_literal(style.background),
                color_literal(style.color),
                text,
                color_literal_opt(hover),
                color_literal_opt(focus),
            ));
            emit_decoration("TextArea", decoration_for("textarea", class, id, &style, source, ancestors, preceding), out, level, var);
            emit_web("TextArea", &style, source, None, out, level, var);
            preceding.push(ElementInfo::new("textarea", class, id));
        }
        AstNode::While { condition, children } => {
            out.push_str(&format!("{}while {} {{\n", indent(level), condition));
            emit_siblings(children, out, level + 1, var, source, ancestors, preceding);
            out.push_str(&format!("{}}}\n", indent(level)));
        }
        // Depuis que le lexer capture une expression complete apres le '.'
        // (`<for.item in liste>`), la condition est directement une
        // expression Rust valide pour `for` : on peut l'emettre telle quelle,
        // comme pour `while`.
        AstNode::For { condition, children } | AstNode::ForEach { condition, children } => {
            out.push_str(&format!("{}for {} {{\n", indent(level), condition));
            emit_siblings(children, out, level + 1, var, source, ancestors, preceding);
            out.push_str(&format!("{}}}\n", indent(level)));
        }
        // Emet un vrai `match` Rust : `condition` est le sujet brut (une
        // expression Rust, comme pour `if`/`while`), chaque `Arm` enfant
        // devient une branche `{motif} => { ... }` avec son motif brut
        // emis tel quel (litteral, `_`, ou motif `|` - n'importe quel motif
        // Rust valide, contrairement a l'interprete qui, lui, ne sait
        // evaluer qu'un sous-ensemble reduit, voir `condition::matches_pattern`).
        // L'exhaustivite du `match` genere (ex: un `_` manquant) n'est PAS
        // verifiee ici, comme le reste de ce module : c'est au compilateur
        // Rust du programme final de la signaler, pas a ce generateur.
        AstNode::Match { condition, children } => {
            let arms: Vec<(&String, &Vec<AstNode>)> = children
                .iter()
                .filter_map(|child| match child {
                    AstNode::Arm { pattern, children: arm_children } => Some((pattern, arm_children)),
                    _ => None,
                })
                .collect();

            if arms.is_empty() {
                // Aucun `arm` : rien a matcher - meme repli tolerant que
                // ElseIf/Else orphelins (voir plus haut), le contenu n'est
                // pas perdu, juste execute sans condition.
                out.push_str(&format!(
                    "{}// ATTENTION: match sans aucun arm, contenu execute sans condition\n",
                    indent(level)
                ));
                emit_siblings(children, out, level, var, source, ancestors, preceding);
                return;
            }

            out.push_str(&format!("{}match {condition} {{\n", indent(level)));
            for (pattern, arm_children) in arms {
                out.push_str(&format!("{}{pattern} => {{\n", indent(level + 1)));
                emit_siblings(arm_children, out, level + 2, var, source, ancestors, preceding);
                out.push_str(&format!("{}}}\n", indent(level + 1)));
            }
            out.push_str(&format!("{}}}\n", indent(level)));
        }
        // Un `<arm>` isole (jamais consomme par un `<match>` parent - voir
        // ci-dessus, ses enfants sont alors emis directement comme branches)
        // : meme tolerance qu'un ElseIf/Else orphelin, contenu execute sans
        // condition plutot que perdu silencieusement.
        AstNode::Arm { children, .. } => {
            out.push_str(&format!(
                "{}// ATTENTION: arm sans match precedent, contenu execute sans condition\n",
                indent(level)
            ));
            emit_siblings(children, out, level, var, source, ancestors, preceding);
        }
        // Composants et champs (voir `compiler::components`) : resolus a
        // l'execution par l'interpreteur, pas generes en Rust.
        AstNode::Element { tag, .. } => {
            out.push_str(&format!("{}// <{tag}> : composant non genere, utilisez l'interpreteur (RouteTable::view)\n", indent(level)));
        }
        AstNode::If { .. } | AstNode::ElseIf { .. } | AstNode::Else { .. } => {
            unreachable!("les noeuds if/elseif/else sont geres par emit_siblings")
        }
    }
}

#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
fn emit_label(
    tag: &str,
    class: &str,
    id: &str,
    children: &[AstNode],
    out: &mut String,
    level: usize,
    var: &str,
    font_size: f32,
    weight: f32,
    source: &StyleSource,
    ancestors: &[ElementInfo],
    preceding: &mut Vec<ElementInfo>,
) {
    let text = extract_text(children);
    emit_label_text(tag, class, id, &text, out, level, var, font_size, weight, source, ancestors, preceding);
}

#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
fn emit_label_text(
    tag: &str,
    class: &str,
    id: &str,
    text: &str,
    out: &mut String,
    level: usize,
    var: &str,
    font_size: f32,
    weight: f32,
    source: &StyleSource,
    ancestors: &[ElementInfo],
    preceding: &mut Vec<ElementInfo>,
) {
    let style = resolve(tag, class, id, label_base_style(tag, font_size, weight, source), source, ancestors, preceding);
    out.push_str(&format!(
        "{}{}.push(UiNode::Label(Label::new({}, {:?}.to_string(), {}, {:.1}, {:.1})));\n",
        indent(level),
        var,
        layout_literal(&style),
        text,
        color_literal(style.color),
        style.font_size.unwrap_or(font_size),
        style.font_weight.unwrap_or(weight),
    ));
    emit_decoration("Label", decoration_for(tag, class, id, &style, source, ancestors, preceding), out, level, var);
    emit_web("Label", &style, source, Some(WebText { text, font_size: style.font_size.unwrap_or(font_size), button_hover: None }), out, level, var);
    preceding.push(ElementInfo::new(tag, class, id));
}

// Concatene recursivement tout le texte brut porte par des descendants
// RawText (utilise pour le contenu des Title/Text/Button, qui n'ont pas de
// champ "text" direct sur leur token mais le portent via leurs enfants).
fn extract_text(nodes: &[AstNode]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            AstNode::RawText(t) => text.push_str(t),
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
            | AstNode::Element { children, .. } => text.push_str(&extract_text(children)),
        }
    }
    text
}
