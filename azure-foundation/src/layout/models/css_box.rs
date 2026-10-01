// Le modele de boite CSS d'un nœud construit depuis rsC : longueurs avec
// unites (px, %, auto), marges et padding par cote, tailles min/max, et le
// mode d'affichage des enfants (flux normal, flex, grid, none). C'est ce que
// lit le moteur de mise en page "web" (`layout::managers::web_layout`),
// comme un navigateur ; l'ancien modele en pourcentages de `LayoutProps`
// reste celui des arbres construits directement en Rust.
use crate::layout::models::layout_props::{AlignItems, FlexDirection, JustifyContent};
use std::cell::{Cell, RefCell};

/// Une longueur CSS deja resolue en px quand c'est possible (`em`/`rem`
/// sont convertis a la lecture de la feuille) : `Auto`, des pixels, ou un
/// pourcentage de la boite de reference (la largeur du bloc conteneur pour
/// `width`, `margin` et `padding`, sa hauteur pour `height` - comme en CSS).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

impl Length {
    /// Valeur en px, `None` pour `auto` ou un pourcentage d'une reference
    /// inconnue (`reference = None`, ex: hauteur d'un parent en `auto`).
    pub fn resolve(self, reference: Option<f32>) -> Option<f32> {
        match self {
            Length::Auto => None,
            Length::Px(v) => Some(v),
            Length::Percent(p) => reference.map(|r| r * p / 100.0),
        }
    }

    /// Comme `resolve`, mais `auto` (et un % sans reference) valent 0 -
    /// pour le padding, et les marges hors centrage.
    pub fn or_zero(self, reference: f32) -> f32 {
        self.resolve(Some(reference)).unwrap_or(0.0)
    }

    pub fn is_auto(self) -> bool {
        self == Length::Auto
    }
}

/// Quatre valeurs, une par cote, dans l'ordre CSS (haut, droite, bas, gauche).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sides {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

impl Sides {
    pub fn all(v: Length) -> Sides {
        Sides { top: v, right: v, bottom: v, left: v }
    }

    pub fn zero() -> Sides {
        Sides::all(Length::Px(0.0))
    }
}

/// `display` : comment ce nœud place ses enfants. `None` le retire
/// entierement (ni dessine, ni mesure, ni cliquable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssDisplay {
    #[default]
    Block,
    Flex,
    Grid,
    None,
}

/// `align-content` : repartition des lignes d'un conteneur flex qui revient
/// a la ligne, sur l'axe secondaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignContent {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    #[default]
    Stretch,
}

/// `position` hors `fixed` (voir `CssBox::fixed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssPosition {
    #[default]
    Static,
    /// Place normalement, puis decale de `top`/`left`/`bottom`/`right`.
    Relative,
    /// Hors du flux, place dans la boite (padding compris) de son conteneur
    /// et dessine par-dessus ses freres.
    Absolute,
}

/// Taille d'une piste de grille : px, %, `fr` (part de l'espace restant) ou
/// `auto` (la taille de son contenu).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CssTrack {
    Px(f32),
    Percent(f32),
    Fr(f32),
    Auto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CssBox {
    pub display: CssDisplay,
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub min_height: Length,
    /// `Auto` = pas de maximum (`none` en CSS).
    pub max_width: Length,
    pub max_height: Length,
    /// `Auto` sur `left`/`right` d'un bloc de largeur connue : centrage
    /// horizontal (`margin: 0 auto`).
    pub margin: Sides,
    pub padding: Sides,
    /// Epaisseur de bordure de chaque cote (px) - vient de `border` et de
    /// `border-top`/`-right`/`-bottom`/`-left`, et compte dans la taille de
    /// la boite comme en CSS.
    pub border: azure_engine::rendering::models::paint::BorderWidths,
    /// `box-sizing: border-box` : `width`/`height` incluent padding et
    /// bordure. Sinon (`content-box`, le defaut CSS) ils ne mesurent que le
    /// contenu.
    pub border_box: bool,

    // --- Enfants en flex / grid.
    pub flex_direction: FlexDirection,
    pub flex_wrap: bool,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
    pub row_gap: Length,
    pub column_gap: Length,
    pub grid_template_columns: Vec<CssTrack>,
    pub grid_template_rows: Vec<CssTrack>,

    // --- Ce nœud comme element flex / grid.
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Length,
    pub align_self: Option<AlignItems>,
    pub grid_column: Option<usize>,
    pub grid_column_span: usize,
    pub grid_row: Option<usize>,
    pub grid_row_span: usize,

    /// `position: fixed` : hors du flux, place par rapport a la fenetre et
    /// dessine par-dessus la page (voir `web_layout::fixed_box`).
    pub fixed: bool,
    /// `position: relative | absolute`.
    pub position: CssPosition,
    /// `top`, `right`, `bottom`, `left` (`Auto` si absents).
    pub inset: Sides,
    /// Ordre des couches `fixed` : la plus grande au-dessus.
    pub z_index: i32,

    /// Tailles deja mesurees (voir `web_layout::measure`) : la mesure d'un
    /// sous-arbre (retour a la ligne du texte compris) ne depend que de la
    /// largeur disponible, elle est donc refaite seulement quand celle-ci
    /// change - pas a chaque image. Vide pour un arbre neuf.
    pub(crate) cache: RefCell<Vec<(MeasureKey, (f32, f32))>>,
    /// Padding + bordure resolus en px (haut, droite, bas, gauche) lors de
    /// la derniere mesure - un padding en % depend de la largeur du bloc
    /// conteneur, que seul le parent connait.
    pub(crate) frame: Cell<[f32; 4]>,
}

/// Cle de cache d'une mesure : largeur disponible et hauteur de reference
/// (au dixieme de px, -1 si inconnue) et ce qui est mesure (voir
/// `web_layout`).
pub(crate) type MeasureKey = (i32, i32, u8);

impl Default for CssBox {
    fn default() -> CssBox {
        CssBox {
            display: CssDisplay::Block,
            width: Length::Auto,
            height: Length::Auto,
            min_width: Length::Auto,
            min_height: Length::Auto,
            max_width: Length::Auto,
            max_height: Length::Auto,
            margin: Sides::zero(),
            padding: Sides::zero(),
            border: azure_engine::rendering::models::paint::BorderWidths::uniform(0.0),
            border_box: false,
            flex_direction: FlexDirection::Row,
            flex_wrap: false,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            align_content: AlignContent::Stretch,
            row_gap: Length::Px(0.0),
            column_gap: Length::Px(0.0),
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Length::Auto,
            align_self: None,
            grid_column: None,
            grid_column_span: 1,
            grid_row: None,
            grid_row_span: 1,
            fixed: false,
            position: CssPosition::Static,
            inset: Sides::all(Length::Auto),
            z_index: 0,
            cache: RefCell::new(Vec::new()),
            frame: Cell::new([0.0; 4]),
        }
    }
}
