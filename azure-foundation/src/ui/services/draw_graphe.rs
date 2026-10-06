// Le dessin des graphes (`ui::models::graphe`) : courbes et aires
// antialiasees, barres, anneaux, jauge, radar. Couleurs : `accent-color`
// puis la palette du theme pour les series, `color` pour les textes,
// `background-color` pour la grille et les pistes.
use crate::layout::managers::layout_manager::Rect;
use crate::ui::models::control::Control;
use crate::ui::models::graphe::{echelle, lisible, Genre, Graphe};
use crate::ui::services::draw_ui::FONT_PATH;
use azure_engine::rendering::managers::renderer::{draw_box, draw_text, measure_text_width};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::BoxStyle;
use std::f32::consts::{PI, TAU};

/// La zone ou dessiner : (gauche, haut, droite, bas), bornes exclues.
type Zone = (i32, i32, i32, i32);

const PETIT: f32 = 11.0;

/// Ce qu'il faut pour dessiner : la toile, sa zone, les couleurs du champ.
struct Plume<'a> {
    canvas: &'a mut Canvas,
    zone: Zone,
    texte: Color,
    piste: Color,
    fort: Color,
}

impl Plume<'_> {
    /// Melange `couleur` dans un pixel, avec l'opacite `a`.
    fn pixel(&mut self, x: i32, y: i32, couleur: Color, a: f32) {
        if x < self.zone.0 || y < self.zone.1 || x >= self.zone.2 || y >= self.zone.3 || a <= 0.0 {
            return;
        }
        let i = ((y as u32 * self.canvas.width + x as u32) * 4) as usize;
        let a = a.min(1.0) * couleur.a as f32 / 255.0;
        let b = &mut self.canvas.buffer[i..i + 4];
        for (k, c) in [(0, couleur.b), (1, couleur.g), (2, couleur.r)] {
            b[k] = (b[k] as f32 * (1.0 - a) + c as f32 * a).round() as u8;
        }
        b[3] = 255;
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, couleur: Color) {
        for py in y.round() as i32..(y + h).round() as i32 {
            for px in x.round() as i32..(x + w).round() as i32 {
                self.pixel(px, py, couleur, 1.0);
            }
        }
    }

    /// Une boite aux coins arrondis.
    fn boite(&mut self, x: f32, y: f32, w: f32, h: f32, rayon: f32, couleur: Color) {
        if w < 1.0 || h < 1.0 {
            return;
        }
        let mut style = BoxStyle::solid(couleur);
        style.radius = rayon.min(w / 2.0).min(h / 2.0);
        draw_box(x.round() as i32, y.round() as i32, w.round() as u32, h.round() as u32, &style, self.canvas);
    }

    /// Un trait antialiase d'epaisseur `e` : chaque pixel proche prend
    /// l'opacite de sa distance au segment.
    fn segment(&mut self, (x1, y1): (f32, f32), (x2, y2): (f32, f32), e: f32, couleur: Color) {
        let demi = e / 2.0;
        let (dx, dy) = (x2 - x1, y2 - y1);
        let long2 = (dx * dx + dy * dy).max(1e-6);
        let m = demi.ceil() as i32 + 1;
        for py in (y1.min(y2).floor() as i32 - m)..=(y1.max(y2).ceil() as i32 + m) {
            for px in (x1.min(x2).floor() as i32 - m)..=(x1.max(x2).ceil() as i32 + m) {
                let (qx, qy) = (px as f32 + 0.5 - x1, py as f32 + 0.5 - y1);
                let t = ((qx * dx + qy * dy) / long2).clamp(0.0, 1.0);
                let d = ((qx - t * dx).powi(2) + (qy - t * dy).powi(2)).sqrt();
                self.pixel(px, py, couleur, demi + 0.5 - d);
            }
        }
    }

    fn ligne(&mut self, points: &[(f32, f32)], e: f32, couleur: Color) {
        for paire in points.windows(2) {
            self.segment(paire[0], paire[1], e, couleur);
        }
    }

    fn disque(&mut self, (cx, cy): (f32, f32), r: f32, couleur: Color) {
        for py in (cy - r - 1.0) as i32..=(cy + r + 1.0) as i32 {
            for px in (cx - r - 1.0) as i32..=(cx + r + 1.0) as i32 {
                let d = ((px as f32 + 0.5 - cx).powi(2) + (py as f32 + 0.5 - cy).powi(2)).sqrt();
                self.pixel(px, py, couleur, r + 0.5 - d);
            }
        }
    }

    /// Le dessous d'une courbe, de plus en plus leger vers le bas.
    fn aire(&mut self, points: &[(f32, f32)], haut: f32, bas: f32, couleur: Color) {
        for paire in points.windows(2) {
            let ((x1, y1), (x2, y2)) = (paire[0], paire[1]);
            for px in x1.round() as i32..x2.round() as i32 {
                let t = if x2 > x1 { ((px as f32 + 0.5 - x1) / (x2 - x1)).clamp(0.0, 1.0) } else { 0.0 };
                let y = y1 + (y2 - y1) * t;
                for py in y.floor() as i32..bas.round() as i32 {
                    let bord = (py as f32 + 1.0 - y).clamp(0.0, 1.0);
                    let fond = ((py as f32 - haut) / (bas - haut).max(1.0)).clamp(0.0, 1.0);
                    self.pixel(px, py, couleur, bord * (0.34 - 0.30 * fond));
                }
            }
        }
    }

    /// L'interieur d'un polygone, a l'opacite `a`.
    fn polygone(&mut self, points: &[(f32, f32)], couleur: Color, a: f32) {
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (x, y) in points {
            (x0, y0, x1, y1) = (x0.min(*x), y0.min(*y), x1.max(*x), y1.max(*y));
        }
        for py in y0 as i32..=y1 as i32 {
            for px in x0 as i32..=x1 as i32 {
                let (qx, qy) = (px as f32 + 0.5, py as f32 + 0.5);
                let mut dedans = false;
                for i in 0..points.len() {
                    let ((ax, ay), (bx, by)) = (points[i], points[(i + 1) % points.len()]);
                    if (ay > qy) != (by > qy) && qx < ax + (qy - ay) / (by - ay) * (bx - ax) {
                        dedans = !dedans;
                    }
                }
                if dedans {
                    self.pixel(px, py, couleur, a);
                }
            }
        }
    }

    fn largeur(texte: &str, taille: f32, graisse: f32) -> f32 {
        measure_text_width(texte, FONT_PATH, taille, graisse).unwrap_or(0.0)
    }

    fn ecrire(&mut self, texte: &str, x: f32, y: f32, taille: f32, graisse: f32, couleur: Color) {
        if x >= 0.0 && y >= 0.0 && !texte.is_empty() {
            let _ = draw_text(texte, FONT_PATH, x as u32, y as u32, taille, graisse, &couleur, self.canvas);
        }
    }

    /// Texte centre sur `cx`.
    fn centrer(&mut self, texte: &str, cx: f32, y: f32, taille: f32, graisse: f32, couleur: Color) {
        self.ecrire(texte, cx - Self::largeur(texte, taille, graisse) / 2.0, y, taille, graisse, couleur);
    }

    /// Une couronne (ou un disque si `r == 0`) dont chaque pixel prend la
    /// couleur que `part` donne a son angle (0..1, depuis `depart`, sens des
    /// aiguilles) ; `tour` : la portion du cercle couverte.
    fn couronne(&mut self, (cx, cy): (f32, f32), r: f32, grand: f32, depart: f32, tour: f32, part: &dyn Fn(f32) -> (Color, f32)) {
        for py in (cy - grand - 1.0) as i32..=(cy + grand + 1.0) as i32 {
            for px in (cx - grand - 1.0) as i32..=(cx + grand + 1.0) as i32 {
                let (dx, dy) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
                let d = (dx * dx + dy * dy).sqrt();
                let bord = (grand + 0.5 - d).clamp(0.0, 1.0) * if r > 0.0 { (d - r + 0.5).clamp(0.0, 1.0) } else { 1.0 };
                if bord <= 0.0 {
                    continue;
                }
                let angle = (dy.atan2(dx) - depart).rem_euclid(TAU);
                if angle > tour {
                    continue;
                }
                // `ecart` : distance (en part de tour) au bord de part le plus
                // proche, pour laisser un filet entre deux parts.
                let (couleur, ecart) = part(angle / tour);
                let filet = (ecart * tour * d - 0.6).clamp(0.0, 1.0);
                self.pixel(px, py, couleur, bord * filet);
            }
        }
    }
}

/// Une courbe qui passe par `points` : arrondie (Catmull-Rom) si `lisse`,
/// sans sortir de `haut..bas`.
fn courbe(points: &[(f32, f32)], lisse: bool, haut: f32, bas: f32) -> Vec<(f32, f32)> {
    if !lisse || points.len() < 3 {
        return points.to_vec();
    }
    let p = |i: isize| points[i.clamp(0, points.len() as isize - 1) as usize];
    let mut out = vec![points[0]];
    for i in 0..points.len() as isize - 1 {
        let (a, b, c, d) = (p(i - 1), p(i), p(i + 1), p(i + 2));
        // Points serres : inutile d'arrondir.
        let pas = (((c.0 - b.0) / 3.0).ceil() as usize).clamp(1, 12);
        for k in 1..=pas {
            let t = k as f32 / pas as f32;
            let (t2, t3) = (t * t, t * t * t);
            let f = |a: f32, b: f32, c: f32, d: f32| 0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (3.0 * b - a - 3.0 * c + d) * t3);
            // Jamais plus loin que les deux points qu'on relie : pas de bosse inventee.
            out.push((f(a.0, b.0, c.0, d.0), f(a.1, b.1, c.1, d.1).clamp(b.1.min(c.1), b.1.max(c.1)).clamp(haut, bas)));
        }
    }
    out
}

pub fn draw_graphe(g: &Graphe, control: &Control, own: Rect, canvas: &mut Canvas) {
    let (cx, cy, cw, ch) = canvas.clip_bounds();
    let zone = (own.0.max(cx as i32), own.1.max(cy as i32), (own.0 + own.2 as i32).min((cx + cw) as i32), (own.1 + own.3 as i32).min((cy + ch) as i32));
    if zone.2 <= zone.0 || zone.3 <= zone.1 {
        return;
    }
    let mut p = Plume { canvas, zone, texte: control.text_color, piste: control.track, fort: g.fort };
    let r = (own.0 as f32, own.1 as f32, own.2 as f32, own.3 as f32);
    if g.series.iter().all(|s| s.valeurs.is_empty()) {
        return p.centrer("Pas encore de données", r.0 + r.2 / 2.0, r.1 + r.3 / 2.0 - 8.0, 12.0, 400.0, p.texte);
    }
    match g.genre {
        Genre::Ligne | Genre::Aire | Genre::Barres | Genre::Empile | Genre::Points => axes(g, &mut p, r),
        Genre::Spark => spark(g, &mut p, r),
        Genre::BarresH => classement(g, &mut p, r),
        Genre::Anneau | Genre::Secteurs => parts(g, &mut p, r),
        Genre::Jauge => jauge(g, &mut p, r),
        Genre::Radar => radar(g, &mut p, r),
    }
}

/// Les noms des series, en ligne ; rend la hauteur prise.
fn legende(g: &Graphe, p: &mut Plume, x: f32, y: f32) -> f32 {
    if !g.legende || g.series.iter().all(|s| s.nom.is_empty()) {
        return 0.0;
    }
    let mut x = x;
    for (i, s) in g.series.iter().enumerate() {
        p.disque((x + 4.0, y + 8.0), 4.0, g.couleur(i));
        p.ecrire(&s.nom, x + 13.0, y + 1.0, PETIT, 500.0, p.texte);
        x += 13.0 + Plume::largeur(&s.nom, PETIT, 500.0) + 14.0;
    }
    20.0
}

/// Courbes, aires, barres, points : une echelle a gauche, les etiquettes
/// en bas.
fn axes(g: &Graphe, p: &mut Plume, (x, y, w, h): (f32, f32, f32, f32)) {
    let n = g.points();
    let empile = g.genre == Genre::Empile;
    let sommet = if empile {
        (0..n).map(|i| g.series.iter().map(|s| s.valeurs.get(i).copied().unwrap_or(0.0).max(0.0)).sum::<f64>()).fold(0.0, f64::max)
    } else {
        g.series.iter().flat_map(|s| s.valeurs.iter().copied()).fold(f64::MIN, f64::max)
    };
    let creux = g.series.iter().flat_map(|s| s.valeurs.iter().copied()).fold(0.0, f64::min);
    let (lo, hi, pas) = echelle(g.min.unwrap_or(creux), g.max.unwrap_or(sommet).max(g.min.unwrap_or(creux)));

    let haut = y + 6.0 + legende(g, p, x, y);
    let bas = y + h - if g.etiquettes.is_empty() { 5.0 } else { 20.0 };
    let graduations: Vec<f64> = (0..=((hi - lo) / pas).round() as usize).map(|k| lo + k as f64 * pas).collect();
    let marge = if g.grille { graduations.iter().map(|v| Plume::largeur(&lisible(*v, &g.unite), PETIT, 400.0)).fold(0.0, f32::max) + 8.0 } else { 0.0 };
    let (gauche, droite) = (x + marge, x + w - 4.0);
    if droite - gauche < 8.0 || bas - haut < 8.0 {
        return;
    }
    let py = |v: f64| bas - ((v - lo) / (hi - lo)) as f32 * (bas - haut);
    if g.grille {
        for v in &graduations {
            let ligne = py(*v).round();
            p.rect(gauche, ligne, droite - gauche, 1.0, p.piste);
            let texte = lisible(*v, &g.unite);
            p.ecrire(&texte, gauche - 8.0 - Plume::largeur(&texte, PETIT, 400.0), ligne - 7.0, PETIT, 400.0, p.texte);
        }
    }

    let barres = matches!(g.genre, Genre::Barres | Genre::Empile);
    let case = (droite - gauche) / n.max(1) as f32;
    let px = |i: usize| if barres { gauche + (i as f32 + 0.5) * case } else if n > 1 { gauche + 3.0 + i as f32 * (droite - gauche - 6.0) / (n - 1) as f32 } else { (gauche + droite) / 2.0 };

    // Les etiquettes du bas : autant qu'il en tient.
    if !g.etiquettes.is_empty() {
        // Des etiquettes laissees vides : l'app a deja choisi lesquelles montrer.
        let choisies = g.etiquettes.iter().any(String::is_empty);
        let chaque = if choisies { 1 } else { (n as f32 / ((droite - gauche) / 64.0).max(1.0)).ceil().max(1.0) as usize };
        for (i, e) in g.etiquettes.iter().enumerate().take(n).filter(|(i, _)| i % chaque == 0) {
            let l = Plume::largeur(e, PETIT, 400.0);
            p.ecrire(e, (px(i) - l / 2.0).clamp(gauche, (droite - l).max(gauche)), bas + 5.0, PETIT, 400.0, p.texte);
        }
    }

    let zero = py(0f64.clamp(lo, hi));
    match g.genre {
        Genre::Barres => {
            let k = g.series.len().max(1) as f32;
            let large = (case * 0.72 / k).clamp(1.0, 26.0);
            for (j, s) in g.series.iter().enumerate() {
                for (i, v) in s.valeurs.iter().enumerate() {
                    let bx = px(i) - large * k / 2.0 + j as f32 * large;
                    let (a, b) = (py(*v).min(zero), py(*v).max(zero));
                    let epais = if case < 4.0 { large } else { (large - if k > 1.0 { 2.0 } else { 0.0 }).max(1.0) };
                    p.boite(bx, a, epais, (b - a).max(if *v != 0.0 { 1.0 } else { 0.0 }), 3.0, g.couleur(j));
                }
            }
        }
        Genre::Empile => {
            let large = (case * 0.72).clamp(1.0, 30.0);
            for i in 0..n {
                let mut pied = 0.0;
                for (j, s) in g.series.iter().enumerate() {
                    let v = s.valeurs.get(i).copied().unwrap_or(0.0).max(0.0);
                    let (a, b) = (py(pied + v), py(pied));
                    // Un filet entre deux etages.
                    p.rect(px(i) - large / 2.0, a, large, (b - a - if j > 0 { 1.0 } else { 0.0 }).max(0.0), g.couleur(j));
                    pied += v;
                }
            }
        }
        _ => {
            for (j, s) in g.series.iter().enumerate() {
                let points: Vec<(f32, f32)> = s.valeurs.iter().enumerate().map(|(i, v)| (px(i), py(*v).clamp(haut, bas))).collect();
                let couleur = g.couleur(j);
                if g.genre == Genre::Points {
                    for pt in &points {
                        p.disque(*pt, 3.5, couleur);
                    }
                    continue;
                }
                let trace = courbe(&points, g.lisse, haut, bas);
                if g.genre == Genre::Aire {
                    p.aire(&trace, haut, bas, couleur);
                }
                p.ligne(&trace, 2.0, couleur);
                // Peu de points : on les marque ; sinon seulement le dernier.
                if points.len() <= 12 {
                    for pt in &points {
                        p.disque(*pt, 3.0, couleur);
                    }
                } else if let Some(dernier) = points.last() {
                    p.disque(*dernier, 3.0, couleur);
                }
            }
        }
    }
}

/// Une petite courbe sans axes : l'allure, rien d'autre.
fn spark(g: &Graphe, p: &mut Plume, (x, y, w, h): (f32, f32, f32, f32)) {
    for (j, s) in g.series.iter().enumerate() {
        let (lo, hi) = s.valeurs.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
        let (lo, hi) = (g.min.unwrap_or(lo), g.max.unwrap_or(hi));
        let etendue = if hi > lo { hi - lo } else { 1.0 };
        let (haut, bas) = (y + 4.0, y + h - 2.0);
        let n = s.valeurs.len();
        let points: Vec<(f32, f32)> = s.valeurs.iter().enumerate().map(|(i, v)| (if n > 1 { x + 3.0 + i as f32 * (w - 7.0) / (n - 1) as f32 } else { x + w / 2.0 }, (bas - 3.0 - ((v - lo) / etendue) as f32 * (bas - haut - 6.0)).clamp(haut, bas))).collect();
        let trace = courbe(&points, g.lisse, haut, bas);
        p.aire(&trace, haut, bas, g.couleur(j));
        p.ligne(&trace, 1.6, g.couleur(j));
        if let Some(dernier) = points.last() {
            p.disque(*dernier, 2.5, g.couleur(j));
        }
    }
}

/// Barres horizontales : etiquette, barre, valeur.
fn classement(g: &Graphe, p: &mut Plume, (x, y, w, h): (f32, f32, f32, f32)) {
    let valeurs = g.premiere();
    let max = g.max.unwrap_or_else(|| valeurs.iter().copied().fold(0.0, f64::max)).max(f64::MIN_POSITIVE);
    let rang = (h / valeurs.len() as f32).min(30.0);
    let nom = |i: usize| g.etiquettes.get(i).map(String::as_str).unwrap_or("");
    let noms = (0..valeurs.len()).map(|i| Plume::largeur(nom(i), 12.0, 400.0)).fold(0.0, f32::max).min(w * 0.4);
    let chiffres = valeurs.iter().map(|v| Plume::largeur(&lisible(*v, &g.unite), 12.0, 600.0)).fold(0.0, f32::max);
    let (gauche, droite) = (x + noms + if noms > 0.0 { 12.0 } else { 0.0 }, x + w - chiffres - 10.0);
    for (i, v) in valeurs.iter().enumerate() {
        let milieu = y + (i as f32 + 0.5) * rang;
        p.ecrire(nom(i), x, milieu - 8.0, 12.0, 400.0, p.texte);
        p.boite(gauche, milieu - 4.0, droite - gauche, 8.0, 4.0, p.piste);
        p.boite(gauche, milieu - 4.0, ((v / max).clamp(0.0, 1.0) as f32 * (droite - gauche)).max(if *v > 0.0 { 8.0 } else { 0.0 }), 8.0, 4.0, g.couleur(if g.couleurs.len() > 1 && g.series.len() == 1 && g.etiquettes.is_empty() { i } else { 0 }));
        let texte = lisible(*v, &g.unite);
        p.ecrire(&texte, x + w - Plume::largeur(&texte, 12.0, 600.0), milieu - 8.0, 12.0, 600.0, p.fort);
    }
}

/// Anneau ou disque : les parts a gauche, leur legende a droite.
fn parts(g: &Graphe, p: &mut Plume, (x, y, w, h): (f32, f32, f32, f32)) {
    let valeurs: Vec<f64> = g.premiere().iter().map(|v| v.max(0.0)).collect();
    let total: f64 = valeurs.iter().sum();
    let avec_legende = !g.etiquettes.is_empty() && w > h * 1.5;
    let diametre = if avec_legende { h.min(w * 0.5) } else { h.min(w) };
    let grand = diametre / 2.0 - 2.0;
    let centre = (if avec_legende { x + diametre / 2.0 } else { x + w / 2.0 }, y + h / 2.0);
    let petit = if g.genre == Genre::Anneau { grand * 0.64 } else { 0.0 };
    if total <= 0.0 {
        let piste = p.piste;
        p.couronne(centre, petit, grand, -PI / 2.0, TAU, &|_| (piste, 1.0));
    } else {
        // Bornes cumulees des parts, en part de tour.
        let mut bornes = vec![0.0f32];
        for v in &valeurs {
            bornes.push(bornes.last().unwrap() + (*v / total) as f32);
        }
        let seule = valeurs.iter().filter(|v| **v > 0.0).count() <= 1;
        p.couronne(centre, petit, grand, -PI / 2.0, TAU, &|t| {
            let i = (0..valeurs.len()).find(|i| t < bornes[i + 1]).unwrap_or(valeurs.len() - 1);
            (g.couleur(i), if seule { 1.0 } else { (t - bornes[i]).min(bornes[i + 1] - t) })
        });
    }
    if g.genre == Genre::Anneau {
        let texte = if g.centre.is_empty() { lisible(total, &g.unite) } else { g.centre.clone() };
        let taille = (petit * 0.42).clamp(11.0, 22.0);
        p.centrer(&texte, centre.0, centre.1 - taille * 0.65, taille, 700.0, p.fort);
    }
    if avec_legende {
        let rang = 22.0f32.min(h / valeurs.len().max(1) as f32);
        let depart = y + (h - rang * valeurs.len() as f32) / 2.0;
        let gauche = x + diametre + 18.0;
        for (i, v) in valeurs.iter().enumerate() {
            let milieu = depart + (i as f32 + 0.5) * rang;
            p.disque((gauche + 4.0, milieu), 4.0, g.couleur(i));
            p.ecrire(g.etiquettes.get(i).map(String::as_str).unwrap_or(""), gauche + 15.0, milieu - 8.0, 12.0, 400.0, p.texte);
            let part = if total > 0.0 { format!("{:.0} %", v / total * 100.0) } else { String::new() };
            p.ecrire(&part, x + w - Plume::largeur(&part, 12.0, 600.0), milieu - 8.0, 12.0, 600.0, p.fort);
        }
    }
}

/// Demi-cercle rempli jusqu'a la valeur, qui s'ecrit au milieu.
fn jauge(g: &Graphe, p: &mut Plume, (x, y, w, h): (f32, f32, f32, f32)) {
    let v = g.premiere().first().copied().unwrap_or(0.0);
    let (lo, hi) = (g.min.unwrap_or(0.0), g.max.unwrap_or(100.0));
    let part = if hi > lo { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) as f32 } else { 0.0 };
    let grand = (w / 2.0 - 4.0).min(h - 22.0).max(8.0);
    let epais = (grand * 0.2).max(6.0);
    let centre = (x + w / 2.0, y + (h - 16.0 + grand) / 2.0);
    let (plein, piste) = (g.couleur(0), p.piste);
    p.couronne(centre, grand - epais, grand, PI, PI, &|t| if t <= part { (plein, 1.0) } else { (piste, 1.0) });
    let taille = (grand * 0.36).clamp(12.0, 28.0);
    p.centrer(&lisible(v, &g.unite), centre.0, centre.1 - taille * 1.25, taille, 700.0, p.fort);
    p.centrer(&lisible(lo, ""), centre.0 - grand + epais / 2.0, centre.1 + 3.0, PETIT, 400.0, p.texte);
    p.centrer(&lisible(hi, ""), centre.0 + grand - epais / 2.0, centre.1 + 3.0, PETIT, 400.0, p.texte);
}

/// Toile d'araignee : une branche par etiquette, un polygone par serie.
fn radar(g: &Graphe, p: &mut Plume, (x, y, w, h): (f32, f32, f32, f32)) {
    let n = g.points().max(g.etiquettes.len());
    if n < 3 {
        return;
    }
    let haut_legende = legende(g, p, x, y);
    let centre = (x + w / 2.0, y + haut_legende + (h - haut_legende) / 2.0);
    let grand = (w.min(h - haut_legende) / 2.0 - 22.0).max(8.0);
    let max = g.max.unwrap_or_else(|| echelle(0.0, g.series.iter().flat_map(|s| s.valeurs.iter().copied()).fold(0.0, f64::max)).1);
    let sommet = |i: usize, part: f32| {
        let angle = -PI / 2.0 + i as f32 * TAU / n as f32;
        (centre.0 + angle.cos() * grand * part, centre.1 + angle.sin() * grand * part)
    };
    for anneau in 1..=3 {
        let mut tour: Vec<(f32, f32)> = (0..n).map(|i| sommet(i, anneau as f32 / 3.0)).collect();
        tour.push(tour[0]);
        p.ligne(&tour, 1.0, p.piste);
    }
    for i in 0..n {
        p.segment(centre, sommet(i, 1.0), 1.0, p.piste);
        if let Some(e) = g.etiquettes.get(i) {
            let (ex, ey) = sommet(i, 1.0 + 14.0 / grand);
            let l = Plume::largeur(e, PETIT, 400.0);
            // A gauche du centre : le texte finit sur la branche ; a droite, il en part.
            let dx = if (ex - centre.0).abs() < 4.0 { -l / 2.0 } else if ex < centre.0 { -l } else { 0.0 };
            p.ecrire(e, ex + dx, ey - 7.0, PETIT, 400.0, p.texte);
        }
    }
    for (j, s) in g.series.iter().enumerate() {
        let mut tour: Vec<(f32, f32)> = (0..n).map(|i| sommet(i, (s.valeurs.get(i).copied().unwrap_or(0.0) / max).clamp(0.0, 1.0) as f32)).collect();
        p.polygone(&tour, g.couleur(j), 0.16);
        tour.push(tour[0]);
        p.ligne(&tour, 2.0, g.couleur(j));
        for pt in &tour[..n] {
            p.disque(*pt, 2.5, g.couleur(j));
        }
    }
}
