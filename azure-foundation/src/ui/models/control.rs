// Les champs de formulaire autres que le texte : case a cocher, bouton
// radio, interrupteur, curseur, barre de progression, liste deroulante,
// choix segmente. Un seul type de noeud (`kind` dit lequel) : ils partagent
// la meme valeur (`checked` / `value` / `selected`), la meme mise en forme
// (couleur d'accent, texte) et les memes regles de clic.
use crate::layout::models::layout_props::LayoutProps;
use crate::ui::models::decoration::Decoration;
use azure_engine::rendering::models::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    Checkbox,
    Radio,
    Switch,
    Slider,
    Progress,
    Select,
    Segmented,
    /// Note de 1 a `max` (pastilles cliquables).
    Rating,
    /// Surface de dessin facon draw.io (voir `ui::models::toile`).
    Toile,
}

impl ControlKind {
    pub fn from_tag(tag: &str) -> Option<ControlKind> {
        Some(match tag {
            "checkbox" => ControlKind::Checkbox,
            "radio" => ControlKind::Radio,
            "switch" | "toggle" => ControlKind::Switch,
            "slider" | "range" => ControlKind::Slider,
            "progress" => ControlKind::Progress,
            "select" | "dropdown" => ControlKind::Select,
            "segmented" => ControlKind::Segmented,
            "rating" | "stars" => ControlKind::Rating,
            "toile" | "canvas" => ControlKind::Toile,
            _ => return None,
        })
    }

    /// Reagit au clic (tous sauf la barre de progression).
    pub fn interactive(self) -> bool {
        self != ControlKind::Progress
    }
}

pub struct Control {
    pub kind: ControlKind,
    pub layout: LayoutProps,
    pub decoration: Decoration,
    /// `#id` rsH : rapporte par `WindowContext::clicked`, et cle de la
    /// valeur dans `WindowContext::value`.
    pub id: String,
    /// Groupe d'un bouton radio (`name="taille"`) ; la valeur du groupe est
    /// la `value` du radio coche.
    pub name: String,
    /// Texte a cote de la case / de l'interrupteur.
    pub label: String,
    pub checked: bool,
    /// Curseur et progression : valeur entre `min` et `max`, par pas de `step`.
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    /// Radio : ce que vaut le groupe quand ce radio est coche.
    pub radio_value: String,
    /// Liste deroulante et choix segmente : (valeur, texte affiche).
    pub options: Vec<(String, String)>,
    pub selected: usize,
    /// Liste deroulante ouverte.
    pub open: bool,
    /// Curseur en cours de glissement.
    pub dragging: bool,
    pub disabled: bool,
    /// Focalise (Tab, ou clic) : clavier actif (Espace, fleches...).
    pub focused: bool,
    /// Focalise au clavier : contour visible.
    pub focus_ring: bool,
    /// Couleur de l'element coche / rempli (`accent-color` en rsC).
    pub accent: Color,
    /// Couleur de la case, de la piste (`background-color`).
    pub track: Color,
    pub text_color: Color,
    pub font_size: f32,
    /// Le dessin et la vue d'une toile (`ControlKind::Toile`).
    pub toile: Option<Box<crate::ui::models::toile::Toile>>,
}

pub const DEFAULT_ACCENT: Color = Color::new(201, 168, 120, 255);
pub const DEFAULT_TRACK: Color = Color::new(46, 43, 39, 255);
pub const DEFAULT_TEXT: Color = Color::new(231, 229, 225, 255);

impl Control {
    pub fn new(kind: ControlKind, layout: LayoutProps) -> Control {
        Control {
            kind,
            layout,
            decoration: Decoration::default(),
            id: String::new(),
            name: String::new(),
            label: String::new(),
            checked: false,
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            radio_value: String::new(),
            options: Vec::new(),
            selected: 0,
            open: false,
            dragging: false,
            disabled: false,
            focused: false,
            focus_ring: false,
            accent: DEFAULT_ACCENT,
            track: DEFAULT_TRACK,
            text_color: DEFAULT_TEXT,
            font_size: 14.0,
            toile: None,
        }
    }

    /// Place la valeur (bornee, arrondie au pas).
    pub fn set_value(&mut self, value: f64) -> bool {
        let step = if self.step > 0.0 { self.step } else { 1.0 };
        let v = ((value.clamp(self.min, self.max) - self.min) / step).round() * step + self.min;
        let v = v.clamp(self.min, self.max);
        let changed = (v - self.value).abs() > f64::EPSILON;
        self.value = v;
        changed
    }

    /// Part remplie (0..1) du curseur / de la barre.
    pub fn fraction(&self) -> f64 {
        if self.max > self.min { ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0) } else { 0.0 }
    }

    pub fn selected_value(&self) -> Option<&str> {
        self.options.get(self.selected).map(|(value, _)| value.as_str())
    }

    pub fn selected_label(&self) -> &str {
        self.options.get(self.selected).map(|(_, label)| label.as_str()).unwrap_or("")
    }
}
