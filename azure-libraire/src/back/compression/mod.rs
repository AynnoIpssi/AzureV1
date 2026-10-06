// Compression ecrite a la main, sans dependance.
//
// - `deflate::inflate` / `compresser::deflate` : le flux DEFLATE brut
//   (RFC 1951), dans les deux sens ;
// - `zlib` : DEFLATE dans son enveloppe zlib (PNG, objets Git) ;
// - `zip` : les archives (.zip, .xlsx, .docx), en lecture et en ecriture.
pub mod compresser;
pub mod deflate;
pub mod zip;
pub mod zlib;

pub use compresser::deflate as compresser;
pub use deflate::inflate as decompresser;
