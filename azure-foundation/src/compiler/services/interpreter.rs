// Meme resolution de style que `codegen` (balise/classe/id/ancetres via
// `StyleSource`), mais qui construit directement de vrais `UiNode` en
// memoire plutot que du texte Rust a compiler - utile pour tout ce qui doit
// consommer l'arbre .rsh tout de suite (par exemple l'ouvrir dans une vraie
// fenetre, voir `ui::services::draw_ui`).
use crate::compiler::rsh::mangers::parser::AstNode;
use crate::compiler::services::web;
use crate::style::models::web_style::WhiteSpace;
use crate::compiler::services::codegen::{resolve_pseudo_color, base_button_style, base_container_style, base_media_style, label_base_style, base_textarea_style, resolve, resolve_pseudo_background, decoration_for, StyleSource};
use crate::compiler::services::condition::{evaluate, evaluate_value, interpolate, matches_pattern, ConditionValue, Context};
use crate::layout::models::layout_props::LayoutProps;
use crate::compiler::rsc::models::selector::PseudoState;
use crate::compiler::rsc::services::link::ElementInfo;
use crate::style::models::style::Style;
use crate::ui::models::button::Button;
use crate::ui::models::container::Container;
use crate::ui::models::image::Image;
use crate::ui::models::label::Label;
use crate::ui::models::textarea::TextArea;
use crate::ui::models::ui_node::UiNode;
use crate::ui::models::video::Video;
use azure_engine::rendering::models::color::Color;

/// Un bouton appuye : survole ET actif (`button:active` l'emporte sur
/// `button:hover` a specificite egale, comme en CSS).
const ACTIVE: PseudoState = PseudoState { hover: true, focus: false, active: true };

/// Construit l'arbre de `UiNode` correspondant a l'AST donne, style par
/// `source` (l'ancien systeme `.style` ou le moteur de selecteurs rsC), avec
/// un `Context` VIDE pour les conditions (voir `build_ui_with_context`) :
/// aucun `if`/`elseif` sans `else` ne peut alors etre vrai (voir
/// `condition::Context::new`), donc ne s'affiche jamais - a utiliser
/// seulement quand l'AST n'a pas de logique conditionnelle a previsualiser
/// (c'est le cas de tous les exemples actuels, voir `examples/run_window_demo.rs`),
/// sinon preferer `build_ui_with_context` avec de vraies valeurs.
pub fn build_ui(nodes: &[AstNode], source: &StyleSource) -> Vec<UiNode> {
    build_ui_with_context(nodes, source, &Context::new())
}

/// Meme construction que `build_ui`, mais les noeuds `if`/`elseif`/`else`
/// sont reellement evalues contre `ctx` (voir `condition::evaluate`) plutot
/// que d'afficher systematiquement toutes les branches empilees - voir
/// `build_if_chain`. C'est l'equivalent direct de `codegen`, qui emet lui
/// un vrai `if`/`else` Rust execute plus tard avec de vraies variables ;
/// ici, `ctx` EST ces variables, fournies d'avance puisqu'il n'y a pas de
/// programme separe qui tournera ensuite pour les evaluer.
pub fn build_ui_with_context(nodes: &[AstNode], source: &StyleSource, ctx: &Context) -> Vec<UiNode> {
    crate::compiler::services::codegen::clear_cascade_cache();
    build_siblings(nodes, source, &[], ctx)
}

/// Construit `nodes` comme s'ils etaient a cet endroit de la page (sous
/// `ancestors`, apres `preceding`) - utilise par le code genere pour ses
/// parties dynamiques (voir `codegen_runtime`).
pub fn build_nodes(nodes: &[AstNode], source: &StyleSource, ancestors: &[ElementInfo], preceding: &mut Vec<ElementInfo>, ctx: &Context) -> Vec<UiNode> {
    let mut out = Vec::new();
    build_flat(nodes, source, ancestors, ctx, preceding, &mut out);
    out
}

pub(crate) fn build_siblings(nodes: &[AstNode], source: &StyleSource, ancestors: &[ElementInfo], ctx: &Context) -> Vec<UiNode> {
    let mut out = Vec::new();
    let mut preceding: Vec<ElementInfo> = Vec::new();
    build_flat(nodes, source, ancestors, ctx, &mut preceding, &mut out);
    out
}

// Un seul niveau d'imbrication CSS (voir `link::resolve_flat`, dont ceci
// est l'equivalent cote interpreteur) : `preceding` est alimentee au fur et
// a mesure, jamais reinitialisee pour un noeud structurel (If/While/...) -
// necessaire aux combinateurs freres (`+`/`~`, voir `link::Combinator`).
// Seul un nouveau niveau d'imbrication reel (les enfants d'un `Container`,
// voir `build_node`) en demarre une nouvelle via `build_siblings`. Les
// chaines If/ElseIf/Else consecutives sont reperees ICI (comme
// `codegen::emit_siblings` le fait pour emettre un seul if/else-if/else
// Rust) plutot que noeud par noeud dans `build_node`, pour pouvoir arreter
// des qu'une branche a matche.
pub(crate) fn build_flat(nodes: &[AstNode], source: &StyleSource, ancestors: &[ElementInfo], ctx: &Context, preceding: &mut Vec<ElementInfo>, out: &mut Vec<UiNode>) {
    let mut idx = 0;
    while idx < nodes.len() {
        match &nodes[idx] {
            AstNode::If { .. } => {
                idx = build_if_chain(nodes, idx, source, ancestors, ctx, preceding, out);
            }
            // ElseIf/Else rencontre sans If juste avant : meme anomalie de
            // template que documentee par `codegen::emit_siblings` - un
            // ElseIf orphelin est traite comme un If independant (sa
            // condition est evaluee normalement), un Else orphelin
            // s'affiche toujours (il n'a par construction pas de condition).
            AstNode::ElseIf { condition, children } => {
                if evaluate(condition, ctx).unwrap_or(false) {
                    build_flat(children, source, ancestors, ctx, preceding, out);
                }
                idx += 1;
            }
            AstNode::Else { children, .. } => {
                build_flat(children, source, ancestors, ctx, preceding, out);
                idx += 1;
            }
            _ => {
                build_node(&nodes[idx], source, ancestors, ctx, preceding, out);
                idx += 1;
            }
        }
    }
}

// Suppose que nodes[start] est un If (voir `codegen::emit_if_chain`, dont
// ceci est l'equivalent cote interpreteur). Consomme aussi les ElseIf et le
// Else optionnel qui suivent immediatement, evalue chaque condition dans
// l'ordre (voir `condition::evaluate`) et ne construit les `UiNode` QUE de
// la premiere branche qui matche - une condition non evaluable (hors du
// sous-ensemble supporte, voir `condition`) est traitee comme fausse : sa
// branche ne s'affiche pas, elle n'est pas non plus affichee "par defaut"
// comme avant ce correctif. Retourne l'index juste apres la chaine
// consommee.
fn build_if_chain(
    nodes: &[AstNode],
    start: usize,
    source: &StyleSource,
    ancestors: &[ElementInfo],
    ctx: &Context,
    preceding: &mut Vec<ElementInfo>,
    out: &mut Vec<UiNode>,
) -> usize {
    let mut idx = start;

    let (condition, children) = match &nodes[idx] {
        AstNode::If { condition, children } => (condition, children),
        _ => unreachable!("build_if_chain doit demarrer sur un If"),
    };
    let mut matched = evaluate(condition, ctx).unwrap_or(false);
    if matched {
        build_flat(children, source, ancestors, ctx, preceding, out);
    }
    idx += 1;

    while let Some(AstNode::ElseIf { condition, children }) = nodes.get(skip_blank_text(nodes, idx)) {
        idx = skip_blank_text(nodes, idx);
        if !matched && evaluate(condition, ctx).unwrap_or(false) {
            build_flat(children, source, ancestors, ctx, preceding, out);
            matched = true;
        }
        idx += 1;
    }

    if let Some(AstNode::Else { children, .. }) = nodes.get(skip_blank_text(nodes, idx)) {
        idx = skip_blank_text(nodes, idx);
        if !matched {
            build_flat(children, source, ancestors, ctx, preceding, out);
        }
        idx += 1;
    }

    idx
}

// Le parseur rsH garde l'indentation entre deux balises comme un
// `RawText` blanc (ex: "\n    " entre `<!if>` et `<elseif...>`) : sans ce
// saut, un if/elseif/else ecrit sur plusieurs lignes n'etait jamais vu
// comme une chaine - l'ElseIf/Else devenait orphelin et le Else s'affichait
// toujours. Retourne l'index du premier noeud non blanc a partir de `idx`.
pub(crate) fn skip_blank_text(nodes: &[AstNode], mut idx: usize) -> usize {
    while let Some(AstNode::RawText(text)) = nodes.get(idx) {
        if !text.trim().is_empty() {
            break;
        }
        idx += 1;
    }
    idx
}

// `condition` est de la forme "item in liste" (voir
// `rsh::services::lexer::tokenize`) : "liste" est resolue contre une
// `ConditionValue::List` de `ctx`, et chaque tour voit l'element courant
// sous le nom "item" (`{{item.nom}}`, `<if.item.actif>`), plus
// `item_index` (a partir de 0) et `item_rang` (a partir de 1). Zero iteration si "liste" n'est pas dans
// `ctx` ou si la condition n'a pas la forme attendue - fail-closed, comme
// `if`.
fn build_for(
    condition: &str,
    children: &[AstNode],
    source: &StyleSource,
    ancestors: &[ElementInfo],
    ctx: &Context,
    preceding: &mut Vec<ElementInfo>,
    out: &mut Vec<UiNode>,
) {
    let Some((binding, iterable)) = condition.split_once(" in ") else { return };
    let Some(ConditionValue::List(items)) = ctx.get(iterable.trim()) else { return };
    let binding = binding.trim();

    for (index, item) in items.iter().enumerate() {
        let inner = ctx.clone().with_value(binding, item.clone()).with_value(&format!("{binding}_index"), index as f64).with_value(&format!("{binding}_rang"), (index + 1) as f64);
        build_flat(children, source, ancestors, &inner, preceding, out);
    }
}

// `LayoutProps` d'un element, avec sa boite CSS quand la feuille est rsC :
// l'arbre est alors mis en page comme sur le web (voir
// `layout::managers::web_layout`).
pub(crate) fn layout_for(style: &Style, source: &StyleSource) -> LayoutProps {
    let mut layout = layout_of(style);
    if matches!(source, StyleSource::Rsc(_)) {
        layout.css = Some(Box::new(web::css_box(style)));
    }
    layout
}

fn layout_of(style: &Style) -> LayoutProps {
    let mut layout = LayoutProps::new(
        style.x.unwrap_or(0.0),
        style.y.unwrap_or(0.0),
        style.width.unwrap_or(100.0),
        style.height.unwrap_or(100.0),
        style.margin.unwrap_or(0.0),
        style.padding.unwrap_or(0.0),
    );
    // Flex/grid (voir `layout::managers::layout_manager::resolve_children`) :
    // n'ecrase que les champs qu'une regle rsC a effectivement definis,
    // laissant `LayoutProps::new` a ses valeurs par defaut (mode `Block`)
    // pour le reste.
    if let Some(v) = style.display {
        layout.display = v;
    }
    if let Some(v) = style.flex_direction {
        layout.flex_direction = v;
    }
    if let Some(v) = style.flex_wrap {
        layout.flex_wrap = v;
    }
    if let Some(v) = style.justify_content {
        layout.justify_content = v;
    }
    if let Some(v) = style.align_items {
        layout.align_items = v;
    }
    if let Some(v) = style.gap {
        layout.gap = v;
    }
    if let Some(v) = style.flex_grow {
        layout.flex_grow = v;
    }
    if let Some(v) = style.flex_shrink {
        layout.flex_shrink = v;
    }
    if style.flex_basis.is_some() {
        layout.flex_basis = style.flex_basis;
    }
    if let Some(ref v) = style.grid_template_columns {
        layout.grid_template_columns = v.clone();
    }
    if let Some(ref v) = style.grid_template_rows {
        layout.grid_template_rows = v.clone();
    }
    if style.grid_column.is_some() {
        layout.grid_column = style.grid_column;
    }
    if let Some(v) = style.grid_column_span {
        layout.grid_column_span = v;
    }
    if style.grid_row.is_some() {
        layout.grid_row = style.grid_row;
    }
    if let Some(v) = style.grid_row_span {
        layout.grid_row_span = v;
    }
    if let Some(v) = style.overflow {
        layout.overflow = v;
    }
    if let Some(v) = style.overflow_x {
        layout.overflow_x = v;
    }
    layout
}

pub(crate) fn color_of(color: Option<Color>) -> Color {
    color.unwrap_or(Color::new(255, 255, 255, 255))
}

// Classes et id calcules (`<text.badge.{{statut}}>`, `<button#ouvrir-{{app.nom}}>`) :
// remplaces avant de chercher les regles rsC qui s'appliquent.
// Construit a chaque redessin pour chaque element : on ne copie que ce qui
// a vraiment un nom calcule, et un conteneur sans ses enfants (ils sont
// pris dans l'original, voir `build_node`) - sinon chaque element copiait
// tout son sous-arbre.
fn with_names(node: &AstNode, ctx: &Context) -> Option<AstNode> {
    let computed = |s: &str| s.contains("{{");
    let mut copy = match node {
        AstNode::Container { class, id, .. } if computed(class) || computed(id) => AstNode::Container { class: class.clone(), id: id.clone(), children: Vec::new() },
        AstNode::Container { .. } => return None,
        AstNode::Title { class, id, .. }
        | AstNode::Title1 { class, id, .. }
        | AstNode::Title2 { class, id, .. }
        | AstNode::Title3 { class, id, .. }
        | AstNode::Text { class, id, .. }
        | AstNode::Button { class, id, .. }
        | AstNode::Image { class, id, .. }
        | AstNode::Video { class, id, .. }
        | AstNode::Textarea { class, id, .. }
        | AstNode::Element { class, id, .. }
            if computed(class) || computed(id) =>
        {
            node.clone()
        }
        _ => return None,
    };
    let (class, id) = match &mut copy {
        AstNode::Container { class, id, .. }
        | AstNode::Title { class, id, .. }
        | AstNode::Title1 { class, id, .. }
        | AstNode::Title2 { class, id, .. }
        | AstNode::Title3 { class, id, .. }
        | AstNode::Text { class, id, .. }
        | AstNode::Button { class, id, .. }
        | AstNode::Image { class, id, .. }
        | AstNode::Video { class, id, .. }
        | AstNode::Textarea { class, id, .. }
        | AstNode::Element { class, id, .. } => (class, id),
        _ => return None,
    };
    if !computed(class) && !computed(id) {
        return None;
    }
    *class = interpolate(class, ctx).split_whitespace().collect::<Vec<_>>().join(" ");
    *id = interpolate(id, ctx);
    Some(copy)
}

fn build_node(node: &AstNode, source: &StyleSource, ancestors: &[ElementInfo], ctx: &Context, preceding: &mut Vec<ElementInfo>, out: &mut Vec<UiNode>) {
    let computed = with_names(node, ctx);
    let original = node;
    let node = computed.as_ref().unwrap_or(node);
    match node {
        AstNode::Container { class, id, .. } => {
            let AstNode::Container { children, .. } = original else { unreachable!() };
            let style = resolve("container", class, id, base_container_style(), source, ancestors, preceding);
            let mut next_ancestors = ancestors.to_vec();
            next_ancestors.push(ElementInfo::new("container", class, id));
            let kids = build_siblings(children, source, &next_ancestors, ctx);
            let mut container = Container::new(layout_for(&style, source), color_of(style.background), kids);
            container.decoration = decoration_for("container", class, id, &style, source, ancestors, preceding);
            out.push(UiNode::Container(container));
            preceding.push(ElementInfo::new("container", class, id));
        }
        AstNode::Title { class, id, children } => out.push(build_label(ctx, "title", class, id, children, 24.0, 700.0, source, ancestors, preceding)),
        AstNode::Title1 { class, id, children } => out.push(build_label(ctx, "title1", class, id, children, 32.0, 700.0, source, ancestors, preceding)),
        AstNode::Title2 { class, id, children } => out.push(build_label(ctx, "title2", class, id, children, 24.0, 600.0, source, ancestors, preceding)),
        AstNode::Title3 { class, id, children } => out.push(build_label(ctx, "title3", class, id, children, 18.0, 600.0, source, ancestors, preceding)),
        AstNode::Text { class, id, children } => out.push(build_label(ctx, "text", class, id, children, 14.0, 400.0, source, ancestors, preceding)),
        AstNode::RawText(text) => {
            if !text.trim().is_empty() {
                out.push(build_label_text("text", "", "", &interpolate(text, ctx), 14.0, 400.0, source, ancestors, preceding));
            }
        }
        AstNode::Button { class, id, children } => {
            let text = interpolate(&extract_text(children), ctx);
            let style = resolve("button", class, id, base_button_style(), source, ancestors, preceding);
            let mut button = Button::new(layout_for(&style, source), color_of(style.background), false, text);
            button.hover_color = resolve_pseudo_background("button", class, id, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: false })
                .filter(|c| Some(*c) != style.background);
            button.active_color = resolve_pseudo_background("button", class, id, source, ancestors, preceding, ACTIVE)
                .filter(|c| Some(*c) != button.hover_color.or(style.background));
            button.text_color = style.color;
            // Le style vient du `#id` ecrit ; le clic voit l'id interpole
            // (`<button#ouvrir-{{app.nom}}>` dans une boucle).
            button.id = interpolate(id, ctx);
            button.font_size = style.font_size.unwrap_or(button.font_size);
            button.font_weight = style.font_weight.unwrap_or(button.font_weight);
            button.decoration = decoration_for("button", class, id, &style, source, ancestors, preceding);
            if matches!(source, StyleSource::Rsc(_)) {
                let mut text_style = web::text_style(&style, button.font_size);
                // Comme un bouton HTML : texte centre sauf `text-align` explicite.
                if style.web.text_align.is_none() {
                    text_style.align = crate::style::models::web_style::TextAlign::Center;
                }
                button.text_style = Some(text_style);
                button.text = web::collapse_whitespace(&button.text, WhiteSpace::Nowrap);
                button.hover_text_color = resolve_pseudo_color("button", class, id, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: false })
                    .filter(|c| Some(*c) != style.color);
                button.active_text_color = resolve_pseudo_color("button", class, id, source, ancestors, preceding, ACTIVE)
                    .filter(|c| Some(*c) != button.hover_text_color.or(style.color));
            }
            out.push(UiNode::Button(button));
            preceding.push(ElementInfo::new("button", class, id));
        }
        AstNode::Image { class, id, src, .. } => {
            let style = resolve("image", class, id, base_media_style(), source, ancestors, preceding);
            let mut image = Image::new(layout_for(&style, source), interpolate(src, ctx));
            image.decoration = decoration_for("image", class, id, &style, source, ancestors, preceding);
            out.push(UiNode::Image(image));
            preceding.push(ElementInfo::new("image", class, id));
        }
        AstNode::Video { class, id, src, .. } => {
            let style = resolve("video", class, id, base_media_style(), source, ancestors, preceding);
            out.push(UiNode::Video(Video::new(layout_for(&style, source), interpolate(src, ctx))));
            preceding.push(ElementInfo::new("video", class, id));
        }
        AstNode::Textarea { class, id, children } => {
            // Le texte des enfants sert de contenu initial (comme le texte
            // d'un bouton) - l'utilisateur peut ensuite le modifier au
            // clavier une fois la fenetre ouverte, voir ui::services::interact.
            let text = interpolate(&extract_text(children), ctx);
            let style = resolve("textarea", class, id, base_textarea_style(), source, ancestors, preceding);
            let mut area = TextArea::new(layout_for(&style, source), color_of(style.background), color_of(style.color), text);
            area.hover_background = resolve_pseudo_background("textarea", class, id, source, ancestors, preceding, PseudoState { hover: true, focus: false, active: false })
                .filter(|c| Some(*c) != style.background);
            area.focus_background = resolve_pseudo_background("textarea", class, id, source, ancestors, preceding, PseudoState { hover: false, focus: true, active: false })
                .filter(|c| Some(*c) != style.background);
            area.decoration = decoration_for("textarea", class, id, &style, source, ancestors, preceding);
            area.id = id.clone();
            area.placeholder = String::new();
            area.font_size = style.font_size.unwrap_or(area.font_size);
            area.font_weight = style.font_weight.unwrap_or(area.font_weight);
            out.push(UiNode::TextArea(area));
            preceding.push(ElementInfo::new("textarea", class, id));
        }
        // If/ElseIf/Else n'atteignent jamais ce point : `build_flat` les
        // intercepte avant, en chaine, via `build_if_chain` (voir sa doc).
        AstNode::If { .. } | AstNode::ElseIf { .. } | AstNode::Else { .. } => {
            unreachable!("les noeuds if/elseif/else sont geres par build_flat")
        }
        // Meme condition evaluee UNE fois (voir `condition::evaluate`) :
        // contrairement a `codegen` (un vrai `while` Rust, ré-evalue par le
        // programme final a chaque tour), l'interpreteur n'a pas de moyen
        // de faire varier `ctx` entre deux tours - le mieux qu'il puisse
        // faire honnetement est un aperçu a un seul "tour" (affiche si vrai,
        // rien sinon), pas une vraie boucle.
        AstNode::While { condition, children } => {
            if evaluate(condition, ctx).unwrap_or(false) {
                build_flat(children, source, ancestors, ctx, preceding, out);
            }
        }
        AstNode::For { condition, children } | AstNode::ForEach { condition, children } => {
            build_for(condition, children, source, ancestors, ctx, preceding, out);
        }
        // Evalue le sujet une seule fois (voir `condition::evaluate_value`),
        // puis cherche le PREMIER `Arm` dont le motif matche (voir
        // `condition::matches_pattern`) - comme un `match` Rust, une seule
        // branche s'affiche jamais plusieurs empilees. Un sujet inevaluable,
        // ou aucun `Arm` (motif litteral ou `_`) qui matche : rien ne
        // s'affiche, fail-closed comme `if`/`elseif` sans `else` (voir
        // `build_if_chain`). Les enfants qui ne sont pas des `Arm` (anomalie
        // de template) sont ignores ici : un contenu directement sous
        // `<match>`, hors de tout `<arm>`, n'a pas de motif a comparer.
        AstNode::Match { condition, children } => {
            let subject = evaluate_value(condition, ctx);
            let matched_arm = children.iter().find(|child| match child {
                AstNode::Arm { pattern, .. } => matches_pattern(pattern, subject.as_ref()),
                _ => false,
            });
            if let Some(AstNode::Arm { children: arm_children, .. }) = matched_arm {
                build_flat(arm_children, source, ancestors, ctx, preceding, out);
            }
        }
        // Un `<arm>` isole, jamais consomme par un `<match>` parent (voir
        // ci-dessus - a l'interieur d'un match, ses enfants sont traites
        // directement par le bras `AstNode::Match`, ce point n'est atteint
        // que pour un `arm` orphelin) : meme tolerance qu'un `ElseIf`/`Else`
        // sans `if` precedent, le contenu s'affiche sans condition plutot que
        // d'etre perdu silencieusement.
        AstNode::Arm { children, .. } => build_flat(children, source, ancestors, ctx, preceding, out),
        AstNode::Element { tag, class, id, attrs, children } => crate::compiler::components::build_element(tag, class, id, attrs, children, source, ancestors, ctx, preceding, out),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_label(
    ctx: &Context,
    tag: &str,
    class: &str,
    id: &str,
    children: &[AstNode],
    font_size: f32,
    weight: f32,
    source: &StyleSource,
    ancestors: &[ElementInfo],
    preceding: &mut Vec<ElementInfo>,
) -> UiNode {
    let text = interpolate(&extract_text(children), ctx);
    build_label_text(tag, class, id, &text, font_size, weight, source, ancestors, preceding)
}

#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
pub(crate) fn build_label_text(
    tag: &str,
    class: &str,
    id: &str,
    text: &str,
    font_size: f32,
    weight: f32,
    source: &StyleSource,
    ancestors: &[ElementInfo],
    preceding: &mut Vec<ElementInfo>,
) -> UiNode {
    let style = resolve(tag, class, id, label_base_style(tag, font_size, weight, source), source, ancestors, preceding);
    let decoration = decoration_for(tag, class, id, &style, source, ancestors, preceding);
    preceding.push(ElementInfo::new(tag, class, id));
    let mut label = Label::new(
        layout_for(&style, source),
        text.to_string(),
        color_of(style.color),
        style.font_size.unwrap_or(font_size),
        style.font_weight.unwrap_or(weight),
    );
    label.decoration = decoration;
    if matches!(source, StyleSource::Rsc(_)) {
        let text_style = web::text_style(&style, label.font_size);
        label.text = web::collapse_whitespace(&label.text, text_style.white_space);
        label.text_style = Some(text_style);
    }
    UiNode::Label(label)
}

// Identique a `codegen::extract_text` : concatene le texte brut porte par
// les descendants RawText.
pub(crate) fn extract_text(nodes: &[AstNode]) -> String {
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
