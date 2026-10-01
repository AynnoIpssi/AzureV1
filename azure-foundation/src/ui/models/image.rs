use crate::ui::models::decoration::Decoration;
use crate::layout::models::layout_props::LayoutProps;

/// `src` (rempli depuis l'attribut `src="..."` d'une balise `<image>` -
/// voir `compiler::rsh::services::lexer`) est un chemin de fichier PNG,
/// relatif au repertoire de travail comme un chemin de police (voir
/// `ui::services::draw_ui::FONT_PATH`). Vide = rien a dessiner (voir
/// `draw_ui::draw_image`), pas une erreur en soi.
pub struct Image {
    pub layout: LayoutProps,
    pub src: String,
    pub decoration: Decoration,
}

impl Image {
    pub fn new(layout: LayoutProps, src: String) -> Image {
        Image { layout, src, decoration: Decoration::default() }
    }
}
