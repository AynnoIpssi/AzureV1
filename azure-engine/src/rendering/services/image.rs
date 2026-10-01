// Chargement (avec cache) et composition d'images PNG decodees par
// `codec::png` sur un `Canvas` - l'equivalent, pour les images, de ce que
// `text::glyph`/`text::renderer` font pour les glyphes de police.
use crate::codec::png::{self, DecodedImage};
use crate::rendering::models::canvas::Canvas;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// Decoder un PNG (inflate + reconstruction des scanlines) est trop couteux
// pour le refaire a chaque redessin (potentiellement 60 fois par seconde,
// voir `window::models::window`) alors que le fichier ne change jamais en
// cours de session - mis en cache par chemin, comme `GLYPH_CACHE` pour les
// glyphes. Le resultat (succes ou erreur) est mis en cache tel quel : un
// chemin invalide ne re-tente pas une lecture disque a chaque frame non
// plus, il retourne la meme erreur immediatement.
//
// `Rc`, pas `Arc` : cette boucle de rendu est mono-thread (une seule
// fenetre Wayland, voir `platform::wayland::managers::surface_manager`),
// inutile de payer le cout atomique d'un `Arc` pour partager le buffer de
// pixels entre le cache et chaque appelant.
thread_local! {
    static IMAGE_CACHE: RefCell<HashMap<String, Result<Rc<DecodedImage>, String>>> = RefCell::new(HashMap::new());
}

/// Charge et decode le PNG a `path` (relatif au repertoire de travail,
/// comme les chemins de police - voir `text::loader::load_font`), en
/// passant par le cache. Cloner un `Rc` est une simple incrementation de
/// compteur de references, pas une copie du tampon de pixels.
pub fn load_png(path: &str) -> Result<Rc<DecodedImage>, String> {
    if let Some(cached) = IMAGE_CACHE.with(|cache| cache.borrow().get(path).cloned()) {
        return cached;
    }

    let result = std::fs::read(path)
        .map_err(|e| format!("impossible de lire {path}: {e}"))
        .and_then(|bytes| png::decode(&bytes))
        .map(Rc::new);

    IMAGE_CACHE.with(|cache| cache.borrow_mut().insert(path.to_string(), result.clone()));
    result
}

/// Compose `pixels` (RGBA, `img_width * img_height * 4` octets - voir
/// `codec::png::DecodedImage`) sur `canvas` a `(x, y)`, a sa taille
/// naturelle (pas de mise a l'echelle : l'equivalent d'un CSS
/// `object-fit: none`, un agrandissement/retrecissement reste a faire plus
/// tard si besoin). Alpha-blende chaque pixel avec ce qu'il y a deja sous
/// lui (contrairement a `rect::draw_rect`, qui ecrase directement) : une
/// image avec de la transparence (un logo au fond transparent, typiquement)
/// doit laisser voir le fond du conteneur au travers, pas peindre du noir.
/// Ne deborde jamais de la zone de decoupage active (voir `Canvas::set_clip`)
/// ni des bords du canvas.
pub fn draw_image(pixels: &[u8], img_width: u32, img_height: u32, x: u32, y: u32, canvas: &mut Canvas) {
    draw_image_scaled(pixels, img_width, img_height, x, y, img_width, img_height, canvas);
}

/// Comme `draw_image`, mais compose `pixels` a la taille `(dst_width,
/// dst_height)` plutot qu'a sa taille naturelle - echantillonnage au plus
/// proche voisin (pas d'interpolation bilineaire) : suffisant pour une
/// icone d'application reduite a la taille d'une petite vignette (voir
/// `window::services::draw_header`), pas destine a un redimensionnement
/// de grande photo ou l'aliasing se verrait. `draw_image` est le cas
/// particulier `dst_width == img_width && dst_height == img_height`
/// (l'echantillonnage au plus proche voisin retombe alors exactement sur
/// le pixel source correspondant, sans aucune distorsion).
#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
pub fn draw_image_scaled(pixels: &[u8], img_width: u32, img_height: u32, x: u32, y: u32, dst_width: u32, dst_height: u32, canvas: &mut Canvas) {
    if img_width == 0 || img_height == 0 || dst_width == 0 || dst_height == 0 {
        return;
    }
    let (clip_x, clip_y, clip_w, clip_h) = canvas.clip_bounds();
    let clip_x_end = clip_x + clip_w;
    let clip_y_end = clip_y + clip_h;

    for row in 0..dst_height {
        let py = y + row;
        if py < clip_y || py >= clip_y_end || py >= canvas.height {
            continue;
        }
        let src_row = (row * img_height / dst_height).min(img_height - 1);
        for col in 0..dst_width {
            let px = x + col;
            if px < clip_x || px >= clip_x_end || px >= canvas.width {
                continue;
            }
            let src_col = (col * img_width / dst_width).min(img_width - 1);

            let src_idx = ((src_row * img_width + src_col) * 4) as usize;
            let a = pixels[src_idx + 3];
            if a == 0 {
                continue;
            }
            let (r, g, b) = (pixels[src_idx], pixels[src_idx + 1], pixels[src_idx + 2]);

            let dst_idx = ((py * canvas.width + px) * 4) as usize;
            if a == 255 {
                canvas.buffer[dst_idx] = b;
                canvas.buffer[dst_idx + 1] = g;
                canvas.buffer[dst_idx + 2] = r;
                canvas.buffer[dst_idx + 3] = 255;
            } else {
                let af = a as u32;
                let inv = 255 - af;
                canvas.buffer[dst_idx] = ((b as u32 * af + canvas.buffer[dst_idx] as u32 * inv) / 255) as u8;
                canvas.buffer[dst_idx + 1] = ((g as u32 * af + canvas.buffer[dst_idx + 1] as u32 * inv) / 255) as u8;
                canvas.buffer[dst_idx + 2] = ((r as u32 * af + canvas.buffer[dst_idx + 2] as u32 * inv) / 255) as u8;
                canvas.buffer[dst_idx + 3] = 255;
            }
        }
    }
}

/// Comme `draw_image_scaled`, a une position SIGNEE : l'image peut depasser
/// a gauche ou en haut (fond `cover` plus grand que sa boite) ; seule la
/// partie dans la zone de decoupage est peinte.
#[allow(clippy::too_many_arguments)]
pub fn draw_image_at(pixels: &[u8], img_width: u32, img_height: u32, x: i32, y: i32, dst_width: u32, dst_height: u32, canvas: &mut Canvas) {
    if img_width == 0 || img_height == 0 || dst_width == 0 || dst_height == 0 {
        return;
    }
    let (clip_x, clip_y, clip_w, clip_h) = canvas.clip_bounds();
    let x0 = x.max(clip_x as i32).max(0);
    let y0 = y.max(clip_y as i32).max(0);
    let x1 = (x + dst_width as i32).min((clip_x + clip_w) as i32).min(canvas.width as i32);
    let y1 = (y + dst_height as i32).min((clip_y + clip_h) as i32).min(canvas.height as i32);
    for py in y0..y1 {
        let src_row = (((py - y) as u64 * img_height as u64 / dst_height as u64) as u32).min(img_height - 1);
        for px in x0..x1 {
            let src_col = (((px - x) as u64 * img_width as u64 / dst_width as u64) as u32).min(img_width - 1);
            let src = ((src_row * img_width + src_col) * 4) as usize;
            let a = pixels[src + 3] as u32;
            if a == 0 {
                continue;
            }
            let dst = ((py as u32 * canvas.width + px as u32) * 4) as usize;
            let inv = 255 - a;
            for (c, s) in [(0usize, 2usize), (1, 1), (2, 0)] {
                canvas.buffer[dst + c] = ((pixels[src + s] as u32 * a + canvas.buffer[dst + c] as u32 * inv) / 255) as u8;
            }
            canvas.buffer[dst + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opaque_2x1_red_green() -> Vec<u8> {
        vec![255, 0, 0, 255, 0, 255, 0, 255]
    }

    #[test]
    fn draws_an_opaque_image_pixel_for_pixel() {
        let mut canvas = Canvas::new(4, 4);
        draw_image(&opaque_2x1_red_green(), 2, 1, 1, 1, &mut canvas);

        let idx = |x: u32, y: u32| ((y * 4 + x) * 4) as usize;
        // BGRA dans le buffer (voir Canvas/draw_rect) : rouge -> (0,0,255,255).
        assert_eq!(&canvas.buffer[idx(1, 1)..idx(1, 1) + 4], &[0, 0, 255, 255]);
        assert_eq!(&canvas.buffer[idx(2, 1)..idx(2, 1) + 4], &[0, 255, 0, 255]);
    }

    #[test]
    fn fully_transparent_pixels_leave_the_background_untouched() {
        let mut canvas = Canvas::new(2, 2);
        canvas.buffer.copy_from_slice(&[9, 9, 9, 255, 9, 9, 9, 255, 9, 9, 9, 255, 9, 9, 9, 255]);

        let transparent_pixel = vec![255, 0, 0, 0]; // rouge, alpha 0
        draw_image(&transparent_pixel, 1, 1, 0, 0, &mut canvas);

        assert_eq!(&canvas.buffer[0..4], &[9, 9, 9, 255]);
    }

    #[test]
    fn half_transparent_pixel_blends_with_the_background() {
        let mut canvas = Canvas::new(1, 1);
        canvas.buffer.copy_from_slice(&[0, 0, 0, 255]); // fond noir opaque

        let half_red = vec![255, 0, 0, 128]; // rouge, alpha ~50%
        draw_image(&half_red, 1, 1, 0, 0, &mut canvas);

        // ~50% rouge sur fond noir -> rouge attenue, pas rouge pur ni noir.
        let r = canvas.buffer[2];
        assert!(r > 100 && r < 155, "r = {r}, attendu entre 100 et 155");
        assert_eq!(canvas.buffer[3], 255);
    }

    #[test]
    fn never_draws_outside_the_active_clip() {
        let mut canvas = Canvas::new(4, 4);
        canvas.set_clip(0, 0, 2, 2);
        let pixels = vec![255u8; 4 * 4 * 4]; // 4x4 blanc opaque
        draw_image(&pixels, 4, 4, 0, 0, &mut canvas);

        let idx = |x: u32, y: u32| ((y * 4 + x) * 4) as usize;
        assert_eq!(&canvas.buffer[idx(1, 1)..idx(1, 1) + 4], &[255, 255, 255, 255]);
        // (3,3) est hors du clip (2x2) : doit rester a zero (fond par defaut de Canvas::new).
        assert_eq!(&canvas.buffer[idx(3, 3)..idx(3, 3) + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn draw_image_is_the_identity_scale_of_draw_image_scaled() {
        let mut a = Canvas::new(4, 4);
        let mut b = Canvas::new(4, 4);
        let pixels = opaque_2x1_red_green();
        draw_image(&pixels, 2, 1, 1, 1, &mut a);
        draw_image_scaled(&pixels, 2, 1, 1, 1, 2, 1, &mut b);
        assert_eq!(a.buffer, b.buffer);
    }

    #[test]
    fn scaling_down_keeps_every_destination_pixel_a_real_source_color() {
        // Image 4x1 rouge/vert/bleu/blanc, reduite a 2x1 : chaque pixel de
        // sortie doit rester une des 4 couleurs sources (plus proche
        // voisin), jamais une couleur inventee par interpolation.
        let pixels = vec![
            255, 0, 0, 255, // rouge
            0, 255, 0, 255, // vert
            0, 0, 255, 255, // bleu
            255, 255, 255, 255, // blanc
        ];
        let mut canvas = Canvas::new(2, 1);
        draw_image_scaled(&pixels, 4, 1, 0, 0, 2, 1, &mut canvas);

        // BGRA des 4 couleurs sources (rouge/vert/bleu/blanc) - le point du
        // test n'est pas de deviner quels indices le plus proche voisin
        // choisit, juste qu'il ne peut jamais produire une couleur qui
        // n'existe pas parmi elles (une interpolation le pourrait).
        let known_colors = [[0, 0, 255, 255], [0, 255, 0, 255], [255, 0, 0, 255], [255, 255, 255, 255]];
        for chunk in canvas.buffer.chunks(4) {
            assert!(known_colors.contains(&[chunk[0], chunk[1], chunk[2], chunk[3]]), "couleur inattendue: {chunk:?}");
        }
    }

    #[test]
    fn scaling_up_stretches_without_leaving_any_pixel_untouched() {
        let pixels = vec![255u8, 0, 0, 255]; // 1x1 rouge opaque
        let mut canvas = Canvas::new(3, 3);
        draw_image_scaled(&pixels, 1, 1, 0, 0, 3, 3, &mut canvas);

        for chunk in canvas.buffer.chunks(4) {
            assert_eq!(chunk, &[0, 0, 255, 255]);
        }
    }
}
