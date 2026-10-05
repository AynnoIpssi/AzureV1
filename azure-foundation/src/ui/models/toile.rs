// La toile (`<toile#id valeur="{{dessin}}"/>`) : une surface de dessin facon
// draw.io. L'app decrit ce qu'il y a dessus (`Dessin` : des boites avec un
// titre et des lignes, des liens entre boites avec un texte) ; la toile le
// dessine et gere la souris :
//
// - glisser une boite la deplace ; relachee, l'app recoit
//   `<id>@deplacer@<boite>@<x>@<y>` (sa nouvelle position) ;
// - un clic sur une boite : `<id>@choisir@<boite>` ; dans le vide :
//   `<id>@choisir@` ; un double-clic : `<id>@ouvrir@<boite>` ;
// - en mode relier (`mode="relier"`), glisser d'une boite a une autre :
//   `<id>@relier@<de>@<vers>` ;
// - glisser le fond deplace la vue, la molette aussi (Maj : de cote),
//   Ctrl + molette zoome ; double-clic dans le vide : tout cadrer.
//
// La vue (decalage, zoom) est gardee quand l'app redessine la page, tant
// que son attribut `vue` ne change pas ; une nouvelle `vue` (autre niveau
// d'une carte, par exemple) est cadree sur tout le dessin.
use std::time::{Duration, Instant};

// Separateurs du texte `valeur` : ni l'app ni l'utilisateur n'en tapent.
const ENREGISTREMENT: char = '\u{1e}';
const CHAMP: char = '\u{1f}';
const SOUS_CHAMP: char = '\u{1d}';

/// La forme d'une boite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenreBoite {
    /// Rectangle a bandeau de titre (entite du MCD).
    Entite,
    /// Arrondie, titre centre (association du MCD).
    Association,
    /// Rectangle a bandeau sombre (table du MLD / MPD).
    Table,
    /// Simple note.
    Note,
}

impl GenreBoite {
    fn code(self) -> &'static str {
        match self {
            GenreBoite::Entite => "entite",
            GenreBoite::Association => "association",
            GenreBoite::Table => "table",
            GenreBoite::Note => "note",
        }
    }

    fn depuis(code: &str) -> GenreBoite {
        match code {
            "association" => GenreBoite::Association,
            "table" => GenreBoite::Table,
            "note" => GenreBoite::Note,
            _ => GenreBoite::Entite,
        }
    }
}

/// L'allure d'une ligne de boite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleLigne {
    Normal,
    /// Soulignee (identifiant, cle primaire).
    Souligne,
    /// Plus pale (type, commentaire).
    Discret,
    /// En couleur d'accent (cle etrangere).
    Accent,
}

impl StyleLigne {
    fn code(self) -> char {
        match self {
            StyleLigne::Normal => 'n',
            StyleLigne::Souligne => 'u',
            StyleLigne::Discret => 'd',
            StyleLigne::Accent => 'a',
        }
    }

    fn depuis(c: char) -> StyleLigne {
        match c {
            'u' => StyleLigne::Souligne,
            'd' => StyleLigne::Discret,
            'a' => StyleLigne::Accent,
            _ => StyleLigne::Normal,
        }
    }
}

/// La teinte d'une boite (couleur du bandeau et du titre).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Teinte {
    #[default]
    Defaut,
    /// Bandeau sable.
    Sable,
    /// Bandeau olive.
    Olive,
    /// Pale, bord en tirets (ce qui est « a cote » : hors du dossier...).
    Pale,
}

impl Teinte {
    fn code(self) -> &'static str {
        match self {
            Teinte::Defaut => "",
            Teinte::Sable => "s",
            Teinte::Olive => "o",
            Teinte::Pale => "p",
        }
    }

    fn depuis(code: &str) -> Teinte {
        match code {
            "s" => Teinte::Sable,
            "o" => Teinte::Olive,
            "p" => Teinte::Pale,
            _ => Teinte::Defaut,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LigneBoite {
    pub texte: String,
    pub style: StyleLigne,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Boite {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub genre: GenreBoite,
    pub titre: String,
    pub lignes: Vec<LigneBoite>,
    /// Encadree en rouge (erreur).
    pub alerte: bool,
    pub teinte: Teinte,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lien {
    pub de: String,
    pub vers: String,
    /// Ecrit pres de l'extremite `vers` (une cardinalite).
    pub texte: String,
    /// Une fleche vers `vers` (cle etrangere).
    pub fleche: bool,
    /// Epaisseur du trait (a zoom 1) ; 0 : celle par defaut.
    pub epaisseur: f32,
    /// Fleche aux deux bouts (A <-> B).
    pub double: bool,
    /// Le texte au milieu du trait (et non pres de `vers`).
    pub milieu: bool,
    /// En rouge (un cycle, une erreur).
    pub alerte: bool,
    /// Legerement courbe (deux liens qui se croisent se distinguent).
    pub courbe: bool,
}

impl Lien {
    pub fn simple(de: &str, vers: &str, texte: &str, fleche: bool) -> Lien {
        Lien { de: de.to_string(), vers: vers.to_string(), texte: texte.to_string(), fleche, epaisseur: 0.0, double: false, milieu: false, alerte: false, courbe: false }
    }
}

/// Ce que l'app veut sur la toile ; `encoder` donne la `valeur` du rsH.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dessin {
    pub boites: Vec<Boite>,
    pub liens: Vec<Lien>,
}

impl Dessin {
    pub fn boite(&mut self, id: &str, x: f32, y: f32, genre: GenreBoite, titre: &str) -> &mut Boite {
        self.boites.push(Boite { id: id.to_string(), x, y, genre, titre: titre.to_string(), lignes: Vec::new(), alerte: false, teinte: Teinte::Defaut });
        self.boites.last_mut().expect("vient d'etre ajoutee")
    }

    pub fn lien(&mut self, de: &str, vers: &str, texte: &str, fleche: bool) {
        self.liens.push(Lien::simple(de, vers, texte, fleche));
    }

    pub fn encoder(&self) -> String {
        let propre = |s: &str| s.replace([ENREGISTREMENT, CHAMP, SOUS_CHAMP, '\n'], " ");
        let mut out = Vec::new();
        for b in &self.boites {
            let lignes: Vec<String> = b.lignes.iter().map(|l| format!("{}{}", l.style.code(), propre(&l.texte))).collect();
            out.push(["b", &propre(&b.id), &b.x.to_string(), &b.y.to_string(), b.genre.code(), &propre(&b.titre), if b.alerte { "!" } else { "" }, &lignes.join(&SOUS_CHAMP.to_string()), b.teinte.code()].join(&CHAMP.to_string()));
        }
        for l in &self.liens {
            let options: String = [(l.double, 'd'), (l.milieu, 'm'), (l.alerte, 'a'), (l.courbe, 'c')].iter().filter(|x| x.0).map(|x| x.1).collect();
            let epaisseur = if l.epaisseur > 0.0 { l.epaisseur.to_string() } else { String::new() };
            out.push(["l", &propre(&l.de), &propre(&l.vers), &propre(&l.texte), if l.fleche { ">" } else { "" }, &epaisseur, &options].join(&CHAMP.to_string()));
        }
        out.join(&ENREGISTREMENT.to_string())
    }

    pub fn decoder(texte: &str) -> Dessin {
        let mut d = Dessin::default();
        for e in texte.split(ENREGISTREMENT).filter(|e| !e.is_empty()) {
            let c: Vec<&str> = e.split(CHAMP).collect();
            let champ = |i: usize| c.get(i).copied().unwrap_or("");
            match champ(0) {
                "b" => {
                    let lignes = champ(7).split(SOUS_CHAMP).filter(|l| !l.is_empty()).map(|l| {
                        let mut ch = l.chars();
                        let style = StyleLigne::depuis(ch.next().unwrap_or('n'));
                        LigneBoite { texte: ch.collect(), style }
                    });
                    d.boites.push(Boite { id: champ(1).into(), x: champ(2).parse().unwrap_or(0.0), y: champ(3).parse().unwrap_or(0.0), genre: GenreBoite::depuis(champ(4)), titre: champ(5).into(), alerte: champ(6) == "!", lignes: lignes.collect(), teinte: Teinte::depuis(champ(8)) });
                }
                "l" => {
                    let o = champ(6);
                    d.liens.push(Lien { de: champ(1).into(), vers: champ(2).into(), texte: champ(3).into(), fleche: champ(4) == ">", epaisseur: champ(5).parse().unwrap_or(0.0), double: o.contains('d'), milieu: o.contains('m'), alerte: o.contains('a'), courbe: o.contains('c') });
                }
                _ => {}
            }
        }
        d
    }
}

/// Un geste de souris en cours sur la toile.
#[derive(Clone, Debug, PartialEq)]
pub enum Geste {
    /// Une boite tenue : ou on l'a prise (en coordonnees de la toile).
    Deplacer { id: String, prise: (f32, f32), depart: (i32, i32), bouge: bool },
    /// Le fond tenu : la vue suit.
    Vue { depart: (i32, i32), decalage: (f32, f32), bouge: bool },
    /// Mode relier : un trait part de `de` et suit la souris.
    Relier { de: String, souris: (i32, i32) },
    /// Une barre de defilement tenue : la vue suit, `echelle` pixels de
    /// decalage par pixel de barre.
    Barre { verticale: bool, depart: i32, decalage: f32, echelle: f32 },
}

/// Une barre de defilement de la toile (pixels ecran) : le rail, la poignee.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Barre {
    pub rail: (f32, f32, f32, f32),
    pub poignee: (f32, f32, f32, f32),
    /// Pixels de decalage de la vue par pixel de deplacement de la poignee.
    pub echelle: f32,
}

/// Epaisseur des barres de defilement.
pub const EPAISSEUR_BARRE: f32 = 8.0;

/// Les crans de zoom (Ctrl + molette).
pub const ZOOMS: [f32; 11] = [0.4, 0.5, 0.67, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

/// Tailles des boites, a zoom 1.
pub const TITRE: f32 = 13.0;
pub const TEXTE: f32 = 12.0;
pub const MARGE: f32 = 10.0;
pub const HAUT_TITRE: f32 = 26.0;
pub const HAUT_LIGNE: f32 = 18.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Toile {
    pub dessin: Dessin,
    /// Ou est l'origine de la toile dans sa boite (pixels).
    pub decalage: (f32, f32),
    pub zoom: f32,
    pub choisi: Option<String>,
    /// Mode relier (attribut `mode="relier"`).
    pub relier: bool,
    pub geste: Option<Geste>,
    /// Pour le double-clic : quand, ou (boite ou vide).
    dernier_appui: Option<(Instant, Option<String>)>,
    /// L'attribut `vue` : quand il change, la vue est cadree.
    pub vue: String,
    /// `focus="survol"` : la boite survolee (sinon la choisie) et ses liens
    /// ressortent, le reste palit.
    pub focus: bool,
    /// La boite sous la souris (avec `focus`).
    pub survol: Option<String>,
    /// A cadrer des que sa boite est connue (voir `interact::cadrer_toiles`).
    pub a_cadrer: bool,
}

impl Default for Toile {
    fn default() -> Toile {
        Toile { dessin: Dessin::default(), decalage: (24.0, 24.0), zoom: 1.0, choisi: None, relier: false, geste: None, dernier_appui: None, vue: String::new(), a_cadrer: false, focus: false, survol: None }
    }
}

type Mesure<'a> = &'a dyn Fn(&str, f32, bool) -> f32;

/// La largeur d'un texte (taille, gras), pour dimensionner les boites.
pub fn mesure_par_defaut(texte: &str, taille: f32, gras: bool) -> f32 {
    let font = crate::ui::services::draw_ui::FONT_PATH;
    azure_engine::rendering::managers::renderer::measure_text_width(texte, font, taille, if gras { 700.0 } else { 400.0 }).unwrap_or(texte.chars().count() as f32 * taille * 0.55)
}

impl Toile {
    /// La taille d'une boite (coordonnees de la toile, zoom 1).
    pub fn taille(b: &Boite, mesure: Mesure) -> (f32, f32) {
        let titre = mesure(&b.titre, TITRE, true);
        // Une ligne soulignee est dessinee en gras : mesuree en gras.
        let lignes = b.lignes.iter().map(|l| mesure(&l.texte, TEXTE, l.style == StyleLigne::Souligne)).fold(0.0, f32::max);
        let largeur = (titre.max(lignes) + 2.0 * MARGE).max(if b.genre == GenreBoite::Association { 90.0 } else { 110.0 });
        let largeur = if b.genre == GenreBoite::Association { largeur + 2.0 * MARGE } else { largeur };
        let hauteur = match b.genre {
            GenreBoite::Association if b.lignes.is_empty() => 34.0,
            GenreBoite::Association => 28.0 + b.lignes.len() as f32 * HAUT_LIGNE + 8.0,
            _ => HAUT_TITRE + b.lignes.len() as f32 * HAUT_LIGNE + if b.lignes.is_empty() { 4.0 } else { 8.0 },
        };
        (largeur.ceil(), hauteur.ceil())
    }

    /// Ecran -> toile.
    pub fn vers_toile(&self, own: (i32, i32, u32, u32), x: i32, y: i32) -> (f32, f32) {
        (((x - own.0) as f32 - self.decalage.0) / self.zoom, ((y - own.1) as f32 - self.decalage.1) / self.zoom)
    }

    /// Toile -> ecran.
    pub fn vers_ecran(&self, own: (i32, i32, u32, u32), x: f32, y: f32) -> (f32, f32) {
        (own.0 as f32 + self.decalage.0 + x * self.zoom, own.1 as f32 + self.decalage.1 + y * self.zoom)
    }

    /// La boite sous `(x, y)` (ecran), la plus au-dessus.
    pub fn boite_a(&self, own: (i32, i32, u32, u32), x: i32, y: i32, mesure: Mesure) -> Option<&Boite> {
        let (tx, ty) = self.vers_toile(own, x, y);
        self.dessin.boites.iter().rev().find(|b| {
            let (w, h) = Toile::taille(b, mesure);
            tx >= b.x && ty >= b.y && tx < b.x + w && ty < b.y + h
        })
    }

    /// Tout ce qu'il y a a voir (en coordonnees de la toile) : les boites et
    /// la vue actuelle, avec une marge.
    pub fn etendue(&self, own: (i32, i32, u32, u32), mesure: Mesure) -> (f32, f32, f32, f32) {
        let (vx0, vy0) = self.vers_toile(own, own.0, own.1);
        let (vx1, vy1) = self.vers_toile(own, own.0 + own.2 as i32, own.1 + own.3 as i32);
        let (mut x0, mut y0, mut x1, mut y1) = (vx0, vy0, vx1, vy1);
        for b in &self.dessin.boites {
            let (w, h) = Toile::taille(b, mesure);
            (x0, y0, x1, y1) = (x0.min(b.x - 40.0), y0.min(b.y - 40.0), x1.max(b.x + w + 40.0), y1.max(b.y + h + 40.0));
        }
        (x0, y0, x1, y1)
    }

    /// Les barres de defilement (horizontale, verticale), seulement quand
    /// le dessin depasse de la vue de ce cote.
    pub fn barres(&self, own: (i32, i32, u32, u32), mesure: Mesure) -> (Option<Barre>, Option<Barre>) {
        let (x0, y0, x1, y1) = self.etendue(own, mesure);
        let (vx0, vy0) = self.vers_toile(own, own.0, own.1);
        let (vue_w, vue_h) = (own.2 as f32 / self.zoom, own.3 as f32 / self.zoom);
        let e = EPAISSEUR_BARRE;
        let barre = |debut_rail: f32, long_rail: f32, travers: f32, total: f32, vue: f32, debut_vue: f32, origine: f32, verticale: bool| -> Option<Barre> {
            if total <= vue + 1.0 || long_rail < 40.0 {
                return None;
            }
            let longueur = (long_rail * vue / total).max(30.0);
            let position = debut_rail + (debut_vue - origine) / total * long_rail;
            let echelle = total / long_rail * self.zoom;
            Some(if verticale {
                Barre { rail: (travers, debut_rail, e, long_rail), poignee: (travers, position, e, longueur), echelle }
            } else {
                Barre { rail: (debut_rail, travers, long_rail, e), poignee: (position, travers, longueur, e), echelle }
            })
        };
        let bas = (own.1 + own.3 as i32) as f32 - e - 3.0;
        let droite = (own.0 + own.2 as i32) as f32 - e - 3.0;
        let h = barre(own.0 as f32 + 4.0, own.2 as f32 - 8.0 - e - 4.0, bas, x1 - x0, vue_w, vx0, x0, false);
        let v = barre(own.1 as f32 + 4.0, own.3 as f32 - 8.0 - e - 4.0, droite, y1 - y0, vue_h, vy0, y0, true);
        (h, v)
    }

    /// Bouton appuye. Rend ce que l'app doit savoir tout de suite
    /// (double-clic) ; sinon un geste commence.
    pub fn appuyer(&mut self, own: (i32, i32, u32, u32), x: i32, y: i32, mesure: Mesure) -> Option<String> {
        // Une barre de defilement : la poignee se prend, un clic sur le rail
        // y amene la vue.
        let dans = |r: (f32, f32, f32, f32)| (x as f32) >= r.0 - 2.0 && (y as f32) >= r.1 - 2.0 && (x as f32) < r.0 + r.2 + 2.0 && (y as f32) < r.1 + r.3 + 2.0;
        let (h, v) = self.barres(own, mesure);
        for (barre, verticale) in [(h, false), (v, true)] {
            let Some(b) = barre else { continue };
            if !dans(b.rail) {
                continue;
            }
            if !dans(b.poignee) {
                // Clic sur le rail : la poignee vient sous la souris.
                let (pos, centre) = if verticale { (y as f32, b.poignee.1 + b.poignee.3 / 2.0) } else { (x as f32, b.poignee.0 + b.poignee.2 / 2.0) };
                let d = (pos - centre) * b.echelle;
                if verticale { self.decalage.1 -= d } else { self.decalage.0 -= d }
            }
            self.geste = Some(Geste::Barre { verticale, depart: if verticale { y } else { x }, decalage: if verticale { self.decalage.1 } else { self.decalage.0 }, echelle: b.echelle });
            return None;
        }
        let sous = self.boite_a(own, x, y, mesure).map(|b| (b.id.clone(), b.x, b.y));
        let id = sous.as_ref().map(|s| s.0.clone());
        let double = self.dernier_appui.as_ref().is_some_and(|(quand, ou)| quand.elapsed() < Duration::from_millis(400) && *ou == id);
        self.dernier_appui = Some((Instant::now(), id.clone()));
        if double {
            self.dernier_appui = None;
            self.geste = None;
            return Some(match id {
                Some(id) => format!("ouvrir@{id}"),
                None => {
                    self.cadrer(own, mesure);
                    return None;
                }
            });
        }
        let (tx, ty) = self.vers_toile(own, x, y);
        self.geste = Some(match sous {
            Some((id, _, _)) if self.relier => Geste::Relier { de: id, souris: (x, y) },
            Some((id, bx, by)) => Geste::Deplacer { id, prise: (tx - bx, ty - by), depart: (x, y), bouge: false },
            None => Geste::Vue { depart: (x, y), decalage: self.decalage, bouge: false },
        });
        None
    }

    /// La souris bouge, bouton tenu. `true` : a redessiner.
    pub fn glisser(&mut self, own: (i32, i32, u32, u32), x: i32, y: i32) -> bool {
        let (tx, ty) = self.vers_toile(own, x, y);
        let loin = |d: (i32, i32)| (x - d.0).abs() + (y - d.1).abs() > 3;
        match &mut self.geste {
            Some(Geste::Deplacer { id, prise, depart, bouge }) => {
                if !*bouge && !loin(*depart) {
                    return false;
                }
                *bouge = true;
                // Aligne sur une grille de 10.
                let (nx, ny) = (((tx - prise.0) / 10.0).round() * 10.0, ((ty - prise.1) / 10.0).round() * 10.0);
                let id = id.clone();
                if let Some(b) = self.dessin.boites.iter_mut().find(|b| b.id == id) {
                    let change = b.x != nx || b.y != ny;
                    (b.x, b.y) = (nx, ny);
                    return change;
                }
                false
            }
            Some(Geste::Vue { depart, decalage, bouge }) => {
                *bouge |= loin(*depart);
                self.decalage = (decalage.0 + (x - depart.0) as f32, decalage.1 + (y - depart.1) as f32);
                true
            }
            Some(Geste::Relier { souris, .. }) => {
                *souris = (x, y);
                true
            }
            Some(Geste::Barre { verticale, depart, decalage, echelle }) => {
                let d = ((if *verticale { y } else { x }) - *depart) as f32 * *echelle;
                if *verticale { self.decalage.1 = *decalage - d } else { self.decalage.0 = *decalage - d }
                true
            }
            None => false,
        }
    }

    /// Bouton relache : ce que l'app doit faire (deplacer, choisir, relier).
    pub fn relacher(&mut self, own: (i32, i32, u32, u32), x: i32, y: i32, mesure: Mesure) -> Option<String> {
        match self.geste.take()? {
            Geste::Deplacer { id, bouge: true, .. } => {
                let b = self.dessin.boites.iter().find(|b| b.id == id)?;
                Some(format!("deplacer@{id}@{}@{}", b.x, b.y))
            }
            Geste::Deplacer { id, .. } => {
                self.choisi = Some(id.clone());
                Some(format!("choisir@{id}"))
            }
            Geste::Vue { bouge: false, .. } => {
                self.choisi = None;
                Some("choisir@".into())
            }
            Geste::Vue { .. } | Geste::Barre { .. } => None,
            Geste::Relier { de, .. } => {
                let vers = self.boite_a(own, x, y, mesure)?.id.clone();
                (vers != de).then(|| format!("relier@{de}@{vers}"))
            }
        }
    }

    /// La molette au-dessus de la toile : deplace la vue ; avec Ctrl,
    /// zoome autour du pointeur.
    pub fn molette(&mut self, own: (i32, i32, u32, u32), x: i32, y: i32, delta: f64, ctrl: bool, maj: bool) -> bool {
        if ctrl {
            let i = ZOOMS.iter().position(|z| (*z - self.zoom).abs() < 0.01).unwrap_or(5);
            let j = if delta > 0.0 { i.saturating_sub(1) } else { (i + 1).min(ZOOMS.len() - 1) };
            if j == i {
                return false;
            }
            let avant = self.vers_toile(own, x, y);
            self.zoom = ZOOMS[j];
            // Le point sous la souris reste sous la souris.
            self.decalage = ((x - own.0) as f32 - avant.0 * self.zoom, (y - own.1) as f32 - avant.1 * self.zoom);
            return true;
        }
        if maj {
            self.decalage.0 -= delta as f32;
        } else {
            self.decalage.1 -= delta as f32;
        }
        true
    }

    /// La boite mise en avant : survolee, sinon choisie (avec `focus`).
    pub fn en_avant(&self) -> Option<&str> {
        if !self.focus {
            return None;
        }
        self.survol.as_deref().or(self.choisi.as_deref())
    }

    /// La souris passe (bouton relache) : la boite survolee change ?
    pub fn survoler(&mut self, own: (i32, i32, u32, u32), x: i32, y: i32, mesure: Mesure) -> bool {
        if !self.focus {
            return false;
        }
        let dedans = x >= own.0 && y >= own.1 && x < own.0 + own.2 as i32 && y < own.1 + own.3 as i32;
        let id = if dedans && self.geste.is_none() { self.boite_a(own, x, y, mesure).map(|b| b.id.clone()) } else { None };
        if id != self.survol {
            self.survol = id;
            return true;
        }
        false
    }

    /// Tout le dessin dans la vue (zoom au cran qui convient).
    pub fn cadrer(&mut self, own: (i32, i32, u32, u32), mesure: Mesure) {
        if self.dessin.boites.is_empty() {
            self.decalage = (24.0, 24.0);
            self.zoom = 1.0;
            return;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for b in &self.dessin.boites {
            let (w, h) = Toile::taille(b, mesure);
            (x0, y0, x1, y1) = (x0.min(b.x), y0.min(b.y), x1.max(b.x + w), y1.max(b.y + h));
        }
        let (lw, lh) = ((x1 - x0).max(1.0) + 48.0, (y1 - y0).max(1.0) + 48.0);
        let ideal = (own.2 as f32 / lw).min(own.3 as f32 / lh);
        self.zoom = ZOOMS.iter().copied().filter(|z| *z <= ideal).fold(ZOOMS[0], f32::max).min(1.0);
        self.decalage = ((own.2 as f32 - (x1 - x0) * self.zoom) / 2.0 - x0 * self.zoom, (own.3 as f32 - (y1 - y0) * self.zoom) / 2.0 - y0 * self.zoom);
    }

    /// Reprend la vue et le geste d'une toile de l'ecran d'avant (la meme
    /// `vue` ; sinon elle reste a cadrer).
    pub fn reprendre(&mut self, avant: &Toile) {
        self.dernier_appui = avant.dernier_appui.clone();
        self.survol = avant.survol.clone();
        if avant.vue != self.vue {
            return;
        }
        self.a_cadrer = avant.a_cadrer;
        self.decalage = avant.decalage;
        self.zoom = avant.zoom;
        self.dernier_appui = avant.dernier_appui.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesure(t: &str, taille: f32, _: bool) -> f32 {
        t.chars().count() as f32 * taille * 0.5
    }

    fn toile() -> Toile {
        let mut d = Dessin::default();
        d.boite("CLIENT", 0.0, 0.0, GenreBoite::Entite, "CLIENT").lignes.push(LigneBoite { texte: "id_client".into(), style: StyleLigne::Souligne });
        d.boite("passer", 300.0, 0.0, GenreBoite::Association, "passer");
        d.lien("passer", "CLIENT", "0,n", false);
        Toile { dessin: Dessin::decoder(&d.encoder()), ..Toile::default() }
    }

    const OWN: (i32, i32, u32, u32) = (100, 50, 800, 600);

    #[test]
    fn encoder_decoder() {
        let t = toile();
        assert_eq!(t.dessin.boites[0].lignes[0], LigneBoite { texte: "id_client".into(), style: StyleLigne::Souligne });
        assert_eq!(t.dessin.liens[0].texte, "0,n");
        assert_eq!(t.dessin.boites[1].genre, GenreBoite::Association);
    }

    #[test]
    fn deplacer_choisir_relier() {
        let mut t = toile();
        // CLIENT a l'ecran : (100+24, 50+24).
        let (x, y) = (124 + 20, 74 + 10);
        assert_eq!(t.appuyer(OWN, x, y, &mesure), None);
        assert!(t.glisser(OWN, x + 47, y + 21));
        assert_eq!(t.relacher(OWN, x + 47, y + 21, &mesure), Some("deplacer@CLIENT@50@20".into()), "aligne sur la grille");
        // Un clic sans bouger (pas juste apres : ce serait un double-clic) : choisir.
        std::thread::sleep(Duration::from_millis(450));
        t.appuyer(OWN, x + 50, y + 20, &mesure);
        assert_eq!(t.relacher(OWN, x + 50, y + 20, &mesure), Some("choisir@CLIENT".into()));
        // Mode relier : de CLIENT a passer.
        t.relier = true;
        std::thread::sleep(Duration::from_millis(450));
        t.appuyer(OWN, x + 50, y + 20, &mesure);
        let (px, py) = (124 + 300 + 10, 74 + 10);
        t.glisser(OWN, px, py);
        assert_eq!(t.relacher(OWN, px, py, &mesure), Some("relier@CLIENT@passer".into()));
    }

    #[test]
    fn vue_zoom_double_clic() {
        let mut t = toile();
        // Fond tenu : la vue suit.
        t.appuyer(OWN, 700, 500, &mesure);
        t.glisser(OWN, 720, 530);
        assert_eq!(t.relacher(OWN, 720, 530, &mesure), None);
        assert_eq!(t.decalage, (44.0, 54.0));
        // Ctrl + molette : le point sous la souris ne bouge pas.
        let avant = t.vers_toile(OWN, 400, 300);
        assert!(t.molette(OWN, 400, 300, -1.0, true, false));
        assert_eq!(t.zoom, 1.1);
        let apres = t.vers_toile(OWN, 400, 300);
        assert!((avant.0 - apres.0).abs() < 0.01 && (avant.1 - apres.1).abs() < 0.01);
        // Double-clic sur une boite : l'ouvrir.
        std::thread::sleep(Duration::from_millis(450));
        let (bx, by) = t.vers_ecran(OWN, 10.0, 10.0);
        t.appuyer(OWN, bx as i32, by as i32, &mesure);
        t.relacher(OWN, bx as i32, by as i32, &mesure);
        assert_eq!(t.appuyer(OWN, bx as i32, by as i32, &mesure), Some("ouvrir@CLIENT".into()));
    }
}
