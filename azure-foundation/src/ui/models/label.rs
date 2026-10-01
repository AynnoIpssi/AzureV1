use crate::ui::models::decoration::Decoration;
use crate::layout::models::layout_props::LayoutProps;
use azure_engine::rendering::models::color::Color;

pub struct Label{
    pub layout: LayoutProps,
    pub text: String,
    pub color: Color,
    pub font_size: f32,
    pub weight: f32,
    pub decoration: Decoration,
    /// Mise en forme web du texte (voir `TextStyle`), `None` a l'ancienne.
    pub text_style: Option<crate::ui::models::text_style::TextStyle>,
    /// Partie selectionnee a la souris (indices de caracteres, debut < fin),
    /// surlignee au dessin et copiee par Ctrl+C (voir
    /// `ui::services::interact::select`).
    pub selection: Option<(usize, usize)>,
}

impl Label {
    pub fn new(layout: LayoutProps, text: String, color: Color, font_size: f32, weight: f32) -> Label {
        Label{
            layout,
            text,
            color,
            font_size,
            weight,
            decoration: Decoration::default(),
            text_style: None,
            selection: None,
        }
    }
}
