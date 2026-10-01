// ChaCha20-Poly1305 (RFC 8439, section 2.8) : chiffre ET authentifie. Une
// donnee modifiee sur le disque, ou dechiffree avec la mauvaise cle ou le
// mauvais contexte (`aad`), est refusee au lieu de rendre n'importe quoi.
use crate::crypto::chacha20;
use crate::crypto::hmac::constant_time_eq;
use crate::crypto::poly1305::poly1305;
use crate::crypto::random::random_bytes;

pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;

fn tag(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], ciphertext: &[u8]) -> [u8; 16] {
    let mut otk = [0u8; 32];
    otk.copy_from_slice(&chacha20::block(key, 0, nonce)[..32]);
    let pad = |len: usize| vec![0u8; (16 - len % 16) % 16];
    let mut mac_data = aad.to_vec();
    mac_data.extend(pad(aad.len()));
    mac_data.extend_from_slice(ciphertext);
    mac_data.extend(pad(ciphertext.len()));
    mac_data.extend_from_slice(&(aad.len() as u64).to_le_bytes());
    mac_data.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());
    poly1305(&otk, &mac_data)
}

/// Chiffrement RFC 8439 avec un nonce donne : rend `ciphertext || tag`.
pub fn encrypt(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let mut out = plaintext.to_vec();
    chacha20::apply(key, 1, nonce, &mut out);
    let tag = tag(key, nonce, aad, &out);
    out.extend_from_slice(&tag);
    out
}

/// L'inverse d'`encrypt` ; `None` si le tag ne correspond pas.
pub fn decrypt(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], sealed: &[u8]) -> Option<Vec<u8>> {
    let split = sealed.len().checked_sub(TAG_LEN)?;
    let (ciphertext, received) = sealed.split_at(split);
    if !constant_time_eq(&tag(key, nonce, aad, ciphertext), received) {
        return None;
    }
    let mut out = ciphertext.to_vec();
    chacha20::apply(key, 1, nonce, &mut out);
    Some(out)
}

/// Chiffre avec un nonce aleatoire neuf : rend `nonce || ciphertext || tag`.
pub fn seal(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; NONCE_LEN];
    random_bytes(&mut nonce)?;
    let mut out = nonce.to_vec();
    out.extend(encrypt(key, &nonce, aad, plaintext));
    Ok(out)
}

/// L'inverse de `seal`.
pub fn open(key: &[u8; 32], aad: &[u8], sealed: &[u8]) -> Result<Vec<u8>, String> {
    if sealed.len() < NONCE_LEN + TAG_LEN {
        return Err("Donnee chiffree tronquee".to_string());
    }
    let (nonce, rest) = sealed.split_at(NONCE_LEN);
    let nonce: [u8; 12] = nonce.try_into().map_err(|_| "Nonce invalide".to_string())?;
    decrypt(key, &nonce, aad, rest).ok_or_else(|| "Donnee chiffree alteree, ou mauvaise cle".to_string())
}
