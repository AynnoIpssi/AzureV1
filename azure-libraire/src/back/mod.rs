// Les modules du back : la logique reutilisable qui ne dessine rien et ne
// parle a aucun daemon. Tout est ecrit a la main, sans dependance, et se
// teste seul (`cargo test -p azure-libraire`).
//
// - `hachage`     : SHA-256, HMAC, PBKDF2, SHA-1, MD5, CRC-32, Adler-32 ;
// - `chiffrement` : ChaCha20-Poly1305, octets aleatoires, RSA-OAEP ;
// - `compression` : DEFLATE, zlib, archives zip ;
// - `encodage`    : hexadecimal, base64, texte d'encodage inconnu ;
// - `temps`       : dates et heures sans fuseau ;
// - `hasard`      : hasard rejouable (graine) ;
// - `tableur`     : fichiers .csv / .tsv / .xlsx en tableau de cellules ;
// - `texte`       : comparer deux textes (diff) ;
// - `code`        : lire du code source (fonctions, appels, coloration).
//
// Reference : BACK.md.
pub mod chiffrement;
pub mod code;
pub mod compression;
pub mod encodage;
pub mod hachage;
pub mod hasard;
pub mod tableur;
pub mod temps;
pub mod texte;
