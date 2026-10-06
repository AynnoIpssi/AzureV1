// Base64 (RFC 4648, alphabet standard avec remplissage `=`).

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(octets: &[u8]) -> String {
    let mut out = String::new();
    for bloc in octets.chunks(3) {
        let n = (bloc[0] as u32) << 16 | (*bloc.get(1).unwrap_or(&0) as u32) << 8 | *bloc.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= bloc.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn base64_lire(texte: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut n, mut bits) = (0u32, 0);
    for c in texte.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        n = n << 6 | ALPHABET.iter().position(|a| *a == c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((n >> bits) as u8);
        }
    }
    Some(out)
}

// ---- RSA (cle publique seulement) ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour() {
        for (clair, code) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(clair.as_bytes()), code);
            assert_eq!(base64_lire(code).unwrap(), clair.as_bytes());
        }
        assert_eq!(base64_lire("Zm9v\nYmFy").unwrap(), b"foobar");
        assert_eq!(base64_lire("Zm9v*"), None);
    }
}
