// Empreintes et sommes de controle, ecrites a la main et verifiees par les
// vecteurs de leur norme.
//
// - `sha256` (+ `hmac` : HMAC-SHA-256, PBKDF2, comparaison a temps
//   constant) : le hachage a utiliser par defaut.
// - `sha1`, `md5` : casses pour la securite, gardes pour les formats et
//   protocoles qui les imposent (Git, MySQL, PostgreSQL).
// - `crc32`, `adler32` : sommes de controle (zip / PNG, zlib), pas des
//   empreintes de securite.
pub mod adler32;
pub mod crc32;
pub mod hmac;
pub mod md5;
pub mod sha1;
pub mod sha256;
