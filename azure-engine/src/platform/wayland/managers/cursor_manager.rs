use crate::platform::wayland::models::connection::WaylandConnection;

/// `wl_pointer::set_cursor` (requete 0) : serial, surface, hotspot_x,
/// hotspot_y. `serial` DOIT etre celui du dernier `wl_pointer::enter` (voir
/// `CursorState::enter_serial`), sinon le compositeur ignore la requete.
/// `surface_id` recoit le role "curseur" : son buffer attache devient
/// l'image affichee sous la souris tant qu'elle reste sur notre fenetre.
pub fn set_cursor(connection: &mut WaylandConnection, pointer_id: u32, serial: u32, surface_id: u32, hotspot_x: i32, hotspot_y: i32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&pointer_id.to_le_bytes());
    msg.extend_from_slice(&(24u32 << 16).to_le_bytes());
    msg.extend_from_slice(&serial.to_le_bytes());
    msg.extend_from_slice(&surface_id.to_le_bytes());
    msg.extend_from_slice(&hotspot_x.to_le_bytes());
    msg.extend_from_slice(&hotspot_y.to_le_bytes());
    connection.send(&msg)?;
    Ok(())
}

/// `wl_shm_pool::destroy` (requete 1) : les `wl_buffer` deja crees depuis ce
/// pool restent valides (ils gardent leur propre reference a la memoire) -
/// permet de ne garder aucun pool vivant pour une image qui ne changera
/// plus, comme un curseur (voir `Window::register_cursor`).
pub fn destroy_pool(connection: &mut WaylandConnection, pool_id: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&pool_id.to_le_bytes());
    msg.extend_from_slice(&((8u32 << 16) | 1u32).to_le_bytes());
    connection.send(&msg)?;
    Ok(())
}
