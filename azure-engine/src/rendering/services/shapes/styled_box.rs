// Boite decoree : ombre portee, remplissage (couleur unie ou degrade),
// coins arrondis anti-alias et bordure - tout melange avec ce qu'il y a
// deja dessous (transparence reelle), contrairement a `rect::draw_rect` qui
// ecrit ses pixels tels quels.
//
// Chaque pixel est evalue a partir de sa distance signee au bord de la
// boite arrondie (negative a l'interieur) : une seule formule donne a la
// fois l'anti-aliasing des coins, l'epaisseur de la bordure et le flou de
// l'ombre.
use crate::rendering::models::canvas::Canvas;
use crate::rendering::models::color::Color;
use crate::rendering::models::paint::{BorderStyle, BoxStyle, ColorStop, Fill, Shadow};
use crate::rendering::services::shapes::rect;

/// Dessine `style` dans la boite `(x, y, width, height)` - position signee :
/// la boite peut commencer hors du canvas (contenu defile), seule sa partie
/// dans la zone de decoupage active est peinte.
pub fn draw_box(x: i32, y: i32, width: u32, height: u32, style: &BoxStyle, canvas: &mut Canvas) {
    if width == 0 || height == 0 {
        return;
    }
    // `border-style: none` : pas de bordure du tout (le fond va jusqu'au bord).
    if style.border_style == BorderStyle::None && !style.border.is_zero() {
        let mut plain = style.clone();
        plain.border = crate::rendering::models::paint::BorderWidths::uniform(0.0);
        plain.border_style = BorderStyle::Solid;
        return draw_box(x, y, width, height, &plain, canvas);
    }
    // Ombre autour (dessinee ici) ou a l'interieur (voir `draw_inset`).
    let outer: Option<Shadow> = style.shadow.filter(|s| !s.inset);
    let inset: Option<Shadow> = style.shadow.filter(|s| s.inset);
    let (clip_x, clip_y, clip_w, clip_h) = canvas.clip_bounds();
    let (clip_x, clip_y, clip_right, clip_bottom) = (clip_x as i32, clip_y as i32, (clip_x + clip_w) as i32, (clip_y + clip_h) as i32);

    // Cas courant et le plus frequent (fond d'un conteneur) : une copie de
    // lignes, sans rien calculer par pixel.
    if let Fill::Solid(color) = style.fill {
        if color.a == 0 && style.border.is_zero() && style.shadow.is_none() {
            return;
        }
        if color.a == 255 && style.radius <= 0.0 && style.border.is_zero() && style.shadow.is_none() {
            let x0 = x.max(clip_x);
            let y0 = y.max(clip_y);
            let x1 = (x + width as i32).min(clip_right);
            let y1 = (y + height as i32).min(clip_bottom);
            if x0 < x1 && y0 < y1 {
                rect::draw_rect(x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32, &color, canvas);
            }
            return;
        }
    }

    let (w, h) = (width as f32, height as f32);
    let radius = style.radius.clamp(0.0, w.min(h) / 2.0);
    // Bordure identique des 4 cotes : la zone de remplissage est la boite
    // retrecie de `border` (un seul calcul de distance, voir plus bas). Sinon
    // (`border-bottom` seul...), c'est une boite interieure decalee, calculee
    // a part (`inner`).
    let uniform = style.border.is_uniform();
    let border = if uniform { style.border.top.clamp(0.0, w.min(h) / 2.0) } else { 0.0 };
    let sides = [
        style.border.top.clamp(0.0, h),
        style.border.right.clamp(0.0, w),
        style.border.bottom.clamp(0.0, h),
        style.border.left.clamp(0.0, w),
    ];
    let has_border = if uniform { border > 0.0 } else { true };
    let center = (x as f32 + w / 2.0, y as f32 + h / 2.0);
    let half = (w / 2.0, h / 2.0);

    // Zone a parcourir : la boite, agrandie de l'ombre si elle deborde.
    let (mut left, mut top, mut right, mut bottom) = (x as f32, y as f32, x as f32 + w, y as f32 + h);
    if let Some(s) = &outer {
        let reach = s.blur + s.spread.max(0.0) + 1.0;
        left = left.min(x as f32 + s.offset_x - reach);
        top = top.min(y as f32 + s.offset_y - reach);
        right = right.max(x as f32 + w + s.offset_x + reach);
        bottom = bottom.max(y as f32 + h + s.offset_y + reach);
    }
    let x0 = (left.floor() as i32).max(clip_x);
    let y0 = (top.floor() as i32).max(clip_y);
    let x1 = (right.ceil() as i32).min(clip_right);
    let y1 = (bottom.ceil() as i32).min(clip_bottom);
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let palette = Palette::new(&style.fill, w, h);
    let pattern = BorderPattern::new(style.border_style, w, h, radius, style.border.max());

    // Interieur "plein" : la zone ou un pixel est entierement dans le
    // remplissage (ni bord anti-alias, ni bordure, ni ombre visible). C'est
    // l'essentiel de la surface d'une grande boite ; elle est remplie ligne
    // par ligne sans calcul de distance (voir `fill_span`). Tout le reste
    // (bords, coins, bordure, ombre) passe par le calcul complet par pixel.
    let (inner_center, inner_half, inner_radius) = if uniform {
        (center, (half.0 - border, half.1 - border), (radius - border).max(0.0))
    } else {
        let [t, r, b, l] = sides;
        (
            (center.0 + (l - r) / 2.0, center.1 + (t - b) / 2.0),
            ((half.0 - (l + r) / 2.0).max(0.0), (half.1 - (t + b) / 2.0).max(0.0)),
            (radius - sides.iter().cloned().fold(0.0, f32::max)).max(0.0),
        )
    };

    for py in y0..y1 {
        let fy = py as f32 + 0.5;
        let row = (py as u32 * canvas.width) as usize * 4;
        let (mut span_start, mut span_end) = inner_span(fy - inner_center.1, inner_center.0, inner_half, inner_radius, x0, x1);

        // Ligne entierement au-dessus ou au-dessous de la boite, dans le
        // haut/bas de son ombre : entre les deux coins arrondis de l'ombre,
        // son intensite ne depend que de la ligne - un seul calcul, puis la
        // meme couleur melangee sur toute la portion.
        if let Some(s) = &outer {
            let outside_box = fy < y as f32 - 1.0 || fy > y as f32 + h + 1.0;
            let sdy = (fy - center.1 - s.offset_y).abs();
            let (shx, shy, sr) = (half.0 + s.spread, half.1 + s.spread, (radius + s.spread).max(0.0));
            if outside_box && sdy > shy - sr {
                let straight = shx - sr - 1.0;
                let scx = center.0 + s.offset_x;
                let start = ((scx - straight).ceil() as i32).clamp(x0, x1);
                let end = ((scx + straight).floor() as i32).clamp(start, x1);
                if start < end {
                    let sd = sdy - shy;
                    let a = if s.blur > 0.0 { 1.0 - smoothstep(-s.blur / 2.0, s.blur / 2.0, sd) } else { (0.5 - sd).clamp(0.0, 1.0) };
                    let color = Color::new(s.color.r, s.color.g, s.color.b, (s.color.a as f32 * a).round() as u8);
                    for p in canvas.buffer[row + start as usize * 4..row + end as usize * 4].chunks_exact_mut(4) {
                        blend_full(p, color);
                    }
                    // Le reste de la ligne passe par le calcul complet.
                    span_start = start;
                    span_end = end;
                }
            }
        }
        let inner_fill = span_start < span_end && !(fy < y as f32 - 1.0 || fy > y as f32 + h + 1.0);

        for px in (x0..span_start).chain(span_end..x1) {
            let fx = px as f32 + 0.5;
            let idx = row + px as usize * 4;
            let d = rounded_box_distance(fx - center.0, fy - center.1, half.0, half.1, radius);
            let box_cov = (0.5 - d).clamp(0.0, 1.0);

            if let Some(s) = &outer {
                // L'ombre n'est visible qu'autour de la boite, jamais a
                // travers un fond transparent.
                let outside = 1.0 - box_cov;
                if outside > 0.0 {
                    let sd = rounded_box_distance(
                        fx - center.0 - s.offset_x,
                        fy - center.1 - s.offset_y,
                        half.0 + s.spread,
                        half.1 + s.spread,
                        (radius + s.spread).max(0.0),
                    );
                    let a = if s.blur > 0.0 { 1.0 - smoothstep(-s.blur / 2.0, s.blur / 2.0, sd) } else { (0.5 - sd).clamp(0.0, 1.0) };
                    blend(&mut canvas.buffer[idx..idx + 4], s.color, a * outside);
                }
            }
            if box_cov <= 0.0 {
                continue;
            }

            let fill_cov = if !has_border {
                box_cov
            } else if uniform {
                (0.5 - (d + border)).clamp(0.0, 1.0)
            } else {
                let inner_d = rounded_box_distance(fx - inner_center.0, fy - inner_center.1, inner_half.0, inner_half.1, inner_radius);
                (0.5 - inner_d).clamp(0.0, box_cov)
            };
            if has_border {
                let ring = box_cov - fill_cov;
                if style.border_style == BorderStyle::Solid {
                    blend(&mut canvas.buffer[idx..idx + 4], style.border_color, ring);
                } else if ring > 0.0 {
                    // Tirets, points, double trait : le fond se voit entre
                    // deux traits (comme `background-clip: border-box`).
                    let on = pattern.coverage(fx - x as f32, fy - y as f32, -d);
                    let color = palette.at(fx - x as f32, fy - y as f32);
                    blend(&mut canvas.buffer[idx..idx + 4], color, ring * (1.0 - on));
                    blend(&mut canvas.buffer[idx..idx + 4], style.border_color, ring * on);
                }
            }
            if fill_cov > 0.0 {
                let color = palette.at(fx - x as f32, fy - y as f32);
                blend(&mut canvas.buffer[idx..idx + 4], color, fill_cov);
                if let Some(s) = &inset {
                    let a = inset_alpha(s, fx - inner_center.0, fy - inner_center.1, inner_half, inner_radius);
                    blend(&mut canvas.buffer[idx..idx + 4], s.color, a * fill_cov);
                }
            }
        }

        if inner_fill {
            let start = row + span_start as usize * 4;
            let end = row + span_end as usize * 4;
            palette.fill_span(&mut canvas.buffer[start..end], span_start as f32 + 0.5 - x as f32, fy - y as f32);
            if let Some(s) = &inset {
                // Seulement pres des bords : au fond du « trou », rien.
                let hole_center = (inner_center.0 + s.offset_x, inner_center.1 + s.offset_y);
                let shrink = s.spread + s.blur / 2.0 + 1.0;
                let hole_half = ((inner_half.0 - shrink).max(0.0), (inner_half.1 - shrink).max(0.0));
                let (skip_start, skip_end) = inner_span(fy - hole_center.1, hole_center.0, hole_half, (inner_radius - shrink).max(0.0), span_start, span_end);
                for px in (span_start..skip_start).chain(skip_end..span_end) {
                    let a = inset_alpha(s, px as f32 + 0.5 - inner_center.0, fy - inner_center.1, inner_half, inner_radius);
                    if a > 0.0 {
                        let idx = row + px as usize * 4;
                        blend(&mut canvas.buffer[idx..idx + 4], s.color, a);
                    }
                }
            }
        }
    }
}

/// Intensite d'une ombre interieure au point `(px, py)` (relatif au centre
/// de la boite interieure) : 0 au fond du « trou » (la boite decalee et
/// retrecie de `spread`), 1 contre le bord.
fn inset_alpha(s: &Shadow, px: f32, py: f32, half: (f32, f32), radius: f32) -> f32 {
    let hole_half = ((half.0 - s.spread).max(0.0), (half.1 - s.spread).max(0.0));
    let sd = rounded_box_distance(px - s.offset_x, py - s.offset_y, hole_half.0, hole_half.1, (radius - s.spread).max(0.0));
    if s.blur > 0.0 { smoothstep(-s.blur / 2.0, s.blur / 2.0, sd) } else { (sd + 0.5).clamp(0.0, 1.0) }
}

/// Motif d'une bordure non pleine : ou tombent tirets et points, le long du
/// contour de la boite (coins arrondis compris).
struct BorderPattern {
    style: BorderStyle,
    w: f32,
    h: f32,
    r: f32,
    width: f32,
    /// Longueur d'un motif (trait + espace), ajustee pour tomber juste sur
    /// le tour complet.
    period: f32,
    on: f32,
}

impl BorderPattern {
    fn new(style: BorderStyle, w: f32, h: f32, r: f32, width: f32) -> BorderPattern {
        let width = width.max(1.0);
        let perimeter = 2.0 * (w - 2.0 * r) + 2.0 * (h - 2.0 * r) + std::f32::consts::TAU * r;
        let (period, on) = match style {
            BorderStyle::Dashed => (5.0 * width, 3.0 / 5.0),
            BorderStyle::Dotted => (2.0 * width, 0.5),
            _ => (1.0, 1.0),
        };
        let count = (perimeter / period).round().max(1.0);
        let period = perimeter.max(1.0) / count;
        BorderPattern { style, w, h, r, width, period, on: period * on }
    }

    /// Part du pixel `(lx, ly)` (relatif au coin de la boite, a `depth`
    /// pixels a l'interieur du bord) couverte par le trait.
    fn coverage(&self, lx: f32, ly: f32, depth: f32) -> f32 {
        match self.style {
            BorderStyle::Solid | BorderStyle::None => 1.0,
            BorderStyle::Double => {
                let third = self.width / 3.0;
                ((third - depth + 0.5).clamp(0.0, 1.0) + (depth - 2.0 * third + 0.5).clamp(0.0, 1.0)).min(1.0)
            }
            BorderStyle::Dashed => {
                let m = self.along(lx, ly).rem_euclid(self.period);
                (0.5 + self.on / 2.0 - (m - self.on / 2.0).abs()).clamp(0.0, 1.0)
            }
            BorderStyle::Dotted => {
                let m = self.along(lx, ly).rem_euclid(self.period) - self.period / 2.0;
                let across = depth - self.width / 2.0;
                (self.width / 2.0 - (m * m + across * across).sqrt() + 0.5).clamp(0.0, 1.0)
            }
        }
    }

    /// Distance parcourue le long du contour (sens horaire, depuis le haut a
    /// gauche) jusqu'au point le plus proche de `(lx, ly)`.
    fn along(&self, lx: f32, ly: f32) -> f32 {
        let (w, h, r) = (self.w, self.h, self.r);
        let (sw, sh, arc) = ((w - 2.0 * r).max(0.0), (h - 2.0 * r).max(0.0), std::f32::consts::FRAC_PI_2 * r);
        let corner = (lx < r || lx > w - r) && (ly < r || ly > h - r);
        if corner {
            let quarter = |vx: f32, vy: f32| vy.atan2(vx).clamp(0.0, std::f32::consts::FRAC_PI_2) * r;
            return if lx > w - r && ly < r {
                sw + quarter(r - ly, lx - (w - r))
            } else if lx > w - r {
                sw + arc + sh + quarter(lx - (w - r), ly - (h - r))
            } else if ly > h - r {
                2.0 * sw + 2.0 * arc + sh + quarter(ly - (h - r), r - lx)
            } else {
                2.0 * sw + 3.0 * arc + 2.0 * sh + quarter(r - lx, r - ly)
            };
        }
        let nearest = [ly, w - lx, h - ly, lx];
        let side = (0..4).min_by(|a, b| nearest[*a].total_cmp(&nearest[*b])).unwrap_or(0);
        match side {
            0 => lx - r,
            1 => sw + arc + (ly - r),
            2 => sw + 2.0 * arc + sh + (w - r - lx),
            _ => 2.0 * sw + 3.0 * arc + sh + (h - r - ly),
        }
    }
}

// Colonnes `[debut, fin)` de la ligne (a `dy` du centre) dont les pixels
// sont entierement dans la boite interieure (demi-taille `half`, coins de
// rayon `r`), bornees a `[x0, x1)` - avec une marge d'un pixel pour que
// l'anti-aliasing reste toujours du ressort du calcul complet. Vide si la
// ligne ne traverse pas cet interieur.
fn inner_span(dy: f32, center_x: f32, half: (f32, f32), r: f32, x0: i32, x1: i32) -> (i32, i32) {
    let ady = dy.abs();
    if half.0 <= 1.0 || ady > half.1 - 1.0 {
        return (x0, x0);
    }
    let straight = half.1 - r;
    let reach = if ady <= straight {
        half.0
    } else {
        let k = ady - straight;
        (half.0 - r) + (r * r - k * k).max(0.0).sqrt()
    } - 1.0;
    if reach <= 0.0 {
        return (x0, x0);
    }
    let start = ((center_x - reach).ceil() as i32).clamp(x0, x1);
    let end = ((center_x + reach).floor() as i32).clamp(start, x1);
    (start, end)
}

/// Distance signee du point `(px, py)` (relatif au centre) au bord d'une
/// boite de demi-taille `(hx, hy)` aux coins de rayon `r` : negative a
/// l'interieur, en pixels.
fn rounded_box_distance(px: f32, py: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let qx = px.abs() - (hx - r);
    let qy = py.abs() - (hy - r);
    let outside = if qx > 0.0 && qy > 0.0 {
        (qx * qx + qy * qy).sqrt()
    } else {
        qx.max(qy).max(0.0)
    };
    outside + qx.max(qy).min(0.0) - r
}

fn smoothstep(edge0: f32, edge1: f32, v: f32) -> f32 {
    let t = ((v - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Melange `color` (sa propre transparence multipliee par `coverage`) par
/// dessus le pixel BGRA `dst`. Le resultat reste opaque : c'est ce qui est
/// envoye au compositeur.
// Melange d'une couleur avec une couverture pleine (l'interieur d'une
// boite), en arithmetique entiere.
#[inline]
fn blend_full(dst: &mut [u8], color: Color) {
    match color.a {
        0 => {}
        255 => {
            dst[0] = color.b;
            dst[1] = color.g;
            dst[2] = color.r;
            dst[3] = 255;
        }
        a => {
            let (a, inv) = (a as u32 + 1, 256 - a as u32);
            dst[0] = ((color.b as u32 * a + dst[0] as u32 * inv) >> 8) as u8;
            dst[1] = ((color.g as u32 * a + dst[1] as u32 * inv) >> 8) as u8;
            dst[2] = ((color.r as u32 * a + dst[2] as u32 * inv) >> 8) as u8;
            dst[3] = 255;
        }
    }
}

#[inline]
pub fn blend(dst: &mut [u8], color: Color, coverage: f32) {
    let a = color.a as f32 / 255.0 * coverage;
    if a <= 0.0 {
        return;
    }
    if a >= 1.0 {
        dst[0] = color.b;
        dst[1] = color.g;
        dst[2] = color.r;
        dst[3] = 255;
        return;
    }
    let inv = 1.0 - a;
    dst[0] = (color.b as f32 * a + dst[0] as f32 * inv + 0.5) as u8;
    dst[1] = (color.g as f32 * a + dst[1] as f32 * inv + 0.5) as u8;
    dst[2] = (color.r as f32 * a + dst[2] as f32 * inv + 0.5) as u8;
    dst[3] = 255;
}

// Couleur du remplissage en un point de la boite. Un degrade est
// precalcule en 256 teintes le long de son axe (interpolation en couleurs
// premultipliees, comme CSS : un arret transparent ne "noircit" pas ses
// voisins), puis chaque pixel ne fait qu'une lecture de table.
enum Palette {
    Solid(Color),
    Linear { table: Vec<Color>, dir: (f32, f32), length: f32, half: (f32, f32) },
    // `squared` : la meme rampe indexee par t² (voir `fill_span`), pour
    // eviter une racine carree par pixel.
    Radial { table: Vec<Color>, squared: Vec<Color>, radii: (f32, f32), half: (f32, f32) },
}

const SQUARED_STEPS: usize = 1024;

const GRADIENT_STEPS: usize = 256;

impl Palette {
    fn new(fill: &Fill, w: f32, h: f32) -> Palette {
        let half = (w / 2.0, h / 2.0);
        match fill {
            Fill::Solid(c) => Palette::Solid(*c),
            Fill::Linear { angle, stops } => {
                let rad = angle.to_radians();
                let dir = (rad.sin(), -rad.cos());
                // Longueur de la ligne de degrade CSS : les coins opposes
                // sont exactement a 0% et 100%.
                let length = (w * dir.0.abs() + h * dir.1.abs()).max(1.0);
                Palette::Linear { table: gradient_table(stops), dir, length, half }
            }
            Fill::Radial { stops } => {
                // Ellipse "farthest-corner" : 100% passe par les coins.
                let radii = ((half.0 * std::f32::consts::SQRT_2).max(1.0), (half.1 * std::f32::consts::SQRT_2).max(1.0));
                let table = gradient_table(stops);
                let squared = (0..SQUARED_STEPS)
                    .map(|i| {
                        let t = (i as f32 / (SQUARED_STEPS - 1) as f32).sqrt();
                        table[(t * (GRADIENT_STEPS - 1) as f32).round() as usize]
                    })
                    .collect();
                Palette::Radial { table, squared, radii, half }
            }
        }
    }

    // Remplit une suite de pixels BGRA consecutifs d'une meme ligne, le
    // premier au point `(lx, ly)` de la boite (coordonnees locales).
    fn fill_span(&self, pixels: &mut [u8], lx: f32, ly: f32) {
        match self {
            Palette::Solid(c) if c.a == 255 => {
                for p in pixels.chunks_exact_mut(4) {
                    p.copy_from_slice(&[c.b, c.g, c.r, 255]);
                }
            }
            Palette::Solid(c) => {
                for p in pixels.chunks_exact_mut(4) {
                    blend_full(p, *c);
                }
            }
            Palette::Linear { table, dir, length, half } => {
                // Position dans la table en virgule fixe (16 bits de
                // fraction), qui avance d'un pas constant le long de la ligne.
                let scale = (GRADIENT_STEPS - 1) as f32 * 65536.0;
                let mut t = ((((lx - half.0) * dir.0 + (ly - half.1) * dir.1) / length + 0.5) * scale) as i64 + 32768;
                let step = (dir.0 / length * scale) as i64;
                let last = (GRADIENT_STEPS - 1) as i64;
                for p in pixels.chunks_exact_mut(4) {
                    blend_full(p, table[(t >> 16).clamp(0, last) as usize]);
                    t += step;
                }
            }
            Palette::Radial { squared, radii, half, .. } => {
                // t² = dx² + dy², ou dx avance de 1/rx a chaque pixel.
                let dy = (ly - half.1) / radii.1;
                let dy2 = dy * dy;
                let inv_rx = 1.0 / radii.0;
                let mut dx = (lx - half.0) * inv_rx;
                let scale = (SQUARED_STEPS - 1) as f32;
                for p in pixels.chunks_exact_mut(4) {
                    let t2 = (dx * dx + dy2).min(1.0);
                    blend_full(p, squared[(t2 * scale) as usize]);
                    dx += inv_rx;
                }
            }
        }
    }

    #[inline]
    fn at(&self, lx: f32, ly: f32) -> Color {
        match self {
            Palette::Solid(c) => *c,
            Palette::Linear { table, dir, length, half } => {
                let t = ((lx - half.0) * dir.0 + (ly - half.1) * dir.1) / length + 0.5;
                table[(t.clamp(0.0, 1.0) * (GRADIENT_STEPS - 1) as f32).round() as usize]
            }
            Palette::Radial { table, radii, half, .. } => {
                let (dx, dy) = ((lx - half.0) / radii.0, (ly - half.1) / radii.1);
                let t = (dx * dx + dy * dy).sqrt();
                table[(t.clamp(0.0, 1.0) * (GRADIENT_STEPS - 1) as f32).round() as usize]
            }
        }
    }
}

fn gradient_table(stops: &[ColorStop]) -> Vec<Color> {
    let transparent = Color::new(0, 0, 0, 0);
    (0..GRADIENT_STEPS)
        .map(|i| {
            let t = i as f32 / (GRADIENT_STEPS - 1) as f32;
            let Some(first) = stops.first() else { return transparent };
            let last = stops.last().unwrap();
            if t <= first.position {
                return first.color;
            }
            if t >= last.position {
                return last.color;
            }
            let k = stops.windows(2).position(|w| t >= w[0].position && t <= w[1].position).unwrap_or(0);
            let (a, b) = (stops[k], stops[k + 1]);
            let span = (b.position - a.position).max(f32::EPSILON);
            mix_premultiplied(a.color, b.color, (t - a.position) / span)
        })
        .collect()
}

fn mix_premultiplied(a: Color, b: Color, t: f32) -> Color {
    let (aa, ba) = (a.a as f32 / 255.0, b.a as f32 / 255.0);
    let alpha = aa + (ba - aa) * t;
    if alpha <= 0.0 {
        return Color::new(0, 0, 0, 0);
    }
    let channel = |ca: u8, cb: u8| ((ca as f32 * aa + (cb as f32 * ba - ca as f32 * aa) * t) / alpha).round().clamp(0.0, 255.0) as u8;
    Color::new(channel(a.r, b.r), channel(a.g, b.g), channel(a.b, b.b), (alpha * 255.0).round() as u8)
}

/// Opacite de groupe (`opacity` CSS) : `layer` est un rendu de la zone
/// `(x, y, width, height)` de `canvas` fait par-dessus une copie de son
/// contenu ; on garde `opacity` de ce rendu et `1 - opacity` de l'original.
pub fn blend_layer(canvas: &mut Canvas, layer: &Canvas, x: u32, y: u32, opacity: f32) {
    let a = opacity.clamp(0.0, 1.0);
    let inv = 1.0 - a;
    let row_len = layer.width.min(canvas.width.saturating_sub(x)) as usize * 4;
    for ly in 0..layer.height.min(canvas.height.saturating_sub(y)) {
        let src = (ly * layer.width) as usize * 4;
        let dst = ((y + ly) * canvas.width + x) as usize * 4;
        for i in (0..row_len).step_by(4) {
            for c in 0..3 {
                let v = layer.buffer[src + i + c] as f32 * a + canvas.buffer[dst + i + c] as f32 * inv;
                canvas.buffer[dst + i + c] = (v + 0.5) as u8;
            }
            canvas.buffer[dst + i + 3] = 255;
        }
    }
}
