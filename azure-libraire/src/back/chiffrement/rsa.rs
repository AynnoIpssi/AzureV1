// RSA, cle publique seulement : chiffrer un secret pour un serveur
// (RSA-OAEP avec SHA-1, ce que MySQL demande). Rien d'audite : a garder
// pour ces echanges, sur un reseau de confiance.
use crate::back::encodage::base64::base64_lire;
use crate::back::hachage::sha1::sha1;

fn depuis_octets(octets: &[u8], mots: usize) -> Vec<u32> {
    let mut out = vec![0u32; mots];
    for (i, b) in octets.iter().rev().enumerate() {
        if i / 4 < mots {
            out[i / 4] |= (*b as u32) << (8 * (i % 4));
        }
    }
    out
}

fn plus_petit(a: &[u32], b: &[u32]) -> bool {
    for i in (0..a.len()).rev() {
        if a[i] != b[i] {
            return a[i] < b[i];
        }
    }
    false
}

fn soustraire(a: &mut [u32], b: &[u32]) {
    let mut emprunt = 0u64;
    for i in 0..a.len() {
        let (x, y) = (a[i] as u64, b[i] as u64 + emprunt);
        a[i] = x.wrapping_sub(y) as u32;
        emprunt = (x < y) as u64;
    }
}

// (a * b) mod n, bit par bit : doubler, ajouter, reduire. Lent mais
// simple ; l'exposant public n'a que 17 bits.
fn fois_mod(a: &[u32], b: &[u32], n: &[u32]) -> Vec<u32> {
    let mut r = vec![0u32; n.len()];
    for bit in (0..a.len() * 32).rev() {
        let mut retenue = 0;
        for mot in r.iter_mut() {
            let suivant = *mot >> 31;
            *mot = *mot << 1 | retenue;
            retenue = suivant;
        }
        if !plus_petit(&r, n) {
            soustraire(&mut r, n);
        }
        if a[bit / 32] >> (bit % 32) & 1 == 1 {
            let mut retenue = 0u64;
            for (mot, x) in r.iter_mut().zip(b) {
                let s = *mot as u64 + *x as u64 + retenue;
                *mot = s as u32;
                retenue = s >> 32;
            }
            if !plus_petit(&r, n) {
                soustraire(&mut r, n);
            }
        }
    }
    r
}

/// Une cle publique RSA : module et exposant (octets, gros-boutiste).
#[derive(Debug, Clone, PartialEq)]
pub struct ClePublique {
    pub module: Vec<u8>,
    pub exposant: Vec<u8>,
}

// Un element DER : (etiquette, contenu, reste).
fn der(d: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (etiquette, premier) = (*d.first()?, *d.get(1)? as usize);
    let (longueur, debut) = if premier < 0x80 {
        (premier, 2)
    } else {
        let n = premier & 0x7f;
        (d.get(2..2 + n)?.iter().fold(0usize, |l, b| l << 8 | *b as usize), 2 + n)
    };
    Some((etiquette, d.get(debut..debut + longueur)?, &d[debut + longueur..]))
}

impl ClePublique {
    /// Depuis un texte PEM (`BEGIN PUBLIC KEY` ou `BEGIN RSA PUBLIC KEY`).
    pub fn depuis_pem(pem: &str) -> Option<ClePublique> {
        let corps: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
        let binaire = base64_lire(&corps)?;
        let (_, mut sequence, _) = der(&binaire)?;
        // Forme longue : (algorithme, chaine de bits contenant la cle).
        if let Some((0x30, _, reste)) = der(sequence) {
            let (_, bits, _) = der(reste)?;
            sequence = der(bits.get(1..)?)?.1;
        }
        let (_, module, reste) = der(sequence)?;
        let (_, exposant, _) = der(reste)?;
        let sans_zeros = |x: &[u8]| x.iter().skip_while(|b| **b == 0).copied().collect::<Vec<u8>>();
        Some(ClePublique { module: sans_zeros(module), exposant: sans_zeros(exposant) })
    }

    // message ^ exposant mod module, sur la longueur du module.
    fn puissance(&self, message: &[u8]) -> Vec<u8> {
        let mots = self.module.len().div_ceil(4) + 1;
        let n = depuis_octets(&self.module, mots);
        let m = depuis_octets(message, mots);
        let mut r = depuis_octets(&[1], mots);
        for bit in (0..self.exposant.len() * 8).rev() {
            r = fois_mod(&r, &r, &n);
            if self.exposant[self.exposant.len() - 1 - bit / 8] >> (bit % 8) & 1 == 1 {
                r = fois_mod(&r, &m, &n);
            }
        }
        (0..self.module.len()).rev().map(|i| (r[i / 4] >> (8 * (i % 4))) as u8).collect()
    }

    /// Chiffre `message` (RSA-OAEP, SHA-1, sans etiquette) ; `graine` :
    /// 20 octets au hasard.
    pub fn chiffrer_oaep(&self, message: &[u8], graine: &[u8; 20]) -> Result<Vec<u8>, String> {
        let k = self.module.len();
        if message.len() + 42 > k {
            return Err("Mot de passe trop long pour la clé du serveur".to_string());
        }
        let masque = |source: &[u8], longueur: usize| {
            let mut out = Vec::new();
            let mut compteur = 0u32;
            while out.len() < longueur {
                out.extend(sha1(&[source, &compteur.to_be_bytes()].concat()));
                compteur += 1;
            }
            out.truncate(longueur);
            out
        };
        let mut bloc = sha1(b"").to_vec();
        bloc.resize(k - 21 - message.len() - 1, 0);
        bloc.push(1);
        bloc.extend(message);
        for (b, m) in bloc.iter_mut().zip(masque(graine, k - 21)) {
            *b ^= m;
        }
        let mut graine = *graine;
        for (g, m) in graine.iter_mut().zip(masque(&bloc, 20)) {
            *g ^= m;
        }
        Ok(self.puissance(&[&[0u8][..], &graine, &bloc].concat()))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::back::encodage::hexa::hexa;

    #[test]
    fn la_puissance_modulaire_est_juste() {
        // 4 ^ 13 mod 497 = 445 ; puis sur plusieurs mots.
        let cle = ClePublique { module: vec![0x01, 0xf1], exposant: vec![13] };
        assert_eq!(cle.puissance(&[4]), [0x01, 0xbd]);
        // (2^64 + 3) ^ 65537 mod (2^89 - 1), calcule a part (python : pow).
        let cle = ClePublique { module: [vec![0x01], vec![0xff; 11]].concat(), exposant: vec![1, 0, 1] };
        assert_eq!(hexa(&cle.puissance(&[1, 0, 0, 0, 0, 0, 0, 0, 3])), "00b7f5f3ab18f1ae685d22e7");
    }
}
