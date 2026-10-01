use crate::rendering::models::color::Color;

/// Un arret de couleur d'un degrade : la couleur, et sa position de 0.0
/// (debut) a 1.0 (fin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorStop {
    pub color: Color,
    pub position: f32,
}

/// Remplissage d'une boite : une couleur unie, un degrade lineaire (angle
/// en degres, convention CSS : 0 = vers le haut, 90 = vers la droite,
/// 180 = vers le bas) ou un degrade radial (cercle ou ellipse centre sur la
/// boite, du centre vers le coin le plus eloigne). Les couleurs peuvent etre
/// transparentes : tout est melange avec ce qu'il y a dessous.
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid(Color),
    Linear { angle: f32, stops: Vec<ColorStop> },
    Radial { stops: Vec<ColorStop> },
}

/// Ombre (`box-shadow`) : decalage, flou, etalement (en pixels) et
/// couleur, generalement transparente. `inset` : a l'interieur de la boite
/// (par-dessus le fond, sous la bordure), au lieu d'autour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
    pub inset: bool,
}

/// `border-style` : trait plein, tirets, points, double trait, ou rien.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
    Double,
    None,
}

/// Epaisseur de la bordure de chaque cote, en pixels (`border-bottom: 1px`
/// seul -> seul `bottom` est non nul). Une seule couleur pour les 4 cotes
/// (voir `BoxStyle::border_color`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BorderWidths {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl BorderWidths {
    pub const fn uniform(width: f32) -> BorderWidths {
        BorderWidths { top: width, right: width, bottom: width, left: width }
    }

    pub fn max(&self) -> f32 {
        self.top.max(self.right).max(self.bottom).max(self.left)
    }

    pub fn is_zero(&self) -> bool {
        self.max() <= 0.0
    }

    pub fn is_uniform(&self) -> bool {
        self.top == self.right && self.top == self.bottom && self.top == self.left
    }
}

/// Tout ce qui decore une boite (voir
/// `services::shapes::styled_box::draw_box`) : ombre, remplissage, coins
/// arrondis et bordure.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxStyle {
    pub fill: Fill,
    pub radius: f32,
    pub border: BorderWidths,
    pub border_color: Color,
    pub border_style: BorderStyle,
    pub shadow: Option<Shadow>,
}

impl BoxStyle {
    pub fn solid(color: Color) -> BoxStyle {
        BoxStyle { fill: Fill::Solid(color), radius: 0.0, border: BorderWidths::uniform(0.0), border_color: Color::new(0, 0, 0, 0), border_style: BorderStyle::Solid, shadow: None }
    }
}
