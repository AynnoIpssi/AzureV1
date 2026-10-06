// Service `archive` : lire et ecrire des archives zip (voir
// `back::compression::zip`). Les octets voyagent en base64.
use super::{Methode, Service, Valeur};
use crate::back::compression::zip::{Archive, Zip};
use crate::back::encodage::base64::base64;

pub static SERVICE: Service = Service {
    nom: "archive",
    description: "Archives zip",
    methodes: &[
        Methode { nom: "lister", description: "Les fichiers d'une archive", arguments: "base64 (le .zip)", reponse: "[nom]", appeler: lister },
        Methode { nom: "extraire", description: "Un fichier d'une archive", arguments: "base64 (le .zip), nom", reponse: "{ base64 } ou rien s'il n'y est pas", appeler: extraire },
        Methode { nom: "creer", description: "Une archive depuis des fichiers", arguments: "fichiers: [{ nom, texte ou base64 }]", reponse: "{ base64 }", appeler: creer },
    ],
};

fn lister(a: &Valeur) -> Result<Valeur, String> {
    let octets = a.octets()?;
    Ok(Zip::ouvrir(&octets)?.noms().to_vec().into())
}

fn extraire(a: &Valeur) -> Result<Valeur, String> {
    let octets = a.octets()?;
    let contenu = Zip::ouvrir(&octets)?.fichier(a.texte("nom")?)?;
    Ok(contenu.map_or(Valeur::Rien, |c| Valeur::table([("base64", base64(&c).into())])))
}

fn creer(a: &Valeur) -> Result<Valeur, String> {
    let fichiers = a.champ("fichiers").and_then(Valeur::en_liste).ok_or("argument « fichiers » attendu (une liste)")?;
    let mut archive = Archive::new();
    for f in fichiers {
        archive.ajouter(f.texte("nom")?, &f.octets()?)?;
    }
    Ok(Valeur::table([("base64", base64(&archive.finir()).into())]))
}
