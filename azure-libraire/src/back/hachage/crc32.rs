// CRC-32 (celui de zip, gzip et PNG : polynome 0xEDB88320).

const fn table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        t[n] = c;
        n += 1;
    }
    t
}

static TABLE: [u32; 256] = table();

pub fn crc32(data: &[u8]) -> u32 {
    suite(0, data)
}

/// Continue un CRC : `suite(crc32(a), b)` vaut `crc32(a + b)`.
pub fn suite(crc: u32, data: &[u8]) -> u32 {
    let mut c = !crc;
    for &octet in data {
        c = TABLE[((c ^ octet as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_valeurs_de_reference() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"The quick brown fox jumps over the lazy dog"), 0x414F_A339);
        assert_eq!(suite(crc32(b"1234"), b"56789"), crc32(b"123456789"));
    }
}
