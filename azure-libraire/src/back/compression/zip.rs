// Les archives zip, lues et ecrites a la main : c'est aussi le format des
// classeurs .xlsx, des .docx, des .jar... Pas de zip64 (archives de plus
// de 4 Go ou de 65 535 fichiers) ni de chiffrement.
use super::compresser::deflate;
use super::deflate::inflate;
use crate::back::hachage::crc32::crc32;
use std::collections::HashMap;

/// Une archive ouverte en lecture.
pub struct Zip<'a> {
    octets: &'a [u8],
    /// nom -> (methode, taille compressee, taille reelle, debut de l'entete locale).
    fichiers: HashMap<String, (u16, usize, usize, usize)>,
    /// Les noms dans l'ordre de l'archive.
    ordre: Vec<String>,
}

fn u16_a(o: &[u8], p: usize) -> Option<usize> {
    Some(u16::from_le_bytes(o.get(p..p + 2)?.try_into().ok()?) as usize)
}

fn u32_a(o: &[u8], p: usize) -> Option<usize> {
    Some(u32::from_le_bytes(o.get(p..p + 4)?.try_into().ok()?) as usize)
}

impl<'a> Zip<'a> {
    pub fn ouvrir(octets: &'a [u8]) -> Result<Zip<'a>, String> {
        let abime = || "archive zip abîmée".to_string();
        if !octets.starts_with(b"PK") {
            return Err("ce n'est pas une archive zip".to_string());
        }
        // La fin du repertoire central : cherchee depuis la fin (elle peut
        // etre suivie d'un commentaire).
        let debut_recherche = octets.len().saturating_sub(22 + 65_535);
        let fin = (debut_recherche..=octets.len().saturating_sub(22)).rev().find(|p| octets[*p..].starts_with(b"PK\x05\x06")).ok_or_else(abime)?;
        let nombre = u16_a(octets, fin + 10).ok_or_else(abime)?;
        let mut p = u32_a(octets, fin + 16).ok_or_else(abime)?;
        if nombre == 0xFFFF || p == 0xFFFF_FFFF {
            return Err("archive zip trop grosse (zip64)".to_string());
        }
        let mut fichiers = HashMap::new();
        let mut ordre = Vec::new();
        for _ in 0..nombre {
            if !octets.get(p..).is_some_and(|r| r.starts_with(b"PK\x01\x02")) {
                return Err(abime());
            }
            let lu = (|| Some((u16_a(octets, p + 10)?, u32_a(octets, p + 20)?, u32_a(octets, p + 24)?, u16_a(octets, p + 28)?, u16_a(octets, p + 30)?, u16_a(octets, p + 32)?, u32_a(octets, p + 42)?)))();
            let (methode, compresse, reel, nom, extra, commentaire, locale) = lu.ok_or_else(abime)?;
            let texte = String::from_utf8_lossy(octets.get(p + 46..p + 46 + nom).ok_or_else(abime)?).into_owned();
            ordre.push(texte.clone());
            fichiers.insert(texte, (methode as u16, compresse, reel, locale));
            p += 46 + nom + extra + commentaire;
        }
        Ok(Zip { octets, fichiers, ordre })
    }

    /// Les noms des fichiers, dans l'ordre de l'archive (les dossiers
    /// finissent par `/`).
    pub fn noms(&self) -> &[String] {
        &self.ordre
    }

    /// Le contenu d'un fichier ; `None` s'il n'est pas dans l'archive.
    pub fn fichier(&self, nom: &str) -> Result<Option<Vec<u8>>, String> {
        // Les noms sont compares sans la casse (certains outils ecrivent
        // `xl/SharedStrings.xml`).
        let Some((methode, compresse, reel, locale)) = self.fichiers.get(nom).or_else(|| self.fichiers.iter().find(|(n, _)| n.eq_ignore_ascii_case(nom)).map(|(_, v)| v)).copied() else {
            return Ok(None);
        };
        let abime = || format!("archive zip abîmée ({nom})");
        let o = self.octets;
        if !o.get(locale..).is_some_and(|r| r.starts_with(b"PK\x03\x04")) {
            return Err(abime());
        }
        let debut = locale + 30 + u16_a(o, locale + 26).ok_or_else(abime)? + u16_a(o, locale + 28).ok_or_else(abime)?;
        let donnees = o.get(debut..debut + compresse).ok_or_else(abime)?;
        match methode {
            0 => Ok(Some(donnees.to_vec())),
            8 => {
                let sortie = inflate(donnees).map_err(|e| format!("{nom} : {e}"))?;
                if sortie.len() != reel {
                    return Err(abime());
                }
                Ok(Some(sortie))
            }
            autre => Err(format!("{nom} : compression zip {autre} non lue")),
        }
    }

    /// Le contenu d'un fichier, en texte (marque UTF-8 retiree).
    pub fn texte(&self, nom: &str) -> Result<Option<String>, String> {
        Ok(self.fichier(nom)?.map(|o| String::from_utf8_lossy(o.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&o)).into_owned()))
    }
}

/// Une archive en cours d'ecriture : `ajouter` chaque fichier, puis
/// `finir` donne les octets du .zip.
#[derive(Default)]
pub struct Archive {
    out: Vec<u8>,
    /// Les entrees du repertoire central, deja encodees.
    repertoire: Vec<u8>,
    nombre: usize,
}

impl Archive {
    pub fn new() -> Archive {
        Archive::default()
    }

    /// Ajoute un fichier (noms avec des `/`, en UTF-8). Refuse ce qui
    /// demanderait zip64.
    pub fn ajouter(&mut self, nom: &str, contenu: &[u8]) -> Result<(), String> {
        if self.nombre >= 0xFFFE || contenu.len() >= 0xFFFF_FFFF || nom.len() > 0xFFFF {
            return Err(format!("{nom} : trop gros pour une archive zip (zip64 non écrit)"));
        }
        let compresse = deflate(contenu);
        // Ranger tel quel ce qui ne gagne rien a etre compresse.
        let (methode, donnees): (u16, &[u8]) = if compresse.len() < contenu.len() { (8, &compresse) } else { (0, contenu) };
        let locale = self.out.len();
        if locale + donnees.len() + nom.len() + 30 >= 0xFFFF_FFFF {
            return Err(format!("{nom} : archive zip trop grosse (zip64 non écrit)"));
        }
        // Commun aux deux entetes : version, drapeaux (bit 11 = noms en
        // UTF-8), methode, heure et date (1er janvier 1980), CRC, tailles,
        // longueur du nom, longueur du champ extra.
        let mut commun = Vec::new();
        commun.extend(20u16.to_le_bytes());
        commun.extend(0x0800u16.to_le_bytes());
        commun.extend(methode.to_le_bytes());
        commun.extend(0u16.to_le_bytes());
        commun.extend(0x0021u16.to_le_bytes());
        commun.extend(crc32(contenu).to_le_bytes());
        commun.extend((donnees.len() as u32).to_le_bytes());
        commun.extend((contenu.len() as u32).to_le_bytes());
        commun.extend((nom.len() as u16).to_le_bytes());
        commun.extend(0u16.to_le_bytes());

        self.out.extend(b"PK\x03\x04");
        self.out.extend(&commun);
        self.out.extend(nom.as_bytes());
        self.out.extend(donnees);

        self.repertoire.extend(b"PK\x01\x02");
        self.repertoire.extend(20u16.to_le_bytes());
        self.repertoire.extend(&commun);
        // Commentaire, disque, attributs internes, attributs externes.
        self.repertoire.extend([0u8; 10]);
        self.repertoire.extend((locale as u32).to_le_bytes());
        self.repertoire.extend(nom.as_bytes());
        self.nombre += 1;
        Ok(())
    }

    pub fn finir(mut self) -> Vec<u8> {
        let debut = self.out.len() as u32;
        let taille = self.repertoire.len() as u32;
        self.out.append(&mut self.repertoire);
        self.out.extend(b"PK\x05\x06");
        self.out.extend([0u8; 4]);
        self.out.extend((self.nombre as u16).to_le_bytes());
        self.out.extend((self.nombre as u16).to_le_bytes());
        self.out.extend(taille.to_le_bytes());
        self.out.extend(debut.to_le_bytes());
        self.out.extend(0u16.to_le_bytes());
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_archive_ecrite_se_relit() {
        let mut a = Archive::new();
        let long = "une ligne qui revient\n".repeat(200);
        a.ajouter("lisez-moi.txt", b"bonjour").unwrap();
        a.ajouter("dossier/long.txt", long.as_bytes()).unwrap();
        a.ajouter("vide", b"").unwrap();
        let octets = a.finir();
        assert!(octets.len() < long.len());
        let z = Zip::ouvrir(&octets).unwrap();
        assert_eq!(z.noms(), ["lisez-moi.txt", "dossier/long.txt", "vide"]);
        assert_eq!(z.fichier("lisez-moi.txt").unwrap().unwrap(), b"bonjour");
        assert_eq!(z.texte("DOSSIER/long.txt").unwrap().unwrap(), long);
        assert_eq!(z.fichier("vide").unwrap().unwrap(), b"");
        assert_eq!(z.fichier("absent").unwrap(), None);
    }

    #[test]
    fn ce_qui_n_est_pas_un_zip_est_refuse() {
        assert!(Zip::ouvrir(b"bonjour").is_err());
        assert!(Zip::ouvrir(b"PK\x03\x04tronque").is_err());
    }
}
