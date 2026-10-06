// Service `tableur` : un texte CSV ou un classeur .xlsx en tableau de
// cellules (voir `back::tableur`).
use super::{Methode, Service, Valeur};
use crate::back::tableur::{coller, rectangle, xlsx};

pub static SERVICE: Service = Service {
    nom: "tableur",
    description: "Lire des tableaux (CSV, Excel)",
    methodes: &[
        Methode { nom: "csv", description: "Un texte CSV / TSV en cellules (séparateur deviné)", arguments: "texte", reponse: "[[cellule]]", appeler: csv },
        Methode { nom: "xlsx", description: "Une feuille d'un classeur Excel", arguments: "base64 (le fichier .xlsx), feuille? (rang, 0 = la première)", reponse: "{ lignes: [[cellule]], feuilles: [nom] }", appeler: classeur },
    ],
};

fn cellules(lignes: Vec<Vec<String>>) -> Valeur {
    Valeur::liste(lignes.into_iter().map(Valeur::from))
}

fn csv(a: &Valeur) -> Result<Valeur, String> {
    Ok(cellules(coller(a.texte("texte")?)))
}

fn classeur(a: &Valeur) -> Result<Valeur, String> {
    let octets = a.octets()?;
    let lu = xlsx::lire(&octets, a.entier_ou("feuille", 0).max(0) as usize)?;
    Ok(Valeur::table([("lignes", cellules(rectangle(lu.lignes))), ("feuilles", lu.feuilles.into())]))
}
