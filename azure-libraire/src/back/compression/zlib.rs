// L'enveloppe zlib (RFC 1950) autour d'un flux DEFLATE brut - c'est ce
// format, pas DEFLATE nu, que les chunks IDAT d'un PNG contiennent (voir
// `codec::png`). Deux octets d'en-tete, le flux DEFLATE, puis un Adler-32
// des donnees DECOMPRESSEES pour detecter une corruption.
use super::compresser::deflate;
use super::deflate::inflate;
use crate::back::hachage::adler32::adler32;

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

/// Compresse `data` en un flux zlib complet (en-tete, DEFLATE, Adler-32).
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x9C];
    out.extend(deflate(data));
    out.extend(adler32(data).to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_then_decompress_gives_the_data_back() {
        for data in [&b""[..], b"a", b"abcabcabcabcabcabc", &[7u8; 5000]] {
            assert_eq!(decompress(&compress(data)).unwrap(), data);
        }
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
