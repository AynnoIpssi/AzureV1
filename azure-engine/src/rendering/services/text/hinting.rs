// Auto-hinting VERTICAL (equivalent du mode "slight" de FreeType, celui
// des bureaux Linux par defaut) : sans lui, une police de 8-13px met ses
// lignes horizontales (ligne de base, hauteur des minuscules, barre du e,
// haut du T...) a cheval sur deux rangees de pixels, qui sortent alors
// grises et baveuses. On deforme l'outline, sur l'axe Y seulement, pour
// que ces bords tombent pile entre deux rangees.
//
// Etapes (voir README §15) :
// 1. Zones bleues : hauteurs de reference de la police (ligne de base,
//    hauteur des minuscules, des capitales, ascendantes, descendantes),
//    lues sur des lettres temoins (H, x, o, O, l, p) et arrondies au pixel.
//    Le depassement des lettres rondes (le bas du o descend un peu sous la
//    ligne de base) est supprime s'il fait moins d'un demi-pixel.
// 2. Segments : morceaux presque horizontaux de l'outline qui sont un
//    extremum local (haut d'un arc, dessus d'une barre...), avec le cote
//    ou se trouve l'encre (dessus ou dessous).
// 3. Bords : segments a la meme hauteur regroupes (les pieds des deux
//    jambes du H forment un seul bord).
// 4. Calage : un bord dans une zone bleue va sur la zone ; deux bords
//    opposes qui se font face forment une barre horizontale, dont
//    l'epaisseur est arrondie (1px minimum) puis posee sur la grille.
// 5. Deformation : chaque point de l'outline est deplace par interpolation
//    lineaire entre les bords cales qui l'entourent. La fonction est
//    croissante : l'ordre vertical des points ne change jamais, donc
//    l'outline ne se replie pas sur elle-meme.
//
// L'axe X n'est pas touche : les chasses et l'espacement restent ceux de
// la police (le texte ne change pas de largeur).

use std::collections::HashMap;
use std::rc::Rc;

/// Pente maximale (|dy| / |dx|) d'un trait encore considere horizontal.
const MAX_SLOPE: f32 = 0.2;

/// Une hauteur de reference de la police (en unites de police) et sa
/// position calee sur la grille (en pixels, vers le haut depuis la ligne
/// de base).
#[derive(Clone, Copy, Debug)]
pub struct BlueZone {
    /// Zone de haut de lettre (encre en dessous) ou de bas (encre au-dessus).
    pub top: bool,
    /// Bord plat (haut du x, pied du H).
    pub reference: f32,
    /// Bord rond (haut du o), qui depasse un peu `reference`.
    pub overshoot: f32,
    pub reference_px: f32,
    pub overshoot_px: f32,
}

struct NoOutline;

impl ttf_parser::OutlineBuilder for NoOutline {
    fn move_to(&mut self, _: f32, _: f32) {}
    fn line_to(&mut self, _: f32, _: f32) {}
    fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {}
    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {}
    fn close(&mut self) {}
}

// Etendue verticale (bas, haut) de l'encre d'un caractere, variation de
// poids comprise (`outline_glyph` calcule la boite sur l'outline reelle).
fn y_extent(face: &ttf_parser::Face, character: char) -> Option<(f32, f32)> {
    let id = face.glyph_index(character)?;
    let rect = face.outline_glyph(id, &mut NoOutline)?;
    Some((rect.y_min as f32, rect.y_max as f32))
}

fn first_extent(face: &ttf_parser::Face, characters: &[char]) -> Option<(f32, f32)> {
    characters.iter().find_map(|&c| y_extent(face, c))
}

fn zone(top: bool, reference: f32, overshoot: f32, scale: f32) -> BlueZone {
    // Un depassement dans le mauvais sens (police atypique) n'en est pas un.
    let overshoot = if top { overshoot.max(reference) } else { overshoot.min(reference) };
    let reference_px = (reference * scale).round();
    let delta = (overshoot - reference) * scale;
    // Moins d'un demi-pixel : le o s'aligne sur le x, sinon il parait
    // plus grand que ses voisines d'un pixel entier.
    let delta_px = if delta.abs() < 0.5 { 0.0 } else { delta.round() };
    BlueZone { top, reference, overshoot, reference_px, overshoot_px: reference_px + delta_px }
}

fn compute_zones(face: &ttf_parser::Face, scale: f32) -> Vec<BlueZone> {
    let mut zones = Vec::new();
    if let Some((flat, _)) = first_extent(face, &['H', 'x', 'n']) {
        let round = first_extent(face, &['o', 'O']).map_or(flat, |e| e.0);
        zones.push(zone(false, flat, round, scale));
    }
    if let Some((flat, _)) = first_extent(face, &['p', 'q']) {
        zones.push(zone(false, flat, flat, scale));
    }
    if let Some((_, flat)) = first_extent(face, &['x', 'z']) {
        let round = first_extent(face, &['o', 'e']).map_or(flat, |e| e.1);
        zones.push(zone(true, flat, round, scale));
    }
    if let Some((_, flat)) = first_extent(face, &['H', 'E']) {
        let round = first_extent(face, &['O', '0']).map_or(flat, |e| e.1);
        zones.push(zone(true, flat, round, scale));
    }
    if let Some((_, flat)) = first_extent(face, &['l', 'd', 'h']) {
        zones.push(zone(true, flat, flat, scale));
    }
    zones
}

// (police, echelle, poids) -> zones.
type ZoneCache = HashMap<(usize, u32, u32), Rc<Vec<BlueZone>>>;

thread_local! {
    static ZONES: std::cell::RefCell<ZoneCache> =
        std::cell::RefCell::new(HashMap::new());
}

/// Zones bleues de `face` (variation de poids deja appliquee) a cette
/// echelle. `weight` ne sert qu'a la cle du cache : les zones dependent du
/// poids via l'outline, que `face` porte deja.
pub fn blue_zones(face: &ttf_parser::Face, scale: f32, weight: f32) -> Rc<Vec<BlueZone>> {
    let key = (face.raw_face().data.as_ptr() as usize, scale.to_bits(), weight.to_bits());
    if let Some(zones) = ZONES.with(|z| z.borrow().get(&key).cloned()) {
        return zones;
    }
    let zones = Rc::new(compute_zones(face, scale));
    ZONES.with(|z| z.borrow_mut().insert(key, zones.clone()));
    zones
}

#[derive(Debug)]
struct Segment {
    y: f32,
    ink_below: bool,
    x_min: f32,
    x_max: f32,
}

// Aire signee de toutes les contours : positive = sens trigonometrique
// (contours externes des polices CFF), negative = sens horaire (TrueType).
fn counter_clockwise(contours: &[Vec<(f32, f32)>]) -> bool {
    let mut area = 0.0f32;
    for contour in contours {
        for i in 0..contour.len() {
            let (x0, y0) = contour[i];
            let (x1, y1) = contour[(i + 1) % contour.len()];
            area += x0 * y1 - x1 * y0;
        }
    }
    area > 0.0
}

fn find_segments(contours: &[Vec<(f32, f32)>], min_length: f32) -> Vec<Segment> {
    let ccw = counter_clockwise(contours);
    let mut segments = Vec::new();

    for raw in contours {
        // Points consecutifs confondus (fermeture explicite d'un contour) :
        // un trait de longueur nulle n'a pas de direction.
        let mut points: Vec<(f32, f32)> = Vec::with_capacity(raw.len());
        for &p in raw {
            if points.last().is_none_or(|q: &(f32, f32)| (q.0 - p.0).abs() > 1e-3 || (q.1 - p.1).abs() > 1e-3) {
                points.push(p);
            }
        }
        while points.len() > 1 && {
            let (a, b) = (points[0], points[points.len() - 1]);
            (a.0 - b.0).abs() <= 1e-3 && (a.1 - b.1).abs() <= 1e-3
        } {
            points.pop();
        }
        let n = points.len();
        if n < 3 {
            continue;
        }

        let point = |i: usize| points[i % n];
        // Direction d'un trait presque horizontal : +1 vers la droite,
        // -1 vers la gauche, 0 s'il ne l'est pas.
        let direction = |i: usize| {
            let (a, b) = (point(i), point(i + 1));
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            if dx != 0.0 && dy.abs() <= dx.abs() * MAX_SLOPE { dx.signum() as i32 } else { 0 }
        };

        // On commence juste apres un trait non horizontal : aucune suite de
        // traits horizontaux ne chevauche alors le debut de la boucle.
        let Some(start) = (0..n).find(|&i| direction(i + n - 1) == 0) else { continue };

        let mut k = 0;
        while k < n {
            let i0 = start + k;
            let dir = direction(i0);
            if dir == 0 {
                k += 1;
                continue;
            }
            let mut len = 1;
            while k + len < n && direction(i0 + len) == dir {
                len += 1;
            }
            k += len;

            // Points du segment : i0 ..= i0 + len ; voisins : i0 - 1 et i0 + len + 1.
            let (mut y_min, mut y_max) = (f32::MAX, f32::MIN);
            let (mut x_min, mut x_max) = (f32::MAX, f32::MIN);
            for j in i0..=i0 + len {
                let (x, y) = point(j);
                y_min = y_min.min(y);
                y_max = y_max.max(y);
                x_min = x_min.min(x);
                x_max = x_max.max(x);
            }
            if x_max - x_min < min_length {
                continue;
            }
            let before = point(i0 + n - 1).1;
            let after = point(i0 + len + 1).1;
            // Un bord n'est retenu que s'il est un extremum local : un
            // morceau de pente douce au milieu d'une diagonale n'en est pas un.
            let y = match (before < y_max && after < y_max, before > y_min && after > y_min) {
                (true, false) => y_max,
                (false, true) => y_min,
                _ => continue,
            };
            // Encre a droite du sens de parcours pour un contour horaire.
            let ink_below = (dir > 0) != ccw;
            segments.push(Segment { y, ink_below, x_min, x_max });
        }
    }
    segments
}

struct Edge {
    y: f32,
    // Hauteurs extremes des segments regroupes : tous vont sur la MEME
    // rangee (le haut du fut et le haut du bras du r de Sora, 534 et 540,
    // sinon le bras depasse d'une fraction de pixel).
    y_lo: f32,
    y_hi: f32,
    ink_below: bool,
    ranges: Vec<(f32, f32)>,
    // Position calee, en pixels.
    hinted: Option<f32>,
    // Calee sur une zone bleue (prioritaire sur une barre).
    blue: bool,
}

fn overlaps(a: &Edge, b: &Edge) -> bool {
    a.ranges.iter().any(|ra| b.ranges.iter().any(|rb| ra.0 < rb.1 && rb.0 < ra.1))
}

fn build_edges(mut segments: Vec<Segment>, merge: f32) -> Vec<Edge> {
    segments.sort_by(|a, b| (a.ink_below, a.y).partial_cmp(&(b.ink_below, b.y)).unwrap());
    let mut edges: Vec<(Edge, f32)> = Vec::new();
    for s in segments {
        let length = s.x_max - s.x_min;
        if let Some((edge, longest)) = edges.last_mut()
            && edge.ink_below == s.ink_below
            && (s.y - edge.y).abs() <= merge
        {
            edge.ranges.push((s.x_min, s.x_max));
            edge.y_lo = edge.y_lo.min(s.y);
            edge.y_hi = edge.y_hi.max(s.y);
            // Hauteur du bord = celle de son plus long segment.
            if length > *longest {
                edge.y = s.y;
                *longest = length;
            }
            continue;
        }
        edges.push((Edge { y: s.y, y_lo: s.y, y_hi: s.y, ink_below: s.ink_below, ranges: vec![(s.x_min, s.x_max)], hinted: None, blue: false }, length));
    }
    edges.into_iter().map(|(e, _)| e).collect()
}

/// Deformation verticale d'un glyphe : unites de police -> pixels.
pub struct VerticalWarp {
    // (y en unites de police, y cale en pixels), y croissants.
    anchors: Vec<(f32, f32)>,
    scale: f32,
}

impl VerticalWarp {
    pub fn map(&self, y: f32) -> f32 {
        let a = &self.anchors;
        let Some(first) = a.first() else { return y * self.scale };
        let i = a.partition_point(|p| p.0 <= y);
        if i == 0 {
            return first.1 + (y - first.0) * self.scale;
        }
        if i == a.len() {
            let last = a[a.len() - 1];
            return last.1 + (y - last.0) * self.scale;
        }
        let (lo, hi) = (a[i - 1], a[i]);
        lo.1 + (y - lo.0) / (hi.0 - lo.0) * (hi.1 - lo.1)
    }
}

/// Calcule la deformation verticale d'un glyphe dont l'outline (aplatie,
/// en unites de police) est `contours`.
pub fn vertical_warp(contours: &[Vec<(f32, f32)>], zones: &[BlueZone], scale: f32, units_per_em: f32) -> VerticalWarp {
    let segments = find_segments(contours, units_per_em * 0.01);
    let mut edges = build_edges(segments, units_per_em / 128.0);

    // Zones bleues.
    let fuzz = units_per_em * 0.02;
    for edge in &mut edges {
        let best = zones
            .iter()
            .filter(|z| z.top == edge.ink_below)
            .map(|z| {
                let (lo, hi) = (z.reference.min(z.overshoot), z.reference.max(z.overshoot));
                let dist = if edge.y < lo { lo - edge.y } else if edge.y > hi { edge.y - hi } else { 0.0 };
                (dist, z)
            })
            .filter(|(dist, _)| *dist <= fuzz)
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if let Some((_, z)) = best {
            let flat = (edge.y - z.reference).abs() <= (edge.y - z.overshoot).abs();
            edge.hinted = Some(if flat { z.reference_px } else { z.overshoot_px });
            edge.blue = true;
        }
    }

    // Barres horizontales : un bord avec l'encre au-dessus (bas de la
    // barre) face a un bord avec l'encre en dessous (haut), qui se
    // chevauchent en X. Les paires les plus fines d'abord.
    let max_stem = units_per_em * 0.25;
    let mut pairs = Vec::new();
    for (b, bottom) in edges.iter().enumerate() {
        if bottom.ink_below {
            continue;
        }
        for (t, top) in edges.iter().enumerate() {
            let width = top.y - bottom.y;
            if top.ink_below && width > 0.0 && width <= max_stem && overlaps(bottom, top) {
                pairs.push((width, b, t));
            }
        }
    }
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut used = vec![false; edges.len()];
    for (width, b, t) in pairs {
        if used[b] || used[t] {
            continue;
        }
        used[b] = true;
        used[t] = true;
        let width_px = (width * scale).round().max(1.0);
        match (edges[b].blue, edges[t].blue) {
            (true, true) => {}
            (true, false) => edges[t].hinted = edges[b].hinted.map(|y| y + width_px),
            (false, true) => edges[b].hinted = edges[t].hinted.map(|y| y - width_px),
            (false, false) => {
                let center = (edges[b].y + edges[t].y) * 0.5 * scale;
                let bottom = (center - width_px * 0.5).round();
                edges[b].hinted = Some(bottom);
                edges[t].hinted = Some(bottom + width_px);
            }
        }
    }

    // Bords cales -> points d'ancrage, ordonnes, sans jamais redescendre.
    let mut anchors: Vec<(f32, f32)> = edges
        .iter()
        .filter_map(|e| e.hinted.map(|h| [(e.y_lo, h), (e.y, h), (e.y_hi, h)]))
        .flatten()
        .collect();
    anchors.sort_by(|a, b| a.partial_cmp(b).unwrap());
    anchors.dedup_by(|next, prev| (next.0 - prev.0).abs() < 1e-3);
    for i in 1..anchors.len() {
        if anchors[i].1 < anchors[i - 1].1 {
            anchors[i].1 = anchors[i - 1].1;
        }
    }
    VerticalWarp { anchors, scale }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UPEM: f32 = 1000.0;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(f32, f32)> {
        // Sens horaire (convention TrueType).
        vec![(x0, y0), (x0, y1), (x1, y1), (x1, y0)]
    }

    fn zones(scale: f32) -> Vec<BlueZone> {
        vec![zone(false, 0.0, -12.0, scale), zone(true, 500.0, 512.0, scale)]
    }

    #[test]
    fn baseline_and_x_height_land_on_pixels() {
        // Pave de la ligne de base a la hauteur des minuscules, a 13px :
        // 500 unites = 6.5px, cale sur 6 ou 7 pixels entiers.
        let scale = 13.0 / UPEM;
        let warp = vertical_warp(&[rect(0.0, 0.0, 400.0, 500.0)], &zones(scale), scale, UPEM);
        assert_eq!(warp.map(0.0), 0.0);
        let top = warp.map(500.0);
        assert_eq!(top, top.round());
    }

    #[test]
    fn horizontal_bar_gets_whole_pixel_thickness() {
        // Barre de 70 unites (0.77px a 11px) flottant au milieu : 1px pile.
        let scale = 11.0 / UPEM;
        let warp = vertical_warp(&[rect(0.0, 230.0, 400.0, 300.0)], &[], scale, UPEM);
        let (bottom, top) = (warp.map(230.0), warp.map(300.0));
        assert_eq!(bottom, bottom.round());
        assert_eq!(top - bottom, 1.0);
    }

    #[test]
    fn warp_is_monotonic() {
        let scale = 9.0 / UPEM;
        let contours = [rect(0.0, 0.0, 400.0, 80.0), rect(0.0, 230.0, 400.0, 300.0), rect(0.0, 420.0, 400.0, 500.0)];
        let warp = vertical_warp(&contours, &zones(scale), scale, UPEM);
        let mut prev = f32::MIN;
        for y in (-100..700).map(|v| v as f32) {
            let m = warp.map(y);
            assert!(m >= prev, "deformation non croissante en y={y}");
            prev = m;
        }
    }

    #[test]
    fn merged_segments_share_one_row() {
        // Deux hauts presque a la meme hauteur (534 et 540, comme le r de
        // Sora) : les deux sur la meme rangee de pixels.
        let scale = 12.0 / UPEM;
        let zones = [zone(false, 0.0, -18.0, scale), zone(true, 534.0, 552.0, scale)];
        let contours = [rect(0.0, 0.0, 80.0, 534.0), rect(100.0, 450.0, 300.0, 540.0)];
        let warp = vertical_warp(&contours, &zones, scale, UPEM);
        assert_eq!(warp.map(534.0), warp.map(540.0));
        assert_eq!(warp.map(540.0), (534.0 * scale).round());
    }

    #[test]
    fn small_overshoot_is_suppressed() {
        let z = zone(true, 500.0, 512.0, 12.0 / UPEM);
        assert_eq!(z.reference_px, z.overshoot_px);
    }
}
