use crate::layout::models::layout_props::LayoutProps;
use crate::ui::models::ui_node::UiNode;
use crate::ui::models::decoration::Decoration;
use azure_engine::rendering::models::color::Color;

pub struct Container{
    pub layout: LayoutProps,
    pub background: Color,
    pub children: Vec<UiNode>,
    /// Decalage de defilement courant, en PIXELS (pas en lignes, contrairement
    /// a `TextArea::scroll_offset` - les enfants d'un `Container` vivent dans
    /// des boites arbitraires, pas du texte replie) - sans effet tant que
    /// `layout.content_height` vaut `None` (voir sa doc). Mutee par
    /// `ui::services::interact::scroll_at`, lue par
    /// `ui::services::draw_ui::draw_container` et par `interact` lui-meme
    /// pour que rendu et hit-test restent toujours d'accord sur la position
    /// reelle des enfants.
    pub scroll_offset: u32,
    /// Ou le defilement va : la molette deplace cette cible, et
    /// `ui::services::interact::animate_scroll` fait glisser `scroll_offset`
    /// vers elle a chaque tic (defilement fluide).
    pub scroll_target: u32,
    /// Defilement horizontal (`overflow-x`), en pixels, et sa cible.
    pub scroll_x: u32,
    pub scroll_x_target: u32,
    pub decoration: Decoration,
}

impl Container {
    pub fn new(layout: LayoutProps, background: Color, children: Vec<UiNode>) -> Container {
        Container { layout, background, children, scroll_offset: 0, scroll_target: 0, scroll_x: 0, scroll_x_target: 0, decoration: Decoration::default() }
    }
}
