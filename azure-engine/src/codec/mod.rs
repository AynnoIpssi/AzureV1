// Decodeurs de formats de fichiers, ecrits de zero (aucune dependance
// externe, meme philosophie que le reste du moteur - rasterizer de police,
// protocole Wayland brut...). `deflate`/`zlib`, les briques generiques dont
// `png` a besoin pour ses donnees de pixels compressees, vivent dans la
// librairie (azure-libraire, `back::compression`).
pub mod png;
pub use azure_libraire::back::compression::{deflate, zlib};
