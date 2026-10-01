// Traduction d'un `Style` resolu depuis rsC vers le modele web des widgets :
// boite CSS (`CssBox`) et mise en forme du texte (`TextStyle`). Partage par
// l'interpreteur et le codegen, pour que les deux construisent exactement
// les memes valeurs.
use crate::layout::models::css_box::{CssBox, CssDisplay, Length, Sides};
use crate::layout::models::layout_props::DisplayMode;
use crate::style::models::style::Style;
use crate::style::models::web_style::{LineHeight, TextAlign, WhiteSpace};
use crate::ui::models::text_style::TextStyle;
use crate::ui::services::fonts::font_for;

pub fn css_box(style: &Style) -> CssBox {
    let web = &style.web;
    let sides = |values: &[Option<Length>; 4]| Sides {
        top: values[0].unwrap_or(Length::Px(0.0)),
        right: values[1].unwrap_or(Length::Px(0.0)),
        bottom: values[2].unwrap_or(Length::Px(0.0)),
        left: values[3].unwrap_or(Length::Px(0.0)),
    };
    let display = web.display.unwrap_or(match style.display {
        Some(DisplayMode::Flex) => CssDisplay::Flex,
        Some(DisplayMode::Grid) => CssDisplay::Grid,
        _ => CssDisplay::Block,
    });
    let defaults = CssBox::default();
    CssBox {
        display,
        width: web.width.unwrap_or(Length::Auto),
        height: web.height.unwrap_or(Length::Auto),
        min_width: web.min_width.unwrap_or(Length::Auto),
        min_height: web.min_height.unwrap_or(Length::Auto),
        max_width: web.max_width.unwrap_or(Length::Auto),
        max_height: web.max_height.unwrap_or(Length::Auto),
        margin: sides(&web.margin),
        padding: sides(&web.padding),
        border: style.border_widths(),
        border_box: web.border_box.unwrap_or(false),
        flex_direction: style.flex_direction.unwrap_or(defaults.flex_direction),
        flex_wrap: style.flex_wrap.unwrap_or(false),
        justify_content: style.justify_content.unwrap_or(defaults.justify_content),
        align_items: style.align_items.unwrap_or(defaults.align_items),
        row_gap: web.row_gap.unwrap_or(Length::Px(0.0)),
        column_gap: web.column_gap.unwrap_or(Length::Px(0.0)),
        grid_template_columns: web.grid_template_columns.clone().unwrap_or_default(),
        grid_template_rows: web.grid_template_rows.clone().unwrap_or_default(),
        flex_grow: style.flex_grow.unwrap_or(0.0),
        flex_shrink: style.flex_shrink.unwrap_or(1.0),
        flex_basis: web.flex_basis.unwrap_or(Length::Auto),
        align_self: web.align_self,
        grid_column: style.grid_column,
        grid_column_span: style.grid_column_span.unwrap_or(1),
        grid_row: style.grid_row,
        grid_row_span: style.grid_row_span.unwrap_or(1),
        fixed: web.fixed.unwrap_or(false),
        inset: Sides { top: web.inset[0].unwrap_or(Length::Auto), right: web.inset[1].unwrap_or(Length::Auto), bottom: web.inset[2].unwrap_or(Length::Auto), left: web.inset[3].unwrap_or(Length::Auto) },
        z_index: web.z_index.unwrap_or(0),
        align_content: style.align_content.unwrap_or(defaults.align_content),
        position: web.position.unwrap_or_default(),
        ..defaults
    }
}

/// Mise en forme du texte d'un element dont la taille de police finale est
/// `font_size`.
pub fn text_style(style: &Style, font_size: f32) -> TextStyle {
    let web = &style.web;
    TextStyle::new(
        font_for(web.font_family.as_deref()),
        web.line_height.unwrap_or(LineHeight::Normal).resolve(font_size),
        web.text_align.unwrap_or(TextAlign::Left),
        web.white_space.unwrap_or(WhiteSpace::Normal),
    )
    .with_options(web.italic.unwrap_or(false), web.letter_spacing.unwrap_or(0.0), web.text_decoration.unwrap_or_default())
}

/// Le texte tel qu'un navigateur l'afficherait : hors `white-space: pre`,
/// chaque suite d'espaces/retours a la ligne (l'indentation du fichier
/// rsH) devient un seul espace, et les bords sont retires.
pub fn collapse_whitespace(text: &str, white_space: WhiteSpace) -> String {
    match white_space {
        WhiteSpace::Pre => text.to_string(),
        _ => text.split_whitespace().collect::<Vec<_>>().join(" "),
    }
}
