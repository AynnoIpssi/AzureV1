/// Mode de mise en page d'un `Container` pour ses ENFANTS - n'affecte en
/// rien comment le nœud lui-même est positionné dans SON propre parent (ça,
/// c'est toujours x/y/width/height/margin en % via `layout_manager::resolve`).
/// `Block` (par defaut) : chaque enfant se positionne indépendamment par ses
/// propres pourcentages, exactement le comportement historique. `Flex`/`Grid` :
/// voir `layout_manager::resolve_children`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
    #[default]
    Block,
    Flex,
    Grid,
}

/// Axe principal d'un conteneur `Flex` - `Row` (par defaut) empile
/// horizontalement, `Column` verticalement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

/// Repartition de l'espace libre sur l'axe PRINCIPAL d'un conteneur `Flex`,
/// une fois `flex-basis`/`flex-grow`/`flex-shrink` et `gap` pris en compte -
/// l'equivalent CSS `justify-content`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Alignement des enfants sur l'axe SECONDAIRE (transversal) d'un conteneur
/// `Flex` - l'equivalent CSS `align-items`. `Stretch` (par defaut, comme en
/// CSS) force la taille transversale de l'enfant a 100% de celle du
/// conteneur, plutot que d'utiliser son propre pourcentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    Start,
    End,
    Center,
    #[default]
    Stretch,
}

/// Taille d'une piste (colonne ou ligne) d'un conteneur `Grid`. `Percent` est
/// un pourcentage FIXE de la boîte de contenu du conteneur (sur l'axe
/// concerné) ; `Fr` se partage l'espace qui RESTE une fois toutes les pistes
/// `Percent` (et les `gap` entre pistes) retirees, au prorata de son poids -
/// exactement la semantique de l'unite CSS `fr`, sauf que l'espace total de
/// depart est lui-meme un pourcentage du parent (comme tout le reste de ce
/// moteur de layout), pas des pixels bruts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Track {
    Percent(f32),
    Fr(f32),
}

/// `overflow`/`overflow-y` (rsC) d'un `Container` : ce qui arrive a des
/// enfants qui depassent en bas de sa boite de contenu. `Visible` (defaut)
/// et `Hidden` coupent tous les deux ce qui depasse (un conteneur decoupe
/// toujours ses enfants, voir `ui::services::draw_ui::draw_container`) ;
/// `Auto`/`Scroll` le rendent en plus defilable a la molette - l'etendue
/// defilable est calculee a partir des boites des enfants (voir
/// `layout_manager::layout_container`). Vertical uniquement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Auto,
    Scroll,
}

pub struct LayoutProps {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub margin: f32,
    pub padding: f32,

    // --- Cote CONTENEUR : n'a d'effet que si ce nœud est un `Container`
    // dont `display` n'est pas `Block` (voir `layout_manager::resolve_children`).
    pub display: DisplayMode,
    pub flex_direction: FlexDirection,
    /// `true` = `flex-wrap: wrap` (plusieurs lignes empilees sur l'axe
    /// secondaire des qu'une ligne deborde de l'axe principal), `false`
    /// (par defaut) = `nowrap`, une seule ligne quitte a deborder/retrecir.
    pub flex_wrap: bool,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    /// Espace entre enfants adjacents (et entre lignes/pistes) - un
    /// pourcentage de la boîte de contenu du conteneur sur l'axe concerne,
    /// comme `margin`/`padding` ailleurs dans ce moteur.
    pub gap: f32,
    pub grid_template_columns: Vec<Track>,
    pub grid_template_rows: Vec<Track>,

    // --- Cote ITEM : n'a d'effet que si le PARENT de ce nœud est un
    // `Container` en mode `Flex`/`Grid` - ignore en mode `Block` (defaut).
    /// Part de l'espace libre du conteneur attribuee a cet enfant en plus de
    /// sa base (`flex_basis`) - 0.0 par defaut (CSS `flex-grow: 0`) : ne
    /// grandit pas au-dela de sa base tant que ce n'est pas demande.
    pub flex_grow: f32,
    /// Poids de retrecissement quand la somme des bases depasse l'axe
    /// principal disponible - 1.0 par defaut (CSS `flex-shrink: 1`) : se
    /// retrecit proportionnellement par defaut plutot que de deborder.
    pub flex_shrink: f32,
    /// Taille de depart sur l'axe principal, en % du conteneur - `None`
    /// (equivalent CSS `flex-basis: auto`) retombe sur `width` (axe Row) ou
    /// `height` (axe Column) de cet enfant.
    pub flex_basis: Option<f32>,
    /// Piste de depart (1-based, comme CSS `grid-column-start`) - `None` =
    /// place automatiquement en flux ligne par ligne (voir
    /// `layout_manager::grid_layout`).
    pub grid_column: Option<usize>,
    pub grid_column_span: usize,
    pub grid_row: Option<usize>,
    pub grid_row_span: usize,

    /// Hauteur "virtuelle" du contenu d'un `Container`, en % de sa PROPRE
    /// hauteur resolue (`resolved.height`, avant retrecissement par le
    /// padding) - `None` (par defaut) : comportement historique inchange,
    /// les enfants se positionnent contre la boite de contenu reelle. `Some`
    /// (typiquement > 100.0) : les enfants se positionnent contre une boite
    /// de contenu plus haute que ce qui est reellement visible, et
    /// deviennent defilables a la molette (voir
    /// `layout_manager::scrollable_content_box`,
    /// `ui::services::interact::scroll_at`) - meme principe que
    /// `TextArea::scroll_offset`, generalise a un `Container` quelconque
    /// plutot que limite au texte replie en lignes.
    pub content_height: Option<f32>,
    /// Voir `Overflow`.
    pub overflow: Overflow,
    /// `overflow-x` : defilement horizontal.
    pub overflow_x: Overflow,
    /// Modele de boite CSS (voir `css_box::CssBox`) pour un nœud construit
    /// depuis rsC : quand il est present, ce nœud et ses enfants sont mis en
    /// page comme sur le web (voir `layout::managers::web_layout`), et les
    /// pourcentages ci-dessus sont ignores.
    pub css: Option<Box<crate::layout::models::css_box::CssBox>>,
}

impl LayoutProps {
    /// `true` si ce conteneur defile a la molette : `overflow: auto|scroll`
    /// en rsC, ou l'ancienne hauteur virtuelle `content_height`.
    pub fn scrollable(&self) -> bool {
        matches!(self.overflow, Overflow::Auto | Overflow::Scroll) || self.content_height.is_some()
    }

    /// `true` si ce conteneur defile horizontalement (`overflow-x: auto`).
    pub fn scrollable_x(&self) -> bool {
        matches!(self.overflow_x, Overflow::Auto | Overflow::Scroll)
    }

    pub fn new(x:f32,y:f32,width:f32,height:f32,margin:f32,padding:f32) -> LayoutProps {
        LayoutProps {
            x,
            y,
            width,
            height,
            margin,
            padding,
            display: DisplayMode::Block,
            flex_direction: FlexDirection::Row,
            flex_wrap: false,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            gap: 0.0,
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: None,
            grid_column: None,
            grid_column_span: 1,
            grid_row: None,
            grid_row_span: 1,
            content_height: None,
            overflow: Overflow::Visible,
            overflow_x: Overflow::Visible,
            css: None,
        }
    }
}
