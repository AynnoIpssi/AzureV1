// Chiffrement ecrit a la main, verifie par les vecteurs officiels de
// chaque norme (azure-stockage/tests/crypto_vectors.rs). Non audite.
//
// - `aead` : ChaCha20-Poly1305 (`seal` / `open`), le chiffrement a
//   utiliser par defaut ; `chacha20` et `poly1305` sont ses deux briques.
// - `random` : octets aleatoires du noyau (cles, nonces, sels).
// - `rsa` : chiffrer pour une cle publique (RSA-OAEP).
pub mod aead;
pub mod chacha20;
pub mod poly1305;
pub mod random;
pub mod rsa;
