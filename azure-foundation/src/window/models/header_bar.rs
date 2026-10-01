// La barre d'en-tete auto-dessinee (pas de decoration cote compositeur -
// voir `window::services::draw_header`) : icone d'application, titre, et
// les 3 boutons classiques (reduire/plein ecran/fermer). Volontairement
// simple pour l'instant - une seule fenetre, pas d'onglets ni de menu -
// a completer une fois le rooter (multi-pages/multi-fenetres) integre.
use azure_engine::codec::png::DecodedImage;
use std::rc::Rc;

pub const HEADER_HEIGHT: u32 = 28;
/// Largeur d'un bouton dans la disposition a droite (icones sur fond de survol).
pub const BUTTON_WIDTH: u32 = 46;
/// Disposition a gauche (feux tricolores, comme sur macOS) : cercles de
/// 12 px, 8 px d'ecart, a 8 px du bord - chaque bouton occupe une case de
/// `TRAFFIC_STEP` px de large a partir de `TRAFFIC_START`.
pub const TRAFFIC_START: u32 = 8;
pub const TRAFFIC_STEP: u32 = 20;

/// Les 3 boutons de la barre d'en-tete - voir `ButtonLayout` pour leur
/// ordre/cote reel, `button_at`/`button_rects`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderButton {
    Minimize,
    Fullscreen,
    Close,
}

/// Disposition des boutons - determinee par
/// `window::services::system_theme::detect` a partir du reglage
/// `button-layout` du bureau (gsettings `org.gnome.desktop.wm.preferences`,
/// meme cle que GNOME utilise pour les dispositions "a la mac", boutons a
/// gauche dans l'ordre close/minimize/maximize) plutot que figee en dur -
/// voir `window::services::draw_header::draw_traffic_light`, qui bascule
/// sur un rendu "feux tricolores" quand `on_left` est vrai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonLayout {
    pub order: Vec<HeaderButton>,
    pub on_left: bool,
}

impl ButtonLayout {
    /// Disposition par defaut (Windows/GNOME classique) : minimiser, plein
    /// ecran, fermer, a droite - utilisee quand le bureau ne dit rien de
    /// particulier sur `button-layout` (voir `system_theme::detect`).
    pub fn default_right() -> ButtonLayout {
        ButtonLayout {
            order: vec![HeaderButton::Minimize, HeaderButton::Fullscreen, HeaderButton::Close],
            on_left: false,
        }
    }

    /// Disposition d'Azure : fermer, reduire, agrandir, a gauche et
    /// colles, en feux tricolores - comme sur macOS.
    pub fn mac() -> ButtonLayout {
        ButtonLayout {
            order: vec![HeaderButton::Close, HeaderButton::Minimize, HeaderButton::Fullscreen],
            on_left: true,
        }
    }

    /// Largeur d'un bouton dans cette disposition.
    pub fn button_width(&self) -> u32 {
        if self.on_left { TRAFFIC_STEP } else { BUTTON_WIDTH }
    }

    /// Largeur occupee par tous les boutons (marge comprise a gauche).
    pub fn width(&self) -> u32 {
        let buttons = self.order.len() as u32 * self.button_width();
        if self.on_left { TRAFFIC_START + buttons } else { buttons }
    }
}

/// Le contenu (pas l'etat d'interaction, qui vit dans `window::models::window::LoopState`
/// - survol, plein ecran courant) de la barre d'en-tete.
pub struct HeaderBar {
    pub title: String,
    /// Decode une seule fois au demarrage (voir `AzureWindow::icon`), pas
    /// a chaque redessin - `Rc`, pas une image possedee, pour rester bon
    /// marche a cloner si besoin.
    pub icon: Option<Rc<DecodedImage>>,
    pub layout: ButtonLayout,
    /// Palette sombre ou claire - voir `window::services::system_theme::detect`
    /// (reglage `color-scheme`/`gtk-theme` du bureau) et
    /// `draw_header::palette`.
    pub dark: bool,
}

impl HeaderBar {
    pub fn new(title: String, icon: Option<Rc<DecodedImage>>, layout: ButtonLayout, dark: bool) -> HeaderBar {
        HeaderBar { title, icon, layout, dark }
    }
}

// Position `(x, y, largeur, hauteur)` du bouton a l'index `index` de
// `layout.order`, sans se soucier de savoir si elle deborde de la fenetre
// (voir `button_at`, qui applique lui la garde "fenetre trop etroite") -
// a droite, les boutons se comptent depuis le bord droit (le dernier de
// `order` est le plus a droite, comme avant cette generalisation) ; a
// gauche, depuis le bord gauche dans l'ordre de `order`.
fn button_start(window_width: u32, layout: &ButtonLayout, index: usize) -> u32 {
    if layout.on_left {
        TRAFFIC_START + index as u32 * TRAFFIC_STEP
    } else {
        let count = layout.order.len() as u32;
        window_width.saturating_sub((count - index as u32) * BUTTON_WIDTH)
    }
}

/// Boite `(x, y, largeur, hauteur)` de chaque bouton de `layout.order`, dans
/// cet ordre - point d'entree commun a `draw_header` (qui dessine chaque
/// boite) et `button_at` (qui cherche celle qui contient un point), pour ne
/// pas dupliquer la geometrie entre les deux.
pub fn button_rects(window_width: u32, layout: &ButtonLayout) -> Vec<(HeaderButton, u32, u32, u32, u32)> {
    layout.order.iter().enumerate()
        .map(|(i, button)| (*button, button_start(window_width, layout, i), 0, layout.button_width(), HEADER_HEIGHT))
        .collect()
}

/// Le bouton de la barre d'en-tete sous `(x, y)` (coordonnees fenetre
/// absolues), ou `None` si `(x, y)` n'est pas dans la barre d'en-tete, ou
/// si la fenetre est trop etroite pour afficher tous les boutons de
/// `layout.order` - dans ce cas, aucun bouton n'est cliquable plutot que
/// de risquer d'en faire se chevaucher.
pub fn button_at(window_width: u32, layout: &ButtonLayout, x: i32, y: i32) -> Option<HeaderButton> {
    let count = layout.order.len() as u32;
    if x < 0 || y < 0 || y as u32 >= HEADER_HEIGHT || count == 0 || window_width < layout.width() {
        return None;
    }
    let x = x as u32;
    button_rects(window_width, layout)
        .into_iter()
        .find(|(_, bx, _, bw, _)| x >= *bx && x < *bx + *bw)
        .map(|(button, ..)| button)
}

/// La boite de CONTENU sous la barre d'en-tete - c'est ce rectangle, pas
/// `(0, 0, window_width, window_height)` directement, qu'il faut
/// transmettre comme boite racine a `ui::services::draw_ui`/
/// `event::services::dispatch` pour que le contenu de l'application
/// commence bien apres l'en-tete plutot que de se dessiner derriere.
/// Independant de `ButtonLayout` : la hauteur de l'en-tete ne change pas
/// selon le cote des boutons.
pub fn content_box(window_width: u32, window_height: u32) -> (u32, u32, u32, u32) {
    (0, HEADER_HEIGHT, window_width, window_height.saturating_sub(HEADER_HEIGHT))
}
