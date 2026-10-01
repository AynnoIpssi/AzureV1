// Decodeur PNG ecrit de zero (chunks + filtres de scanline), au-dessus de
// `codec::zlib` pour les donnees de pixels compressees (chunks IDAT). Perimetre
// volontairement restreint a ce qui couvre l'immense majorite des images
// d'interface (logos/icones exportes par un outil de design courant) sans
// exploser la taille de l'implementation :
//   - profondeur 8 bits uniquement (pas 1/2/4/16 bits par canal)
//   - type couleur RGB (2) ou RGBA (6) uniquement (pas niveaux de gris, pas
//     palette indexee)
//   - pas d'entrelacement Adam7 (`interlace method` doit etre 0)
//   - CRC de chunk ignore (pas verifie) : une corruption serait de toute
//     facon detectee par l'Adler-32 de `codec::zlib` ou par un decodage qui
//     echoue plus loin, verifier le CRC en plus n'ajoute pas de garantie
//     utile ici pour la taille de code que ca coute.
// Toute image hors de ce perimetre echoue avec une erreur explicite plutot
// que d'etre mal decodee silencieusement.
use crate::codec::zlib;

const SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Une image decodee en memoire : RGBA, 4 octets par pixel, alpha "straight"
/// (pas pre-multipliee) - a composer sur un `Canvas` via
/// `rendering::services::image::draw_image` (qui fait, lui, l'alpha
/// blending).
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ColorType {
    Rgb,
    Rgba,
}

impl ColorType {
    fn from_byte(b: u8) -> Result<ColorType, String> {
        match b {
            2 => Ok(ColorType::Rgb),
            6 => Ok(ColorType::Rgba),
            0 => Err("PNG: type couleur 0 (niveaux de gris) non supporte - seuls RGB et RGBA le sont".to_string()),
            3 => Err("PNG: type couleur 3 (palette indexee) non supporte - seuls RGB et RGBA le sont".to_string()),
            4 => Err("PNG: type couleur 4 (niveaux de gris + alpha) non supporte - seuls RGB et RGBA le sont".to_string()),
            other => Err(format!("PNG: type couleur {other} inconnu")),
        }
    }

    fn channels(self) -> usize {
        match self {
            ColorType::Rgb => 3,
            ColorType::Rgba => 4,
        }
    }
}

struct Header {
    width: u32,
    height: u32,
    color_type: ColorType,
}

fn parse_header(data: &[u8]) -> Result<Header, String> {
    if data.len() < 13 {
        return Err("PNG: chunk IHDR trop court".to_string());
    }
    let width = u32::from_be_bytes(data[0..4].try_into().unwrap());
    let height = u32::from_be_bytes(data[4..8].try_into().unwrap());
    let bit_depth = data[8];
    let color_type = ColorType::from_byte(data[9])?;
    let compression_method = data[10];
    let filter_method = data[11];
    let interlace_method = data[12];

    if bit_depth != 8 {
        return Err(format!("PNG: profondeur {bit_depth} bits non supportee (seule 8 bits l'est)"));
    }
    if compression_method != 0 {
        return Err("PNG: methode de compression non standard".to_string());
    }
    if filter_method != 0 {
        return Err("PNG: methode de filtrage non standard".to_string());
    }
    if interlace_method != 0 {
        return Err("PNG: entrelacement Adam7 non supporte".to_string());
    }
    if width == 0 || height == 0 {
        return Err("PNG: dimensions nulles".to_string());
    }

    Ok(Header { width, height, color_type })
}

// Le predicteur Paeth (RFC 2083 section 6.6) : choisit, parmi les trois
// voisins deja decodes (gauche, haut, haut-gauche), celui dont la
// combinaison lineaire predit le mieux la valeur du pixel courant.
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let (a, b, c) = (a as i32, b as i32, c as i32);
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

// Annule le filtre applique a CHAQUE scanline avant compression (RFC 2083
// section 6) : `raw` est mutee en place, `prev` est la ligne precedente
// DEJA reconstituee (toute a zero pour la toute premiere ligne, comme le
// veut la specification). `bpp` est la taille d'un pixel complet en octets
// (3 pour RGB, 4 pour RGBA a 8 bits) - c'est la distance "vers la gauche"
// que les filtres Sub/Average/Paeth regardent, PAS forcement 1 octet.
fn unfilter_scanline(filter_type: u8, raw: &mut [u8], prev: &[u8], bpp: usize) -> Result<(), String> {
    match filter_type {
        0 => {} // None : rien a faire.
        1 => {
            // Sub : chaque octet + celui `bpp` octets a sa gauche sur la MEME ligne.
            for i in bpp..raw.len() {
                raw[i] = raw[i].wrapping_add(raw[i - bpp]);
            }
        }
        2 => {
            // Up : chaque octet + celui a la meme colonne sur la ligne precedente.
            for i in 0..raw.len() {
                raw[i] = raw[i].wrapping_add(prev[i]);
            }
        }
        3 => {
            // Average : moyenne (division entiere, sans arrondi) du voisin
            // de gauche et de celui du dessus.
            for i in 0..raw.len() {
                let left = if i >= bpp { raw[i - bpp] as u32 } else { 0 };
                let up = prev[i] as u32;
                raw[i] = raw[i].wrapping_add(((left + up) / 2) as u8);
            }
        }
        4 => {
            for i in 0..raw.len() {
                let left = if i >= bpp { raw[i - bpp] } else { 0 };
                let up = prev[i];
                let up_left = if i >= bpp { prev[i - bpp] } else { 0 };
                raw[i] = raw[i].wrapping_add(paeth_predictor(left, up, up_left));
            }
        }
        other => return Err(format!("PNG: type de filtre de scanline {other} inconnu")),
    }
    Ok(())
}

/// Decode un fichier PNG complet (les octets bruts du fichier, tels que lus
/// depuis le disque) en pixels RGBA - voir la doc du module pour le
/// perimetre exact supporte.
pub fn decode(data: &[u8]) -> Result<DecodedImage, String> {
    if data.len() < 8 || data[..8] != SIGNATURE {
        return Err("PNG: signature de fichier invalide".to_string());
    }

    let mut header: Option<Header> = None;
    let mut idat: Vec<u8> = Vec::new();
    let mut pos = 8usize;

    while pos + 8 <= data.len() {
        let length = u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        let chunk_type = &data[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = data_start.checked_add(length).ok_or("PNG: longueur de chunk invalide")?;
        if data_end > data.len() {
            return Err("PNG: chunk tronque".to_string());
        }
        let chunk_data = &data[data_start..data_end];

        match chunk_type {
            b"IHDR" => header = Some(parse_header(chunk_data)?),
            // Plusieurs IDAT consecutifs forment un seul flux zlib decoupe
            // arbitrairement par l'encodeur : on les concatene avant de
            // decompresser quoi que ce soit.
            b"IDAT" => idat.extend_from_slice(chunk_data),
            b"IEND" => break,
            // Chunks ancillaires (gAMA, sRGB, tRNS, tEXt, pHYs...) : ignores
            // - les couleurs sont prises telles quelles, sans correction de
            // gamma ni gestion de la transparence par cle de couleur.
            _ => {}
        }

        // +4 pour le CRC de fin de chunk, jamais lu (voir la doc du module).
        pos = data_end + 4;
    }

    let header = header.ok_or("PNG: chunk IHDR manquant")?;
    if idat.is_empty() {
        return Err("PNG: aucune donnee de pixel (chunk IDAT manquant)".to_string());
    }

    let raw = zlib::decompress(&idat)?;

    let channels = header.color_type.channels();
    let bpp = channels; // 8 bits/canal : bpp en octets = nombre de canaux.
    let stride = header.width as usize * channels;
    let expected_len = (stride + 1) * header.height as usize; // +1 : octet de type de filtre en tete de chaque ligne.
    if raw.len() < expected_len {
        return Err("PNG: donnees de pixel decompressees plus courtes qu'attendu".to_string());
    }

    let mut pixels = Vec::with_capacity(header.width as usize * header.height as usize * 4);
    let mut prev_line = vec![0u8; stride];

    for y in 0..header.height as usize {
        let line_start = y * (stride + 1);
        let filter_type = raw[line_start];
        let mut line = raw[line_start + 1..line_start + 1 + stride].to_vec();
        unfilter_scanline(filter_type, &mut line, &prev_line, bpp)?;

        for chunk in line.chunks(channels) {
            match header.color_type {
                ColorType::Rgb => pixels.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]),
                ColorType::Rgba => pixels.extend_from_slice(&[chunk[0], chunk[1], chunk[2], chunk[3]]),
            }
        }

        prev_line = line;
    }

    Ok(DecodedImage { width: header.width, height: header.height, pixels })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures produites par Pillow (voir `codec/fixtures/`) - un vrai
    // encodeur PNG standard, donc un test contre ce que produit
    // effectivement un outil dans la nature, pas seulement contre ma propre
    // comprehension du format. `rgba_3x2.png` compresse avec un bloc
    // DEFLATE a Huffman DYNAMIQUE, `rgb_4x4.png` avec un bloc a Huffman
    // FIXE - a elles deux, les deux chemins de decodage Huffman de
    // `codec::deflate` sont exerces par un flux reel, pas seulement par les
    // tests unitaires de ce module-la.
    const RGBA_3X2: &[u8] = include_bytes!("fixtures/rgba_3x2.png");
    const RGB_4X4: &[u8] = include_bytes!("fixtures/rgb_4x4.png");

    #[test]
    fn decodes_a_real_rgba_png_pixel_for_pixel() {
        let img = decode(RGBA_3X2).expect("decodage reussi");
        assert_eq!((img.width, img.height), (3, 2));
        let expected: [(u8, u8, u8, u8); 6] = [
            (255, 0, 0, 255),
            (0, 255, 0, 128),
            (0, 0, 255, 0),
            (255, 255, 0, 255),
            (0, 255, 255, 64),
            (255, 0, 255, 200),
        ];
        for (i, &(r, g, b, a)) in expected.iter().enumerate() {
            let px = &img.pixels[i * 4..i * 4 + 4];
            assert_eq!(px, [r, g, b, a], "pixel {i}");
        }
    }

    #[test]
    fn decodes_a_real_rgb_png_as_fully_opaque() {
        let img = decode(RGB_4X4).expect("decodage reussi");
        assert_eq!((img.width, img.height), (4, 4));
        assert_eq!(img.pixels.len(), 4 * 4 * 4);
        for chunk in img.pixels.chunks(4) {
            assert_eq!(chunk[3], 255, "une image RGB (sans canal alpha) doit toujours donner alpha=255");
        }
        // Premier pixel (x=0, y=0) : (0, 0, 0) d'apres le script de
        // generation de la fixture.
        assert_eq!(&img.pixels[0..4], &[0, 0, 0, 255]);
        // Pixel (x=2, y=1) : ((2*60)%256, (1*60)%256, ((2+1)*30)%256) = (120, 60, 90).
        let idx = (4 + 2) * 4;
        assert_eq!(&img.pixels[idx..idx + 4], &[120, 60, 90, 255]);
    }

    #[test]
    fn rejects_a_file_without_the_png_signature() {
        assert!(decode(b"not a png").is_err());
    }

    #[test]
    fn rejects_a_signature_only_file_with_no_chunks() {
        assert!(decode(&SIGNATURE).is_err());
    }
}
