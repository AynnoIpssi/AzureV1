// Live visual check of Roboto across the sizes/weights we care about.
// Roboto's wght axis is 100-900 (default 400). Run with:
//   cargo test --test text_roboto_size_weight_matrix -- --nocapture
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
fn show_roboto_matrix() {
    let font = "src/Roboto-VariableFont_wdth,wght.ttf";
    let sizes: [f32; 8] = [10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0];
    let weights: [f32; 4] = [100.0, 400.0, 700.0, 900.0];
    let sample = "AaBbGgQqYyJj";

    let row_h = 34u32; // vertical step between rows, fits size 24 + margin
    let rows = (sizes.len() * weights.len()) as u32;
    let win_w = 700u32;
    let win_h = rows * row_h + 20;

    let mut window = window_create(win_w as i32, win_h as i32).expect("Failed to create window");

    let xdg_surface_id = window.xdg_surface_id();
    let xdg_toplevel_id = window.xdg_toplevel_id();
    let surface_id = window.surface_id();
    let xdg_wm_id = window.xdg_wm_id();

    let mut canvas = Canvas::new(win_w, win_h);
    canvas.buffer.chunks_mut(4).for_each(|p| {
        p[0] = 50; p[1] = 30; p[2] = 30; p[3] = 255;
    });

    let green = Color::new(0, 255, 0, 255);

    let mut y = 10u32;
    for &size in sizes.iter() {
        for &weight in weights.iter() {
            let label = format!("{:.0}px/{:.0} {}", size, weight, sample);
            draw_text(&label, font, 10, y, size, weight, &green, &mut canvas)
                .expect("Failed to draw text");
            y += row_h;
        }
    }

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
