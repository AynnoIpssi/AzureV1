use crate::style::models::web_style::{TextAlign, TextDecoration, WhiteSpace};
use azure_engine::rendering::managers::renderer::TextOptions;
use std::cell::RefCell;

/// Une ligne decoupee : debut, fin (en caracteres) et largeur en px.
pub type Line = (usize, usize, f32);

/// Mise en forme d'un texte construit depuis rsC : police, interligne,
/// alignement et retour a la ligne, comme un paragraphe web. `None` sur un
/// widget construit a l'ancienne : une seule ligne, en haut a gauche.
#[derive(Debug)]
pub struct TextStyle {
    /// Chemin du fichier de police (voir `ui::services::fonts`).
    pub font: &'static str,
    /// Hauteur d'une ligne, en px.
    pub line_height: f32,
    pub align: TextAlign,
    pub white_space: WhiteSpace,
    /// Italique et espacement des lettres.
    pub options: TextOptions,
    pub decoration: TextDecoration,
    /// Lignes deja decoupees pour une largeur donnee (arrondie au dixieme
    /// de px) : le decoupage n'est refait que si la largeur change.
    pub(crate) lines: RefCell<Option<(i32, Vec<Line>)>>,
}

impl TextStyle {
    pub fn new(font: &'static str, line_height: f32, align: TextAlign, white_space: WhiteSpace) -> TextStyle {
        TextStyle { font, line_height, align, white_space, options: TextOptions::default(), decoration: TextDecoration::default(), lines: RefCell::new(None) }
    }

    /// `font-style`, `letter-spacing` et `text-decoration`.
    pub fn with_options(mut self, italic: bool, letter_spacing: f32, decoration: TextDecoration) -> TextStyle {
        self.options = TextOptions { italic, letter_spacing };
        self.decoration = decoration;
        self
    }
}
