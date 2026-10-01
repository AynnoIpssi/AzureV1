//Canva is blueprint for drow on window.

pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u8>,
    // Zone hors de laquelle rien ne doit etre dessine (`x, y, width,
    // height`), le temps qu'elle reste active (voir `set_clip`/
    // `clear_clip`) - `None` = pas de restriction, tout le canvas.
    // Permet de dessiner un contenu plus grand que sa boite (ex: une zone
    // de texte qui defile) sans deborder visuellement dessus : les
    // primitives de dessin (`rect::draw_rect`, `draw_glyph`) verifient
    // `clip_bounds()` en plus des dimensions du canvas.
    clip: Option<(u32, u32, u32, u32)>,
}

impl Canvas {
    pub fn new(width: u32, height: u32,) -> Canvas {
        Canvas {
            width,
            height,
            buffer: vec!(0u8; (width * height * 4) as usize),
            clip: None,
        }
    }

    /// Restreint tout dessin a `(x, y, width, height)` jusqu'au prochain
    /// `set_clip`/`clear_clip`.
    pub fn set_clip(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.clip = Some((x, y, width, height));
    }

    /// Retire toute restriction : le dessin s'applique de nouveau a tout
    /// le canvas.
    pub fn clear_clip(&mut self) {
        self.clip = None;
    }

    /// La zone de dessin actuellement autorisee, toujours ramenee dans les
    /// bornes reelles du canvas (un `set_clip` errone ou trop large ne
    /// peut donc jamais faire deborder au-dela du buffer) - c'est ce que
    /// les primitives de dessin consultent, pas `clip` directement.
    pub fn clip_bounds(&self) -> (u32, u32, u32, u32) {
        let (x, y, w, h) = self.clip.unwrap_or((0, 0, self.width, self.height));
        let x = x.min(self.width);
        let y = y.min(self.height);
        let w = w.min(self.width.saturating_sub(x));
        let h = h.min(self.height.saturating_sub(y));
        (x, y, w, h)
    }
}
