// Le contenu de la documentation : un dossier par section, un fichier par
// page, dans `contenu/`. Les numeros en tete des noms fixent l'ordre et
// ne font pas partie des identifiants :
//
//   contenu/03-rsc/_section        titre: + resume: de la section « rsc »
//   contenu/03-rsc/04-flex.page    la page « flex » de la section « rsc »
//
// Format d'une page (volontairement petit) :
//
//   titre: Flexbox
//   resume: Aligner des elements sur une ligne ou une colonne.
//
//   ## Titre de partie          ### sous-partie
//   Un paragraphe (les lignes qui se suivent sont reunies).
//   - un element de liste
//   > note: texte               (aussi astuce: et attention:)
//   > demo routeur: texte       une demonstration qui ouvre de vraies
//                               fenetres (voir `demos::DEMOS`)
//   ```apercu "Legende"          rendu en direct, sous l'exemple rsC qui
//   <container.barre>...          precede : du rsH, puis apres une ligne
//   ---                          `---` des styles en plus (facultatif),
//   .barre { ... }               voir `apercu`
//   ```
//   | a | b |                   tableau ; la 1re ligne est l'en-tete,
//   |---|---|                   la ligne de tirets est ignoree
//   ```rsc rsc.flex.1 "Une barre d'outils"
//   ... code ...
//   ```
//
// Chaque bloc de code porte un identifiant `<section>.<page>.<n>` : unique,
// stable, affiche au-dessus du code. C'est par lui qu'un outil (l'IDE)
// retrouve un exemple (voir `index`).
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Bloc {
    Titre { niveau: u8, texte: String },
    Paragraphe(String),
    Liste(Vec<String>),
    Note { genre: String, texte: String },
    /// Un bouton qui ouvre la demonstration `nom` (voir `demos`).
    Demo { nom: String, texte: String },
    Tableau { entetes: Vec<String>, lignes: Vec<Vec<String>> },
    Code(Exemple),
    Apercu(Apercu),
}

/// Le rendu en direct d'un exemple rsC (voir `apercu`).
#[derive(Debug, Clone, PartialEq)]
pub struct Apercu {
    /// Rang dans la page (0, 1, 2...) : place de son rendu (`#apercu-<n>`).
    pub n: usize,
    pub legende: String,
    /// La page de demonstration.
    pub rsh: String,
    /// L'exemple rsC qui precede, tel quel.
    pub rsc: String,
    /// Styles propres a l'apercu (tailles, couleurs pour rendre visible).
    pub rsc_en_plus: String,
}

/// Un bloc de code.
#[derive(Debug, Clone, PartialEq)]
pub struct Exemple {
    /// `rsc.flex.1`
    pub id: String,
    /// `rsh`, `rsc`, `rust`, `toml`, `sh`, `texte`...
    pub langage: String,
    pub titre: String,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub section: String,
    pub id: String,
    pub titre: String,
    pub resume: String,
    pub blocs: Vec<Bloc>,
    pub fichier: PathBuf,
}

impl Page {
    pub fn exemples(&self) -> impl Iterator<Item = &Exemple> {
        self.blocs.iter().filter_map(|b| match b {
            Bloc::Code(e) => Some(e),
            _ => None,
        })
    }

    pub fn apercus(&self) -> impl Iterator<Item = &Apercu> {
        self.blocs.iter().filter_map(|b| match b {
            Bloc::Apercu(a) => Some(a),
            _ => None,
        })
    }

    /// `/doc/rsc/flex`
    pub fn chemin(&self) -> String {
        format!("/doc/{}/{}", self.section, self.id)
    }
}

#[derive(Debug, Clone)]
pub struct Section {
    pub id: String,
    pub titre: String,
    pub resume: String,
    pub pages: Vec<Page>,
}

/// Toute la documentation.
#[derive(Debug, Clone, Default)]
pub struct Docs {
    pub sections: Vec<Section>,
}

impl Docs {
    /// Lit `dossier` (le dossier `contenu/`). Une page mal formee est une
    /// erreur : elle doit se voir tout de suite, pas disparaitre.
    pub fn charger(dossier: &Path) -> Result<Docs, String> {
        let mut sections = Vec::new();
        for dir in entrees(dossier)? {
            if !dir.is_dir() {
                continue;
            }
            let id = identifiant(&dir);
            let entete = std::fs::read_to_string(dir.join("_section")).map_err(|e| format!("{}/_section : {e}", dir.display()))?;
            let (champs, _) = champs(&entete);
            let mut pages = Vec::new();
            for fichier in entrees(&dir)? {
                if fichier.extension().is_some_and(|e| e == "page") {
                    let texte = std::fs::read_to_string(&fichier).map_err(|e| format!("{} : {e}", fichier.display()))?;
                    let page = lire_page(&id, &identifiant(&fichier), &texte).map_err(|e| format!("{} : {e}", fichier.display()))?;
                    pages.push(Page { fichier, ..page });
                }
            }
            sections.push(Section { id, titre: champ(&champs, "titre"), resume: champ(&champs, "resume"), pages });
        }
        Ok(Docs { sections })
    }

    pub fn section(&self, id: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.id == id)
    }

    pub fn page(&self, section: &str, id: &str) -> Option<&Page> {
        self.section(section)?.pages.iter().find(|p| p.id == id)
    }

    pub fn pages(&self) -> impl Iterator<Item = &Page> {
        self.sections.iter().flat_map(|s| s.pages.iter())
    }

    pub fn exemple(&self, id: &str) -> Option<(&Page, &Exemple)> {
        self.pages().find_map(|p| p.exemples().find(|e| e.id == id).map(|e| (p, e)))
    }

    /// La page d'avant et celle d'apres dans l'ordre de lecture (toutes
    /// sections confondues).
    pub fn voisines(&self, section: &str, id: &str) -> (Option<&Page>, Option<&Page>) {
        let toutes: Vec<&Page> = self.pages().collect();
        let Some(i) = toutes.iter().position(|p| p.section == section && p.id == id) else { return (None, None) };
        (i.checked_sub(1).map(|j| toutes[j]), toutes.get(i + 1).copied())
    }
}

/// Le dossier `contenu/` livre avec l'app : a cote de l'executable installe,
/// sinon celui des sources.
pub fn dossier_par_defaut() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/contenu"))
}

fn entrees(dossier: &Path) -> Result<Vec<PathBuf>, String> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dossier).map_err(|e| format!("{} : {e}", dossier.display()))?.filter_map(|e| e.ok().map(|e| e.path())).collect();
    v.sort();
    Ok(v)
}

/// `04-flex.page` -> `flex`
fn identifiant(chemin: &Path) -> String {
    let nom = chemin.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    match nom.split_once('-') {
        Some((n, reste)) if n.chars().all(|c| c.is_ascii_digit()) => reste.to_string(),
        _ => nom,
    }
}

/// Les lignes `cle: valeur` du debut, puis le reste du texte.
fn champs(texte: &str) -> (Vec<(String, String)>, &str) {
    let mut out = Vec::new();
    let mut reste = texte;
    while !reste.is_empty() {
        let (ligne, suite) = reste.split_once('\n').unwrap_or((reste, ""));
        match ligne.split_once(':') {
            Some((k, v)) if !k.is_empty() && k.chars().all(|c| c.is_ascii_lowercase()) => out.push((k.to_string(), v.trim().to_string())),
            _ => break,
        }
        reste = suite;
    }
    (out, reste)
}

fn champ(champs: &[(String, String)], nom: &str) -> String {
    champs.iter().find(|(k, _)| k == nom).map(|(_, v)| v.clone()).unwrap_or_default()
}

/// Texte courant : les marques `code` et **gras** sont retirees (le moteur
/// n'affiche pas de styles differents au milieu d'une ligne).
fn en_ligne(texte: &str) -> String {
    texte.replace("**", "").replace('`', "")
}

fn cellules(ligne: &str) -> Vec<String> {
    let l = ligne.trim().trim_start_matches('|').trim_end_matches('|');
    l.split('|').map(|c| en_ligne(c.trim())).collect()
}

/// Lit le texte d'une page.
pub fn lire_page(section: &str, id: &str, texte: &str) -> Result<Page, String> {
    let (ch, corps) = champs(texte);
    let titre = champ(&ch, "titre");
    if titre.is_empty() {
        return Err("« titre: » manquant".to_string());
    }
    // Numeros de ligne du fichier (en-tete compris) dans les erreurs.
    let decalage = texte[..texte.len() - corps.len()].lines().count() + 1;
    let mut lecture = Lecture::default();
    let mut lignes = corps.lines().enumerate().map(|(n, l)| (n + decalage, l));
    while let Some((n, ligne)) = lignes.next() {
        let t = ligne.trim_end();
        if let Some(entete) = t.strip_prefix("```apercu") {
            lecture.fermer();
            let mut corps = Vec::new();
            loop {
                match lignes.next() {
                    Some((_, l)) if l.trim_end() == "```" => break,
                    Some((_, l)) => corps.push(l.trim_end()),
                    None => return Err(format!("ligne {n} : aperçu jamais fermé")),
                }
            }
            let rsc = lecture.blocs.iter().rev().find_map(|b| match b {
                Bloc::Code(e) if e.langage == "rsc" => Some(e.code.clone()),
                _ => None,
            });
            let rsc = rsc.ok_or_else(|| format!("ligne {n} : aperçu sans exemple rsC avant lui"))?;
            let corps = corps.join("\n");
            let (rsh, en_plus) = match corps.split_once("\n---\n") {
                Some((a, b)) => (a.to_string(), b.to_string()),
                None => (corps, String::new()),
            };
            let n = lecture.blocs.iter().filter(|b| matches!(b, Bloc::Apercu(_))).count();
            let legende = entete.trim().trim_matches('"').to_string();
            lecture.blocs.push(Bloc::Apercu(Apercu { n, legende, rsh, rsc, rsc_en_plus: en_plus }));
        } else if let Some(entete) = t.strip_prefix("```") {
            lecture.fermer();
            let exemple = entete_code(entete).map_err(|e| format!("ligne {n} : {e}"))?;
            let mut code = Vec::new();
            loop {
                match lignes.next() {
                    Some((_, l)) if l.trim_end() == "```" => break,
                    Some((_, l)) => code.push(l.trim_end()),
                    None => return Err(format!("ligne {n} : bloc de code jamais ferme")),
                }
            }
            lecture.blocs.push(Bloc::Code(Exemple { code: code.join("\n"), ..exemple }));
        } else if t.is_empty() {
            lecture.fermer();
        } else if let Some(r) = t.strip_prefix("### ") {
            lecture.fermer();
            lecture.blocs.push(Bloc::Titre { niveau: 3, texte: en_ligne(r) });
        } else if let Some(r) = t.strip_prefix("## ") {
            lecture.fermer();
            lecture.blocs.push(Bloc::Titre { niveau: 2, texte: en_ligne(r) });
        } else if let Some(r) = t.strip_prefix("- ") {
            lecture.fermer_sauf_liste();
            lecture.liste.push(en_ligne(r));
        } else if let Some(r) = t.strip_prefix("> demo ") {
            lecture.fermer();
            let (nom, texte) = r.split_once(':').ok_or_else(|| format!("ligne {n} : > demo <nom>: texte"))?;
            lecture.blocs.push(Bloc::Demo { nom: nom.trim().to_string(), texte: en_ligne(texte.trim()) });
        } else if let Some(r) = t.strip_prefix("> ") {
            lecture.fermer();
            let (genre, texte) = match r.split_once(':') {
                Some((g, reste)) if ["note", "astuce", "attention"].contains(&g) => (g.to_string(), reste.trim()),
                _ => ("note".to_string(), r),
            };
            lecture.blocs.push(Bloc::Note { genre, texte: en_ligne(texte) });
        } else if t.starts_with('|') {
            lecture.fermer_sauf_tableau();
            let c = cellules(t);
            // La ligne |---|---| ne separe que l'en-tete.
            if !c.iter().all(|x| !x.is_empty() && x.chars().all(|ch| ch == '-' || ch == ':')) {
                lecture.tableau.push(c);
            }
        } else if ligne.starts_with("  ") && let Some(dernier) = lecture.liste.last_mut() {
            // Suite d'un element de liste sur la ligne suivante.
            dernier.push(' ');
            dernier.push_str(&en_ligne(t.trim()));
        } else {
            lecture.fermer_sauf_paragraphe();
            lecture.paragraphe.push(t.trim().to_string());
        }
    }
    lecture.fermer();
    let blocs = lecture.blocs;
    Ok(Page { section: section.to_string(), id: id.to_string(), titre, resume: champ(&ch, "resume"), blocs, fichier: PathBuf::new() })
}

/// Les blocs deja lus, et celui en cours (paragraphe, liste ou tableau).
#[derive(Default)]
struct Lecture {
    blocs: Vec<Bloc>,
    paragraphe: Vec<String>,
    liste: Vec<String>,
    tableau: Vec<Vec<String>>,
}

impl Lecture {
    fn fermer_paragraphe(&mut self) {
        if !self.paragraphe.is_empty() {
            self.blocs.push(Bloc::Paragraphe(en_ligne(&self.paragraphe.join(" "))));
            self.paragraphe.clear();
        }
    }

    fn fermer_liste(&mut self) {
        if !self.liste.is_empty() {
            self.blocs.push(Bloc::Liste(std::mem::take(&mut self.liste)));
        }
    }

    fn fermer_tableau(&mut self) {
        if !self.tableau.is_empty() {
            let mut lignes = std::mem::take(&mut self.tableau);
            let entetes = lignes.remove(0);
            self.blocs.push(Bloc::Tableau { entetes, lignes });
        }
    }

    fn fermer(&mut self) {
        self.fermer_paragraphe();
        self.fermer_liste();
        self.fermer_tableau();
    }

    fn fermer_sauf_paragraphe(&mut self) {
        self.fermer_liste();
        self.fermer_tableau();
    }

    fn fermer_sauf_liste(&mut self) {
        self.fermer_paragraphe();
        self.fermer_tableau();
    }

    fn fermer_sauf_tableau(&mut self) {
        self.fermer_paragraphe();
        self.fermer_liste();
    }
}

/// ```` ```rsc rsc.flex.1 "Une barre d'outils" ````
fn entete_code(entete: &str) -> Result<Exemple, String> {
    let (avant, titre) = match entete.split_once('"') {
        Some((a, t)) => (a, t.trim_end().trim_end_matches('"').to_string()),
        None => (entete, String::new()),
    };
    let mut mots = avant.split_whitespace();
    let langage = mots.next().ok_or("bloc de code sans langage")?.to_string();
    let id = mots.next().ok_or("bloc de code sans identifiant (```langage section.page.n \"titre\")")?.to_string();
    Ok(Exemple { id, langage, titre, code: String::new() })
}
