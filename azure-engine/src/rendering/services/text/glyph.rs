use crate::rendering::models::glyph::Glyph;

use crate::rendering::services::text::hinting;

// Ecart maximal (en PIXELS) entre une courbe et les segments qui la
// remplacent. Avant, ce seuil etait de 0.5 unite de police, quelle que soit
// la taille : des centaines de segments invisibles pour un glyphe de 10px
// (une unite y vaut ~0.005px), et a l'inverse trop peu au-dela de ~400px.
const FLATNESS_PX: f32 = 0.05;
const MAX_DEPTH: u32 = 10;

struct GlyphOutline {
    contours: Vec<Vec<(f32, f32)>>,
    current: Vec<(f32, f32)>,
    // `FLATNESS_PX` ramene en unites de police (l'outline est aplatie avant
    // la mise a l'echelle, pour que le hinting travaille sur ces unites).
    flatness: f32,
}

impl GlyphOutline {
    pub fn new(flatness: f32) -> GlyphOutline {
        GlyphOutline {
            contours: Vec::new(),
            current: Vec::new(),
            flatness,
        }
    }
}

// Activer/desactiver l'auto-hinting vertical (voir `hinting`) - actif par
// defaut ; le couper sert a comparer les deux rendus.
thread_local! {
    static HINTING: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

pub fn set_hinting(enabled: bool) {
    HINTING.with(|h| h.set(enabled));
}

pub fn hinting_enabled() -> bool {
    HINTING.with(|h| h.get())
}

fn subdivide_quad(out: &mut Vec<(f32, f32)>, p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), flatness: f32, depth: u32) {
    let dx = p2.0 - p0.0;
    let dy = p2.1 - p0.1;
    let len_sq = dx * dx + dy * dy;
    let d = if len_sq < 1e-6 {
        let ex = p1.0 - p0.0;
        let ey = p1.1 - p0.1;
        (ex * ex + ey * ey).sqrt()
    } else {
        ((p1.0 - p0.0) * dy - (p1.1 - p0.1) * dx).abs() / len_sq.sqrt()
    };

    if depth >= MAX_DEPTH || d < flatness {
        out.push(p2);
        return;
    }
    let m01 = ((p0.0 + p1.0) * 0.5, (p0.1 + p1.1) * 0.5);
    let m12 = ((p1.0 + p2.0) * 0.5, (p1.1 + p2.1) * 0.5);
    let mid = ((m01.0 + m12.0) * 0.5, (m01.1 + m12.1) * 0.5);
    subdivide_quad(out, p0, m01, mid, flatness, depth + 1);
    subdivide_quad(out, mid, m12, p2, flatness, depth + 1);
}

#[allow(clippy::too_many_arguments)]
fn subdivide_cubic(out: &mut Vec<(f32, f32)>, p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), flatness: f32, depth: u32) {
    let dx = p3.0 - p0.0;
    let dy = p3.1 - p0.1;
    let len_sq = dx * dx + dy * dy;
    let (d1, d2) = if len_sq < 1e-6 {
        let e1 = { let ex = p1.0 - p0.0; let ey = p1.1 - p0.1; (ex * ex + ey * ey).sqrt() };
        let e2 = { let ex = p2.0 - p0.0; let ey = p2.1 - p0.1; (ex * ex + ey * ey).sqrt() };
        (e1, e2)
    } else {
        let len = len_sq.sqrt();
        (
            ((p1.0 - p0.0) * dy - (p1.1 - p0.1) * dx).abs() / len,
            ((p2.0 - p0.0) * dy - (p2.1 - p0.1) * dx).abs() / len,
        )
    };

    // Le segment [p0, p3] s'ecarte au plus de 3/4 de max(d1, d2) de la
    // cubique : d1 + d2 < flatness garantit donc bien l'ecart voulu.
    if depth >= MAX_DEPTH || d1 + d2 < flatness {
        out.push(p3);
        return;
    }
    let m01  = ((p0.0 + p1.0) * 0.5, (p0.1 + p1.1) * 0.5);
    let m12  = ((p1.0 + p2.0) * 0.5, (p1.1 + p2.1) * 0.5);
    let m23  = ((p2.0 + p3.0) * 0.5, (p2.1 + p3.1) * 0.5);
    let m012 = ((m01.0 + m12.0) * 0.5, (m01.1 + m12.1) * 0.5);
    let m123 = ((m12.0 + m23.0) * 0.5, (m12.1 + m23.1) * 0.5);
    let mid  = ((m012.0 + m123.0) * 0.5, (m012.1 + m123.1) * 0.5);
    subdivide_cubic(out, p0, m01, m012, mid, flatness, depth + 1);
    subdivide_cubic(out, mid, m123, m23, p3, flatness, depth + 1);
}

impl ttf_parser::OutlineBuilder for GlyphOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.current.is_empty() {
            self.contours.push(self.current.clone());
            self.current.clear();
        }
        self.current.push((x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push((x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p0 = *self.current.last().unwrap_or(&(0.0, 0.0));
        subdivide_quad(&mut self.current, p0, (x1, y1), (x, y), self.flatness, 0);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let p0 = *self.current.last().unwrap_or(&(0.0, 0.0));
        subdivide_cubic(&mut self.current, p0, (x1, y1), (x2, y2), (x, y), self.flatness, 0);
    }

    fn close(&mut self) {
        if !self.current.is_empty() {
            self.contours.push(self.current.clone());
            self.current.clear();
        }
    }
}

pub fn exctract_glyph(font_data: &[u8], character: char, size: f32, weight: f32) -> Result<Glyph, String> {
    let mut face = ttf_parser::Face::parse(font_data, 0).map_err(|e| e.to_string())?;
    extract_glyph_from_face(&mut face, character, size, weight)
}

// Cache l'outline deja aplatie (contours + metriques) d'un glyphe, cle par
// (caractere, taille, poids) - mesure : re-extraire un meme caractere a
// chaque frame (ex: le texte deja tape d'une textarea, redessine en
// entier a chaque touche) fait passer un redessin de ~25ms a ~60ms avec
// seulement 300 caracteres accumules, l'essentiel du delai ressenti en
// tapant du texte. Volontairement PAS de rasterisation en cache (la partie
// anti-aliasing/couleur de `draw_glyph` reste refaite a chaque fois, elle
// depend de la position et du fond) : juste la geometrie, qui ne depend
// que du caractere/taille/poids.
//
// Limitation : la cle ne porte pas d'identifiant de police - correct tant
// qu'une seule police est chargee dans le processus (le seul cas reel
// aujourd'hui, `FONT_PATH` est une constante unique cote azure_foundation),
// mais a revoir si plusieurs polices distinctes doivent un jour coexister.
// (police, caractere, taille, poids, hinting) -> glyphe.
type GlyphCache = std::collections::HashMap<(usize, char, u32, u32, bool), Glyph>;

thread_local! {
    static GLYPH_CACHE: std::cell::RefCell<GlyphCache> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Meme extraction que `exctract_glyph`, mais a partir d'une `Face` deja
/// analysee. `Face::parse` (et, pour une police variable comme celle-ci,
/// l'interpolation des tables de variation qu'implique `set_variation`)
/// n'est pas gratuit : l'appeler une fois par caractere pour dessiner une
/// chaine entiere (ce que faisait `exctract_glyph` en boucle) fait
/// exploser le cout avec la longueur du texte. `renderer::draw_text` parse
/// la police une seule fois pour toute la chaine et appelle cette fonction
/// pour chaque caractere - qui elle-meme sert le resultat depuis
/// `GLYPH_CACHE` quand ce caractere/taille/poids a deja ete extrait.
pub fn extract_glyph_from_face(face: &mut ttf_parser::Face, character: char, size: f32, weight: f32) -> Result<Glyph, String> {
    let font_key = face.raw_face().data.as_ptr() as usize;
    extract_glyph_for_font(face, font_key, character, size, weight)
}

/// Comme `extract_glyph_from_face`, pour une police identifiee par
/// `font_key` (l'adresse de ses donnees chargees, voir
/// `loader::load_font`) : plusieurs polices peuvent ainsi coexister dans le
/// cache sans que leurs glyphes se melangent.
pub fn extract_glyph_for_font(face: &mut ttf_parser::Face, font_key: usize, character: char, size: f32, weight: f32) -> Result<Glyph, String> {
    let key = (font_key, character, size.to_bits(), weight.to_bits(), hinting_enabled());
    if let Some(cached) = GLYPH_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return Ok(cached);
    }

    let glyph = extract_glyph_uncached(face, character, size, weight)?;
    GLYPH_CACHE.with(|cache| cache.borrow_mut().insert(key, glyph.clone()));
    Ok(glyph)
}

fn extract_glyph_uncached(face: &mut ttf_parser::Face, character: char, size: f32, weight: f32) -> Result<Glyph, String> {
    face.set_variation(ttf_parser::Tag::from_bytes(b"wght"), weight);
    let glyph_id = face.glyph_index(character).ok_or("Glyph not found")?;

    let scale = size / face.units_per_em() as f32;
    let mut outline = GlyphOutline::new(FLATNESS_PX / scale);
    face.outline_glyph(glyph_id, &mut outline);

    if !outline.current.is_empty() {
        outline.contours.push(outline.current.clone());
    }

    let bbox = match face.glyph_bounding_box(glyph_id) {
    Some(b) => b,
    None => return Ok(Glyph::new(
        face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * (size / face.units_per_em() as f32),
        0.0,
        face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * (size / face.units_per_em() as f32),
        0.0,
        0.0,
        vec![],
    )),
    };

    let width = (bbox.x_max - bbox.x_min) as f32 * scale;
    let height = (bbox.y_max - bbox.y_min) as f32 * scale;
    let advance_width = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * scale;
    // Ink below the baseline (y=0 in font units), e.g. the tail of g/p/q/j/y.
    // Signed on purpose: a glyph whose ink starts ABOVE the baseline
    // (- ' " * ^ ~) gets a negative descent, which lifts it back to its real
    // height in `draw_text`. Clamping it to 0 used to drop all of those onto
    // the baseline (- rendered like _, ' like ., " like „).
    let descent = -(bbox.y_min as f32) * scale;
    // Gap between the pen position and where the ink starts horizontally
    let lsb = bbox.x_min as f32 * scale;

    let x_min = bbox.x_min as f32;
    let y_min = bbox.y_min as f32;

    if hinting_enabled() && !outline.contours.is_empty() {
        let zones = hinting::blue_zones(face, scale, weight);
        let warp = hinting::vertical_warp(&outline.contours, &zones, scale, face.units_per_em() as f32);
        let hinted: Vec<Vec<(f32, f32)>> = outline.contours.iter().map(|contour| {
            contour.iter().map(|&(px, py)| ((px - x_min) * scale, warp.map(py))).collect()
        }).collect();
        // Boite du glyphe en pixels ENTIERS autour de la ligne de base : le
        // masque commence alors sur une frontiere de pixel, et les bords
        // cales par le hinting y restent (une boite fractionnaire, comme
        // sans hinting, decalerait tout le glyphe d'une fraction de pixel).
        let (lo, hi) = hinted.iter().flatten().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.1), hi.max(p.1)));
        let bottom = (lo + 1e-3).floor();
        let top = (hi - 1e-3).ceil().max(bottom + 1.0);
        let contours = hinted.into_iter().map(|contour| {
            contour.into_iter().map(|(px, py)| (px, py - bottom)).collect()
        }).collect();
        return Ok(Glyph::new(width, top - bottom, advance_width, -bottom, lsb, contours));
    }

    let scaled_contours = outline.contours.iter().map(|contour| {
        contour.iter().map(|(px, py)| {
            ((px - x_min) * scale, (py - y_min) * scale)
        }).collect()
    }).collect();

    Ok(Glyph::new(width, height, advance_width, descent, lsb, scaled_contours))
}
