use crate::platform::wayland::models::connection::WaylandConnection;
pub fn get_xdg_surface(connection: &mut WaylandConnection, xdg_wm_base_id: u32, surface_id: u32, new_id: u32) -> Result<u32, String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&xdg_wm_base_id.to_le_bytes());
    msg.extend_from_slice(&((16u32 << 16 | 2u32).to_le_bytes()));
    msg.extend_from_slice(&new_id.to_le_bytes());
    msg.extend_from_slice(&surface_id.to_le_bytes());
    connection.send(&msg)?;
    Ok(new_id)
}

pub fn get_toplevel(connection: &mut WaylandConnection, xdg_surface_id: u32, new_id: u32) -> Result<u32, String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&xdg_surface_id.to_le_bytes());
    msg.extend_from_slice(&((12u32 << 16 | 1u32).to_le_bytes()));
    msg.extend_from_slice(&new_id.to_le_bytes());
    connection.send(&msg)?;
    Ok(new_id)
}

pub fn ack_configure(connection: &mut WaylandConnection, xdg_surface_id: u32, serial: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&xdg_surface_id.to_le_bytes());
    msg.extend_from_slice(&((12u32 << 16 | 4u32).to_le_bytes()));
    msg.extend_from_slice(&serial.to_le_bytes());
    connection.send(&msg)?;
    Ok(())
}

pub fn attach(connection: &mut WaylandConnection, surface_id: u32, buffer_id: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&surface_id.to_le_bytes());
    msg.extend_from_slice(&((20u32 << 16 | 1u32).to_le_bytes()));
    msg.extend_from_slice(&buffer_id.to_le_bytes());
    msg.extend_from_slice(&0u32.to_le_bytes()); //X
    msg.extend_from_slice(&0u32.to_le_bytes()); //Y
    connection.send(&msg)?;
    Ok(())
}

/// `xdg_toplevel::set_fullscreen` (requete 11) : demande au compositeur le
/// plein ecran, sur l'ecran qu'il choisit lui-meme (argument `output` mis a
/// 0, "aucune preference" - un vrai `wl_output` precis n'est pas encore
/// selectionnable ici). Le compositeur repond par un `configure` avec la
/// taille de l'ecran (voir `WindowEvent::WindowResize`, deja gere par
/// `Window::resize`) - rien d'autre a faire cote client.
pub fn set_fullscreen(connection: &mut WaylandConnection, toplevel_id: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&toplevel_id.to_le_bytes());
    msg.extend_from_slice(&((12u32 << 16 | 11u32).to_le_bytes()));
    msg.extend_from_slice(&0u32.to_le_bytes()); // output = null (0)
    connection.send(&msg)?;
    Ok(())
}

/// `xdg_toplevel::unset_fullscreen` (requete 12, sans argument) : revient a
/// la taille normale - meme mecanisme de `configure`/`WindowResize` que
/// `set_fullscreen`.
pub fn unset_fullscreen(connection: &mut WaylandConnection, toplevel_id: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&toplevel_id.to_le_bytes());
    msg.extend_from_slice(&((8u32 << 16 | 12u32).to_le_bytes()));
    connection.send(&msg)?;
    Ok(())
}

/// `xdg_toplevel::set_minimized` (requete 13, sans argument) : simple
/// demande, sans confirmation ni etat suivi par le protocole - le
/// compositeur decide seul de la politique de minimisation/restauration
/// (rien a faire cote client apres l'avoir envoyee).
pub fn set_minimized(connection: &mut WaylandConnection, toplevel_id: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&toplevel_id.to_le_bytes());
    msg.extend_from_slice(&((8u32 << 16 | 13u32).to_le_bytes()));
    connection.send(&msg)?;
    Ok(())
}

/// `xdg_toplevel::set_app_id` (requete 3) : l'identifiant que le bureau
/// utilise pour associer cette fenetre a un fichier .desktop - c'est ce
/// fichier (son `Icon=`), pas cette requete seule, qui determine l'icone
/// affichee dans la barre des taches/le dock (voir `AzureWindow::app_id`
/// cote azure-foundation). Sans fichier .desktop installe sous ce nom, la
/// plupart des environnements de bureau affichent une icone generique.
pub fn set_app_id(connection: &mut WaylandConnection, toplevel_id: u32, app_id: &str) -> Result<(), String> {
    let id_bytes = app_id.as_bytes();
    let id_len = id_bytes.len() as u32 + 1; // +1 pour le null terminator
    let padded_len = (id_len + 3) & !3;
    let total_size = 8 + 4 + padded_len;

    let mut msg = Vec::new();
    msg.extend_from_slice(&toplevel_id.to_le_bytes());
    msg.extend_from_slice(&((total_size << 16 | 3u32).to_le_bytes()));
    msg.extend_from_slice(&id_len.to_le_bytes());
    msg.extend_from_slice(id_bytes);
    msg.push(0);
    while msg.len() % 4 != 0 {
        msg.push(0);
    }
    connection.send(&msg)?;
    Ok(())
}

/// `xdg_toplevel::move` (requete 5) : demande au compositeur de demarrer un
/// deplacement interactif de la fenetre (le compositeur prend la main sur le
/// pointeur jusqu'au relachement du bouton, exactement comme s'il faisait
/// glisser sa propre decoration). A appeler depuis un appui bouton gauche sur
/// une zone "poignee" (ex. la barre d'en-tete maison hors des boutons - voir
/// `AzureWindow::run`). `serial` DOIT etre celui de l'evenement
/// `wl_pointer::button` (bouton enfonce) qui a declenche l'appel : un serial
/// perime (deja consomme par une autre requete) fait ignorer la demande
/// silencieusement par le compositeur (protocole xdg-shell) - voir
/// `Window::last_pointer_serial`.
pub fn move_toplevel(connection: &mut WaylandConnection, toplevel_id: u32, seat_id: u32, serial: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&toplevel_id.to_le_bytes());
    msg.extend_from_slice(&((16u32 << 16 | 5u32).to_le_bytes()));
    msg.extend_from_slice(&seat_id.to_le_bytes());
    msg.extend_from_slice(&serial.to_le_bytes());
    connection.send(&msg)?;
    Ok(())
}

/// `xdg_toplevel::resize` (requete 6) : demande au compositeur de demarrer un
/// redimensionnement interactif le long du bord/coin `edges` (voir
/// `azure_foundation::window::models::resize_edge::ResizeEdge` cote appelant -
/// les valeurs correspondent exactement a l'enum `xdg_toplevel::resize_edge`
/// du protocole, transmises telles quelles). Meme exigence de fraicheur sur
/// `serial` que `move_toplevel` : celui du `wl_pointer::button` (bouton
/// enfonce) qui a declenche l'appel, sinon ignore silencieusement.
pub fn resize_toplevel(connection: &mut WaylandConnection, toplevel_id: u32, seat_id: u32, serial: u32, edges: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&toplevel_id.to_le_bytes());
    msg.extend_from_slice(&((20u32 << 16 | 6u32).to_le_bytes()));
    msg.extend_from_slice(&seat_id.to_le_bytes());
    msg.extend_from_slice(&serial.to_le_bytes());
    msg.extend_from_slice(&edges.to_le_bytes());
    connection.send(&msg)?;
    Ok(())
}

pub fn set_title(connection: &mut WaylandConnection, toplevel_id: u32, title: &str) -> Result<(), String> {
    let title_bytes = title.as_bytes();
    let title_len = title_bytes.len() as u32 + 1; // +1 pour le null terminator
    let padded_len = (title_len + 3) & !3; // arrondi à 4 bytes
    let total_size = 8 + 4 + padded_len;
    
    let mut msg = Vec::new();
    msg.extend_from_slice(&toplevel_id.to_le_bytes());
    msg.extend_from_slice(&((total_size << 16 | 2u32).to_le_bytes()));
    msg.extend_from_slice(&title_len.to_le_bytes());
    msg.extend_from_slice(title_bytes);
    msg.push(0); // null terminator
    while msg.len() % 4 != 0 {
        msg.push(0);
    }
    connection.send(&msg)?;
    Ok(())
}