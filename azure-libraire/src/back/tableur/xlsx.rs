// Un classeur Excel (.xlsx), lu a la main : une archive zip de fichiers
// XML. `xl/workbook.xml` nomme les feuilles, `xl/sharedStrings.xml` range
// les textes, `xl/styles.xml` dit quelles cellules sont des dates (Excel
// les garde en nombre de jours), `xl/worksheets/…` porte les cellules.
// Les formules ne sont pas recalculees : on lit la valeur qu'Excel a
// enregistree avec.
use super::Lu;
use crate::back::compression::zip::Zip;
use std::collections::HashMap;

/// Colonnes et lignes au plus dans une feuille (garde-fou).
const MAX_COLONNES: usize = 2000;
const MAX_LIGNES: usize = 1_048_576;

pub fn lire(octets: &[u8], feuille: usize) -> Result<Lu, String> {
    if !octets.starts_with(b"PK") {
        return Err("ce n'est pas un classeur .xlsx (un vieux .xls ? l'enregistrer en .xlsx)".to_string());
    }
    let zip = Zip::ouvrir(octets).map_err(|e| format!("{e} (ce n'est pas un .xlsx lisible)"))?;
    let classeur = zip.texte("xl/workbook.xml")?.ok_or("ce n'est pas un classeur Excel (xl/workbook.xml absent)")?;
    let liens = zip.texte("xl/_rels/workbook.xml.rels")?.unwrap_or_default();
    let mut cibles: HashMap<String, String> = HashMap::new();
    for b in Balises::de(&liens) {
        if let Balise::Ouvre("Relationship", attrs, _) = b
            && let (Some(id), Some(cible)) = (attribut(attrs, "Id"), attribut(attrs, "Target"))
        {
            cibles.insert(id, cible);
        }
    }
    let mut feuilles: Vec<(String, String)> = Vec::new();
    let mut depuis_1904 = false;
    for b in Balises::de(&classeur) {
        match b {
            Balise::Ouvre("sheet", attrs, _) => {
                let nom = attribut(attrs, "name").unwrap_or_default();
                let cible = attribut(attrs, "r:id").and_then(|id| cibles.get(&id).cloned()).unwrap_or_else(|| format!("worksheets/sheet{}.xml", feuilles.len() + 1));
                let chemin = match cible.strip_prefix('/') {
                    Some(absolu) => absolu.to_string(),
                    None => format!("xl/{cible}"),
                };
                feuilles.push((nom, chemin));
            }
            Balise::Ouvre("workbookPr", attrs, _) => depuis_1904 = matches!(attribut(attrs, "date1904").as_deref(), Some("1" | "true")),
            _ => {}
        }
    }
    let (_, chemin) = feuilles.get(feuille).ok_or("ce classeur n'a pas de feuille")?;
    let textes = zip.texte("xl/sharedStrings.xml")?.map(|x| textes_partages(&x)).unwrap_or_default();
    let dates = zip.texte("xl/styles.xml")?.map(|x| styles_de_date(&x)).unwrap_or_default();
    let xml = zip.texte(chemin)?.ok_or_else(|| format!("feuille introuvable dans le classeur ({chemin})"))?;
    Ok(Lu { lignes: cellules(&xml, &textes, &dates, depuis_1904)?, feuilles: feuilles.into_iter().map(|f| f.0).collect() })
}

// ---- Le XML, juste ce qu'il faut ----

#[derive(Debug, PartialEq)]
enum Balise<'a> {
    /// (nom sans prefixe, attributs bruts, se ferme elle-meme).
    Ouvre(&'a str, &'a str, bool),
    Ferme(&'a str),
    Texte(&'a str),
}

struct Balises<'a> {
    xml: &'a str,
}

impl<'a> Balises<'a> {
    fn de(xml: &'a str) -> Balises<'a> {
        Balises { xml }
    }
}

fn sans_prefixe(nom: &str) -> &str {
    nom.rsplit(':').next().unwrap_or(nom)
}

impl<'a> Iterator for Balises<'a> {
    type Item = Balise<'a>;

    fn next(&mut self) -> Option<Balise<'a>> {
        loop {
            if self.xml.is_empty() {
                return None;
            }
            let Some(reste) = self.xml.strip_prefix('<') else {
                let fin = self.xml.find('<').unwrap_or(self.xml.len());
                let (texte, suite) = self.xml.split_at(fin);
                self.xml = suite;
                return Some(Balise::Texte(texte));
            };
            // Declarations, instructions et commentaires : sautes.
            if let Some(apres) = reste.strip_prefix("!--") {
                self.xml = apres.find("-->").map_or("", |p| &apres[p + 3..]);
                continue;
            }
            let fin = reste.find('>')?;
            let (dedans, suite) = (&reste[..fin], &reste[fin + 1..]);
            self.xml = suite;
            if dedans.starts_with('?') || dedans.starts_with('!') {
                continue;
            }
            if let Some(nom) = dedans.strip_prefix('/') {
                return Some(Balise::Ferme(sans_prefixe(nom.trim())));
            }
            let (dedans, seule) = match dedans.strip_suffix('/') {
                Some(d) => (d, true),
                None => (dedans, false),
            };
            let coupe = dedans.find(|c: char| c.is_whitespace()).unwrap_or(dedans.len());
            return Some(Balise::Ouvre(sans_prefixe(&dedans[..coupe]), &dedans[coupe..], seule));
        }
    }
}

/// L'attribut `nom` (ou `prefixe:nom` si `nom` n'a pas de prefixe).
fn attribut(attrs: &str, nom: &str) -> Option<String> {
    let mut reste = attrs;
    loop {
        reste = reste.trim_start();
        let egal = reste.find('=')?;
        let cle = reste[..egal].trim();
        let apres = reste[egal + 1..].trim_start();
        let guillemet = apres.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let fin = apres[1..].find(guillemet)?;
        let valeur = &apres[1..1 + fin];
        if cle == nom || (!nom.contains(':') && sans_prefixe(cle) == nom && !cle.starts_with("xmlns")) {
            return Some(entites(valeur));
        }
        reste = &apres[fin + 2..];
    }
}

/// `&amp;` `&#233;` `&#xE9;` -> le caractere.
fn entites(texte: &str) -> String {
    if !texte.contains('&') {
        return texte.to_string();
    }
    let mut sortie = String::with_capacity(texte.len());
    let mut reste = texte;
    while let Some(p) = reste.find('&') {
        sortie.push_str(&reste[..p]);
        reste = &reste[p..];
        let Some(fin) = reste.find(';').filter(|f| *f <= 10) else {
            sortie.push('&');
            reste = &reste[1..];
            continue;
        };
        let nom = &reste[1..fin];
        let c = match nom {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => nom.strip_prefix("#x").or_else(|| nom.strip_prefix("#X")).map(|h| u32::from_str_radix(h, 16)).or_else(|| nom.strip_prefix('#').map(|d| d.parse())).and_then(|n| n.ok()).and_then(char::from_u32),
        };
        match c {
            Some(c) => {
                sortie.push(c);
                reste = &reste[fin + 1..];
            }
            None => {
                sortie.push('&');
                reste = &reste[1..];
            }
        }
    }
    sortie.push_str(reste);
    sortie
}

/// Excel ecrit `_x000D_` pour un caractere de controle dans un texte.
fn controles(texte: String) -> String {
    if !texte.contains("_x") {
        return texte;
    }
    let mut sortie = String::with_capacity(texte.len());
    let mut reste = texte.as_str();
    while let Some(p) = reste.find("_x") {
        sortie.push_str(&reste[..p]);
        let code = reste.get(p + 2..p + 6).filter(|_| reste[p..].as_bytes().get(6) == Some(&b'_')).and_then(|h| u32::from_str_radix(h, 16).ok()).and_then(char::from_u32);
        match code {
            Some(c) => {
                sortie.push(c);
                reste = &reste[p + 7..];
            }
            None => {
                sortie.push_str("_x");
                reste = &reste[p + 2..];
            }
        }
    }
    sortie.push_str(reste);
    sortie
}

// ---- Le classeur ----

/// Les textes de `sharedStrings.xml`, dans l'ordre : un par `<si>`, fait
/// de tous ses `<t>` (un texte mis en forme en a plusieurs), sans la
/// lecture phonetique (`<rPh>`).
fn textes_partages(xml: &str) -> Vec<String> {
    let mut textes = Vec::new();
    let mut courant = String::new();
    let (mut dans_t, mut phonetique) = (false, false);
    for b in Balises::de(xml) {
        match b {
            Balise::Ouvre("si", _, seule) => {
                courant.clear();
                if seule {
                    textes.push(String::new());
                }
            }
            Balise::Ferme("si") => textes.push(controles(std::mem::take(&mut courant))),
            Balise::Ouvre("rPh", _, false) => phonetique = true,
            Balise::Ferme("rPh") => phonetique = false,
            Balise::Ouvre("t", _, false) => dans_t = true,
            Balise::Ferme("t") => dans_t = false,
            Balise::Texte(t) if dans_t && !phonetique => courant.push_str(&entites(t)),
            _ => {}
        }
    }
    textes
}

/// Pour chaque style de cellule (`cellXfs`), est-ce un format de date ?
fn styles_de_date(xml: &str) -> Vec<bool> {
    let mut formats: HashMap<u32, bool> = HashMap::new();
    let mut styles = Vec::new();
    let mut dans_cellxfs = false;
    for b in Balises::de(xml) {
        match b {
            Balise::Ouvre("numFmt", attrs, _) => {
                if let (Some(id), Some(code)) = (attribut(attrs, "numFmtId").and_then(|i| i.parse().ok()), attribut(attrs, "formatCode")) {
                    formats.insert(id, code_de_date(&code));
                }
            }
            Balise::Ouvre("cellXfs", _, false) => dans_cellxfs = true,
            Balise::Ferme("cellXfs") => dans_cellxfs = false,
            Balise::Ouvre("xf", attrs, _) if dans_cellxfs => {
                let id: u32 = attribut(attrs, "numFmtId").and_then(|i| i.parse().ok()).unwrap_or(0);
                // Les formats de date integres a Excel, puis ceux du classeur.
                let integre = matches!(id, 14..=22 | 27..=36 | 45..=47 | 50..=58);
                styles.push(formats.get(&id).copied().unwrap_or(integre));
            }
            _ => {}
        }
    }
    styles
}

/// Un code de format (`dd/mm/yyyy`, `h:mm`) parle-t-il de date ou d'heure ?
/// Les textes entre guillemets et les `[…]` (couleur, monnaie) ne comptent pas.
fn code_de_date(code: &str) -> bool {
    let (mut guillemets, mut crochets, mut echappe) = (false, false, false);
    for c in code.chars() {
        if echappe {
            echappe = false;
            continue;
        }
        match c {
            '\\' => echappe = true,
            '"' => guillemets = !guillemets,
            '[' if !guillemets => crochets = true,
            ']' if !guillemets => crochets = false,
            'd' | 'm' | 'y' | 'h' | 's' | 'D' | 'M' | 'Y' | 'H' | 'S' if !guillemets && !crochets => return true,
            _ => {}
        }
    }
    false
}

/// « BC12 » -> la colonne 54 (0 = A).
fn colonne_de(reference: &str) -> Option<usize> {
    let mut n = 0usize;
    let mut vu = false;
    for c in reference.chars().take_while(|c| c.is_ascii_alphabetic()) {
        n = n.checked_mul(26)?.checked_add((c.to_ascii_uppercase() as u8 - b'A') as usize + 1)?;
        vu = true;
    }
    vu.then(|| n - 1)
}

fn cellules(xml: &str, textes: &[String], dates: &[bool], depuis_1904: bool) -> Result<Vec<Vec<String>>, String> {
    let mut lignes: Vec<Vec<String>> = Vec::new();
    // La cellule en cours : colonne, type (`t`), style (`s`), valeur.
    let (mut colonne, mut genre, mut style) = (0usize, String::new(), None::<usize>);
    let mut valeur = String::new();
    let (mut dans_v, mut dans_t, mut dans_cellule) = (false, false, false);
    let mut suivante = 0usize;
    for b in Balises::de(xml) {
        match b {
            Balise::Ouvre("row", attrs, seule) => {
                // `r` : le numero de la ligne (les lignes vides sont absentes).
                let rang = attribut(attrs, "r").and_then(|r| r.parse::<usize>().ok()).map_or(lignes.len(), |r| r.saturating_sub(1)).max(lignes.len());
                if rang >= MAX_LIGNES {
                    return Err("feuille trop longue".to_string());
                }
                // Une ligne vide tout en bas (mise en forme seule) ne compte pas.
                if !seule {
                    lignes.resize(rang + 1, Vec::new());
                }
                suivante = 0;
            }
            Balise::Ouvre("c", attrs, seule) => {
                colonne = attribut(attrs, "r").and_then(|r| colonne_de(&r)).unwrap_or(suivante);
                suivante = colonne + 1;
                genre = attribut(attrs, "t").unwrap_or_default();
                style = attribut(attrs, "s").and_then(|s| s.parse().ok());
                valeur.clear();
                dans_cellule = !seule;
            }
            Balise::Ouvre("v", _, false) if dans_cellule => dans_v = true,
            Balise::Ferme("v") => dans_v = false,
            // Texte ecrit dans la cellule meme (`t="inlineStr"`).
            Balise::Ouvre("t", _, false) if dans_cellule => dans_t = true,
            Balise::Ferme("t") => dans_t = false,
            Balise::Texte(t) if dans_v || dans_t => valeur.push_str(&entites(t)),
            Balise::Ferme("c") if dans_cellule => {
                dans_cellule = false;
                if colonne >= MAX_COLONNES {
                    return Err("feuille trop large".to_string());
                }
                let texte = match genre.as_str() {
                    "s" => valeur.trim().parse::<usize>().ok().and_then(|i| textes.get(i)).cloned().unwrap_or_default(),
                    "b" => if valeur.trim() == "1" { "true" } else { "false" }.to_string(),
                    "str" | "inlineStr" => controles(std::mem::take(&mut valeur)),
                    // Une formule en erreur (#DIV/0!) : pas de valeur.
                    "e" => String::new(),
                    // Date ISO ecrite telle quelle.
                    "d" => valeur.trim().replace('T', " ").trim_end_matches('Z').to_string(),
                    _ => {
                        let date = style.and_then(|s| dates.get(s)).copied().unwrap_or(false);
                        nombre(valeur.trim(), date, depuis_1904)
                    }
                };
                if texte.is_empty() {
                    continue;
                }
                let Some(ligne) = lignes.last_mut() else { continue };
                if ligne.len() <= colonne {
                    ligne.resize(colonne + 1, String::new());
                }
                ligne[colonne] = texte;
            }
            _ => {}
        }
    }
    Ok(lignes)
}

/// Un nombre d'Excel en texte : entier sans « .0 », sans les miettes du
/// binaire (0.30000000000000004 -> 0.3) ; une date en `AAAA-MM-JJ`.
fn nombre(brut: &str, date: bool, depuis_1904: bool) -> String {
    let Ok(f) = brut.parse::<f64>() else { return brut.to_string() };
    if !f.is_finite() {
        return brut.to_string();
    }
    if date {
        return date_de(f, depuis_1904);
    }
    if f.fract() == 0.0 && f.abs() < 1e15 {
        return (f as i64).to_string();
    }
    // 15 chiffres significatifs, comme Excel les montre.
    format!("{f:.14e}").parse::<f64>().map_or_else(|_| brut.to_string(), |g| g.to_string())
}

/// Le numero de jour d'Excel (1 = 1er janvier 1900, avec son 29 fevrier
/// 1900 qui n'a jamais existe) -> `AAAA-MM-JJ`, suivi de l'heure s'il y en
/// a une ; moins d'un jour : l'heure seule.
fn date_de(serie: f64, depuis_1904: bool) -> String {
    let secondes_totales = (serie * 86_400.0).round() as i64;
    let (jours, secondes) = (secondes_totales.div_euclid(86_400), secondes_totales.rem_euclid(86_400));
    let heure = format!("{:02}:{:02}:{:02}", secondes / 3600, secondes / 60 % 60, secondes % 60);
    if jours == 0 && !depuis_1904 {
        return heure;
    }
    // Jours depuis le 1er janvier 1970.
    let depuis_1970 = if depuis_1904 { jours - 24_107 } else { jours - 25_569 + i64::from(jours < 61) };
    let (a, m, j) = crate::back::temps::civil(depuis_1970);
    if secondes == 0 { format!("{a:04}-{m:02}-{j:02}") } else { format!("{a:04}-{m:02}-{j:02} {heure}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_xml_se_parcourt() {
        let xml = "<?xml version=\"1.0\"?><!-- c --><a x=\"1\" r:id='rId2'><x:b/>un &amp; deux</a>";
        let balises: Vec<Balise> = Balises::de(xml).collect();
        assert_eq!(balises, [Balise::Ouvre("a", " x=\"1\" r:id='rId2'", false), Balise::Ouvre("b", "", true), Balise::Texte("un &amp; deux"), Balise::Ferme("a")]);
        assert_eq!(attribut(" x=\"1\" r:id='rId2'", "r:id").as_deref(), Some("rId2"));
        assert_eq!(attribut(" x=\"1\" r:id='rId2'", "id").as_deref(), Some("rId2"));
        assert_eq!(attribut(" name=\"A &amp; B\"", "name").as_deref(), Some("A & B"));
        assert_eq!(attribut(" x=\"1\"", "y"), None);
        assert_eq!(entites("&lt;b&gt; &#233;&#xE9; &quot;x&apos; & seul &inconnu;"), "<b> éé \"x' & seul &inconnu;");
        assert_eq!(controles("a_x000D_b_x_c".to_string()), "a\rb_x_c");
    }

    #[test]
    fn les_formats_de_date_se_reconnaissent() {
        assert!(code_de_date("dd/mm/yyyy"));
        assert!(code_de_date("[$-40C]d\\ mmmm\\ yyyy"));
        assert!(code_de_date("h:mm:ss"));
        assert!(!code_de_date("General"));
        assert!(!code_de_date("#,##0.00\\ \"dh\""));
        assert!(!code_de_date("#,##0.00\\ [$€-40C]"));
        assert!(!code_de_date("0.00E+00"));
    }

    #[test]
    fn les_nombres_et_les_dates_d_excel() {
        assert_eq!(nombre("12", false, false), "12");
        assert_eq!(nombre("12.0", false, false), "12");
        assert_eq!(nombre("0.30000000000000004", false, false), "0.3");
        assert_eq!(nombre("1.5E-2", false, false), "0.015");
        assert_eq!(nombre("-3.25", false, false), "-3.25");
        // 45000 = 15 mars 2023 ; 1 = 1er janvier 1900 ; 61 = 1er mars 1900.
        assert_eq!(nombre("45000", true, false), "2023-03-15");
        assert_eq!(nombre("1", true, false), "1900-01-01");
        assert_eq!(nombre("61", true, false), "1900-03-01");
        assert_eq!(nombre("45000.75", true, false), "2023-03-15 18:00:00");
        assert_eq!(nombre("0.5", true, false), "12:00:00");
        assert_eq!(nombre("0", true, true), "1904-01-01");
    }

    #[test]
    fn les_references_de_cellule() {
        assert_eq!(colonne_de("A1"), Some(0));
        assert_eq!(colonne_de("Z9"), Some(25));
        assert_eq!(colonne_de("AA10"), Some(26));
        assert_eq!(colonne_de("BC12"), Some(54));
        assert_eq!(colonne_de("12"), None);
    }

    #[test]
    fn une_feuille_se_lit_avec_ses_trous() {
        let textes = vec!["nom".to_string(), "Dupont".to_string()];
        let dates = vec![false, true];
        let xml = "<worksheet><sheetData>\
            <row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"C1\" t=\"inlineStr\"><is><t>né le</t></is></c></row>\
            <row r=\"3\"><c r=\"A3\" t=\"s\"><v>1</v></c><c r=\"B3\"><v>42</v></c><c r=\"C3\" s=\"1\"><v>45000</v></c><c r=\"D3\" t=\"b\"><v>1</v></c><c r=\"E3\" t=\"e\"><v>#DIV/0!</v></c><c r=\"F3\" t=\"str\"><f>A3&amp;\"!\"</f><v>Dupont!</v></c></row>\
            <row r=\"9\"/></sheetData></worksheet>";
        let lignes = cellules(xml, &textes, &dates, false).unwrap();
        assert_eq!(lignes, [vec!["nom", "", "né le"], vec![], vec!["Dupont", "42", "2023-03-15", "true", "", "Dupont!"]]);
    }

    #[test]
    fn ce_qui_n_est_pas_un_classeur_est_refuse() {
        assert!(lire(b"nom;prix\n", 0).unwrap_err().contains("pas un classeur"));
        assert!(lire(b"PK\x03\x04 pas fini", 0).unwrap_err().contains("abîmée"));
    }
}
