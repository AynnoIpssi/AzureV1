use crate::ui::models::decoration::Decoration;
use crate::layout::models::layout_props::LayoutProps;

/// `src` est bien rempli depuis l'attribut `src="..."` d'une balise
/// `<video>` (voir `compiler::rsh::services::lexer`), mais reste inutilise
/// au rendu (voir `draw_ui::draw_video`) : decoder une video "sans aucune
/// dependance" (H.264/VP9/...) est d'un tout autre ordre de grandeur que
/// decoder une image fixe (voir `codec::png`) - hors de portee ici. Garde
/// pour que le champ existe deja le jour ou ca change.
pub struct Video {
    pub layout: LayoutProps,
    pub src: String,
    pub decoration: Decoration,
}

impl Video {
    pub fn new(layout: LayoutProps, src: String) -> Video {
        Video { layout, src, decoration: Decoration::default() }
    }
}
