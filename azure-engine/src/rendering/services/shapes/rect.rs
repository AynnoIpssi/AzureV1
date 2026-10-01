use crate::rendering::models::canvas::Canvas;
use crate::rendering::models::color::Color;
use crate::rendering::services::buffer::get_pixel_index;
use crate::rendering::services::shapes::circle::draw_circle_filled;
use crate::rendering::services::shapes::line::{draw_line_horizontal, draw_line_vertical};

//-----------------------(Rectangle)------------------------->

// Un rectangle quasi plein ecran (fond, conteneurs) etait auparavant un
// appel a `set_pixel` par pixel individuel (verification de bornes +
// indexation recalculee a chaque fois) - le principal cout d'un redessin
// complet, mesure a plusieurs centaines de milliers d'appels par frame
// pour une fenetre de taille courante. On clippe le rectangle une seule
// fois, puis on copie une ligne entiere d'un coup (`copy_from_slice`, que
// le compilateur transforme en memcpy) plutot que d'ecrire chaque pixel.
pub fn draw_rect(x: u32, y: u32, width: u32, height: u32, color: &Color, canvas: &mut Canvas) {
    if width == 0 || height == 0 {
        return;
    }

    // En plus des bornes du canvas, un rectangle ne deborde jamais de la
    // zone de decoupage active (voir `Canvas::set_clip`) - utilise pour
    // qu'un contenu qui defile (une zone de texte plus longue que sa
    // boite) ne peigne jamais au-dela de sa boite.
    let (clip_x, clip_y, clip_w, clip_h) = canvas.clip_bounds();
    let x_start = x.max(clip_x);
    let y_start = y.max(clip_y);
    let x_end = x.saturating_add(width).min(clip_x + clip_w);
    let y_end = y.saturating_add(height).min(clip_y + clip_h);
    if x_start >= x_end || y_start >= y_end {
        return;
    }
    let (x, y) = (x_start, y_start);

    let row_pixels = (x_end - x) as usize;
    // Wayland's wl_shm buffer is ARGB8888, stored little-endian as B,G,R,A
    let mut row = Vec::with_capacity(row_pixels * 4);
    for _ in 0..row_pixels {
        row.push(color.b);
        row.push(color.g);
        row.push(color.r);
        row.push(color.a);
    }

    for py in y..y_end {
        let start = get_pixel_index(x, py, canvas.width);
        canvas.buffer[start..start + row.len()].copy_from_slice(&row);
    }
}

//---------------------------(RectangleRounded)------------------->

pub fn draw_rect_rounded(x: u32, y: u32, width: u32, height: u32, radius: u32, color: &Color, canvas: &mut Canvas) {
    draw_circle_filled((x + radius) as i32, (y + radius) as i32, radius, color, canvas);
    draw_circle_filled((x + width - radius) as i32 , (y + radius) as i32, radius, color, canvas);
    draw_circle_filled((x + radius) as i32, (y + height - radius) as i32, radius, color, canvas);
    draw_circle_filled((x + width - radius) as i32, (y + height - radius) as i32, radius, color, canvas);
    draw_rect(x, y + radius, width + 1, height - 2*radius, color, canvas);
    draw_rect(x + radius, y, width - 2*radius + 1, radius, color, canvas);
    draw_rect(x + radius, y + height - radius, width - 2*radius, radius + 1, color, canvas);
}

//------------------------(EmptryRectangle)------------------------>

pub fn draw_empty_rect(x: u32, y: u32, width: u32, height: u32, color: &Color, canvas: &mut Canvas) {
    draw_line_horizontal(x, x + width, y, color, canvas);
    draw_line_horizontal(x, x+ width, y + height, color, canvas);
    draw_line_vertical(y, y + height, x, color, canvas);
    draw_line_vertical(y, y+ height, x + width, color, canvas);
}