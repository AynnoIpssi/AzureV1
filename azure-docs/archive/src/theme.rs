// Charte visuelle unique, reutilisee telle quelle par les 4 fenetres
// (launcher + rsH/rsC/routeur) - la police est figee par azure-foundation
// (voir `azure_foundation::ui::services::draw_ui::FONT_PATH`, non
// configurable depuis une app externe), donc c'est UNIQUEMENT la
// couleur/l'espacement/le layout qui rendent les 4 fenetres coherentes entre
// elles plutot que 3 demos visuellement disparates.
use azure_engine::rendering::models::color::Color;

pub const BACKGROUND: Color = Color::new(20, 20, 31, 255);
pub const SURFACE: Color = Color::new(30, 30, 46, 255);
pub const SURFACE_ALT: Color = Color::new(16, 16, 26, 255);
pub const ACCENT: Color = Color::new(124, 156, 255, 255);
pub const ACCENT_HOVER: Color = Color::new(156, 180, 255, 255);
pub const TEXT_PRIMARY: Color = Color::new(242, 242, 247, 255);
pub const TEXT_SECONDARY: Color = Color::new(154, 154, 176, 255);
pub const BORDER: Color = Color::new(51, 51, 74, 255);
pub const SUCCESS: Color = Color::new(74, 222, 128, 255);
pub const WARNING: Color = Color::new(245, 166, 35, 255);

// Echelle d'espacement en pourcentages (`LayoutProps` n'accepte que des %) -
// utilisee uniformement pour tout padding/margin/gap dans les 4 fenetres.
pub const SPACE_XS: f32 = 1.0;
pub const SPACE_SM: f32 = 2.0;
pub const SPACE_MD: f32 = 3.0;
pub const SPACE_LG: f32 = 5.0;

pub const TITLE_SIZE: f32 = 26.0;
pub const TITLE_WEIGHT: f32 = 700.0;
pub const RECIPE_TITLE_SIZE: f32 = 19.0;
pub const RECIPE_TITLE_WEIGHT: f32 = 600.0;
pub const GOAL_SIZE: f32 = 14.0;
pub const GOAL_WEIGHT: f32 = 400.0;
pub const CODE_SIZE: f32 = 13.0;
pub const CODE_WEIGHT: f32 = 400.0;
pub const NOTE_SIZE: f32 = 13.0;
pub const NOTE_WEIGHT: f32 = 400.0;
pub const TOC_SIZE: f32 = 14.0;
pub const TOC_WEIGHT: f32 = 500.0;
