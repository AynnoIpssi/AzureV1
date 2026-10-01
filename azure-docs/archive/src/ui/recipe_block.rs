// Construit UNE recette comme un bloc empile verticalement (titre -> objectif
// -> code source -> apercu live si rsH est fourni -> note) - toutes les
// hauteurs internes sont deduites d'un decompte d'"unites" (voir `weight`),
// pas de pourcentages tapes a la main par recette : une recette avec plus de
// lignes de code ou un apercu live obtient naturellement plus de place,
// sans que chaque recette n'ait a le calculer elle-meme.
use crate::content::recipe::Recipe;
use crate::theme;
use crate::ui::preview;
use azure_foundation::layout::models::layout_props::{DisplayMode, FlexDirection, LayoutProps};
use azure_foundation::ui::models::container::Container;
use azure_foundation::ui::models::label::Label;
use azure_foundation::ui::models::ui_node::UiNode;

const TITLE_UNITS: f32 = 2.2;
const CAPTION_UNITS: f32 = 1.0;
const LINE_UNITS: f32 = 1.0;
const PREVIEW_UNITS: f32 = 8.0;
// Poids par ligne du texte enveloppe (goal/note) - plus grand que
// `LINE_UNITS` (code) : une ligne de prose (14px) veut un peu plus d'air
// verticalement qu'une ligne de code compacte (13px).
const PROSE_LINE_UNITS: f32 = 1.4;
// Nombre de caracteres approximatif qui tient sur une ligne de la largeur
// habituelle d'un bloc (voir `category_window::page` pour les proportions
// de fenetre visees) a la taille de police du goal/de la note - approxime,
// pas mesure via `azure_engine::rendering::managers::renderer::measure_text_width`
// (aucune boite reelle disponible a ce stade de construction) : plutot trop
// prudent (lignes un peu courtes) que trop optimiste (texte tronque).
const PROSE_WRAP_CHARS: usize = 92;

/// Nombre d'"unites" de hauteur necessaires a cette recette - la meme unite
/// sert a la fois a repartir les recettes ENTRE ELLES dans le panneau de
/// contenu (voir `crate::ui::category_window::page`) et a repartir les
/// sections A L'INTERIEUR d'un bloc (`block` ci-dessous) : les deux
/// utilisent `flex_basis = mes_unites / total_unites * 100.0`, donc chaque
/// section obtient toujours la meme proportion REELLE de hauteur d'ecran,
/// que ce soit face aux autres recettes ou face aux autres sections de la
/// meme recette.
pub fn weight(recipe: &Recipe) -> f32 {
    let mut units = TITLE_UNITS + wrap_words(recipe.goal, PROSE_WRAP_CHARS).len() as f32 * PROSE_LINE_UNITS;
    if let Some(src) = recipe.rsh {
        units += CAPTION_UNITS + src.lines().count() as f32 * LINE_UNITS;
    }
    if let Some(src) = recipe.rsc {
        units += CAPTION_UNITS + src.lines().count() as f32 * LINE_UNITS;
    }
    if let Some(src) = recipe.snippet {
        units += src.lines().count() as f32 * LINE_UNITS;
    }
    if recipe.rsh.is_some() {
        units += PREVIEW_UNITS;
    }
    if let Some(note) = recipe.note {
        units += wrap_words(note, PROSE_WRAP_CHARS).len() as f32 * PROSE_LINE_UNITS;
    }
    units.max(1.0)
}

// Retour a la ligne "au mot" naif (pas de mesure reelle de police, voir
// `PROSE_WRAP_CHARS`) - suffisant pour du texte documentaire dans un panneau
// dont on connait la largeur approximative a l'avance, contrairement a
// `ui::services::text_layout::wrap_lines` d'azure-foundation (mesure exacte
// via la police reelle) qui exige une largeur en pixels deja connue, elle,
// au moment du DESSIN plutot qu'a la construction de l'arbre `UiNode`.
fn wrap_words(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > max_chars {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Le bloc complet d'une recette, avec `basis_pct` deja calcule par
/// l'appelant (`weight(recipe) / somme_des_poids * 100.0`) - c'est la
/// hauteur, en % de la hauteur VIRTUELLE du panneau de contenu, que ce bloc
/// occupera parmi ses freres (voir `layout::models::layout_props::LayoutProps::content_height`).
pub fn block(recipe: &Recipe, basis_pct: f32) -> UiNode {
    let total_units = weight(recipe);
    let mut sections: Vec<UiNode> = Vec::new();

    sections.push(text_section(recipe.title, theme::TEXT_PRIMARY, theme::RECIPE_TITLE_SIZE, theme::RECIPE_TITLE_WEIGHT, TITLE_UNITS, total_units));
    sections.push(prose_section(recipe.goal, theme::TEXT_SECONDARY, theme::GOAL_SIZE, theme::GOAL_WEIGHT, total_units));

    if let Some(src) = recipe.rsh {
        sections.push(caption("rsH", total_units));
        sections.push(code_panel(src, src.lines().count() as f32 * LINE_UNITS, total_units));
    }
    if let Some(src) = recipe.rsc {
        sections.push(caption("rsC", total_units));
        sections.push(code_panel(src, src.lines().count() as f32 * LINE_UNITS, total_units));
    }
    if let Some(src) = recipe.snippet {
        sections.push(code_panel(src, src.lines().count() as f32 * LINE_UNITS, total_units));
    }
    if let Some(rsh) = recipe.rsh {
        sections.push(preview_panel(recipe, rsh, total_units));
    }
    if let Some(note) = recipe.note {
        sections.push(prose_section(note, theme::WARNING, theme::NOTE_SIZE, theme::NOTE_WEIGHT, total_units));
    }

    let mut layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, theme::SPACE_SM);
    layout.display = DisplayMode::Flex;
    layout.flex_direction = FlexDirection::Column;
    layout.gap = theme::SPACE_XS;
    layout.flex_basis = Some(basis_pct);
    layout.flex_shrink = 0.0;
    UiNode::Container(Container::new(layout, theme::SURFACE, sections))
}

fn flex_child_layout(units: f32, total_units: f32) -> LayoutProps {
    let mut layout = LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0);
    layout.flex_basis = Some(units / total_units * 100.0);
    layout.flex_shrink = 0.0;
    layout
}

fn text_section(text: &str, color: azure_engine::rendering::models::color::Color, size: f32, weight_: f32, units: f32, total_units: f32) -> UiNode {
    UiNode::Label(Label::new(flex_child_layout(units, total_units), text.to_string(), color, size, weight_))
}

/// Comme `text_section`, mais pour un texte de prose potentiellement plus
/// long qu'une ligne (`goal`/`note`) - un `Label` ne retourne jamais
/// automatiquement a la ligne (voir `ui::services::draw_label`, qui dessine
/// tout son texte sur une seule ligne, contrairement a `TextArea`), donc
/// cette fonction le decoupe elle-meme (voir `wrap_words`) en plusieurs
/// `Label` empiles.
fn prose_section(text: &str, color: azure_engine::rendering::models::color::Color, size: f32, weight_: f32, total_units: f32) -> UiNode {
    let lines = wrap_words(text, PROSE_WRAP_CHARS);
    let line_count = lines.len().max(1) as f32;
    let panel_units = line_count * PROSE_LINE_UNITS;
    let children: Vec<UiNode> = lines
        .into_iter()
        .map(|line| UiNode::Label(Label::new(flex_child_layout(1.0, line_count), line, color, size, weight_)))
        .collect();

    let mut layout = flex_child_layout(panel_units, total_units);
    layout.display = DisplayMode::Flex;
    layout.flex_direction = FlexDirection::Column;
    UiNode::Container(Container::new(layout, theme::SURFACE, children))
}

fn caption(text: &str, total_units: f32) -> UiNode {
    UiNode::Label(Label::new(flex_child_layout(CAPTION_UNITS, total_units), text.to_string(), theme::TEXT_SECONDARY, theme::NOTE_SIZE, 600.0))
}

fn code_panel(source: &str, panel_units: f32, total_units: f32) -> UiNode {
    let lines: Vec<&str> = source.lines().collect();
    let line_count = lines.len().max(1) as f32;
    let children: Vec<UiNode> = lines
        .into_iter()
        .map(|line| {
            let text = if line.is_empty() { " ".to_string() } else { line.to_string() };
            UiNode::Label(Label::new(flex_child_layout(1.0, line_count), text, theme::TEXT_PRIMARY, theme::CODE_SIZE, theme::CODE_WEIGHT))
        })
        .collect();

    let mut layout = flex_child_layout(panel_units, total_units);
    layout.display = DisplayMode::Flex;
    layout.flex_direction = FlexDirection::Column;
    layout.padding = theme::SPACE_XS;
    UiNode::Container(Container::new(layout, theme::SURFACE_ALT, children))
}

fn preview_panel(recipe: &Recipe, rsh_src: &str, total_units: f32) -> UiNode {
    let nodes = match (recipe.rsc, recipe.ctx) {
        (Some(rsc_src), _) => preview::render_rsh_with_rsc(rsh_src, rsc_src),
        (None, Some(ctx_fn)) => preview::render_rsh_ctx(rsh_src, &ctx_fn()),
        (None, None) => preview::render_rsh(rsh_src),
    };

    let mut layout = flex_child_layout(PREVIEW_UNITS, total_units);
    layout.padding = theme::SPACE_XS;
    UiNode::Container(Container::new(layout, theme::BACKGROUND, nodes))
}
