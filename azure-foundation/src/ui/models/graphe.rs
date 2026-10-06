// Un graphe (`<graphe type="aire" valeurs="1, 4, 2">`) : ce qu'il montre.
// Dessine par `ui::services::draw_graphe` ; les modules `chart-*`
// d'azure-libraire l'habillent (titre, valeur, carte).
//
// Donnees :
// - `valeurs="1, 4, 2"` : une serie ;
// - `series="CPU: 1 4 2; RAM: 3 3 5"` : plusieurs, nommees ;
// - `etiquettes="Lun, Mar, Mer"` : ce que vaut chaque point (axe du bas,
//   parts d'un anneau, branches d'un radar).
use azure_engine::rendering::models::color::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genre {
    /// Courbe.
    Ligne,
    /// Courbe remplie dessous.
    Aire,
    /// Barres verticales (cote a cote s'il y a plusieurs series).
    Barres,
    /// Barres horizontales legendees (un classement).
    BarresH,
    /// Barres verticales empilees.
    Empile,
    /// Nuage de points.
    Points,
    /// Parts en anneau, total au centre.
    Anneau,
    /// Parts en disque.
    Secteurs,
    /// Demi-cercle : une valeur entre `min` et `max`.
    Jauge,
    /// Toile d'araignee : une branche par etiquette.
    Radar,
    /// Petite courbe sans axes, a glisser dans une carte.
    Spark,
}

impl Genre {
    pub fn lire(texte: &str) -> Genre {
        match texte.trim() {
            "aire" | "area" => Genre::Aire,
            "barres" | "bars" => Genre::Barres,
            "barres-h" | "hbars" | "classement" => Genre::BarresH,
            "empile" | "stack" => Genre::Empile,
            "points" | "nuage" | "scatter" => Genre::Points,
            "anneau" | "donut" => Genre::Anneau,
            "secteurs" | "camembert" | "pie" => Genre::Secteurs,
            "jauge" | "gauge" => Genre::Jauge,
            "radar" => Genre::Radar,
            "spark" | "mini" => Genre::Spark,
            _ => Genre::Ligne,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Serie {
    pub nom: String,
    pub valeurs: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Graphe {
    pub genre: Genre,
    pub series: Vec<Serie>,
    pub etiquettes: Vec<String>,
    /// Bornes de l'echelle ; sans elles, celles des donnees (a partir de 0).
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// Ajoutee aux valeurs affichees (` W`, ` %`).
    pub unite: String,
    /// Noms des series au-dessus du graphe.
    pub legende: bool,
    /// Lignes et valeurs de l'echelle.
    pub grille: bool,
    /// Courbe arrondie entre les points.
    pub lisse: bool,
    /// Texte au centre d'un anneau (sinon le total).
    pub centre: String,
    /// Une couleur par serie (ou par part) ; au-dela, on recommence.
    pub couleurs: Vec<Color>,
    /// Couleur des chiffres mis en avant (centre, valeurs).
    pub fort: Color,
}

impl Graphe {
    pub fn new(genre: Genre) -> Graphe {
        Graphe { genre, series: Vec::new(), etiquettes: Vec::new(), min: None, max: None, unite: String::new(), legende: false, grille: true, lisse: true, centre: String::new(), couleurs: Vec::new(), fort: Color::new(243, 241, 237, 255) }
    }

    /// `"CPU: 1 4 2; RAM: 3 3 5"`, ou `"1, 4, 2"` pour une serie sans nom.
    pub fn lire_series(texte: &str) -> Vec<Serie> {
        texte
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                let (nom, valeurs) = match s.split_once(':') {
                    Some((nom, valeurs)) => (nom.trim(), valeurs),
                    None => ("", s),
                };
                Serie { nom: nom.to_string(), valeurs: nombres(valeurs) }
            })
            .collect()
    }

    /// La couleur de la serie (ou de la part) `i`.
    pub fn couleur(&self, i: usize) -> Color {
        if self.couleurs.is_empty() { Color::new(201, 168, 120, 255) } else { self.couleurs[i % self.couleurs.len()] }
    }

    /// Le plus grand nombre de points d'une serie.
    pub fn points(&self) -> usize {
        self.series.iter().map(|s| s.valeurs.len()).max().unwrap_or(0)
    }

    /// Les valeurs de la premiere serie (anneau, jauge, classement).
    pub fn premiere(&self) -> &[f64] {
        self.series.first().map(|s| s.valeurs.as_slice()).unwrap_or(&[])
    }
}

/// Les nombres d'une liste separee par des virgules ou des espaces ; ce
/// qui n'est pas un nombre compte pour 0 (le rang des points est garde).
pub fn nombres(texte: &str) -> Vec<f64> {
    texte.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).map(|s| s.parse::<f64>().ok().filter(|v| v.is_finite()).unwrap_or(0.0)).collect()
}

/// Une valeur lisible : `1250`, `12.5`, `0.35`, suivie de l'unite.
pub fn lisible(v: f64, unite: &str) -> String {
    let a = v.abs();
    let mut s = if a >= 100.0 { format!("{v:.0}") } else if a >= 10.0 { format!("{v:.1}") } else { format!("{v:.2}") };
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if s == "-0" {
        s = "0".to_string();
    }
    format!("{s}{unite}")
}

/// Une echelle ronde qui contient `min..max` : (bas, haut, pas).
pub fn echelle(min: f64, max: f64) -> (f64, f64, f64) {
    let (min, max) = if max > min { (min, max) } else { (min, min + 1.0) };
    let brut = (max - min) / 4.0;
    let ordre = 10f64.powf(brut.log10().floor());
    let pas = [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|k| k * ordre).find(|p| *p >= brut).unwrap_or(10.0 * ordre);
    ((min / pas).floor() * pas, (max / pas).ceil() * pas, pas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_et_nombres() {
        assert_eq!(nombres("1, 2.5,x  4"), [1.0, 2.5, 0.0, 4.0]);
        let s = Graphe::lire_series("CPU: 1 4 2; RAM: 3, 3, 5;");
        assert_eq!(s, [Serie { nom: "CPU".into(), valeurs: vec![1.0, 4.0, 2.0] }, Serie { nom: "RAM".into(), valeurs: vec![3.0, 3.0, 5.0] }]);
        assert_eq!(Graphe::lire_series("1, 2")[0], Serie { nom: String::new(), valeurs: vec![1.0, 2.0] });
        assert_eq!(Genre::lire("donut"), Genre::Anneau);
        assert_eq!(Genre::lire("?"), Genre::Ligne);
    }

    #[test]
    fn valeurs_lisibles_et_echelle_ronde() {
        assert_eq!((lisible(1250.4, " W"), lisible(12.50, ""), lisible(0.356, " %"), lisible(3.0, "")), ("1250 W".into(), "12.5".into(), "0.36 %".into(), "3".into()));
        assert_eq!(echelle(0.0, 87.0), (0.0, 100.0, 25.0));
        assert_eq!(echelle(0.0, 7.2), (0.0, 8.0, 2.0));
        assert_eq!(echelle(0.0, 0.0), (0.0, 1.0, 0.25));
        assert_eq!(echelle(-3.0, 12.0), (-5.0, 15.0, 5.0));
    }
}
