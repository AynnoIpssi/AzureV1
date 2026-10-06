// Adler-32 : la somme de controle qui termine un flux zlib (RFC 1950).

const MODULE: u32 = 65521;

pub fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    // 5552 octets au plus avant que `b` ne deborde d'un u32 : un seul
    // modulo par paquet.
    for paquet in data.chunks(5552) {
        for &octet in paquet {
            a += octet as u32;
            b += a;
        }
        a %= MODULE;
        b %= MODULE;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_valeur_de_reference() {
        // Exemple de reference standard (page Wikipedia "Adler-32").
        assert_eq!(adler32(b"Wikipedia"), 0x11E60398);
        assert_eq!(adler32(b""), 1);
        // Assez long pour passer par plusieurs paquets.
        let long: Vec<u8> = (0..20_000u32).map(|i| (i * 7) as u8).collect();
        let (mut a, mut b) = (1u32, 0u32);
        for &o in &long {
            a = (a + o as u32) % MODULE;
            b = (b + a) % MODULE;
        }
        assert_eq!(adler32(&long), (b << 16) | a);
    }
}
