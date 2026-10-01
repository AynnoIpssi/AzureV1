// Generation des icones de curseur, sans aucun fichier ni theme systeme :
// chaque forme est decrite par une fonction de distance signee (SDF -
// negative a l'interieur, positive a l'exterieur, en pixels), puis
// rasterisee pixel par pixel. Une SDF donne gratuitement l'anti-aliasing
// (couverture = distance au bord) ET le contour blanc (meme forme, "gonflee"
// de `OUTLINE` pixels), sans jamais tracer de chemin a la main.
//
// Style inspire d'Adwaita (GNOME) : forme noire, contour blanc, legere
// ombre portee - lisible sur fond clair comme sur fond sombre.
use crate::cursor::models::cursor_kind::CursorKind;
use azure_engine::platform::wayland::models::cursor::CursorImage;

/// Cote de l'image, en pixels - la taille habituelle d'un curseur a
/// l'echelle 1 (le dessin lui-meme occupe ~24px, le reste laisse la place
/// au contour et a l'ombre).
pub const CURSOR_SIZE: u32 = 32;

const OUTLINE: f32 = 1.3;
const SHADOW_OFFSET: (f32, f32) = (0.8, 1.4);
const SHADOW_BLUR: f32 = 1.6;
const SHADOW_ALPHA: f32 = 0.35;
const FILL: (f32, f32, f32) = (0.05, 0.05, 0.07);
const EDGE: (f32, f32, f32) = (1.0, 1.0, 1.0);

// Une forme = sa SDF + les traits "internes" (ex: separation entre les
// doigts de la main), dessines couleur contour par-dessus le remplissage.
struct Shape {
    sdf: fn(f32, f32) -> f32,
    details: fn(f32, f32) -> f32,
    hotspot: (u32, u32),
}

pub fn generate(kind: CursorKind) -> CursorImage {
    let shape = match kind {
        CursorKind::Default => Shape { sdf: arrow, details: none, hotspot: (4, 3) },
        CursorKind::Pointer => Shape { sdf: hand, details: hand_details, hotspot: (12, 3) },
        CursorKind::Text => Shape { sdf: ibeam, details: none, hotspot: (16, 15) },
        CursorKind::NotAllowed => Shape { sdf: not_allowed, details: none, hotspot: (16, 16) },
        CursorKind::Crosshair => Shape { sdf: crosshair, details: none, hotspot: (16, 16) },
        CursorKind::Move => Shape { sdf: move_arrows, details: none, hotspot: (16, 16) },
        CursorKind::Wait => Shape { sdf: wait_ring, details: none, hotspot: (16, 16) },
        CursorKind::Grab => Shape { sdf: hand, details: hand_details, hotspot: (16, 14) },
        CursorKind::Help => Shape { sdf: help, details: none, hotspot: (4, 3) },
        CursorKind::ResizeHorizontal => Shape { sdf: resize_horizontal, details: none, hotspot: (16, 16) },
        CursorKind::ResizeVertical => Shape { sdf: resize_vertical, details: none, hotspot: (16, 16) },
    };
    rasterize(&shape)
}

fn rasterize(shape: &Shape) -> CursorImage {
    let mut pixels = Vec::with_capacity((CURSOR_SIZE * CURSOR_SIZE * 4) as usize);
    for y in 0..CURSOR_SIZE {
        for x in 0..CURSOR_SIZE {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = (shape.sdf)(px, py);
            let fill = coverage(d);
            let outer = coverage(d - OUTLINE);
            // Traits internes : "rognent" le remplissage noir pour laisser
            // voir la couleur du contour, seulement a l'interieur de la forme.
            let fill = fill * (1.0 - coverage((shape.details)(px, py)));

            let shadow_d = (shape.sdf)(px - SHADOW_OFFSET.0, py - SHADOW_OFFSET.1) - OUTLINE;
            let shadow = (0.5 - shadow_d / SHADOW_BLUR).clamp(0.0, 1.0) * SHADOW_ALPHA;

            // Composition premultipliee : noir (remplissage) par-dessus
            // blanc (contour), le tout par-dessus l'ombre.
            let mut rgb = [0.0f32; 3];
            let edge = [EDGE.0, EDGE.1, EDGE.2];
            let body = [FILL.0, FILL.1, FILL.2];
            for c in 0..3 {
                rgb[c] = body[c] * fill + edge[c] * (outer - fill);
            }
            let alpha = outer + shadow * (1.0 - outer);

            let to_u8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            pixels.extend_from_slice(&[to_u8(rgb[2]), to_u8(rgb[1]), to_u8(rgb[0]), to_u8(alpha)]);
        }
    }
    CursorImage { width: CURSOR_SIZE, height: CURSOR_SIZE, hotspot_x: shape.hotspot.0, hotspot_y: shape.hotspot.1, pixels }
}

// Distance -> couverture du pixel (1 = entierement dedans) : le bord est
// lisse sur ~1 pixel, ce qui suffit a eviter tout effet d'escalier.
fn coverage(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

fn none(_: f32, _: f32) -> f32 {
    f32::MAX
}

// ---------------------------------------------------------------- formes

fn arrow(x: f32, y: f32) -> f32 {
    const ARROW: [(f32, f32); 7] = [(4.0, 3.0), (4.0, 20.5), (8.2, 16.4), (11.4, 23.6), (14.2, 22.4), (11.1, 15.4), (16.8, 15.4)];
    polygon(x, y, &ARROW)
}

fn hand(x: f32, y: f32) -> f32 {
    let index = capsule(x, y, (12.0, 5.8), (12.0, 16.0), 2.7);
    let middle = capsule(x, y, (16.9, 13.2), (16.9, 16.5), 2.5);
    let ring = capsule(x, y, (21.3, 14.2), (21.3, 17.5), 2.4);
    let pinky = capsule(x, y, (25.2, 16.2), (25.2, 19.0), 2.1);
    let thumb = capsule(x, y, (7.2, 16.6), (11.0, 22.5), 2.4);
    let palm = rounded_box(x, y, (18.6, 22.4), (7.9, 5.6), 4.0);
    index.min(middle).min(ring).min(pinky).min(thumb).min(palm)
}

// Separations entre les doigts replies, sans quoi la main ne serait qu'une
// silhouette pleine.
fn hand_details(x: f32, y: f32) -> f32 {
    let a = capsule(x, y, (14.5, 14.2), (14.5, 18.6), 0.45);
    let b = capsule(x, y, (19.1, 14.6), (19.1, 19.0), 0.45);
    let c = capsule(x, y, (23.3, 16.4), (23.3, 20.0), 0.45);
    a.min(b).min(c)
}

fn ibeam(x: f32, y: f32) -> f32 {
    let stem = capsule(x, y, (16.0, 6.0), (16.0, 24.0), 1.1);
    let top_left = capsule(x, y, (12.6, 4.4), (15.0, 5.4), 1.1);
    let top_right = capsule(x, y, (17.0, 5.4), (19.4, 4.4), 1.1);
    let bottom_left = capsule(x, y, (12.6, 25.6), (15.0, 24.6), 1.1);
    let bottom_right = capsule(x, y, (17.0, 24.6), (19.4, 25.6), 1.1);
    stem.min(top_left).min(top_right).min(bottom_left).min(bottom_right)
}

fn ring(x: f32, y: f32, center: (f32, f32), radius: f32, thickness: f32) -> f32 {
    (((x - center.0).powi(2) + (y - center.1).powi(2)).sqrt() - radius).abs() - thickness
}

fn not_allowed(x: f32, y: f32) -> f32 {
    ring(x, y, (16.0, 16.0), 8.5, 1.8).min(capsule(x, y, (10.3, 21.7), (21.7, 10.3), 1.8))
}

fn crosshair(x: f32, y: f32) -> f32 {
    capsule(x, y, (16.0, 5.0), (16.0, 27.0), 0.9).min(capsule(x, y, (5.0, 16.0), (27.0, 16.0), 0.9))
}

// Double fleche sur l'axe (ax, ay) (vecteur unitaire), centree en (16, 16).
fn double_arrow(x: f32, y: f32, ax: f32, ay: f32, half_length: f32) -> f32 {
    let (c, n) = ((16.0, 16.0), (-ay, ax));
    let at = |t: f32, s: f32| (c.0 + ax * t + n.0 * s, c.1 + ay * t + n.1 * s);
    let shaft = capsule(x, y, at(-half_length + 3.0, 0.0), at(half_length - 3.0, 0.0), 1.2);
    let head = |dir: f32| polygon(x, y, &[at(dir * half_length, 0.0), at(dir * (half_length - 5.5), 4.2), at(dir * (half_length - 5.5), -4.2)]);
    shaft.min(head(1.0)).min(head(-1.0))
}

fn move_arrows(x: f32, y: f32) -> f32 {
    double_arrow(x, y, 1.0, 0.0, 11.5).min(double_arrow(x, y, 0.0, 1.0, 11.5))
}

fn resize_horizontal(x: f32, y: f32) -> f32 {
    double_arrow(x, y, 1.0, 0.0, 11.0)
}

fn resize_vertical(x: f32, y: f32) -> f32 {
    double_arrow(x, y, 0.0, 1.0, 11.0)
}

fn wait_ring(x: f32, y: f32) -> f32 {
    // Anneau ouvert en haut a droite (un quart), comme une roue qui tourne.
    let (dx, dy) = (x - 16.0, y - 16.0);
    let gap = dx > 0.0 && dy < 0.0;
    let r = ring(x, y, (16.0, 16.0), 8.0, 1.9);
    let r = if gap { r.max(1.0) } else { r };
    // Point central : le point chaud tombe sur la forme.
    r.min(capsule(x, y, (16.0, 16.0), (16.0, 16.0), 1.6))
}

fn help(x: f32, y: f32) -> f32 {
    let question = ring(x, y, (22.5, 19.0), 3.0, 1.0).max(-(y - 19.5)).min(capsule(x, y, (22.5, 22.0), (22.5, 23.5), 1.0)).min(capsule(x, y, (22.5, 26.5), (22.5, 26.6), 1.1));
    arrow(x, y).min(question)
}

// ------------------------------------------------------- primitives SDF

fn capsule(x: f32, y: f32, a: (f32, f32), b: (f32, f32), radius: f32) -> f32 {
    let (px, py) = (x - a.0, y - a.1);
    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
    let t = ((px * ex + py * ey) / (ex * ex + ey * ey).max(f32::EPSILON)).clamp(0.0, 1.0);
    let (dx, dy) = (px - ex * t, py - ey * t);
    (dx * dx + dy * dy).sqrt() - radius
}

fn rounded_box(x: f32, y: f32, center: (f32, f32), half: (f32, f32), radius: f32) -> f32 {
    let qx = (x - center.0).abs() - half.0 + radius;
    let qy = (y - center.1).abs() - half.1 + radius;
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - radius
}

// Distance signee a un polygone quelconque (non forcement convexe) :
// distance au segment le plus proche, signe donne par la regle pair/impair.
fn polygon(x: f32, y: f32, points: &[(f32, f32)]) -> f32 {
    let n = points.len();
    let mut dist = f32::MAX;
    let mut inside = false;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + n - 1) % n];
        dist = dist.min(capsule(x, y, a, b, 0.0));
        if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
    }
    if inside { -dist } else { dist }
}
