// Writes the EXACT SAME canvas buffer both to a .ppm file and to a real
// Wayland window, in the same run. If the file looks clean but a screenshot
// of the window looks broken, the bug is in capture/compositor/display, not
// in the rendering code, since both come from the identical byte buffer.
// Run with:
//   cargo test --test text_file_vs_window -- --nocapture
// Compare /tmp/azure_text_check/file_vs_window.ppm against a screenshot of
// the window that pops up.

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
fn compare_file_vs_window() {
    let mut window = window_create(500, 100).expect("Failed to create window");

    let xdg_surface_id = window.xdg_surface_id();
    let xdg_toplevel_id = window.xdg_toplevel_id();
    let surface_id = window.surface_id();
    let xdg_wm_id = window.xdg_wm_id();

    let mut canvas = Canvas::new(500, 100);
    canvas.buffer.chunks_mut(4).for_each(|p| {
        p[0] = 50; p[1] = 30; p[2] = 30; p[3] = 255;
    });

    let green = Color::new(0, 255, 0, 255);
    draw_text("abcdefghijklmnopqrstuvwxyz", "src/Sora-VariableFont_wght.ttf", 10, 10, 12.0, 100.0, &green, &mut canvas)
        .unwrap();

    // 1) Dump this exact buffer to a file first.
    let mut out = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) {
        out.push(px[2]);
        out.push(px[1]);
        out.push(px[0]);
    }
    std::fs::create_dir_all(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure_text_check")).unwrap();
    std::fs::write(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure_text_check/file_vs_window.ppm"), out).unwrap();
    println!("Wrote /tmp/azure_text_check/file_vs_window.ppm");

    // 2) Copy the SAME buffer bytes to the window.
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
