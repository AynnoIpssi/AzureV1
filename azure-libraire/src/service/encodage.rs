// Service `encodage` : base64, texte d'encodage inconnu, accents (voir
// `back::encodage`).
use super::{Methode, Service, Valeur};
use crate::back::encodage::{base64::base64, texte};

pub static SERVICE: Service = Service {
    nom: "encodage",
    description: "Passer d'une écriture à une autre",
    methodes: &[
        Methode { nom: "base64", description: "Un texte en base64", arguments: "texte", reponse: "texte", appeler: en_base64 },
        Methode { nom: "texte", description: "Le texte d'octets d'encodage inconnu (UTF-8, UTF-16, Windows-1252)", arguments: "base64", reponse: "texte", appeler: en_texte },
        Methode { nom: "sans-accent", description: "Un texte sans ses accents", arguments: "texte", reponse: "texte", appeler: sans_accent },
    ],
};

fn en_base64(a: &Valeur) -> Result<Valeur, String> {
    Ok(base64(a.texte("texte")?.as_bytes()).into())
}

fn en_texte(a: &Valeur) -> Result<Valeur, String> {
    a.texte("base64")?;
    Ok(texte::decoder(&a.octets()?).into())
}

fn sans_accent(a: &Valeur) -> Result<Valeur, String> {
    Ok(texte::sans_accent(a.texte("texte")?).into())
}
