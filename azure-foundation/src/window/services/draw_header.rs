// Dessine la barre d'en-tete auto-geree (icone/titre/boutons) - voir
// `window::models::header_bar` pour la geometrie/le contenu, et
// `window::services::system_theme` pour d'ou viennent `header.dark`/
// `header.layout`.
use crate::window::models::header_bar::{self, HeaderBar, HeaderButton, HEADER_HEIGHT};
use azure_engine::rendering::managers::renderer::{draw_image_scaled, draw_rect, draw_text, measure_text_width};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::services::shapes::circle::draw_circle_filled;
use azure_engine::rendering::services::shapes::line::{draw_line, draw_line_horizontal};
use azure_engine::rendering::services::shapes::rect::draw_empty_rect;

// Meme police que le reste de l'UI (voir `ui::services::draw_ui::FONT_PATH`) -
// ancre au dossier de CE crate (resolu a la compilation), pas au dossier
// depuis lequel le binaire final est lance.
const FONT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../azure-engine/src/Sora-VariableFont_wght.ttf");
const TITLE_FONT_SIZE: f32 = 13.0;
const TITLE_FONT_WEIGHT: f32 = 600.0;

const ICON_SIZE: u32 = 16;
const ICON_PADDING: u32 = 6;

// Couleurs des "feux tricolores" macOS - constantes, pas suivies par
// `HeaderPalette` : un theme "a la mac" (voir `system_theme::detect`,
// `ButtonLayout::on_left`) impose ces couleurs precises, independamment du
// sombre/clair du reste de la barre.
const TRAFFIC_CLOSE: Color = Color::new(255, 95, 87, 255);
const TRAFFIC_MINIMIZE: Color = Color::new(255, 189, 46, 255);
const TRAFFIC_MAXIMIZE: Color = Color::new(40, 200, 64, 255);
const TRAFFIC_GLYPH: Color = Color::new(77, 34, 4, 200);
const TRAFFIC_RADIUS: u32 = 6;

// Palette de couleurs de la barre - deux jeux fixes (voir `palette`)
// choisis selon `header.dark` (voir `system_theme::detect_dark_mode`), pas
// une interpolation depuis un theme GTK arbitraire : azure-foundation ne
// lit/n'interprete aucun CSS de theme, seulement la preference
// sombre/clair du bureau.
struct HeaderPalette {
    background: Color,
    /// Filet sous la barre.
    border: Color,
    title: Color,
    glyph: Color,
    button_hover: Color,
    close_hover: Color,
}

// Gris neutres (pas de teinte bleue), comme les barres de macOS.
const DARK_PALETTE: HeaderPalette = HeaderPalette {
    background: Color::new(26, 26, 26, 255),
    border: Color::new(40, 40, 40, 255),
    title: Color::new(200, 198, 194, 255),
    glyph: Color::new(215, 213, 209, 255),
    button_hover: Color::new(48, 48, 48, 255),
    close_hover: Color::new(196, 60, 60, 255),
};

const LIGHT_PALETTE: HeaderPalette = HeaderPalette {
    background: Color::new(236, 236, 236, 255),
    border: Color::new(214, 214, 214, 255),
    title: Color::new(60, 60, 60, 255),
    glyph: Color::new(70, 70, 70, 255),
    button_hover: Color::new(214, 214, 214, 255),
    close_hover: Color::new(232, 100, 90, 255),
};

fn palette(dark: bool) -> &'static HeaderPalette {
    if dark { &DARK_PALETTE } else { &LIGHT_PALETTE }
}

/// Dessine la barre d'en-tete entiere sur `canvas` : fond, icone
/// d'application (si `header.icon` est renseigne), titre, et les boutons de
/// `header.layout` - `hovered` (calcule par `AzureWindow::run` a chaque
/// `WindowMouseMove`, voir `header_bar::button_at`) pilote le fond de
/// survol, `fullscreen` change l'icone du bouton plein ecran (restaurer
/// plutot qu'agrandir). Quand `header.layout.on_left` (theme "a la mac"),
/// les boutons se dessinent en feux tricolores plutot qu'en icones sur
/// fond de survol - voir `draw_traffic_light`.
pub fn draw_header(canvas: &mut Canvas, window_width: u32, header: &HeaderBar, hovered: Option<HeaderButton>, fullscreen: bool) {
    let palette = palette(header.dark);
    draw_rect(0, 0, window_width, HEADER_HEIGHT, &palette.background, canvas);
    draw_line_horizontal(0, window_width.saturating_sub(1), HEADER_HEIGHT - 1, &palette.border, canvas);

    // Titre (et icone) centres dans la fenetre, comme sur macOS, sans
    // jamais passer sous les boutons.
    let text_width = measure_text_width(&header.title, FONT_PATH, TITLE_FONT_SIZE, TITLE_FONT_WEIGHT).unwrap_or(0.0).ceil() as u32;
    let icon_width = if header.icon.is_some() { ICON_SIZE + ICON_PADDING } else { 0 };
    let block = icon_width + text_width;
    let (left_limit, right_limit) = if header.layout.on_left { (header.layout.width() + ICON_PADDING * 2, window_width) } else { (ICON_PADDING, window_width.saturating_sub(header.layout.width())) };
    let mut x = (window_width.saturating_sub(block) / 2).max(left_limit);
    if x + block > right_limit {
        x = left_limit;
    }
    if let Some(icon) = &header.icon {
        let icon_y = (HEADER_HEIGHT.saturating_sub(ICON_SIZE)) / 2;
        draw_image_scaled(icon, x, icon_y, ICON_SIZE, ICON_SIZE, canvas);
        x += icon_width;
    }
    // Centrage vertical approximatif (pas de metriques de police exactes
    // ici, contrairement a `renderer::draw_text` qui, lui, les a) : suffit
    // pour un texte court sur une seule ligne dans une barre de hauteur
    // fixe.
    let text_y = (HEADER_HEIGHT.saturating_sub(TITLE_FONT_SIZE as u32)) / 2;
    let _ = draw_text(&header.title, FONT_PATH, x, text_y, TITLE_FONT_SIZE, TITLE_FONT_WEIGHT, &palette.title, canvas);

    for (button, bx, by, bw, bh) in header_bar::button_rects(window_width, &header.layout) {
        if header.layout.on_left {
            // Comme sur macOS : survoler le groupe montre les trois glyphes.
            draw_traffic_light(canvas, bx, by, bw, bh, button, hovered.is_some(), fullscreen);
        } else {
            if hovered == Some(button) {
                let color = if button == HeaderButton::Close { palette.close_hover } else { palette.button_hover };
                draw_rect(bx, by, bw, bh, &color, canvas);
            }
            draw_button_glyph(canvas, bx, by, bw, bh, button, fullscreen, palette.glyph);
        }
    }
}

// Icones dessinees a la main avec les primitives existantes (lignes/
// rectangles) - pas de police d'icones ni de format vectoriel (SVG) gere
// ici, juste assez pour que chaque bouton reste reconnaissable.
#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
fn draw_button_glyph(canvas: &mut Canvas, bx: u32, by: u32, bw: u32, bh: u32, button: HeaderButton, fullscreen: bool, glyph_color: Color) {
    let cx = bx + bw / 2;
    let cy = by + bh / 2;

    match button {
        HeaderButton::Minimize => {
            draw_line_horizontal(cx - 6, cx + 6, cy + 5, &glyph_color, canvas);
        }
        HeaderButton::Fullscreen if fullscreen => {
            // Icone "restaurer" : deux carres decales, convention usuelle
            // pour distinguer "quitter le plein ecran" de "y entrer".
            draw_empty_rect(cx - 6, cy - 4, 8, 8, &glyph_color, canvas);
            draw_empty_rect(cx - 2, cy - 8, 8, 8, &glyph_color, canvas);
        }
        HeaderButton::Fullscreen => {
            draw_empty_rect(cx - 6, cy - 6, 12, 12, &glyph_color, canvas);
        }
        HeaderButton::Close => {
            draw_line(cx as i32 - 6, cy as i32 - 6, cx as i32 + 6, cy as i32 + 6, &glyph_color, canvas);
            draw_line(cx as i32 - 6, cy as i32 + 6, cx as i32 + 6, cy as i32 - 6, &glyph_color, canvas);
        }
    }
}

// Bouton style macOS : cercle plein d'une couleur fixe par role (voir
// `TRAFFIC_CLOSE`/`TRAFFIC_MINIMIZE`/`TRAFFIC_MAXIMIZE`), sans fond de
// survol - seul un petit glyphe sombre apparait dans le cercle au survol,
// comme sur macOS, plutot que le rectangle de survol utilise cote droit.
// `fullscreen` fait basculer le glyphe du bouton vert vers une icone
// "restaurer" une fois en plein ecran - meme distinction que
// `draw_button_glyph` cote droit (`HeaderButton::Fullscreen if fullscreen`),
// pour que les deux presentations de la barre restent coherentes plutot que
// l'une des deux figeant toujours le meme glyphe "agrandir".
#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
fn draw_traffic_light(canvas: &mut Canvas, bx: u32, by: u32, bw: u32, bh: u32, button: HeaderButton, hovered: bool, fullscreen: bool) {
    let cx = (bx + bw / 2) as i32;
    let cy = (by + bh / 2) as i32;
    let color = match button {
        HeaderButton::Close => TRAFFIC_CLOSE,
        HeaderButton::Minimize => TRAFFIC_MINIMIZE,
        HeaderButton::Fullscreen => TRAFFIC_MAXIMIZE,
    };
    draw_circle_filled(cx, cy, TRAFFIC_RADIUS, &color, canvas);

    if !hovered {
        return;
    }
    match button {
        HeaderButton::Close => {
            draw_line(cx - 3, cy - 3, cx + 3, cy + 3, &TRAFFIC_GLYPH, canvas);
            draw_line(cx - 3, cy + 3, cx + 3, cy - 3, &TRAFFIC_GLYPH, canvas);
        }
        HeaderButton::Minimize => {
            draw_line_horizontal((cx - 3) as u32, (cx + 3) as u32, cy as u32, &TRAFFIC_GLYPH, canvas);
        }
        HeaderButton::Fullscreen if fullscreen => {
            // Un seul petit carre (contrairement aux deux carres decales de
            // `draw_button_glyph`, qui deborderaient du cercle a ce rayon) -
            // suffisant pour se distinguer de la croix diagonale ci-dessous.
            draw_empty_rect((cx - 3) as u32, (cy - 3) as u32, 6, 6, &TRAFFIC_GLYPH, canvas);
        }
        HeaderButton::Fullscreen => {
            // Deux petits triangles opposes (haut-gauche, bas-droite), comme
            // le bouton vert de macOS - pas une croix, qu'on confondrait
            // avec « fermer ».
            for i in 0..4 {
                draw_line_horizontal((cx - 3) as u32, (cx + 1 - i) as u32, (cy - 3 + i) as u32, &TRAFFIC_GLYPH, canvas);
                draw_line_horizontal((cx - 1 + i) as u32, (cx + 3) as u32, (cy + 3 - i) as u32, &TRAFFIC_GLYPH, canvas);
            }
        }
    }
}
