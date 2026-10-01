use crate::layout::models::layout_props::{AlignItems, DisplayMode, FlexDirection, JustifyContent, Overflow, Track};
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::{BorderWidths, Fill, Shadow};

/// Un ensemble de proprietes visuelles/layout — l'equivalent d'un bloc de
/// regles CSS (`{ width: ...; color: ...; }`). Chaque champ est optionnel
/// pour permettre la cascade : un `Style` partiel (ex: juste une couleur)
/// peut etre fusionne par-dessus un autre sans ecraser le reste.
///
/// C'est ce type, nomme et stocke dans une `Stylesheet`, qui constitue le
/// "composant reutilisable" du systeme : on le definit une fois sous un nom
/// de classe, et n'importe quelle balise .rsh peut l'appliquer via `.nom`.
/// Pas `Copy` (les listes de pistes de grille sont des `Vec`) : `Clone`
/// suffit, ce type n'est jamais partage par valeur plusieurs fois de suite.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub margin: Option<f32>,
    pub padding: Option<f32>,
    pub radius: Option<f32>,
    pub background: Option<Color>,
    pub color: Option<Color>,
    pub font_size: Option<f32>,
    pub font_weight: Option<f32>,

    // --- Flex/grid (voir `layout::models::layout_props::LayoutProps`) -
    // `None` = pas de regle rsC pour cette propriete, `layout_of`/`layout_literal`
    // (compiler::services::{interpreter,codegen}) laissent alors la valeur
    // par defaut de `LayoutProps::new` inchangee.
    pub display: Option<DisplayMode>,
    pub flex_direction: Option<FlexDirection>,
    pub flex_wrap: Option<bool>,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub gap: Option<f32>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<f32>,
    pub grid_template_columns: Option<Vec<Track>>,
    pub grid_template_rows: Option<Vec<Track>>,
    pub grid_column: Option<usize>,
    pub grid_column_span: Option<usize>,
    pub grid_row: Option<usize>,
    pub grid_row_span: Option<usize>,
    /// `overflow`/`overflow-y` (voir `LayoutProps::overflow`).
    pub overflow: Option<Overflow>,

    // --- Decoration (voir `ui::models::decoration::Decoration`).
    /// Degrade (`background` / `background-image: linear-gradient(...)`) -
    /// prend le pas sur `background` (couleur unie) quand il est defini.
    pub fill: Option<Fill>,
    /// `border` / `border-width` : les 4 cotes a la fois.
    pub border_width: Option<f32>,
    /// `border-top` / `border-right` / `border-bottom` / `border-left` (et
    /// leurs `-width`), dans cet ordre. Un cote donne l'emporte toujours sur
    /// `border_width`, quel que soit l'ordre des regles (voir `border_widths`).
    pub border_sides: [Option<f32>; 4],
    pub border_color: Option<Color>,
    /// `box-shadow: none` donne `Some(None)` : annule une ombre heritee de
    /// la cascade.
    pub shadow: Option<Option<Shadow>>,
    pub opacity: Option<f32>,
    /// `background-image: url(...)`.
    pub background_image: Option<crate::ui::models::decoration::BackgroundImage>,
    /// `border-style` (plein si absent).
    pub border_style: Option<azure_engine::rendering::models::paint::BorderStyle>,
    /// `transition` (couleurs de survol / focus).
    pub transition: Option<crate::ui::models::transition::Transition>,
    /// `align-content` (flex sur plusieurs lignes).
    pub align_content: Option<crate::layout::models::css_box::AlignContent>,
    /// `overflow-x` (defilement horizontal).
    pub overflow_x: Option<Overflow>,

    /// Proprietes CSS "web" (voir `WebStyle`).
    pub web: crate::style::models::web_style::WebStyle,
}

impl Style {
    pub fn new() -> Style {
        Style::default()
    }

    /// Epaisseur finale de chaque cote : le cote s'il est donne, sinon
    /// `border` / `border-width`, sinon 0.
    pub fn border_widths(&self) -> BorderWidths {
        let side = |i: usize| self.border_sides[i].or(self.border_width).unwrap_or(0.0).max(0.0);
        BorderWidths { top: side(0), right: side(1), bottom: side(2), left: side(3) }
    }

    /// Fusionne `other` par-dessus `self` : toute propriete definie dans
    /// `other` gagne, comme la derniere regle CSS appliquee a un element.
    /// Utilise pour empiler plusieurs classes sur une meme balise
    /// (`<container.card.highlighted>`) ou pour appliquer un style de
    /// composant par-dessus les valeurs par defaut d'un element.
    pub fn merge(&self, other: &Style) -> Style {
        Style {
            x: other.x.or(self.x),
            y: other.y.or(self.y),
            width: other.width.or(self.width),
            height: other.height.or(self.height),
            margin: other.margin.or(self.margin),
            padding: other.padding.or(self.padding),
            radius: other.radius.or(self.radius),
            background: other.background.or(self.background),
            color: other.color.or(self.color),
            font_size: other.font_size.or(self.font_size),
            font_weight: other.font_weight.or(self.font_weight),
            display: other.display.or(self.display),
            flex_direction: other.flex_direction.or(self.flex_direction),
            flex_wrap: other.flex_wrap.or(self.flex_wrap),
            justify_content: other.justify_content.or(self.justify_content),
            align_items: other.align_items.or(self.align_items),
            gap: other.gap.or(self.gap),
            flex_grow: other.flex_grow.or(self.flex_grow),
            flex_shrink: other.flex_shrink.or(self.flex_shrink),
            flex_basis: other.flex_basis.or(self.flex_basis),
            grid_template_columns: other.grid_template_columns.clone().or_else(|| self.grid_template_columns.clone()),
            grid_template_rows: other.grid_template_rows.clone().or_else(|| self.grid_template_rows.clone()),
            grid_column: other.grid_column.or(self.grid_column),
            grid_column_span: other.grid_column_span.or(self.grid_column_span),
            grid_row: other.grid_row.or(self.grid_row),
            grid_row_span: other.grid_row_span.or(self.grid_row_span),
            overflow: other.overflow.or(self.overflow),
            fill: other.fill.clone().or_else(|| self.fill.clone()),
            border_width: other.border_width.or(self.border_width),
            border_sides: std::array::from_fn(|i| other.border_sides[i].or(self.border_sides[i])),
            border_color: other.border_color.or(self.border_color),
            shadow: other.shadow.or(self.shadow),
            opacity: other.opacity.or(self.opacity),
            border_style: other.border_style.or(self.border_style),
            background_image: other.background_image.clone().or_else(|| self.background_image.clone()),
            transition: other.transition.or(self.transition),
            align_content: other.align_content.or(self.align_content),
            overflow_x: other.overflow_x.or(self.overflow_x),
            web: self.web.merge(&other.web),
        }
    }
}
