use crate::ui::models::decoration::Decoration;
use crate::layout::models::layout_props::LayoutProps;
use azure_engine::rendering::models::color::Color;

pub struct Button {
    pub layout: LayoutProps,
    pub color: Color,
    pub state: bool,
    pub text: String,
    /// Couleur de remplissage a utiliser a la place de `color` quand la
    /// souris survole le bouton - resolue depuis une regle rsC
    /// `button:hover { background-color: ...; }` (voir
    /// `compiler::services::codegen::resolve_pseudo_background`), `None`
    /// par defaut (donc via l'ancien systeme `.style`, qui n'a pas de
    /// notion de pseudo-classe). Toujours `None` pour un `Button` construit
    /// directement (par ex. dans un test) : a fixer explicitement si
    /// besoin, comme `state`.
    pub hover_color: Option<Color>,
    /// Couleur du texte du bouton - resolue depuis `color` en rsC (voir
    /// `compiler::services::codegen`), `None` par defaut (voir
    /// `ui::services::draw_ui::draw_button`, qui retombe alors sur du blanc
    /// fixe, comme avant l'ajout de ce champ).
    pub text_color: Option<Color>,
    /// L'`#id` rsH du bouton (`<button#tab-rsh>`), vide si aucun - c'est
    /// lui que `WindowContext::clicked` rapporte a `AzureWindow::on_click`
    /// pour savoir QUEL bouton a ete clique.
    pub id: String,
    /// Taille/graisse du texte (`font-size`/`font-weight` en rsC). Le texte
    /// est centre dans le bouton.
    pub font_size: f32,
    pub font_weight: f32,
    pub decoration: Decoration,
    /// Mise en forme web du texte (police, interligne) ; `None` a l'ancienne.
    pub text_style: Option<crate::ui::models::text_style::TextStyle>,
    /// Couleur du texte au survol (`button:hover { color: ... }`).
    pub hover_text_color: Option<Color>,
    /// Fond et texte pendant l'appui (`button:active` en rsC). Sans regle
    /// `:active`, le fond est simplement enfonce (voir
    /// `draw_ui::pressed`).
    pub active_color: Option<Color>,
    pub active_text_color: Option<Color>,
    /// Focalise (Tab, ou clic) : Entree / Espace l'activent.
    pub focused: bool,
    /// Focalise AU CLAVIER : un contour le montre (pas apres un clic souris).
    pub focus_ring: bool,
}

impl Button {
    pub fn new(layout: LayoutProps, color: Color, state: bool, text: String) -> Button{
        Button {
            layout,
            color,
            state,
            text,
            hover_color: None,
            text_color: None,
            id: String::new(),
            font_size: 16.0,
            font_weight: 500.0,
            decoration: Decoration::default(),
            text_style: None,
            hover_text_color: None,
            active_color: None,
            active_text_color: None,
            focused: false,
            focus_ring: false,
        }
    }
}
