// Poly1305 (RFC 8439, section 2.5), ecrit a la main : arithmetique modulo
// 2^130 - 5 sur 5 "membres" de 26 bits (methode dite "donna 32 bits").

const MASK: u32 = 0x3ff_ffff;

fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub fn poly1305(key: &[u8; 32], message: &[u8]) -> [u8; 16] {
    let r0 = le32(&key[0..]) & 0x3ff_ffff;
    let r1 = (le32(&key[3..]) >> 2) & 0x3ff_ff03;
    let r2 = (le32(&key[6..]) >> 4) & 0x3ff_c0ff;
    let r3 = (le32(&key[9..]) >> 6) & 0x3f0_3fff;
    let r4 = (le32(&key[12..]) >> 8) & 0x00f_ffff;
    let (s1, s2, s3, s4) = (r1 * 5, r2 * 5, r3 * 5, r4 * 5);
    let (mut h0, mut h1, mut h2, mut h3, mut h4) = (0u32, 0u32, 0u32, 0u32, 0u32);

    for chunk in message.chunks(16) {
        let mut block = [0u8; 17];
        block[..chunk.len()].copy_from_slice(chunk);
        // Bloc complet : le bit 2^128 est ajoute. Dernier bloc partiel : un
        // octet 0x01 juste apres le message, puis des zeros.
        let hibit = if chunk.len() == 16 { 1 << 24 } else {
            block[chunk.len()] = 1;
            0
        };
        h0 += le32(&block[0..]) & MASK;
        h1 += (le32(&block[3..]) >> 2) & MASK;
        h2 += (le32(&block[6..]) >> 4) & MASK;
        h3 += (le32(&block[9..]) >> 6) & MASK;
        h4 += (le32(&block[12..]) >> 8) | hibit;

        let m = |a: u32, b: u32| a as u64 * b as u64;
        let d0 = m(h0, r0) + m(h1, s4) + m(h2, s3) + m(h3, s2) + m(h4, s1);
        let mut d1 = m(h0, r1) + m(h1, r0) + m(h2, s4) + m(h3, s3) + m(h4, s2);
        let mut d2 = m(h0, r2) + m(h1, r1) + m(h2, r0) + m(h3, s4) + m(h4, s3);
        let mut d3 = m(h0, r3) + m(h1, r2) + m(h2, r1) + m(h3, r0) + m(h4, s4);
        let mut d4 = m(h0, r4) + m(h1, r3) + m(h2, r2) + m(h3, r1) + m(h4, r0);

        let mut c = d0 >> 26;
        h0 = d0 as u32 & MASK;
        d1 += c;
        c = d1 >> 26;
        h1 = d1 as u32 & MASK;
        d2 += c;
        c = d2 >> 26;
        h2 = d2 as u32 & MASK;
        d3 += c;
        c = d3 >> 26;
        h3 = d3 as u32 & MASK;
        d4 += c;
        c = d4 >> 26;
        h4 = d4 as u32 & MASK;
        h0 += c as u32 * 5;
        let c = h0 >> 26;
        h0 &= MASK;
        h1 += c;
    }

    // Reduction complete modulo 2^130 - 5.
    let mut c = h1 >> 26;
    h1 &= MASK;
    h2 += c;
    c = h2 >> 26;
    h2 &= MASK;
    h3 += c;
    c = h3 >> 26;
    h3 &= MASK;
    h4 += c;
    c = h4 >> 26;
    h4 &= MASK;
    h0 += c * 5;
    c = h0 >> 26;
    h0 &= MASK;
    h1 += c;

    let mut g0 = h0.wrapping_add(5);
    c = g0 >> 26;
    g0 &= MASK;
    let mut g1 = h1.wrapping_add(c);
    c = g1 >> 26;
    g1 &= MASK;
    let mut g2 = h2.wrapping_add(c);
    c = g2 >> 26;
    g2 &= MASK;
    let mut g3 = h3.wrapping_add(c);
    c = g3 >> 26;
    g3 &= MASK;
    let g4 = h4.wrapping_add(c).wrapping_sub(1 << 26);

    // g = h - p si h >= p, sans branche (temps constant).
    let select_g = (g4 >> 31).wrapping_sub(1);
    let keep_h = !select_g;
    h0 = (h0 & keep_h) | (g0 & select_g);
    h1 = (h1 & keep_h) | (g1 & select_g);
    h2 = (h2 & keep_h) | (g2 & select_g);
    h3 = (h3 & keep_h) | (g3 & select_g);
    h4 = (h4 & keep_h) | (g4 & select_g);

    let w0 = h0 | (h1 << 26);
    let w1 = (h1 >> 6) | (h2 << 20);
    let w2 = (h2 >> 12) | (h3 << 14);
    let w3 = (h3 >> 18) | (h4 << 8);

    let mut out = [0u8; 16];
    let mut carry = 0u64;
    for (i, word) in [w0, w1, w2, w3].into_iter().enumerate() {
        let f = word as u64 + le32(&key[16 + 4 * i..]) as u64 + carry;
        out[4 * i..4 * i + 4].copy_from_slice(&(f as u32).to_le_bytes());
        carry = f >> 32;
    }
    out
}
