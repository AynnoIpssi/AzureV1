// Recherche dans la documentation : pages et exemples de code. C'est ce
// que l'IDE utilisera (`chercher`, et `Docs::exemple` pour ouvrir un
// exemple par son identifiant) ; la page Recherche de l'app aussi.
use crate::contenu::{Bloc, Docs, Exemple, Page};

#[derive(Debug, Clone, PartialEq)]
pub enum Genre {
    Page,
    Exemple,
}

#[derive(Debug, Clone)]
pub struct Resultat {
    pub genre: Genre,
    /// Identifiant de l'exemple (`rsc.flex.1`), ou `section/page`.
    pub id: String,
    pub titre: String,
    /// `rsC › Flexbox`
    pub lieu: String,
    /// Chemin de la page dans l'app (`/doc/rsc/flex`).
    pub chemin: String,
    /// Debut du texte ou du code trouve.
    pub extrait: String,
    pub score: u32,
}

/// Minuscules, sans accents : « Écran » et « ecran » se trouvent.
pub fn normaliser(texte: &str) -> String {
    texte
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

fn texte_page(page: &Page) -> String {
    let mut out = String::new();
    for bloc in &page.blocs {
        match bloc {
            Bloc::Titre { texte, .. } | Bloc::Paragraphe(texte) | Bloc::Note { texte, .. } | Bloc::Demo { texte, .. } => out.push_str(texte),
            Bloc::Liste(items) => out.push_str(&items.join(" ")),
            Bloc::Tableau { entetes, lignes } => {
                out.push_str(&entetes.join(" "));
                for l in lignes {
                    out.push(' ');
                    out.push_str(&l.join(" "));
                }
            }
            Bloc::Code(_) | Bloc::Apercu(_) => {}
        }
        out.push(' ');
    }
    out
}

/// Points pour `mots` dans `champ` (tous les mots doivent y etre, sinon 0).
fn points(champ: &str, mots: &[String], poids: u32) -> u32 {
    if mots.iter().all(|m| champ.contains(m.as_str())) { poids } else { 0 }
}

fn extrait(texte: &str, mots: &[String]) -> String {
    let plat = texte.split_whitespace().collect::<Vec<_>>().join(" ");
    let norm = normaliser(&plat);
    // Autour du premier mot trouve.
    let debut = mots.first().and_then(|m| norm.find(m.as_str())).unwrap_or(0);
    let chars: Vec<char> = plat.chars().collect();
    let pos = norm[..debut].chars().count().saturating_sub(40);
    let fin = (pos + 160).min(chars.len());
    let mut s: String = chars[pos..fin].iter().collect();
    if pos > 0 {
        s.insert_str(0, "… ");
    }
    if fin < chars.len() {
        s.push_str(" …");
    }
    s
}

fn exemple(page: &Page, lieu: &str, e: &Exemple, mots: &[String], requete: &str) -> Option<Resultat> {
    // Un identifiant tape tel quel passe avant tout.
    let exact = if normaliser(&e.id) == requete { 1000 } else { 0 };
    let score = exact + points(&normaliser(&e.id), mots, 60) + points(&normaliser(&e.titre), mots, 40) + points(&normaliser(&e.code), mots, 15) + points(&normaliser(&page.titre), mots, 5);
    (score > 0).then(|| Resultat { genre: Genre::Exemple, id: e.id.clone(), titre: if e.titre.is_empty() { e.id.clone() } else { e.titre.clone() }, lieu: lieu.to_string(), chemin: page.chemin(), extrait: extrait(&e.code, &[]), score })
}

/// Pages et exemples qui contiennent tous les mots de `requete`, les plus
/// pertinents d'abord (titre > resume > texte ; pour un exemple :
/// identifiant > titre > code).
pub fn chercher(docs: &Docs, requete: &str) -> Vec<Resultat> {
    let requete = normaliser(requete.trim());
    let mots: Vec<String> = requete.split_whitespace().map(str::to_string).collect();
    if mots.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for section in &docs.sections {
        for page in &section.pages {
            let lieu = format!("{} › {}", section.titre, page.titre);
            let corps = texte_page(page);
            let score = points(&normaliser(&page.titre), &mots, 100) + points(&normaliser(&page.resume), &mots, 40) + points(&normaliser(&corps), &mots, 10) + points(&normaliser(&section.titre), &mots, 5);
            if score > 0 {
                let norm = normaliser(&corps);
                let source = if mots.iter().all(|m| norm.contains(m.as_str())) { corps.as_str() } else { page.resume.as_str() };
                out.push(Resultat { genre: Genre::Page, id: format!("{}/{}", section.id, page.id), titre: page.titre.clone(), lieu: section.titre.clone(), chemin: page.chemin(), extrait: extrait(source, &mots), score });
            }
            out.extend(page.exemples().filter_map(|e| exemple(page, &lieu, e, &mots, &requete)));
        }
    }
    out.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
    out
}
