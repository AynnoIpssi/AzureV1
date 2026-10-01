use crate::platform::wayland::models::connection::WaylandConnection;
use std::os::fd::RawFd;

pub fn create_shm_pool(connection: &mut WaylandConnection, shm_id: u32, fd: RawFd, size: usize, new_id: u32) -> Result<u32, String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&shm_id.to_le_bytes());
    msg.extend_from_slice(&((16u32 << 16).to_le_bytes()));
    msg.extend_from_slice(&new_id.to_le_bytes());
    msg.extend_from_slice(&(size as i32).to_le_bytes());
    connection.send_with_fd(&msg, fd)?;
    Ok(new_id)
}

pub fn create_buffer(connection: &mut WaylandConnection, pool_id: u32, width: i32, height: i32, new_id: u32) -> Result<u32, String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&pool_id.to_le_bytes());
    msg.extend_from_slice(&((32u32 << 16).to_le_bytes()));
    msg.extend_from_slice(&new_id.to_le_bytes());
    msg.extend_from_slice(&0u32.to_le_bytes());
    msg.extend_from_slice(&width.to_le_bytes());
    msg.extend_from_slice(&height.to_le_bytes());
    msg.extend_from_slice(&(width * 4).to_le_bytes());
    msg.extend_from_slice(&0u32.to_le_bytes());
    connection.send(&msg)?;
    Ok(new_id)
}

/// `wl_shm_pool::resize` (requete 2) : agrandit le pool cote compositeur
/// pour qu'il corresponde au fichier memoire partage deja agrandi (voir
/// `shared_memory_manager::grow_shared_memory`) - necessaire avant de
/// pouvoir creer un `wl_buffer` dont la taille depasse celle du pool
/// d'origine. Jamais appele pour retrecir : le protocole l'autoriserait,
/// mais readapter une allocation existante plus grande que necessaire est
/// inoffensif, alors que le faire a chaque petit redimensionnement serait
/// un aller-retour reseau de plus pour rien.
pub fn resize_pool(connection: &mut WaylandConnection, pool_id: u32, new_size: usize) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&pool_id.to_le_bytes());
    msg.extend_from_slice(&((12u32 << 16 | 2u32).to_le_bytes()));
    msg.extend_from_slice(&(new_size as i32).to_le_bytes());
    connection.send(&msg)?;
    Ok(())
}

/// `wl_buffer::destroy` (requete 0, sans argument) : libere un buffer
/// devenu inutile cote compositeur - les buffers Wayland sont de taille
/// fixe, donc un redimensionnement en cree un nouveau (voir
/// `shm_manager::create_buffer`) et doit detruire l'ancien pour ne pas
/// accumuler des objets morts a chaque redimensionnement de fenetre.
pub fn destroy_buffer(connection: &mut WaylandConnection, buffer_id: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&buffer_id.to_le_bytes());
    msg.extend_from_slice(&((8u32 << 16).to_le_bytes()));
    connection.send(&msg)?;
    Ok(())
}


