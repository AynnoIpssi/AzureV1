// Un tableau de cellules en texte, comme une feuille de tableur : ce que
// un import montre et envoie dans une table. Il vient d'un fichier
// (`csv` : .csv / .tsv / .txt ; `xlsx` : classeur Excel), d'un bloc colle
// depuis un tableur, ou de la saisie.
pub mod csv;
pub mod xlsx;

pub use crate::back::encodage::texte::sans_accent;
use std::path::Path;

/// Taille maximale d'un fichier a importer.
pub const MAX_FICHIER: u64 = 64 * 1024 * 1024;

/// Ce qu'un fichier contient : ses lignes de cellules, et pour un classeur
/// le nom de ses feuilles (une seule est lue a la fois).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lu {
    pub lignes: Vec<Vec<String>>,
    pub feuilles: Vec<String>,
}

/// Lit `chemin` selon son extension. `feuille` : le rang de la feuille
/// d'un classeur (0 = la premiere).
pub fn lire(chemin: &Path, feuille: usize) -> Result<Lu, String> {
    let nom = chemin.display();
    let taille = std::fs::metadata(chemin).map_err(|e| format!("{nom} : {e}"))?.len();
    if taille > MAX_FICHIER {
        return Err(format!("{nom} : fichier trop gros ({} Mo, {} au plus)", taille / 1_048_576, MAX_FICHIER / 1_048_576));
    }
    let octets = std::fs::read(chemin).map_err(|e| format!("{nom} : {e}"))?;
    let extension = chemin.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let lu = match extension.as_str() {
        "xlsx" | "xlsm" => xlsx::lire(&octets, feuille).map_err(|e| format!("{nom} : {e}"))?,
        "xls" | "ods" => return Err(format!("{nom} : ce format n'est pas lu. Enregistrez le classeur en .xlsx ou en .csv.")),
        // Un classeur renomme se reconnait a sa signature (archive zip).
        _ if octets.starts_with(b"PK\x03\x04") => xlsx::lire(&octets, feuille).map_err(|e| format!("{nom} : {e}"))?,
        _ => Lu { lignes: csv::lire(&csv::texte(&octets)), feuilles: Vec::new() },
    };
    Ok(Lu { lignes: rectangle(lu.lignes), ..lu })
}

/// Un bloc colle depuis un tableur (cellules separees par des tabulations)
/// ou du texte CSV.
pub fn coller(texte: &str) -> Vec<Vec<String>> {
    rectangle(csv::lire(texte))
}

/// Toutes les lignes a la meme largeur, sans les lignes vides de la fin ni
/// les colonnes vides de droite.
pub fn rectangle(mut lignes: Vec<Vec<String>>) -> Vec<Vec<String>> {
    let vide = |l: &Vec<String>| l.iter().all(|c| c.trim().is_empty());
    while lignes.last().is_some_and(vide) {
        lignes.pop();
    }
    let largeur = lignes.iter().map(|l| l.iter().rposition(|c| !c.trim().is_empty()).map_or(0, |p| p + 1)).max().unwrap_or(0);
    for l in &mut lignes {
        l.resize(largeur, String::new());
    }
    lignes
}

/// Le nom d'une colonne sans titre : A, B… Z, AA, AB…
pub fn lettre(mut rang: usize) -> String {
    let mut nom = String::new();
    loop {
        nom.insert(0, (b'A' + (rang % 26) as u8) as char);
        if rang < 26 {
            return nom;
        }
        rang = rang / 26 - 1;
    }
}

/// Un nom ramene a l'essentiel pour comparer un titre de colonne du
/// tableau a une colonne de la table : « Prénom », « prenom » et
/// « PRENOM » sont le meme.
pub fn simplifier(nom: &str) -> String {
    sans_accent(nom).to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_colonnes_sans_titre_portent_des_lettres() {
        assert_eq!([0, 1, 25, 26, 27, 51, 52, 701, 702].map(lettre), ["A", "B", "Z", "AA", "AB", "AZ", "BA", "ZZ", "AAA"]);
    }

    #[test]
    fn un_tableau_devient_rectangulaire() {
        let lignes = vec![vec!["a".to_string(), "".to_string(), " ".to_string()], vec!["b".to_string(), "c".to_string()], vec![], vec![" ".to_string()]];
        assert_eq!(rectangle(lignes), [["a", ""], ["b", "c"]]);
    }

    #[test]
    fn les_titres_se_comparent_sans_accent_ni_casse() {
        assert_eq!(simplifier("Prénom "), "prenom");
        assert_eq!(simplifier("Code_Postal"), simplifier("code postal"));
    }
}
