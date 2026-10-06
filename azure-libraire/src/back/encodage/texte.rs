// Le texte d'un fichier dont on ne connait pas l'encodage, et les
// accents retires pour comparer des noms.

/// Le texte d'un fichier : UTF-8 (avec ou sans marque), UTF-16 (« texte
/// Unicode » d'Excel), sinon Windows-1252 (les vieux exports d'Excel).
pub fn decoder(octets: &[u8]) -> String {
    if let Some(reste) = octets.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(reste).into_owned();
    }
    for (marque, grand) in [([0xFF, 0xFE], false), ([0xFE, 0xFF], true)] {
        if let Some(reste) = octets.strip_prefix(&marque) {
            let unites: Vec<u16> = reste.chunks_exact(2).map(|p| if grand { u16::from_be_bytes([p[0], p[1]]) } else { u16::from_le_bytes([p[0], p[1]]) }).collect();
            return String::from_utf16_lossy(&unites);
        }
    }
    match std::str::from_utf8(octets) {
        Ok(t) => t.to_string(),
        Err(_) => octets.iter().map(|&o| windows_1252(o)).collect(),
    }
}

fn windows_1252(o: u8) -> char {
    const HAUT: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    match o {
        0x80..=0x9F => HAUT[(o - 0x80) as usize],
        _ => o as char,
    }
}

/// Le separateur du texte : celui qui revient autant de fois sur chacune

pub fn sans_accent(texte: &str) -> String {
    texte
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'À' | 'Â' | 'Ä' | 'Á' | 'Ã' => 'A',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'î' | 'ï' | 'í' => 'i',
            'Î' | 'Ï' | 'Í' => 'I',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'Ô' | 'Ö' | 'Ó' | 'Õ' => 'O',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'Ù' | 'Û' | 'Ü' | 'Ú' => 'U',
            'ç' => 'c',
            'Ç' => 'C',
            'ñ' => 'n',
            'Ñ' => 'N',
            'ÿ' => 'y',
            autre => autre,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_encodages_se_reconnaissent() {
        assert_eq!(decoder(b"\xEF\xBB\xBFa;\xC3\xA9"), "a;é");
        assert_eq!(decoder("déjà".as_bytes()), "déjà");
        assert_eq!(decoder(b"caf\xE9;5\x80"), "café;5€");
        assert_eq!(decoder(&[0xFF, 0xFE, b'a', 0, 0xE9, 0]), "aé");
        assert_eq!(decoder(&[0xFE, 0xFF, 0, b'a', 0, 0xE9]), "aé");
    }

    #[test]
    fn les_accents_se_retirent() {
        assert_eq!(sans_accent("Prénom Çà et là"), "Prenom Ca et la");
    }
}
