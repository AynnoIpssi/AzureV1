// Live visual check for the minimum-glyph-gap fix (MIN_GLYPH_GAP_PX in
// src/rendering/managers/renderer.rs). Opens a real Wayland window so you
// can see it directly, no image export needed. Run with:
//   cargo test --test text_min_spacing_window -- --nocapture
// Close the window (or Ctrl+C) to end the test.

use azure_engine::platform::wayland::managers::window_manager::window_create;
use azure_engine::platform::wayland::managers::surface_manager::run_event_loop;
use azure_engine::platform::wayland::managers::surface_manager::{commit, damage_buffer};
use azure_engine::platform::wayland::managers::shared_memory_manager::unmap_memory;
use azure_engine::platform::wayland::managers::xdg_manager::attach;
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::managers::renderer::draw_text;

#[test]
#[ignore = "ouvre une vraie fenetre et bloque jusqu'a sa fermeture manuelle"]
fn show_min_spacing_window() {
    let mut window = window_create(700, 800).expect("Failed to create window");

    let xdg_surface_id = window.xdg_surface_id();
    let xdg_toplevel_id = window.xdg_toplevel_id();
    let surface_id = window.surface_id();   
    let xdg_wm_id = window.xdg_wm_id();

    let mut canvas = Canvas::new(700, 800);
    canvas.buffer.chunks_mut(8).for_each(|p| {
        p[0] = 50; p[1] = 30; p[2] = 30; p[3] = 255; p[4] = 50; p[5] = 30; p[6] = 30; p[7] = 255;
    });

    let green = Color::new(0, 255, 0, 255);
    let font = "src/Sora-VariableFont_wght.ttf";
    let text = "abcdefghijklmnopqrstuvwxyz";
    let text_maj = "ABCDEFGHIJKLMNOPQRSTUVWXYZ *$ŝ$";

    // Worst-case combos: small size + heavy weight, where the font's own
    // side bearing shrinks below 1px and glyphs used to touch.
    draw_text(text, font, 20, 20, 12.0, 100.0, &green, &mut canvas).unwrap();
    draw_text(text, font, 20, 60, 12.0, 400.0, &green, &mut canvas).unwrap();
    draw_text(text, font, 20, 100, 12.0, 800.0, &green, &mut canvas).unwrap();
    draw_text(text, font, 20, 150, 14.0, 800.0, &green, &mut canvas).unwrap();
    draw_text(text, font, 20, 210, 28.0, 800.0, &green, &mut canvas).unwrap();
    draw_text(text_maj, font, 20, 250, 12.0, 100.0, &green, &mut canvas).unwrap();
    draw_text(text_maj, font, 20, 290, 12.0, 400.0, &green, &mut canvas).unwrap();
    draw_text(text_maj, font, 20, 330, 12.0, 800.0, &green, &mut canvas).unwrap();
    draw_text(text_maj, font, 20, 380, 14.0, 800.0, &green, &mut canvas).unwrap();
    draw_text(text_maj, font, 20, 440, 28.0, 800.0, &green, &mut canvas).unwrap();

    unsafe {
        std::ptr::copy_nonoverlapping(canvas.buffer.as_ptr(), window.ptr(), canvas.buffer.len());
    }

    let win_width = window.width();
    let win_height = window.height();
    let buffer_id = window.buffer_id();
    attach(window.connection_mut(), surface_id, buffer_id).expect("Failed to attach");
    damage_buffer(window.connection_mut(), surface_id, 0, 0, win_width, win_height).expect("Failed to damage");
    commit(window.connection_mut(), surface_id).expect("Failed to commit");

    run_event_loop(&mut window, xdg_surface_id, xdg_toplevel_id, surface_id, xdg_wm_id)
        .expect("Event loop failed");

    unmap_memory(window.ptr(), (window.width() * window.height() * 4) as usize)
        .expect("Failed to unmap memory");
}
