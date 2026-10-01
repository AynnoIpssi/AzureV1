// L'enveloppe zlib (RFC 1950) autour d'un flux DEFLATE brut - c'est ce
// format, pas DEFLATE nu, que les chunks IDAT d'un PNG contiennent (voir
// `codec::png`). Deux octets d'en-tete, le flux DEFLATE, puis un Adler-32
// des donnees DECOMPRESSEES pour detecter une corruption.
use crate::codec::deflate::inflate;

/// Decompresse un flux zlib complet : verifie l'en-tete (methode de
/// compression = DEFLATE, pas de dictionnaire pre-partage - jamais utilise
/// par un encodeur PNG), decompresse via `codec::deflate::inflate`, puis
/// verifie l'Adler-32 en queue de flux contre les octets obtenus.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 6 {
        return Err("zlib: flux trop court pour contenir un en-tete et une somme de controle".to_string());
    }

    let cmf = data[0];
    let flg = data[1];
    if cmf & 0x0F != 8 {
        return Err(format!("zlib: methode de compression {} non supportee (seul DEFLATE, methode 8, l'est)", cmf & 0x0F));
    }
    if (u16::from(cmf) * 256 + u16::from(flg)) % 31 != 0 {
        return Err("zlib: en-tete invalide (echec de la verification FCHECK)".to_string());
    }
    if flg & 0x20 != 0 {
        return Err("zlib: dictionnaire pre-partage non supporte".to_string());
    }

    let payload = &data[2..data.len() - 4];
    let decoded = inflate(payload)?;

    let expected = u32::from_be_bytes(data[data.len() - 4..].try_into().unwrap());
    let actual = adler32(&decoded);
    if expected != actual {
        return Err("zlib: somme de controle Adler-32 invalide - donnees corrompues".to_string());
    }

    Ok(decoded)
}

const ADLER_MOD: u32 = 65521;

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % ADLER_MOD;
        b = (b + a) % ADLER_MOD;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adler32_matches_the_known_reference_value_for_wikipedia() {
        // Exemple de reference standard (page Wikipedia "Adler-32").
        assert_eq!(adler32(b"Wikipedia"), 0x11E60398);
    }

    #[test]
    fn rejects_a_non_deflate_compression_method() {
        let mut data = vec![0x78, 0x9C, 0, 0, 0, 0, 0, 0];
        data[0] = 0x79; // CM = 9, pas 8
        assert!(decompress(&data).is_err());
    }

    #[test]
    fn rejects_a_truncated_header() {
        assert!(decompress(&[0x78]).is_err());
    }
}
