// Des octets en chiffres hexadecimaux, et retour.

/// En minuscules : `[0xCA, 0xFE]` -> "cafe".
pub fn hexa(octets: &[u8]) -> String {
    const CHIFFRES: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(octets.len() * 2);
    for o in octets {
        out.push(CHIFFRES[(o >> 4) as usize] as char);
        out.push(CHIFFRES[(o & 15) as usize] as char);
    }
    out
}

/// Minuscules ou majuscules ; `None` si la longueur est impaire ou si un
/// caractere n'est pas un chiffre hexadecimal.
pub fn lire(texte: &str) -> Option<Vec<u8>> {
    let t = texte.as_bytes();
    if t.len() % 2 != 0 {
        return None;
    }
    let chiffre = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    t.chunks(2).map(|p| Some(chiffre(p[0])? << 4 | chiffre(p[1])?)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour() {
        assert_eq!(hexa(&[0xCA, 0xFE, 0x00, 0x0f]), "cafe000f");
        assert_eq!(lire("CAfe000f").unwrap(), [0xCA, 0xFE, 0x00, 0x0f]);
        assert_eq!(lire(""), Some(Vec::new()));
        assert_eq!(lire("abc"), None);
        assert_eq!(lire("zz"), None);
    }
}
