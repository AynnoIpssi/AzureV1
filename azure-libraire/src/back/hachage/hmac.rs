// HMAC-SHA256 (RFC 2104) et PBKDF2-HMAC-SHA256 (RFC 8018), ecrits a la main.
use super::sha256::{sha256, Sha256};

pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    if key.len() > 64 {
        block[..32].copy_from_slice(&sha256(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(&block.map(|b| b ^ 0x36));
    inner.update(message);
    let mut outer = Sha256::new();
    outer.update(&block.map(|b| b ^ 0x5c));
    outer.update(&inner.finish());
    outer.finish()
}

/// Derive `out.len()` octets de `password` + `salt` (`iterations` tours) :
/// sert a stocker un mot de passe sans le garder en clair, et rend chaque
/// essai d'un attaquant `iterations` fois plus couteux.
pub fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32, out: &mut [u8]) {
    for (index, chunk) in out.chunks_mut(32).enumerate() {
        let mut first = salt.to_vec();
        first.extend_from_slice(&(index as u32 + 1).to_be_bytes());
        let mut u = hmac_sha256(password, &first);
        let mut t = u;
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for (acc, byte) in t.iter_mut().zip(u) {
                *acc ^= byte;
            }
        }
        chunk.copy_from_slice(&t[..chunk.len()]);
    }
}

/// Comparaison en temps constant (ne s'arrete pas au premier octet
/// different, pour ne rien laisser deviner par le temps de reponse).
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    // `black_box` : le compilateur ne peut pas en faire une boucle qui
    // s'arrete a la premiere difference.
    a.len() == b.len() && std::hint::black_box(a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | std::hint::black_box(x ^ y))) == 0
}
