// Service `temps` : dates et heures sans fuseau (voir `back::temps`).
use super::{Methode, Service, Valeur};
use crate::back::temps;

pub static SERVICE: Service = Service {
    nom: "temps",
    description: "Dates et heures",
    methodes: &[
        Methode { nom: "maintenant", description: "L'instant présent (heure universelle)", arguments: "aucun", reponse: "{ secondes, date, heure }", appeler: maintenant },
        Methode { nom: "lire", description: "Une date écrite en secondes depuis 1970", arguments: "texte (AAAA-MM-JJ, AAAA-MM-JJ HH:MM:SS, JJ/MM/AAAA)", reponse: "{ secondes, date, heure }", appeler: lire },
        Methode { nom: "ecrire", description: "Des secondes depuis 1970 en date écrite", arguments: "secondes", reponse: "{ secondes, date, heure }", appeler: ecrire },
    ],
};

fn instant(secondes: i64) -> Valeur {
    Valeur::table([("secondes", secondes.into()), ("date", temps::date(secondes).into()), ("heure", temps::heure(secondes).into())])
}

fn maintenant(_: &Valeur) -> Result<Valeur, String> {
    Ok(instant(temps::maintenant()))
}

fn lire(a: &Valeur) -> Result<Valeur, String> {
    let texte = a.texte("texte")?;
    temps::lire(texte).map(instant).ok_or_else(|| format!("date illisible : « {texte} »"))
}

fn ecrire(a: &Valeur) -> Result<Valeur, String> {
    a.champ("secondes").and_then(Valeur::en_entier).map(instant).ok_or_else(|| "argument « secondes » attendu (un entier)".to_string())
}
