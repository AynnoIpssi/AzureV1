// Passer d'une ecriture a une autre : octets <-> texte.
//
// - `hexa` : chiffres hexadecimaux ;
// - `base64` ;
// - `texte` : le texte d'un fichier d'encodage inconnu (UTF-8, UTF-16,
//   Windows-1252) et les accents retires.
pub mod base64;
pub mod hexa;
pub mod texte;
