use crate::style::models::style::Style;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::{BorderWidths, BoxStyle, Fill, Shadow};

/// `background-size`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackgroundSize {
    /// Taille naturelle de l'image.
    Auto,
    /// Couvre toute la boite (l'image peut etre coupee).
    Cover,
    /// Tient entiere dans la boite.
    Contain,
    /// Largeur et hauteur en px (`None` : proportionnelle a l'autre).
    Px(Option<f32>, Option<f32>),
    /// Largeur et hauteur en % de la boite.
    Percent(Option<f32>, Option<f32>),
}

/// `background-image: url(...)` avec sa taille et sa position (0 = gauche /
/// haut, 1 = droite / bas, comme les pourcentages de `background-position`).
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundImage {
    pub src: String,
    pub size: BackgroundSize,
    pub position: (f32, f32),
}

impl BackgroundImage {
    /// Boite ou dessiner l'image (taille `image`) dans `own` (x, y, l, h).
    pub fn placement(&self, image: (u32, u32), own: (i32, i32, u32, u32)) -> (i32, i32, u32, u32) {
        let (iw, ih) = (image.0.max(1) as f32, image.1.max(1) as f32);
        let (bw, bh) = (own.2 as f32, own.3 as f32);
        let (w, h) = match self.size {
            BackgroundSize::Auto => (iw, ih),
            BackgroundSize::Cover => {
                let k = (bw / iw).max(bh / ih);
                (iw * k, ih * k)
            }
            BackgroundSize::Contain => {
                let k = (bw / iw).min(bh / ih);
                (iw * k, ih * k)
            }
            BackgroundSize::Px(w, h) | BackgroundSize::Percent(w, h) => {
                let pct = matches!(self.size, BackgroundSize::Percent(..));
                let w = w.map(|v| if pct { bw * v / 100.0 } else { v });
                let h = h.map(|v| if pct { bh * v / 100.0 } else { v });
                match (w, h) {
                    (Some(w), Some(h)) => (w, h),
                    (Some(w), None) => (w, ih * w / iw),
                    (None, Some(h)) => (iw * h / ih, h),
                    (None, None) => (iw, ih),
                }
            }
        };
        let x = own.0 as f32 + (bw - w) * self.position.0;
        let y = own.1 as f32 + (bh - h) * self.position.1;
        (x.round() as i32, y.round() as i32, w.round().max(1.0) as u32, h.round().max(1.0) as u32)
    }
}

/// L'habillage visuel d'un widget, rempli depuis rsC : degrade de fond
/// (`background: linear-gradient(...)`), coins arrondis (`border-radius`),
/// bordure (`border`), ombre (`box-shadow`) et opacite de groupe
/// (`opacity`, appliquee au widget ET a tout son contenu). La couleur de
/// fond unie reste portee par chaque widget (`Container::background`,
/// `Button::color`...), parce qu'elle change au survol/focus ; `fill` la
/// remplace quand un degrade est defini.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoration {
    pub fill: Option<Fill>,
    pub radius: f32,
    pub border: BorderWidths,
    pub border_color: Color,
    pub shadow: Option<Shadow>,
    pub opacity: f32,
    /// Degrade sous `:hover` / `:focus` (boutons, zones de saisie), s'il
    /// differe de `fill`.
    pub hover_fill: Option<Fill>,
    /// Degrade sous `:active` (bouton appuye).
    pub active_fill: Option<Fill>,
    pub focus_fill: Option<Fill>,
    /// Infobulle affichee apres un instant de survol (`<tooltip texte="...">`).
    pub tooltip: String,
    /// `background-image: url(...)`, dessine par-dessus la couleur de fond.
    pub background_image: Option<BackgroundImage>,
    /// L'`#id` rsH de l'element : une cible pour les liens d'ancre (bouton
    /// `#ancre-<id>`, voir `interact::scroll_to_anchor`).
    pub anchor: String,
    /// `visibility: hidden` : ni dessine ni cliquable, sa place reste prise.
    pub visible: bool,
    /// `cursor:` (sinon le curseur depend de l'element : main, barre...).
    pub cursor: Option<crate::cursor::models::cursor_kind::CursorKind>,
    /// `border-style`.
    pub border_style: azure_engine::rendering::models::paint::BorderStyle,
    /// `transition` sur les couleurs de survol / focus (voir
    /// `ui::models::transition`).
    pub transition: Option<crate::ui::models::transition::Transition>,
    /// Ou en est cette transition (survol / focus).
    pub anim: crate::ui::models::transition::Animated,
    /// `scrollbar-color` : poignee des barres de defilement de ce conteneur.
    pub scrollbar_color: Option<Color>,
    /// `<draggable id="...">` : se saisit a la souris (voir `interact::drag`).
    pub drag: String,
    /// `<dropzone id="...">` : recoit ce qu'on y lache.
    pub drop: String,
    /// Groupe de survol : l'ancetre `.bloc` de `.bloc:hover .poignee`
    /// (voir `link::is_hover_group`).
    pub hover_group: bool,
    /// Opacite quand le groupe de survol le plus proche au-dessus est
    /// survole, si elle differe de `opacity`.
    pub group_hover_opacity: Option<f32>,
}

impl Default for Decoration {
    fn default() -> Decoration {
        Decoration {
            fill: None,
            radius: 0.0,
            border: BorderWidths::uniform(0.0),
            border_color: Color::new(0, 0, 0, 0),
            shadow: None,
            opacity: 1.0,
            hover_fill: None,
            active_fill: None,
            focus_fill: None,
            tooltip: String::new(),
            background_image: None,
            anchor: String::new(),
            visible: true,
            cursor: None,
            border_style: Default::default(),
            transition: None,
            anim: Default::default(),
            scrollbar_color: None,
            drag: String::new(),
            drop: String::new(),
            hover_group: false,
            group_hover_opacity: None,
        }
    }
}

impl Decoration {
    pub fn from_style(style: &Style) -> Decoration {
        Decoration {
            fill: style.fill.clone(),
            radius: style.radius.unwrap_or(0.0).max(0.0),
            border: style.border_widths(),
            border_color: style.border_color.unwrap_or(Color::new(0, 0, 0, 0)),
            shadow: style.shadow.flatten(),
            opacity: style.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
            hover_fill: None,
            active_fill: None,
            focus_fill: None,
            tooltip: String::new(),
            background_image: style.background_image.clone(),
            anchor: String::new(),
            visible: style.web.visible.unwrap_or(true),
            cursor: style.web.cursor,
            border_style: style.border_style.unwrap_or_default(),
            transition: style.transition,
            anim: Default::default(),
            scrollbar_color: style.web.scrollbar_color,
            drag: String::new(),
            drop: String::new(),
            hover_group: false,
            group_hover_opacity: None,
        }
    }

    /// `true` si rien d'autre que la couleur de fond n'est a dessiner.
    pub fn is_plain(&self) -> bool {
        self.fill.is_none() && self.radius == 0.0 && self.border.is_zero() && self.shadow.is_none()
    }

    /// Ce que le moteur dessine pour une boite dont la couleur de fond du
    /// moment (survol/focus compris) est `background`.
    pub fn box_style(&self, background: Color) -> BoxStyle {
        self.box_style_with(self.fill.clone().unwrap_or(Fill::Solid(background)))
    }

    /// Meme chose avec un remplissage deja choisi (etat survole/focalise).
    pub fn box_style_with(&self, fill: Fill) -> BoxStyle {
        BoxStyle {
            fill,
            radius: self.radius,
            border: self.border,
            border_color: self.border_color,
            border_style: self.border_style,
            shadow: self.shadow,
        }
    }
}
