// La crypto partagee par les daemons d'Azure. Elle vit dans la librairie
// (azure-libraire, `back::hachage` et `back::chiffrement`) ; ce module
// garde les chemins `azure_core::crypto::...` d'avant.
pub use azure_libraire::back::chiffrement::{aead, chacha20, poly1305, random};
pub use azure_libraire::back::hachage::{hmac, sha256};
