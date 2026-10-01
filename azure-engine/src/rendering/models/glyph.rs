use std::rc::Rc;

#[derive(Clone)]
pub struct Glyph {
    pub width: f32,
    pub height: f32,
    pub advance_width: f32,
    // How far the ink dips below the baseline (0 for most letters, >0 for descenders like g/p/q/j/y)
    pub descent: f32,
    // Left side bearing: gap between the pen position and where the ink starts
    pub lsb: f32,
    // `Rc` : un glyphe sort du cache (voir `services::text::glyph`) a
    // chaque caractere dessine - le cloner ne doit pas recopier ses
    // contours ni son masque.
    pub contours: Rc<Vec<Vec<(f32, f32)>>>,
    // Couverture anti-aliasee de chaque pixel (0.0 a 1.0, deja "stem
    // darkened"), calculee une seule fois a la creation : elle ne depend que
    // de la forme du glyphe, pas de l'endroit ou il est dessine ni de ce
    // qu'il y a dessous (voir `services::text::renderer::draw_glyph`).
    pub mask: Rc<GlyphMask>,
}

pub struct GlyphMask {
    pub width: u32,
    pub height: u32,
    pub coverage: Vec<f32>,
}

impl Glyph {
    pub fn new(width: f32, height: f32, advance_width: f32, descent: f32, lsb: f32, contours: Vec<Vec<(f32, f32)>>,) -> Glyph {
        let mask = crate::rendering::services::text::renderer::rasterize(width, height, &contours);
        Glyph { width, height, advance_width, descent, lsb, contours: Rc::new(contours), mask: Rc::new(mask) }
    }
}
