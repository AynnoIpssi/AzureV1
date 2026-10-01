// Crypto ecrite a la main (aucune dependance), partagee par les daemons
// d'Azure. Chaque algorithme est verifie par les vecteurs officiels de sa
// norme (azure-stockage/tests/crypto_vectors.rs). Non auditee.
pub mod aead;
pub mod chacha20;
pub mod hmac;
pub mod poly1305;
pub mod random;
pub mod sha256;
