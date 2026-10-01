// Decodeurs de formats de fichiers, ecrits de zero (aucune dependance
// externe, meme philosophie que le reste du moteur - rasterizer de police,
// protocole Wayland brut...) : `deflate`/`zlib` sont les briques generiques
// dont `png` a besoin pour ses donnees de pixels compressees.
pub mod deflate;
pub mod png;
pub mod zlib;
