// Le dessin d'une toile (voir `ui::models::toile`) : grille de points, liens
// (epaisseur, courbe, fleche a un ou deux bouts, texte pres de `vers` ou au
// milieu), trait du mode relier, puis les boites (teintes). Avec `focus`, la
// boite mise en avant et ses voisines ressortent, le reste palit. Tout a
// l'echelle du zoom, decoupe a la boite de la toile.
use crate::layout::managers::layout_manager::Rect;
use crate::ui::models::toile::{mesure_par_defaut, Boite, GenreBoite, Geste, Lien, StyleLigne, Teinte, Toile, EPAISSEUR_BARRE, HAUT_LIGNE, HAUT_TITRE, MARGE, TEXTE, TITRE};
use crate::ui::services::draw_ui::FONT_PATH;
use azure_engine::rendering::managers::renderer::{draw_box, draw_text};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::{BorderWidths, BoxStyle};

/// Un rectangle a l'ecran en flottants : (x, y, largeur, hauteur).
type Zone = (f32, f32, f32, f32);

const FOND: Color = Color::new(24, 23, 21, 255);
const POINT: Color = Color::new(52, 50, 46, 255);
const TRAIT: Color = Color::new(138, 133, 122, 255);
const TEXTE_C: Color = Color::new(231, 229, 225, 255);
const DISCRET: Color = Color::new(140, 137, 129, 255);
const ACCENT: Color = Color::new(201, 168, 120, 255);
const ALERTE: Color = Color::new(217, 135, 106, 255);
/// Opacite de ce qui n'est pas en avant (avec `focus`).
const PALI: u8 = 50;

fn pali(c: Color) -> Color {
    Color::new(c.r, c.g, c.b, PALI)
}

// Melange `couleur` dans un pixel (opacite `a`), dans la zone `clip`.
fn pixel(canvas: &mut Canvas, x: i32, y: i32, couleur: Color, a: f32, clip: (i32, i32, i32, i32)) {
    if x < clip.0 || y < clip.1 || x >= clip.2 || y >= clip.3 || a <= 0.0 {
        return;
    }
    let i = ((y as u32 * canvas.width + x as u32) * 4) as usize;
    let a = a.min(1.0) * couleur.a as f32 / 255.0;
    let b = &mut canvas.buffer[i..i + 4];
    for (k, c) in [(0, couleur.b), (1, couleur.g), (2, couleur.r)] {
        b[k] = (b[k] as f32 * (1.0 - a) + c as f32 * a).round() as u8;
    }
    b[3] = 255;
}

// Un trait antialiase d'epaisseur `e`, parcouru le long de son grand axe ;
// `tirets` : un trait sur deux.
fn trait_(canvas: &mut Canvas, (x1, y1): (f32, f32), (x2, y2): (f32, f32), e: f32, couleur: Color, clip: (i32, i32, i32, i32), tirets: bool) {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let long = (dx * dx + dy * dy).sqrt();
    if long < 0.5 {
        return;
    }
    let horizontal = dx.abs() >= dy.abs();
    let pas = if horizontal { dx.abs() } else { dy.abs() }.ceil() as i32;
    // Distance perpendiculaire pour un ecart d'un pixel sur le petit axe.
    let facteur = if horizontal { dx.abs() / long } else { dy.abs() / long };
    let demi = e / 2.0;
    for i in 0..=pas {
        if tirets && (i / 6) % 2 == 1 {
            continue;
        }
        let t = i as f32 / pas.max(1) as f32;
        let (cx, cy) = (x1 + dx * t, y1 + dy * t);
        let (centre, autre) = if horizontal { (cy, cx) } else { (cx, cy) };
        let r = (demi / facteur.max(0.3)).ceil() as i32 + 1;
        let base = centre.floor() as i32;
        for k in (base - r)..=(base + r) {
            let d = ((k as f32 + 0.5) - centre).abs() * facteur;
            let a = (demi + 0.5 - d).clamp(0.0, 1.0);
            let (px, py) = if horizontal { (autre.round() as i32, k) } else { (k, autre.round() as i32) };
            pixel(canvas, px, py, couleur, a, clip);
        }
    }
}

fn texte(canvas: &mut Canvas, s: &str, x: f32, y: f32, taille: f32, gras: bool, couleur: Color) {
    if x < 0.0 || y < 0.0 || taille < 4.0 {
        return;
    }
    let _ = draw_text(s, FONT_PATH, x as u32, y as u32, taille, if gras { 700.0 } else { 400.0 }, &couleur, canvas);
}

// Ou le segment centre(a) -> centre(b) sort du rectangle `r` (x, y, w, h).
fn bord(r: (f32, f32, f32, f32), vers: (f32, f32)) -> (f32, f32) {
    let (cx, cy) = (r.0 + r.2 / 2.0, r.1 + r.3 / 2.0);
    let (dx, dy) = (vers.0 - cx, vers.1 - cy);
    if dx.abs() < 1e-3 && dy.abs() < 1e-3 {
        return (cx, cy);
    }
    let tx = if dx.abs() > 1e-3 { (r.2 / 2.0) / dx.abs() } else { f32::MAX };
    let ty = if dy.abs() > 1e-3 { (r.3 / 2.0) / dy.abs() } else { f32::MAX };
    let t = tx.min(ty).min(1.0);
    (cx + dx * t, cy + dy * t)
}

/// Le rectangle d'une boite a l'ecran.
fn rect_ecran(t: &Toile, own: Rect, b: &Boite) -> (f32, f32, f32, f32) {
    let (w, h) = Toile::taille(b, &mesure_par_defaut);
    let (x, y) = t.vers_ecran(own, b.x, b.y);
    (x, y, w * t.zoom, h * t.zoom)
}

pub fn draw_toile(t: &Toile, own: Rect, canvas: &mut Canvas) {
    // La zone de la toile, dans celle deja permise (un conteneur qui defile).
    let avant = canvas.clip_bounds();
    let x0 = own.0.max(avant.0 as i32).max(0);
    let y0 = own.1.max(avant.1 as i32).max(0);
    let x1 = (own.0 + own.2 as i32).min((avant.0 + avant.2) as i32);
    let y1 = (own.1 + own.3 as i32).min((avant.1 + avant.3) as i32);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    canvas.set_clip(x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32);
    let clip = (x0, y0, x1, y1);
    draw_box(own.0, own.1, own.2, own.3, &BoxStyle::solid(FOND), canvas);

    // Grille de points (tous les 20 de la toile).
    let pas = 20.0 * t.zoom;
    if pas >= 8.0 {
        let debut_x = own.0 as f32 + t.decalage.0.rem_euclid(pas);
        let debut_y = own.1 as f32 + t.decalage.1.rem_euclid(pas);
        let mut y = debut_y;
        while y < y1 as f32 {
            let mut x = debut_x;
            while x < x1 as f32 {
                pixel(canvas, x as i32, y as i32, POINT, 1.0, clip);
                x += pas;
            }
            y += pas;
        }
    }

    let z = t.zoom;
    let rects: Vec<(&Boite, Zone)> = t.dessin.boites.iter().map(|b| (b, rect_ecran(t, own, b))).collect();
    let rect_de = |id: &str| rects.iter().find(|(b, _)| b.id == id).map(|(_, r)| *r);

    // Avec focus : la boite en avant et ses voisines.
    let avant_ = t.en_avant().filter(|id| rect_de(id).is_some());
    let touche = |l: &Lien| avant_.is_some_and(|a| l.de == a || l.vers == a);
    let voisines: Vec<&str> = match avant_ {
        Some(a) => t.dessin.liens.iter().filter(|l| touche(l)).flat_map(|l| [l.de.as_str(), l.vers.as_str()]).chain(std::iter::once(a)).collect(),
        None => Vec::new(),
    };

    // Les liens, sous les boites : ceux qui palissent d'abord, ceux en avant
    // par-dessus.
    let mut ordre: Vec<&Lien> = t.dessin.liens.iter().collect();
    ordre.sort_by_key(|l| touche(l));
    for l in ordre {
        let (Some(a), Some(b)) = (rect_de(&l.de), rect_de(&l.vers)) else { continue };
        let etat = match avant_ {
            None => Etat::Normal,
            Some(_) if touche(l) => Etat::Avant,
            Some(_) => Etat::Pali,
        };
        lien(canvas, l, a, b, z, etat, clip);
    }

    // Le trait du mode relier.
    if let Some(Geste::Relier { de, souris }) = &t.geste
        && let Some(a) = rect_de(de)
    {
        let depart = bord(a, (souris.0 as f32, souris.1 as f32));
        trait_(canvas, depart, (souris.0 as f32, souris.1 as f32), 1.6, ACCENT, clip, true);
    }

    for (b, r) in &rects {
        let choisie = t.choisi.as_deref() == Some(b.id.as_str()) || avant_ == Some(b.id.as_str());
        boite(canvas, b, *r, z, choisie, clip);
        if avant_.is_some() && !voisines.contains(&b.id.as_str()) {
            // Palie : un voile de la couleur du fond.
            let mut voile = BoxStyle::solid(Color::new(FOND.r, FOND.g, FOND.b, 175));
            voile.radius = 6.0 * z;
            draw_box(r.0 as i32, r.1 as i32, r.2.round() as u32 + 1, r.3.round() as u32 + 1, &voile, canvas);
        }
    }

    // Les barres de defilement : ou est la vue dans tout le dessin.
    let (h, v) = t.barres(own, &mesure_par_defaut);
    for barre in [h, v].into_iter().flatten() {
        let tenue = matches!(t.geste, Some(Geste::Barre { .. }));
        for (r, couleur) in [(barre.rail, Color::new(255, 255, 255, 14)), (barre.poignee, if tenue { Color::new(201, 168, 120, 200) } else { Color::new(160, 154, 142, 150) })] {
            let mut s = BoxStyle::solid(couleur);
            s.radius = EPAISSEUR_BARRE / 2.0;
            draw_box(r.0 as i32, r.1 as i32, r.2.max(1.0) as u32, r.3.max(1.0) as u32, &s, canvas);
        }
    }
    canvas.set_clip(avant.0, avant.1, avant.2, avant.3);
}

#[derive(Clone, Copy, PartialEq)]
enum Etat {
    Normal,
    Avant,
    Pali,
}

// Un point de la courbe de Bezier (a, controle, b) en t.
fn bezier(a: (f32, f32), c: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    (u * u * a.0 + 2.0 * u * t * c.0 + t * t * b.0, u * u * a.1 + 2.0 * u * t * c.1 + t * t * b.1)
}

// Une pointe de fleche en `pointe`, venant de `depuis`.
fn pointe(canvas: &mut Canvas, pointe: (f32, f32), depuis: (f32, f32), z: f32, e: f32, couleur: Color, clip: (i32, i32, i32, i32)) {
    let (dx, dy) = (depuis.0 - pointe.0, depuis.1 - pointe.1);
    let n = (dx * dx + dy * dy).sqrt().max(1.0);
    let (ux, uy) = (dx / n, dy / n);
    let taille = (10.0 + e * 1.5) * z;
    let (bx, by) = (pointe.0 + ux * taille, pointe.1 + uy * taille);
    let (px, py) = (-uy * taille * 0.5, ux * taille * 0.5);
    trait_(canvas, pointe, (bx + px, by + py), e.max(1.4), couleur, clip, false);
    trait_(canvas, pointe, (bx - px, by - py), e.max(1.4), couleur, clip, false);
}

fn lien(canvas: &mut Canvas, l: &Lien, a: Zone, b: Zone, z: f32, etat: Etat, clip: (i32, i32, i32, i32)) {
    let (ca, cb) = ((a.0 + a.2 / 2.0, a.1 + a.3 / 2.0), (b.0 + b.2 / 2.0, b.1 + b.3 / 2.0));
    // Le point de controle de la courbe : decale sur le cote du milieu.
    let (mx, my) = ((ca.0 + cb.0) / 2.0, (ca.1 + cb.1) / 2.0);
    let (dx, dy) = (cb.0 - ca.0, cb.1 - ca.1);
    let controle = if l.courbe { (mx - dy * 0.16, my + dx * 0.16) } else { (mx, my) };
    let pa = bord(a, controle);
    let pb = bord(b, controle);
    let base = if l.alerte { ALERTE } else if etat == Etat::Avant { ACCENT } else { TRAIT };
    let couleur = if etat == Etat::Pali { pali(base) } else { base };
    let e = (if l.epaisseur > 0.0 { l.epaisseur } else { 1.6 }) * z.max(0.6);
    let e = if etat == Etat::Avant { e + 0.6 } else { e };
    if l.courbe {
        let mut avant = pa;
        for k in 1..=20 {
            let p = bezier(pa, controle, pb, k as f32 / 20.0);
            trait_(canvas, avant, p, e, couleur, clip, false);
            avant = p;
        }
    } else {
        trait_(canvas, pa, pb, e, couleur, clip, false);
    }
    if l.fleche || l.double {
        pointe(canvas, pb, controle, z, e, couleur, clip);
    }
    if l.double {
        pointe(canvas, pa, controle, z, e, couleur, clip);
    }
    if l.texte.is_empty() || etat == Etat::Pali {
        return;
    }
    let couleur_texte = if l.alerte { ALERTE } else { ACCENT };
    let largeur = mesure_par_defaut(&l.texte, TEXTE * z, true);
    if l.milieu {
        // Au milieu, sur une pastille du fond : lisible par-dessus les traits.
        let (cx, cy) = if l.courbe { bezier(pa, controle, pb, 0.5) } else { ((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0) };
        let (pw, ph) = (largeur + 10.0 * z, TEXTE * z + 6.0 * z);
        let mut pastille = BoxStyle::solid(FOND);
        pastille.radius = ph / 2.0;
        pastille.border = BorderWidths::uniform(1.0);
        pastille.border_color = if etat == Etat::Avant { ACCENT } else { Color::new(62, 58, 52, 255) };
        draw_box((cx - pw / 2.0) as i32, (cy - ph / 2.0) as i32, pw.ceil() as u32, ph.ceil() as u32, &pastille, canvas);
        texte(canvas, &l.texte, cx - largeur / 2.0, cy - ph / 2.0 + 2.5 * z, TEXTE * z, true, couleur_texte);
    } else {
        // Pres de `vers`, un peu a cote du trait.
        let (dx, dy) = (pa.0 - pb.0, pa.1 - pb.1);
        let n = (dx * dx + dy * dy).sqrt().max(1.0);
        let (ux, uy) = (dx / n, dy / n);
        let (lx, ly) = (pb.0 + ux * 16.0 * z - uy * 10.0 * z, pb.1 + uy * 16.0 * z + ux * 10.0 * z);
        texte(canvas, &l.texte, lx - largeur / 2.0, ly - 7.0 * z, TEXTE * z, true, couleur_texte);
    }
}

fn boite(canvas: &mut Canvas, b: &Boite, r: (f32, f32, f32, f32), z: f32, choisie: bool, clip: (i32, i32, i32, i32)) {
    let pale = b.teinte == Teinte::Pale;
    let bordure = if b.alerte { ALERTE } else if choisie { ACCENT } else if pale { Color::new(96, 91, 82, 255) } else { Color::new(74, 69, 61, 255) };
    let epais = if choisie || b.alerte { 2.0 } else { 1.0 };
    let (fond, bandeau) = match b.genre {
        GenreBoite::Entite => (Color::new(38, 36, 32, 255), Some(Color::new(52, 47, 39, 255))),
        GenreBoite::Table => (Color::new(34, 33, 31, 255), Some(Color::new(30, 28, 26, 255))),
        GenreBoite::Association => (Color::new(44, 38, 30, 255), None),
        GenreBoite::Note => (Color::new(46, 44, 36, 255), None),
    };
    let (fond, bandeau) = match b.teinte {
        Teinte::Defaut => (fond, bandeau),
        Teinte::Sable => (fond, Some(Color::new(84, 68, 45, 255))),
        Teinte::Olive => (fond, Some(Color::new(56, 60, 42, 255))),
        Teinte::Pale => (Color::new(29, 28, 26, 255), None),
    };
    let rayon = if b.genre == GenreBoite::Association { (r.3 / 2.0).min(18.0 * z) } else { 6.0 * z };
    let mut style = BoxStyle::solid(fond);
    style.radius = rayon;
    // Pale : bord en tirets, dessine a part.
    if !pale || choisie || b.alerte {
        style.border = BorderWidths::uniform(epais);
        style.border_color = bordure;
    }
    draw_box(r.0 as i32, r.1 as i32, r.2.round() as u32, r.3.round() as u32, &style, canvas);
    if pale && !choisie && !b.alerte {
        let (x0, y0, x1, y1) = (r.0 + 0.5, r.1 + 0.5, r.0 + r.2 - 0.5, r.1 + r.3 - 0.5);
        for (p, q) in [((x0, y0), (x1, y0)), ((x1, y0), (x1, y1)), ((x1, y1), (x0, y1)), ((x0, y1), (x0, y0))] {
            trait_(canvas, p, q, 1.0, bordure, clip, true);
        }
    }
    let (tt, tl) = (TITRE * z, TEXTE * z);
    let largeur = |s: &str, taille: f32, gras: bool| mesure_par_defaut(s, taille, gras);
    match b.genre {
        GenreBoite::Association => {
            let tw = largeur(&b.titre, tt, true);
            let ty = if b.lignes.is_empty() { r.1 + (r.3 - tt) / 2.0 - 2.0 * z } else { r.1 + 7.0 * z };
            texte(canvas, &b.titre, r.0 + (r.2 - tw) / 2.0, ty, tt, true, TEXTE_C);
            for (i, l) in b.lignes.iter().enumerate() {
                let lw = largeur(&l.texte, tl, false);
                let ly = r.1 + 28.0 * z + i as f32 * HAUT_LIGNE * z;
                ligne(canvas, l.style, &l.texte, r.0 + (r.2 - lw) / 2.0, ly, tl, z, clip);
            }
        }
        _ => {
            if let Some(c) = bandeau {
                let mut s = BoxStyle::solid(c);
                s.radius = rayon;
                draw_box(r.0 as i32 + epais as i32, r.1 as i32 + epais as i32, (r.2 - 2.0 * epais).max(0.0) as u32, (HAUT_TITRE * z - epais).max(0.0) as u32, &s, canvas);
                trait_(canvas, (r.0, r.1 + HAUT_TITRE * z), (r.0 + r.2, r.1 + HAUT_TITRE * z), 1.0, bordure, clip, false);
            }
            let tw = largeur(&b.titre, tt, true);
            let couleur_titre = match b.teinte {
                Teinte::Pale => DISCRET,
                Teinte::Sable => Color::new(243, 226, 196, 255),
                _ if b.genre == GenreBoite::Table => ACCENT,
                _ => TEXTE_C,
            };
            texte(canvas, &b.titre, r.0 + (r.2 - tw) / 2.0, r.1 + 6.0 * z, tt, true, couleur_titre);
            for (i, l) in b.lignes.iter().enumerate() {
                let ly = r.1 + (HAUT_TITRE + 4.0) * z + i as f32 * HAUT_LIGNE * z;
                ligne(canvas, l.style, &l.texte, r.0 + MARGE * z, ly, tl, z, clip);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)] // style, texte, position, taille, zoom, zone
fn ligne(canvas: &mut Canvas, style: StyleLigne, s: &str, x: f32, y: f32, taille: f32, z: f32, clip: (i32, i32, i32, i32)) {
    let couleur = match style {
        StyleLigne::Discret => DISCRET,
        StyleLigne::Accent => ACCENT,
        _ => TEXTE_C,
    };
    texte(canvas, s, x, y, taille, style == StyleLigne::Souligne, couleur);
    if style == StyleLigne::Souligne {
        let w = mesure_par_defaut(s, taille, true);
        trait_(canvas, (x, y + taille + 2.0 * z), (x + w, y + taille + 2.0 * z), 1.0, couleur, clip, false);
    }
}
