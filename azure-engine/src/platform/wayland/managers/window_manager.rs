use crate::platform::wayland::models::window::Window;
use crate::platform::wayland::managers::connection_manager::connect;
use crate::platform::wayland::managers::registry_manager::{get_registry, find_global};
use crate::platform::wayland::managers::bind_manager::bind_global;
use crate::platform::wayland::managers::shared_memory_manager::{create_shared_memory, map_memory};
use crate::platform::wayland::managers::shm_manager::{create_shm_pool, create_buffer};
use crate::platform::wayland::managers::compositor_manager::create_surface;
use crate::platform::wayland::managers::surface_manager::{commit, wait_for_configure_and_decoration, damage_buffer};
use crate::platform::wayland::managers::xdg_manager::{get_xdg_surface, get_toplevel, ack_configure, attach};
use crate::platform::wayland::models::object_id_allocator::ObjectIdAllocator;
use crate::platform::wayland::managers::seat_manager::{get_keyboard, get_pointer};
use crate::platform::wayland::managers::decoration_manager::{get_toplevel_decoration, set_mode, DecorationMode, MODE_SERVER_SIDE};

pub fn window_create(width: i32, height: i32) -> Result<Window, String> {
    let mut allocator = ObjectIdAllocator::new();
    let mut connection = connect()?;
    let registry = get_registry(&mut connection)?;

    let shm_name = find_global(&registry, "wl_shm").ok_or("wl_shm not found".to_string())?;
    let compositor_name = find_global(&registry, "wl_compositor").ok_or("wl_compositor not found".to_string())?;
    let xdg_wm_base_name = find_global(&registry, "xdg_wm_base").ok_or("xdg_wm_base not found".to_string())?;
    let seat_name = find_global(&registry, "wl_seat").ok_or("wl_seat not found".to_string())?;

    let shm_id = bind_global(&mut connection, shm_name, "wl_shm", 2, allocator.next_id())?;

    let memory = create_shared_memory((width * height * 4) as usize)?;
    let ptr = map_memory(&memory)?;
    //let pixels = unsafe { std::slice::from_raw_parts_mut(ptr, memory.size()) };

    let pool_id = create_shm_pool(&mut connection, shm_id, memory.fd(), memory.size(), allocator.next_id())?;
    let buffer_id = create_buffer(&mut connection, pool_id, width, height, allocator.next_id())?;

    let compositor_id = bind_global(&mut connection, compositor_name, "wl_compositor", 4, allocator.next_id())?;

    let surface_id = create_surface(&mut connection, compositor_id, allocator.next_id())?;

    let xdg_wm_base_id = bind_global(&mut connection, xdg_wm_base_name, "xdg_wm_base", 1, allocator.next_id())?;

    let xdg_surface_id = get_xdg_surface(&mut connection, xdg_wm_base_id, surface_id, allocator.next_id())?;

    let wl_seat = bind_global(&mut connection, seat_name, "wl_seat", 7, allocator.next_id())?;

    let keyboard_id = get_keyboard(&mut connection, wl_seat, allocator.next_id())?;
    connection.set_keyboard_id(keyboard_id);
    let pointer_id = get_pointer(&mut connection, wl_seat, allocator.next_id())?;
    // Surface sans role pour l'instant : elle devient la surface "curseur"
    // au premier `wl_pointer::set_cursor` (voir `Window::show_cursor`).
    let cursor_surface_id = create_surface(&mut connection, compositor_id, allocator.next_id())?;
    let toplevel_id = get_toplevel(&mut connection, xdg_surface_id, allocator.next_id())?;

    // Decoration native (barre de titre/boutons dessinee par le
    // compositeur) si le protocole `xdg-decoration` est expose par le
    // registre - sinon `decoration_id` reste `None` et le mode retombe sur
    // `ClientSide` (voir `Window::is_server_side_decorated`), utilise par
    // `AzureWindow` pour dessiner sa propre barre d'en-tete. GNOME/Mutter,
    // notamment, n'expose jamais ce global : cote client uniquement, quoi
    // qu'on demande via `set_mode`.
    let decoration_id = match find_global(&registry, "zxdg_decoration_manager_v1") {
        Some(name) => {
            let manager_id = bind_global(&mut connection, name, "zxdg_decoration_manager_v1", 1, allocator.next_id())?;
            let decoration_id = get_toplevel_decoration(&mut connection, manager_id, toplevel_id, allocator.next_id())?;
            set_mode(&mut connection, decoration_id, MODE_SERVER_SIDE)?;
            Some(decoration_id)
        }
        None => None,
    };

    commit(&mut connection, surface_id)?;
    let (serial, decoration_mode) = wait_for_configure_and_decoration(&mut connection, xdg_surface_id, decoration_id)?;
    let decoration_mode = decoration_mode.unwrap_or(DecorationMode::ClientSide);
    ack_configure(&mut connection, xdg_surface_id, serial)?;
    damage_buffer(&mut connection, surface_id, 0, 0, width, height)?;
    attach(&mut connection, surface_id, buffer_id)?;
    commit(&mut connection, surface_id)?;

    let mut window = Window::new(surface_id, buffer_id, pool_id, toplevel_id, xdg_wm_base_id, xdg_surface_id, keyboard_id, pointer_id, wl_seat, width, height, ptr, memory, allocator, connection, decoration_mode, shm_id, cursor_surface_id);

    // Presse-papiers du systeme, s'il existe (tous les compositeurs courants
    // l'ont). Lie apres la premiere configuration : ses evenements arrivent
    // ensuite dans la boucle de la fenetre. Sans lui, copier-coller reste
    // interne a la fenetre.
    if let Some(global) = registry.get_globals().iter().find(|g| g.name() == "wl_data_device_manager") {
        let version = global.version().min(3);
        let new_id = window.next_object_id();
        let manager_id = bind_global(window.connection_mut(), global.id(), "wl_data_device_manager", version, new_id)?;
        window.attach_data_device(manager_id)?;
    }
    // Premier plan (voir `activation_manager`). Lancee par une autre app
    // avec un jeton (`XDG_ACTIVATION_TOKEN`, la convention des lanceurs) :
    // la fenetre s'active avec, et apparait devant.
    if let Some(global) = registry.get_globals().iter().find(|g| g.name() == "xdg_activation_v1") {
        let new_id = window.next_object_id();
        let activation_id = bind_global(window.connection_mut(), global.id(), "xdg_activation_v1", 1, new_id)?;
        window.attach_activation(activation_id);
        if let Ok(token) = std::env::var("XDG_ACTIVATION_TOKEN")
            && !token.is_empty()
        {
            let _ = window.activate(&token);
        }
    }
    Ok(window)
}
