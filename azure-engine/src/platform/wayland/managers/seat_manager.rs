use crate::platform::wayland::models::connection::WaylandConnection;
//--------------------------(Keyboard)-------------------------->
pub fn get_keyboard(connection: &mut WaylandConnection, seat_id: u32, new_id: u32) -> Result<u32, String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&seat_id.to_le_bytes());
    msg.extend_from_slice(&((12u32 << 16 | 1u32).to_le_bytes())); //Opcode = 1
    msg.extend_from_slice(&new_id.to_le_bytes());
    connection.send(&msg)?;
    Ok(new_id)
}
//--------------------------(Pointer)-------------------------->
pub fn get_pointer(connection: &mut WaylandConnection,seat_id: u32, new_id: u32) -> Result<u32, String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&seat_id.to_le_bytes());
    msg.extend_from_slice(&((12u32 << 16).to_le_bytes())); //Opcode = 2
    msg.extend_from_slice(&new_id.to_le_bytes());
    connection.send(&msg)?;
    Ok(new_id)
}


