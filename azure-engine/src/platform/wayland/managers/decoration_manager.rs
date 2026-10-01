use crate::platform::wayland::models::connection::WaylandConnection;

/// Modes du protocole `xdg-decoration-unstable-v1` (`zxdg_toplevel_decoration_v1`).
pub const MODE_CLIENT_SIDE: u32 = 1;
pub const MODE_SERVER_SIDE: u32 = 2;

/// Mode reellement applique par le compositeur, recu via l'evenement
/// `configure` de l'objet decoration - PAS le mode demande par
/// `set_mode` (une simple preference, jamais garantie : voir
/// `wait_for_configure_and_decoration` cote `surface_manager`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationMode {
    ClientSide,
    ServerSide,
}

/// `zxdg_decoration_manager_v1::get_toplevel_decoration` (requete 0) : cree
/// l'objet de decoration associe a CE `xdg_toplevel` - un objet par
/// toplevel, pas un par connexion. N'existe que si le global
/// `zxdg_decoration_manager_v1` a ete trouve dans le registre (voir
/// `window_manager::window_create`) : GNOME/Mutter ne l'expose jamais.
pub fn get_toplevel_decoration(connection: &mut WaylandConnection, manager_id: u32, toplevel_id: u32, new_id: u32) -> Result<u32, String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&manager_id.to_le_bytes());
    msg.extend_from_slice(&(16u32 << 16).to_le_bytes());
    msg.extend_from_slice(&new_id.to_le_bytes());
    msg.extend_from_slice(&toplevel_id.to_le_bytes());
    connection.send(&msg)?;
    Ok(new_id)
}

/// `zxdg_toplevel_decoration_v1::set_mode` (requete 1) : demande au
/// compositeur le mode passe en argument (`MODE_SERVER_SIDE` ici) - une
/// simple preference, pas une garantie. Le mode reellement applique arrive
/// ensuite par l'evenement `configure` de ce meme objet (voir
/// `surface_manager::wait_for_configure_and_decoration`).
pub fn set_mode(connection: &mut WaylandConnection, decoration_id: u32, mode: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&decoration_id.to_le_bytes());
    msg.extend_from_slice(&((12u32 << 16) | 1u32).to_le_bytes());
    msg.extend_from_slice(&mode.to_le_bytes());
    connection.send(&msg)?;
    Ok(())
}
