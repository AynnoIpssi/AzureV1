use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::managers::renderer::draw_rect;
use azure_engine::platform::wayland::managers::surface_manager::{commit, damage_buffer};
use azure_engine::platform::wayland::managers::xdg_manager::attach;
use azure_engine::platform::wayland::managers::shared_memory_manager::unmap_memory;
use azure_engine::platform::wayland::managers::window_manager::window_create;
use azure_core::rules::window_provider::AzureWindowProvider;
use azure_core::rules::window_event::WindowEvent;

fn main() {
    let mut window = window_create(800, 600).expect("Failed to create window");

    let _xdg_surface_id = window.xdg_surface_id();
    let surface_id = window.surface_id();

    let mut canvas = Canvas::new(800, 600);
    canvas.buffer.chunks_mut(4).for_each(|p| {
        p[0] = 30;
        p[1] = 30;
        p[2] = 50;
        p[3] = 255;
    });

    let color = Color::new(0, 255, 0, 255);
    draw_rect(300, 200, 200, 150, &color, &mut canvas);

    unsafe {
        std::ptr::copy_nonoverlapping(canvas.buffer.as_ptr(), window.ptr(), canvas.buffer.len());
    }

    let buffer_id = window.buffer_id();
    attach(window.connection_mut(), surface_id, buffer_id).expect("attach");
    damage_buffer(window.connection_mut(), surface_id, 0, 0, 800, 600).expect("damage");
    commit(window.connection_mut(), surface_id).expect("commit");

    println!("Fenêtre créée. Bouge la souris, clique, tape des touches...");

    let mut keys_down: std::collections::HashSet<u32> = std::collections::HashSet::new();

    loop {
        if let Some(event) = window.poll_event() {
            match event {
                WindowEvent::WindowClose => {
                    println!("Fermeture demandée");
                    break;
                }
                WindowEvent::WindowResize(w, h) => {
                    println!("Resize: {}x{}", w, h);
                }
                WindowEvent::WindowKeyPress(key, pressed) => {
                    if pressed {
                        keys_down.insert(key);
                    } else {
                        keys_down.remove(&key);
                    }
                    println!("Touche: code={} pressed={} | actuellement enfoncees={:?}", key, pressed, keys_down);
                }
                WindowEvent::WindowMouseMove(x, y) => {
                    println!("Souris: x={} y={}", x, y);
                }
                WindowEvent::WindowMouseButton(button, pressed) => {
                    println!("Clic: bouton={} pressed={}", button, pressed);
                }
                WindowEvent::WindowScroll(delta) => {
                    println!("Molette: {}", delta);
                }
                WindowEvent::WindowScrollH(delta) => {
                    println!("Molette horizontale: {}", delta);
                }
            }
        }
    }

    unmap_memory(window.ptr(), (800 * 600 * 4) as usize).expect("unmap");
}