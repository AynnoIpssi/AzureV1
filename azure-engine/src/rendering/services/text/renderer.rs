use crate::rendering::models::glyph::{Glyph, GlyphMask};
use crate::rendering::models::color::Color;
use crate::rendering::models::canvas::Canvas;
use crate::rendering::services::buffer::get_pixel_index;

const AA_SAMPLES: usize = 8;

// Without real hinting, thin strokes at small sizes only cover a fraction of
// a pixel and render as faint gray instead of a crisp line. System text
// renderers compensate with "stem darkening": boost low coverage values
// before blending so thin strokes stay legible. Exponent < 1 lifts mid/low
// coverage up while leaving 0 and 1 untouched.
const STEM_DARKEN_GAMMA: f32 = 0.7;

// Conversions sRGB <-> lumiere lineaire par tables : `powf` sur chaque
// canal de chaque pixel d'encre etait l'essentiel du cout d'un texte.
// `linear_to_srgb` cherche le niveau par dichotomie dans les seuils exacts
// entre deux niveaux sRGB, donc donne le meme resultat que le calcul direct
// (a un niveau pres, au pire, sur une frontiere).
fn srgb_to_linear_exact(f: f64) -> f64 {
    if f <= 0.04045 { f / 12.92 } else { ((f + 0.055) / 1.055).powf(2.4) }
}

struct SrgbTables {
    to_linear: [f32; 256],
    // thresholds[k - 1] : plus petite valeur lineaire qui s'arrondit au niveau k.
    thresholds: [f32; 255],
}

fn tables() -> &'static SrgbTables {
    static TABLES: std::sync::OnceLock<SrgbTables> = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        let mut to_linear = [0.0f32; 256];
        for (i, v) in to_linear.iter_mut().enumerate() {
            *v = srgb_to_linear_exact(i as f64 / 255.0) as f32;
        }
        let mut thresholds = [0.0f32; 255];
        for (k, t) in thresholds.iter_mut().enumerate() {
            *t = srgb_to_linear_exact((k as f64 + 0.5) / 255.0) as f32;
        }
        SrgbTables { to_linear, thresholds }
    })
}

#[inline]
fn srgb_to_linear(tables: &SrgbTables, c: u8) -> f32 {
    tables.to_linear[c as usize]
}

#[inline]
fn linear_to_srgb(tables: &SrgbTables, c: f32) -> u8 {
    tables.thresholds.partition_point(|&t| t <= c) as u8
}

/// Couverture anti-aliasee (0.0 a 1.0, "stem darkening" compris) de chaque
/// pixel d'un glyphe - la partie chere du rendu du texte, faite UNE fois
/// par glyphe (voir `Glyph::new`) au lieu d'a chaque dessin.
pub fn rasterize(glyph_width: f32, glyph_height: f32, contours: &[Vec<(f32, f32)>]) -> GlyphMask {
    let height = (glyph_height.ceil() as u32).max(1);
    let width = (glyph_width.ceil() as u32).max(1);
    let cov_len = (width + 1) as usize;
    let mut out = vec![0.0f32; cov_len * height as usize];
    let mut coverage = vec![0.0f32; cov_len];
    let mut intersections: Vec<(f32, i32)> = Vec::new();

    for py in 0..height {
        coverage.fill(0.0);

        for s in 0..AA_SAMPLES {
            // Sample at 8 evenly spaced sub-pixel positions within this row
            let py_f = glyph_height - (py as f32 + (s as f32 + 0.5) / AA_SAMPLES as f32);
            // (x, winding direction) — needed for nonzero-rule fill since
            // glyph contours can overlap (e.g. crossbar/stem strokes)
            intersections.clear();

            for contour in contours {
                for i in 0..contour.len() {
                    let p1 = contour[i];
                    let p2 = contour[(i + 1) % contour.len()];
                    // Half-open interval avoids double-counting shared vertices
                    if p1.1 <= py_f && p2.1 > py_f {
                        let t = (py_f - p1.1) / (p2.1 - p1.1);
                        intersections.push((p1.0 + t * (p2.0 - p1.0), 1));
                    } else if p2.1 <= py_f && p1.1 > py_f {
                        let t = (py_f - p1.1) / (p2.1 - p1.1);
                        intersections.push((p1.0 + t * (p2.0 - p1.0), -1));
                    }
                }
            }

            intersections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            let mut winding = 0i32;
            for w in 0..intersections.len().saturating_sub(1) {
                winding += intersections[w].1;
                if winding == 0 { continue; }

                let x_start = intersections[w].0.max(0.0);
                let x_end = intersections[w + 1].0.min(width as f32);
                if x_end <= x_start { continue; }

                let px_start = x_start.floor() as usize;
                let px_end = (x_end.ceil() as usize).min(cov_len - 1);

                for (px, slot) in coverage.iter_mut().enumerate().take(px_end + 1).skip(px_start) {
                    // Exact fractional coverage of [px, px+1] inside the span
                    let left  = (px as f32).max(x_start);
                    let right = ((px + 1) as f32).min(x_end);
                    *slot += (right - left).max(0.0);
                }
            }
        }

        let row = &mut out[py as usize * cov_len..(py as usize + 1) * cov_len];
        for (dst, &cov) in row.iter_mut().zip(&coverage) {
            if cov > 0.0 {
                // Stem-darkening boost applied to the raw coverage.
                *dst = (cov / AA_SAMPLES as f32).min(1.0).powf(STEM_DARKEN_GAMMA);
            }
        }
    }

    GlyphMask { width: cov_len as u32, height, coverage: out }
}

pub fn draw_glyph(glyph: &Glyph, x: u32, y: u32, color: &Color, canvas: &mut Canvas) {
    let mask = &glyph.mask;
    let tables = tables();

    // Un glyphe ne peint jamais au-dela de la zone de decoupage active
    // (voir `Canvas::set_clip`) - utilise pour qu'une ligne de texte qui
    // defile hors de sa boite (une zone de texte plus longue que sa
    // hauteur) ne peigne jamais au-dela de cette boite.
    let (clip_x, clip_y, clip_w, clip_h) = canvas.clip_bounds();
    let (clip_x_end, clip_y_end) = (clip_x + clip_w, clip_y + clip_h);

    // Pre-convert foreground color to linear light once
    let fg_r = srgb_to_linear(tables, color.r);
    let fg_g = srgb_to_linear(tables, color.g);
    let fg_b = srgb_to_linear(tables, color.b);
    let fg_a = color.a as f32 / 255.0;

    // Seules les lignes/colonnes du masque qui tombent dans le clip sont
    // parcourues : un texte decoupe (ex: a moitie hors d'une zone qui
    // defile) ne coute que sa partie visible.
    let row_start = clip_y.saturating_sub(y).min(mask.height);
    let row_end = clip_y_end.saturating_sub(y).min(mask.height);
    let col_start = clip_x.saturating_sub(x).min(mask.width);
    let col_end = clip_x_end.saturating_sub(x).min(mask.width);

    for py in row_start..row_end {
        let row = &mask.coverage[(py * mask.width) as usize..((py + 1) * mask.width) as usize];
        for px in col_start..col_end {
            let cov = row[px as usize];
            if cov <= 0.0 { continue; }

            // Combined alpha: glyph coverage × color's own alpha.
            let alpha = (fg_a * cov).min(1.0);

            let idx = get_pixel_index(x + px, y + py, canvas.width);
            let buf = &mut canvas.buffer;

            // Read background and convert to linear light for correct blending.
            // Wayland's wl_shm buffer is ARGB8888, stored little-endian as B,G,R,A
            let bg_b = srgb_to_linear(tables, buf[idx]);
            let bg_g = srgb_to_linear(tables, buf[idx + 1]);
            let bg_r = srgb_to_linear(tables, buf[idx + 2]);

            // Porter-Duff "over" in linear light, then back to sRGB
            buf[idx]     = linear_to_srgb(tables, fg_b * alpha + bg_b * (1.0 - alpha));
            buf[idx + 1] = linear_to_srgb(tables, fg_g * alpha + bg_g * (1.0 - alpha));
            buf[idx + 2] = linear_to_srgb(tables, fg_r * alpha + bg_r * (1.0 - alpha));
            buf[idx + 3] = 255;
        }
    }
}

/// Comme `draw_glyph`, penche vers la droite (oblique synthetique) : chaque
/// ligne du masque est decalee de `(baseline - ligne) * slant` pixels, la
/// ligne de base restant en place. Le decalage fractionnaire est reparti
/// entre deux colonnes (anti-aliasing conserve).
pub fn draw_glyph_slanted(glyph: &Glyph, x: u32, y: u32, baseline: f32, slant: f32, color: &Color, canvas: &mut Canvas) {
    let mask = &glyph.mask;
    let tables = tables();
    let (clip_x, clip_y, clip_w, clip_h) = canvas.clip_bounds();
    let (clip_x_end, clip_y_end) = (clip_x + clip_w, clip_y + clip_h);
    let fg = (srgb_to_linear(tables, color.r), srgb_to_linear(tables, color.g), srgb_to_linear(tables, color.b));
    let fg_a = color.a as f32 / 255.0;
    for py in 0..mask.height {
        let dy = y + py;
        if dy < clip_y || dy >= clip_y_end {
            continue;
        }
        let row = &mask.coverage[(py * mask.width) as usize..((py + 1) * mask.width) as usize];
        let shift = (baseline - (py as f32 + 0.5)) * slant;
        let whole = shift.floor();
        let frac = shift - whole;
        let whole = whole as i64;
        // Colonne de destination c (relative a x) : melange des colonnes
        // c - whole et c - whole - 1 du masque.
        for c in (whole.min(0))..(mask.width as i64 + whole.max(0) + 1) {
            let a = row.get((c - whole) as usize).copied().filter(|_| c - whole >= 0).unwrap_or(0.0);
            let b = row.get((c - whole - 1) as usize).copied().filter(|_| c - whole > 0).unwrap_or(0.0);
            let cov = a * (1.0 - frac) + b * frac;
            if cov <= 0.0 {
                continue;
            }
            let dx = x as i64 + c;
            if dx < clip_x as i64 || dx >= clip_x_end as i64 {
                continue;
            }
            let alpha = (fg_a * cov).min(1.0);
            let idx = get_pixel_index(dx as u32, dy, canvas.width);
            let buf = &mut canvas.buffer;
            let bg_b = srgb_to_linear(tables, buf[idx]);
            let bg_g = srgb_to_linear(tables, buf[idx + 1]);
            let bg_r = srgb_to_linear(tables, buf[idx + 2]);
            buf[idx] = linear_to_srgb(tables, fg.2 * alpha + bg_b * (1.0 - alpha));
            buf[idx + 1] = linear_to_srgb(tables, fg.1 * alpha + bg_g * (1.0 - alpha));
            buf[idx + 2] = linear_to_srgb(tables, fg.0 * alpha + bg_r * (1.0 - alpha));
            buf[idx + 3] = 255;
        }
    }
}
