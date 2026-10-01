use crate::platform::wayland::managers::connection_manager::connect;
use crate::platform::wayland::managers::registry_manager::{get_registry, find_global};
use crate::platform::wayland::managers::bind_manager::bind_global;
use crate::platform::wayland::models::screen_output::ScreenOutput;

pub fn get_screen_resolution() -> Result<ScreenOutput, String> {
    let mut connection = connect()?;
    let registry = get_registry(&mut connection)?;
    let output_name = find_global(&registry, "wl_output").ok_or("wl_output not found")?;

    bind_global(&mut connection, output_name, "wl_output", 2, 4)?;

    loop {
        let mut header = [0u8; 8];
        connection.receive(&mut header)?;
        let object_id = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let size_opcode = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let size = (size_opcode >> 16) as u16;
        let opcode = (size_opcode & 0xFFFF) as u16;
        let args_size = (size - 8) as usize;
        let mut args = vec![0u8; args_size];
        connection.receive(&mut args)?;
        
        if object_id == 4 && opcode == 1 {
            let width = i32::from_le_bytes(args[4..8].try_into().unwrap());
            let height = i32::from_le_bytes(args[8..12].try_into().unwrap());
            return Ok(ScreenOutput::new(width, height));
        }
    }
}