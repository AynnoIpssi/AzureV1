use crate::layout::models::layout_props::LayoutProps;
use crate::ui::models::button::Button;
use crate::ui::models::container::Container;
use crate::ui::models::control::Control;
use crate::ui::models::image::Image;
use crate::ui::models::label::Label;
use crate::ui::models::textarea::TextArea;
use crate::ui::models::video::Video;

// Arbre de widgets construit par le code genere a partir d'un .rsh.
pub enum UiNode {
    Container(Container),
    Label(Label),
    Button(Button),
    Image(Image),
    Video(Video),
    TextArea(TextArea),
    Control(Control),
}

impl UiNode {
    /// Le `LayoutProps` de ce nœud, quel que soit son type concret - utilise
    /// par `layout::managers::layout_manager::resolve_children` (qui a
    /// besoin des `LayoutProps` de TOUS les enfants d'un conteneur a la fois
    /// pour le flex/grid) et par les traversees de `ui::services::draw_ui`/
    /// `ui::services::interact` pour resoudre la boîte d'un nœud racine.
    pub fn layout(&self) -> &LayoutProps {
        match self {
            UiNode::Container(c) => &c.layout,
            UiNode::Label(l) => &l.layout,
            UiNode::Button(b) => &b.layout,
            UiNode::Image(i) => &i.layout,
            UiNode::Video(v) => &v.layout,
            UiNode::TextArea(t) => &t.layout,
            UiNode::Control(c) => &c.layout,
        }
    }

    pub fn decoration_mut(&mut self) -> &mut crate::ui::models::decoration::Decoration {
        match self {
            UiNode::Container(c) => &mut c.decoration,
            UiNode::Label(l) => &mut l.decoration,
            UiNode::Button(b) => &mut b.decoration,
            UiNode::Image(i) => &mut i.decoration,
            UiNode::Video(v) => &mut v.decoration,
            UiNode::TextArea(t) => &mut t.decoration,
            UiNode::Control(c) => &mut c.decoration,
        }
    }

    /// L'habillage rsC de ce nœud (voir `Decoration`).
    pub fn decoration(&self) -> &crate::ui::models::decoration::Decoration {
        match self {
            UiNode::Container(c) => &c.decoration,
            UiNode::Label(l) => &l.decoration,
            UiNode::Button(b) => &b.decoration,
            UiNode::Image(i) => &i.decoration,
            UiNode::Video(v) => &v.decoration,
            UiNode::TextArea(t) => &t.decoration,
            UiNode::Control(c) => &c.decoration,
        }
    }
}
