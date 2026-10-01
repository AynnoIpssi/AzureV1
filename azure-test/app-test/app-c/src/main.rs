use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::services::shapes::line::draw_line;
use azure_engine::platform::wayland::managers::surface_manager::{commit, damage_buffer, run_event_loop};
use azure_engine::platform::wayland::managers::xdg_manager::attach;
use azure_engine::platform::wayland::managers::shared_memory_manager::unmap_memory;
use azure_engine::platform::wayland::managers::window_manager::window_create;
use azure_rooter::services::client::{register, receive, subscribe};

fn main() {
    let mut window = window_create(800, 600).expect("Failed to create window");
    let xdg_surface_id = window.xdg_surface_id();
    let xdg_toplevel_id = window.xdg_toplevel_id();
    let surface_id = window.surface_id();
    let xdg_wm_id = window.xdg_wm_id();
    let mut canvas = Canvas::new(800, 600);
    canvas.buffer.chunks_mut(4).for_each(|p| { p[0]=30; p[1]=30; p[2]=50; p[3]=255; });

    let color = Color::new(255, 255, 0, 255);
    draw_line(400, 200, 300, 400, &color, &mut canvas);
    draw_line(300, 400, 500, 400, &color, &mut canvas);
    draw_line(500, 400, 400, 200, &color, &mut canvas);

    unsafe { std::ptr::copy_nonoverlapping(canvas.buffer.as_ptr(), window.ptr(), canvas.buffer.len()); }
    let buffer_id = window.buffer_id();
    attach(window.connection_mut(), surface_id, buffer_id).expect("attach");
    damage_buffer(window.connection_mut(), surface_id, 0, 0, 800, 600).expect("damage");
    commit(window.connection_mut(), surface_id).expect("commit");

    let mut connection = register(3).expect("Failed to register");
    subscribe(&mut connection, 3, "notification").expect("Failed to subscribe");
    println!("App C enregistrée et abonnée, en attente de message...");

    let message = receive(&mut connection).expect("Failed to receive");
    println!("Message reçu: {}", message);

    run_event_loop(&mut window, xdg_surface_id, xdg_toplevel_id, surface_id, xdg_wm_id).expect("loop");
    unmap_memory(window.ptr(), 800*600*4).expect("unmap");
}