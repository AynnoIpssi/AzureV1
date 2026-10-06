// Service `empreinte` : l'empreinte d'un contenu (voir `back::hachage`).
use super::{Methode, Service, Valeur};
use crate::back::encodage::hexa::hexa;
use crate::back::hachage::{crc32::crc32, md5::md5, sha1::sha1, sha256::sha256};

pub static SERVICE: Service = Service {
    nom: "empreinte",
    description: "Empreintes et sommes de contrôle",
    methodes: &[Methode { nom: "calculer", description: "L'empreinte d'un contenu, en hexadécimal", arguments: "texte ou base64, algo? (sha256 par défaut, sha1, md5, crc32)", reponse: "texte", appeler: calculer }],
};

fn calculer(a: &Valeur) -> Result<Valeur, String> {
    let octets = a.octets()?;
    Ok(match a.texte_ou("algo", "sha256") {
        "sha256" => hexa(&sha256(&octets)),
        "sha1" => hexa(&sha1(&octets)),
        "md5" => hexa(&md5(&octets)),
        "crc32" => format!("{:08x}", crc32(&octets)),
        autre => return Err(format!("algo inconnu : « {autre} » (sha256, sha1, md5, crc32)")),
    }
    .into())
}
