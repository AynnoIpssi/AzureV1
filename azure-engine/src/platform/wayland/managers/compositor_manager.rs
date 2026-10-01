use crate::platform::wayland::models::connection::WaylandConnection;

pub fn create_surface(connection: &mut WaylandConnection, compositor_id: u32, new_id: u32) -> Result<u32, String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&compositor_id.to_le_bytes());
    msg.extend_from_slice(&((12u32 << 16).to_le_bytes()));
    msg.extend_from_slice(&new_id.to_le_bytes());
    connection.send(&msg)?;
    Ok(new_id)
}