use crate::layout::models::css_box::{CssDisplay, CssTrack, Length};
use crate::layout::models::layout_props::AlignItems;

/// `text-decoration` : traits dessines avec le texte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextDecoration {
    pub underline: bool,
    pub line_through: bool,
    pub overline: bool,
}

impl TextDecoration {
    pub fn any(&self) -> bool {
        self.underline || self.line_through || self.overline
    }
}

/// `line-height` : `normal`, un multiple de la taille de police (`1.6`) ou
/// une hauteur fixe en px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
    Normal,
    Multiplier(f32),
    Px(f32),
}

impl LineHeight {
    /// Hauteur d'une ligne en px pour une taille de police donnee.
    pub fn resolve(self, font_size: f32) -> f32 {
        match self {
            LineHeight::Normal => font_size * 1.25,
            LineHeight::Multiplier(m) => font_size * m,
            LineHeight::Px(px) => px,
        }
    }
}

/// `white-space` : `Normal` regroupe les espaces et revient a la ligne,
/// `Nowrap` regroupe sans revenir a la ligne, `Pre` garde le texte tel quel
/// (indentation du code comprise) sans revenir a la ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhiteSpace {
    #[default]
    Normal,
    Nowrap,
    Pre,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Les proprietes CSS "web" d'un `Style` (unites, cotes, tailles min/max,
/// texte...), toutes optionnelles pour la cascade : `None` = aucune regle.
/// Ne concerne que les arbres construits depuis rsC (voir
/// `layout::models::css_box::CssBox`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WebStyle {
    pub display: Option<CssDisplay>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub min_width: Option<Length>,
    pub min_height: Option<Length>,
    pub max_width: Option<Length>,
    pub max_height: Option<Length>,
    pub margin: [Option<Length>; 4],
    pub padding: [Option<Length>; 4],
    pub border_box: Option<bool>,
    pub row_gap: Option<Length>,
    pub column_gap: Option<Length>,
    pub flex_basis: Option<Length>,
    pub align_self: Option<AlignItems>,
    pub grid_template_columns: Option<Vec<CssTrack>>,
    pub grid_template_rows: Option<Vec<CssTrack>>,
    /// `position: fixed` : place par rapport a la fenetre, par-dessus la page.
    pub fixed: Option<bool>,
    /// `position: relative | absolute | static`.
    pub position: Option<crate::layout::models::css_box::CssPosition>,
    /// `top`, `right`, `bottom`, `left` d'un element `fixed`.
    pub inset: [Option<Length>; 4],
    pub z_index: Option<i32>,

    // --- Texte (heritees par les enfants, comme en CSS).
    pub font_family: Option<String>,
    pub line_height: Option<LineHeight>,
    pub text_align: Option<TextAlign>,
    pub white_space: Option<WhiteSpace>,
    /// `font-style: italic | oblique`.
    pub italic: Option<bool>,
    /// `letter-spacing`, en px.
    pub letter_spacing: Option<f32>,
    /// `text-decoration` (dessine aussi sous le texte des enfants, comme en
    /// CSS, d'ou sa place parmi les proprietes transmises).
    pub text_decoration: Option<TextDecoration>,
    /// `visibility: hidden` -> `Some(false)`.
    pub visible: Option<bool>,
    /// `cursor:`.
    pub cursor: Option<crate::cursor::models::cursor_kind::CursorKind>,
    /// `scrollbar-color:` (poignee des barres de defilement ; heritee,
    /// comme en CSS).
    pub scrollbar_color: Option<azure_engine::rendering::models::color::Color>,
}

impl WebStyle {
    pub fn merge(&self, other: &WebStyle) -> WebStyle {
        let sides = |a: &[Option<Length>; 4], b: &[Option<Length>; 4]| [b[0].or(a[0]), b[1].or(a[1]), b[2].or(a[2]), b[3].or(a[3])];
        WebStyle {
            display: other.display.or(self.display),
            width: other.width.or(self.width),
            height: other.height.or(self.height),
            min_width: other.min_width.or(self.min_width),
            min_height: other.min_height.or(self.min_height),
            max_width: other.max_width.or(self.max_width),
            max_height: other.max_height.or(self.max_height),
            margin: sides(&self.margin, &other.margin),
            padding: sides(&self.padding, &other.padding),
            border_box: other.border_box.or(self.border_box),
            row_gap: other.row_gap.or(self.row_gap),
            column_gap: other.column_gap.or(self.column_gap),
            flex_basis: other.flex_basis.or(self.flex_basis),
            align_self: other.align_self.or(self.align_self),
            grid_template_columns: other.grid_template_columns.clone().or_else(|| self.grid_template_columns.clone()),
            grid_template_rows: other.grid_template_rows.clone().or_else(|| self.grid_template_rows.clone()),
            fixed: other.fixed.or(self.fixed),
            position: other.position.or(self.position),
            inset: sides(&self.inset, &other.inset),
            z_index: other.z_index.or(self.z_index),
            font_family: other.font_family.clone().or_else(|| self.font_family.clone()),
            line_height: other.line_height.or(self.line_height),
            text_align: other.text_align.or(self.text_align),
            white_space: other.white_space.or(self.white_space),
            italic: other.italic.or(self.italic),
            letter_spacing: other.letter_spacing.or(self.letter_spacing),
            text_decoration: other.text_decoration.or(self.text_decoration),
            visible: other.visible.or(self.visible),
            cursor: other.cursor.or(self.cursor),
            scrollbar_color: other.scrollbar_color.or(self.scrollbar_color),
        }
    }

    /// Seulement les proprietes que CSS fait heriter d'un parent a ses
    /// enfants (le texte).
    pub fn inherited(&self) -> WebStyle {
        WebStyle {
            font_family: self.font_family.clone(),
            line_height: self.line_height,
            text_align: self.text_align,
            white_space: self.white_space,
            italic: self.italic,
            letter_spacing: self.letter_spacing,
            text_decoration: self.text_decoration,
            visible: self.visible,
            cursor: self.cursor,
            scrollbar_color: self.scrollbar_color,
            ..WebStyle::default()
        }
    }
}
