// Compresseur DEFLATE (RFC 1951), le pendant de `deflate::inflate`.
//
// Simple plutot qu'optimal : recherche des repetitions (LZ77, fenetre de
// 32 Ko, chaines de hachage) puis codes de Huffman FIXES (ceux de la
// norme, pas de table calculee par bloc). Le resultat est lu par tout
// decompresseur (zlib, zip, git) ; il est seulement un peu plus gros que
// celui de zlib. Des donnees qui ne se compressent pas sont rangees
// telles quelles (blocs « stockes »).

const FENETRE: usize = 32_768;
const MASQUE: usize = FENETRE - 1;
const MIN: usize = 3;
const MAX: usize = 258;
/// Candidats essayes au plus pour une position.
const CHAINE: usize = 48;
const TABLE: usize = 1 << 15;

const LONGUEURS: [(u16, u8); 29] = [
    (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), (11, 1), (13, 1), (15, 1), (17, 1), (19, 2), (23, 2), (27, 2), (31, 2),
    (35, 3), (43, 3), (51, 3), (59, 3), (67, 4), (83, 4), (99, 4), (115, 4), (131, 5), (163, 5), (195, 5), (227, 5), (258, 0),
];

const DISTANCES: [(u16, u8); 30] = [
    (1, 0), (2, 0), (3, 0), (4, 0), (5, 1), (7, 1), (9, 2), (13, 2), (17, 3), (25, 3), (33, 4), (49, 4), (65, 5), (97, 5), (129, 6), (193, 6),
    (257, 7), (385, 7), (513, 8), (769, 8), (1025, 9), (1537, 9), (2049, 10), (3073, 10), (4097, 11), (6145, 11), (8193, 12), (12289, 12),
    (16385, 13), (24577, 13),
];

/// Ecrit des bits dans l'ordre DEFLATE : le moins significatif d'abord.
struct Bits {
    out: Vec<u8>,
    tampon: u64,
    nombre: u32,
}

impl Bits {
    fn ecrire(&mut self, valeur: u32, bits: u32) {
        self.tampon |= (valeur as u64) << self.nombre;
        self.nombre += bits;
        while self.nombre >= 8 {
            self.out.push(self.tampon as u8);
            self.tampon >>= 8;
            self.nombre -= 8;
        }
    }

    /// Un code de Huffman : ses bits partent du plus significatif.
    fn code(&mut self, code: u32, bits: u32) {
        self.ecrire(code.reverse_bits() >> (32 - bits), bits);
    }

    /// Un symbole litteral / longueur, avec les codes fixes de la norme.
    fn symbole(&mut self, s: u32) {
        match s {
            0..=143 => self.code(0x30 + s, 8),
            144..=255 => self.code(0x190 + s - 144, 9),
            256..=279 => self.code(s - 256, 7),
            _ => self.code(0xC0 + s - 280, 8),
        }
    }

    fn finir(mut self) -> Vec<u8> {
        if self.nombre > 0 {
            self.out.push(self.tampon as u8);
        }
        self.out
    }
}

fn hachage(d: &[u8], i: usize) -> usize {
    let v = (d[i] as u32) << 16 | (d[i + 1] as u32) << 8 | d[i + 2] as u32;
    (v.wrapping_mul(0x9E37_79B1) >> 17) as usize & (TABLE - 1)
}

/// Compresse `data` en un flux DEFLATE brut (sans enveloppe : voir
/// `zlib::compress` pour l'enveloppe zlib).
pub fn deflate(data: &[u8]) -> Vec<u8> {
    let compresse = fixe(data);
    let stocke_taille = data.len() + 5 * data.len().div_ceil(65_535).max(1);
    if compresse.len() > stocke_taille { stocke(data) } else { compresse }
}

fn stocke(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 5);
    let blocs: Vec<&[u8]> = if data.is_empty() { vec![data] } else { data.chunks(65_535).collect() };
    for (i, bloc) in blocs.iter().enumerate() {
        out.push((i + 1 == blocs.len()) as u8);
        out.extend((bloc.len() as u16).to_le_bytes());
        out.extend((!(bloc.len() as u16)).to_le_bytes());
        out.extend(*bloc);
    }
    out
}

// Retient la position `i` pour les recherches suivantes.
fn noter(d: &[u8], tete: &mut [i32], precedent: &mut [i32], i: usize) {
    if i + MIN <= d.len() {
        let h = hachage(d, i);
        precedent[i & MASQUE] = tete[h];
        tete[h] = i as i32;
    }
}

fn fixe(d: &[u8]) -> Vec<u8> {
    let mut b = Bits { out: Vec::with_capacity(d.len() / 2 + 16), tampon: 0, nombre: 0 };
    // Un seul bloc, le dernier, a codes fixes.
    b.ecrire(1, 1);
    b.ecrire(1, 2);
    let mut tete = vec![-1i32; TABLE];
    let mut precedent = vec![-1i32; FENETRE];
    let mut i = 0;
    while i < d.len() {
        let (mut longueur, mut distance) = (0, 0);
        if i + MIN <= d.len() {
            let limite = (d.len() - i).min(MAX);
            let mut candidat = tete[hachage(d, i)];
            let mut essais = CHAINE;
            while candidat >= 0 && essais > 0 {
                let c = candidat as usize;
                if i - c > FENETRE {
                    break;
                }
                // Le dernier octet d'abord : ecarte vite les candidats
                // qui ne feraient pas mieux.
                if longueur < limite && d[c + longueur] == d[i + longueur] {
                    let n = d[c..].iter().zip(&d[i..i + limite]).take_while(|(x, y)| x == y).count();
                    if n > longueur {
                        (longueur, distance) = (n, i - c);
                        if n == limite {
                            break;
                        }
                    }
                }
                let suivant = precedent[c & MASQUE];
                if suivant >= candidat {
                    break;
                }
                candidat = suivant;
                essais -= 1;
            }
        }
        if longueur >= MIN {
            let l = LONGUEURS.iter().rposition(|(base, _)| *base as usize <= longueur).unwrap();
            b.symbole(257 + l as u32);
            b.ecrire((longueur - LONGUEURS[l].0 as usize) as u32, LONGUEURS[l].1 as u32);
            let k = DISTANCES.iter().rposition(|(base, _)| *base as usize <= distance).unwrap();
            b.code(k as u32, 5);
            b.ecrire((distance - DISTANCES[k].0 as usize) as u32, DISTANCES[k].1 as u32);
            for j in i..i + longueur {
                noter(d, &mut tete, &mut precedent, j);
            }
            i += longueur;
        } else {
            b.symbole(d[i] as u32);
            noter(d, &mut tete, &mut precedent, i);
            i += 1;
        }
    }
    b.symbole(256);
    b.finir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::back::compression::deflate::inflate;
    use crate::back::hasard::Alea;

    fn aller_retour(data: &[u8]) -> usize {
        let compresse = deflate(data);
        assert_eq!(inflate(&compresse).unwrap(), data, "{} octets", data.len());
        compresse.len()
    }

    #[test]
    fn ce_qui_est_compresse_se_relit() {
        aller_retour(b"");
        aller_retour(b"a");
        aller_retour(b"ab");
        aller_retour(b"abcabcabcabcabcabcabcabc");
        aller_retour(&(0..=255u8).collect::<Vec<u8>>());
        let texte = "Le meme texte, encore et encore. ".repeat(400);
        assert!(aller_retour(texte.as_bytes()) < texte.len() / 10);
        // Une longue plage du meme octet : longueurs de 258, distance 1.
        assert!(aller_retour(&[0u8; 100_000]) < 1500);
    }

    #[test]
    fn le_hasard_est_stocke_sans_grossir() {
        let mut alea = Alea::new(1);
        let bruit: Vec<u8> = (0..200_000).map(|_| alea.u64() as u8).collect();
        let taille = aller_retour(&bruit);
        assert!(taille <= bruit.len() + 5 * 4, "{taille}");
    }

    #[test]
    fn des_repetitions_lointaines_et_melangees() {
        let mut alea = Alea::new(9);
        let mots: Vec<Vec<u8>> = (0..300).map(|_| (0..alea.entre(2, 40)).map(|_| alea.u64() as u8).collect()).collect();
        let mut data = Vec::new();
        while data.len() < 300_000 {
            data.extend(&mots[alea.sous(300) as usize]);
        }
        assert!(aller_retour(&data) < data.len());
    }
}
