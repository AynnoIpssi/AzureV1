use crate::platform::wayland::managers::xdg_manager::ack_configure;
use crate::platform::wayland::models::connection::WaylandConnection;
use crate::platform::wayland::models::window::Window;
use crate::platform::wayland::managers::xdg_manager::attach;
use crate::platform::wayland::managers::decoration_manager::DecorationMode;
use azure_core::rules::window_event::WindowEvent;
use std::time::{Duration, Instant};

/// Facteur applique aux crans de molette (pas au pave tactile).
pub const WHEEL_SPEED: f64 = 3.0;

pub fn damage_buffer(connection: &mut WaylandConnection, surface_id: u32, x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&surface_id.to_le_bytes());
    msg.extend_from_slice(&((24u32 << 16 | 9u32).to_le_bytes()));
    msg.extend_from_slice(&x.to_le_bytes());
    msg.extend_from_slice(&y.to_le_bytes());
    msg.extend_from_slice(&width.to_le_bytes());
    msg.extend_from_slice(&height.to_le_bytes());
    connection.send(&msg)?;
    Ok(())
}

pub fn commit(connection: &mut WaylandConnection, surface_id: u32) -> Result<(), String> {
    let mut msg = Vec::new();
    msg.extend_from_slice(&(surface_id).to_le_bytes());
    msg.extend_from_slice(&((8u32 << 16 | 6u32).to_le_bytes()));
    connection.send(&msg)?;
    Ok(())
}

pub fn wait_for_configure(connection: &mut WaylandConnection, xdg_surface_id: u32) -> Result<u32, String> {
    loop {
        let (object_id, opcode, args) = connection.read_message()?;

        if object_id == 1 && opcode == 0 {
            let error_object_id = u32::from_le_bytes(args[0..4].try_into().unwrap());
            let error_code = u32::from_le_bytes(args[4..8].try_into().unwrap());
            let msg_len = u32::from_le_bytes(args[8..12].try_into().unwrap()) as usize;
            let message = String::from_utf8_lossy(&args[12..12 + msg_len - 1]).to_string();
            return Err(format!("Wayland error on object {}: code {} - {}", error_object_id, error_code, message));
        }

        if object_id == xdg_surface_id && opcode == 0 {
            let serial = u32::from_le_bytes(args[0..4].try_into().unwrap());
            return Ok(serial);
        }
    }
}

/// Meme boucle que `wait_for_configure`, mais qui capture EN PLUS le
/// premier evenement `zxdg_toplevel_decoration_v1::configure` (mode
/// reellement applique par le compositeur - voir
/// `decoration_manager::get_toplevel_decoration`) quand `decoration_id`
/// est fourni. Les deux evenements arrivent en reponse au meme `commit()`
/// initial (voir `window_manager::window_create`), donc on attend les deux
/// avant de rendre la main. Sans `decoration_id` (protocole absent du
/// registre, ex. GNOME/Mutter), se comporte exactement comme
/// `wait_for_configure`, second element toujours `None`.
pub fn wait_for_configure_and_decoration(
    connection: &mut WaylandConnection,
    xdg_surface_id: u32,
    decoration_id: Option<u32>,
) -> Result<(u32, Option<DecorationMode>), String> {
    let mut serial = None;
    let mut mode = None;

    loop {
        let (object_id, opcode, args) = connection.read_message()?;

        if object_id == 1 && opcode == 0 {
            let error_object_id = u32::from_le_bytes(args[0..4].try_into().unwrap());
            let error_code = u32::from_le_bytes(args[4..8].try_into().unwrap());
            let msg_len = u32::from_le_bytes(args[8..12].try_into().unwrap()) as usize;
            let message = String::from_utf8_lossy(&args[12..12 + msg_len - 1]).to_string();
            return Err(format!("Wayland error on object {}: code {} - {}", error_object_id, error_code, message));
        }

        if object_id == xdg_surface_id && opcode == 0 {
            serial = Some(u32::from_le_bytes(args[0..4].try_into().unwrap()));
        }

        if decoration_id == Some(object_id) && opcode == 0 {
            let raw_mode = u32::from_le_bytes(args[0..4].try_into().unwrap());
            mode = Some(if raw_mode == 2 { DecorationMode::ServerSide } else { DecorationMode::ClientSide });
        }

        let waiting_on_decoration = decoration_id.is_some() && mode.is_none();
        if let Some(serial) = serial
            && !waiting_on_decoration {
                return Ok((serial, mode));
            }
    }
}

pub fn run_event_loop(connection: &mut Window, xdg_surface_id: u32, xdg_toplevel_id: u32, surface_id: u32, xdg_wm_base_id: u32) -> Result<(), String> {
    loop {
        let (object_id, opcode, args) = connection.connection_mut().read_message()?;
        if object_id == xdg_toplevel_id && opcode == 0 {
            // ignore
        }
        if object_id == xdg_toplevel_id && opcode == 1 {
            return Ok(());
        }
        if object_id == xdg_surface_id && opcode == 0 {
            let serial = u32::from_le_bytes(args[0..4].try_into().unwrap());
            ack_configure(connection.connection_mut(), xdg_surface_id, serial)?;
            let buffer_id = connection.buffer_id();
            attach(connection.connection_mut(), surface_id, buffer_id)?;
        }
        if object_id == xdg_wm_base_id && opcode == 0 {
            let serial = u32::from_le_bytes(args[0..4].try_into().unwrap());
            let mut msg = Vec::new();
            msg.extend_from_slice(&(xdg_wm_base_id).to_le_bytes());
            msg.extend_from_slice(&((12u32 << 16 | 3u32).to_le_bytes()));
            msg.extend_from_slice(&serial.to_le_bytes());
            connection.connection_mut().send(&msg)?;
        }
        if object_id == 1 && opcode == 0 {
            let error_object_id = u32::from_le_bytes(args[0..4].try_into().unwrap());
            let error_code = u32::from_le_bytes(args[4..8].try_into().unwrap());
            let msg_len = u32::from_le_bytes(args[8..12].try_into().unwrap()) as usize;
            let message = String::from_utf8_lossy(&args[12..12 + msg_len - 1]).to_string();
            return Err(format!("Wayland error on object {}: code {} - {}", error_object_id, error_code, message));
        }
    }
}

// Meme boucle que `run_event_loop` (ack des configure xdg_surface, fermeture
// sur xdg_toplevel::close, pong sur les ping xdg_wm_base), mais qui fait en
// plus remonter les evenements souris/clavier/redimensionnement a
// `on_event`, ET se reveille toute seule (via `WaylandConnection::wait_readable`)
// meme sans aucun message du compositeur, pour appeler `on_tick` -
// necessaire pour tout ce qui doit avancer avec le temps plutot qu'en
// reaction a un evenement (clignotement d'un curseur de saisie, repetition
// d'une touche maintenue). Une fonction separee plutot qu'un parametre
// optionnel sur `run_event_loop` pour ne pas casser sa signature existante
// (plusieurs appelants dans ce workspace, hors de ce qui a motive cet ajout).
// `on_event`/`on_tick` retournent `false` pour demander l'arret de la
// boucle DE L'INTERIEUR (par exemple un bouton "fermer" dessine par
// l'application elle-meme, pas un vrai `xdg_toplevel::close` du
// compositeur) - `true` pour continuer normalement. Sans ca, rien ne
// permettait a l'appelant de mettre fin a sa propre boucle autrement qu'en
// attendant un evenement externe (close du compositeur, erreur protocole).
#[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
pub fn run_event_loop_interactive(
    connection: &mut Window,
    xdg_surface_id: u32,
    xdg_toplevel_id: u32,
    surface_id: u32,
    xdg_wm_base_id: u32,
    tick_interval_ms: i32,
    mut on_event: impl FnMut(&mut Window, WindowEvent) -> bool,
    mut on_tick: impl FnMut(&mut Window) -> bool,
) -> Result<(), String> {
    // Le tic passe a heure fixe, qu'il y ait des evenements ou non : avant,
    // il n'etait appele que quand AUCUN message n'arrivait pendant tout un
    // intervalle. Une molette rapide ou un pave tactile (un evenement toutes
    // les quelques ms) l'affamait alors completement - le defilement anime
    // et les redessins mis en attente (qui avancent dans `on_tick`) restaient
    // figes jusqu'a ce que l'utilisateur s'arrete. Negatif = jamais de tic.
    let interval = (tick_interval_ms >= 0).then(|| Duration::from_millis(tick_interval_ms as u64));
    let mut last_tick = Instant::now();
    // Source du defilement de la trame en cours (`wl_pointer::axis_source`) :
    // 0 molette, 1 doigt (pave tactile), 2 continu, 3 molette inclinee.
    let mut axis_source = 0u32;
    // Deplacement de la souris pas encore remonte. Une souris envoie des
    // centaines de `motion` par seconde et chacun coute un survol (mise en
    // page de l'ecran) : seul le DERNIER d'une rafale est remonte, quand
    // plus rien n'attend ou avant tout autre message (un clic voit donc la
    // bonne position). Sans ca, une page lourde prend du retard sur la
    // souris et la fenetre fige.
    let mut pending_move: Option<(i32, i32)> = None;
    loop {
        // Attente jusqu'au prochain tic (arrondie au-dessus : un 0 ferait
        // tourner la boucle a vide pendant la derniere milliseconde).
        let timeout = interval.map_or(-1, |i| i.saturating_sub(last_tick.elapsed()).as_micros().div_ceil(1000) as i32);
        let timeout = if pending_move.is_some() { 0 } else { timeout };
        let readable = connection.connection_mut().wait_readable(timeout)?;
        if !readable
            && let Some((x, y)) = pending_move.take()
            && !on_event(connection, WindowEvent::WindowMouseMove(x, y))
        {
            return Ok(());
        }
        if interval.is_some_and(|i| last_tick.elapsed() >= i) {
            last_tick = Instant::now();
            if !on_tick(connection) {
                return Ok(());
            }
        }
        if !readable {
            continue;
        }

        let (object_id, opcode, args) = connection.connection_mut().read_message()?;
        let pointer = object_id == connection.pointer_id();
        if pointer && opcode == 2 {
            // wl_pointer::motion : serial+time (ignores), x, y en fixed 24.8.
            let x_fixed = i32::from_le_bytes(args[4..8].try_into().unwrap());
            let y_fixed = i32::from_le_bytes(args[8..12].try_into().unwrap());
            pending_move = Some((x_fixed / 256, y_fixed / 256));
            continue;
        }
        // wl_pointer::frame ferme chaque `motion` : il ne coupe pas la rafale.
        if !(pointer && opcode == 5)
            && let Some((x, y)) = pending_move.take()
            && !on_event(connection, WindowEvent::WindowMouseMove(x, y))
        {
            return Ok(());
        }
        // Presse-papiers (voir `Window::handle_clipboard_event`).
        if connection.handle_clipboard_event(object_id, opcode, &args) {
            continue;
        }

        if object_id == xdg_toplevel_id && opcode == 1 {
            return Ok(());
        }
        if object_id == xdg_toplevel_id && opcode == 0 {
            let width = i32::from_le_bytes(args[0..4].try_into().unwrap());
            let height = i32::from_le_bytes(args[4..8].try_into().unwrap());
            if !on_event(connection, WindowEvent::WindowResize(width, height)) {
                return Ok(());
            }
        }
        if object_id == xdg_surface_id && opcode == 0 {
            let serial = u32::from_le_bytes(args[0..4].try_into().unwrap());
            ack_configure(connection.connection_mut(), xdg_surface_id, serial)?;
            let buffer_id = connection.buffer_id();
            attach(connection.connection_mut(), surface_id, buffer_id)?;
        }
        if object_id == xdg_wm_base_id && opcode == 0 {
            let serial = u32::from_le_bytes(args[0..4].try_into().unwrap());
            let mut msg = Vec::new();
            msg.extend_from_slice(&(xdg_wm_base_id).to_le_bytes());
            msg.extend_from_slice(&((12u32 << 16 | 3u32).to_le_bytes()));
            msg.extend_from_slice(&serial.to_le_bytes());
            connection.connection_mut().send(&msg)?;
        }
        if object_id == connection.pointer_id() && opcode == 0 {
            // wl_pointer::enter : serial, surface, x, y (fixed 24.8). Le
            // curseur choisi par l'application doit etre redonne a chaque
            // entree (voir `Window::pointer_entered`), puis l'entree est
            // remontee comme un deplacement : le survol (et donc la forme du
            // curseur) est ainsi juste des le premier pixel.
            let serial = u32::from_le_bytes(args[0..4].try_into().unwrap());
            connection.pointer_entered(serial)?;
            let x_fixed = i32::from_le_bytes(args[8..12].try_into().unwrap());
            let y_fixed = i32::from_le_bytes(args[12..16].try_into().unwrap());
            if !on_event(connection, WindowEvent::WindowMouseMove(x_fixed / 256, y_fixed / 256)) {
                return Ok(());
            }
        }
        if object_id == connection.pointer_id() && opcode == 3 {
            // wl_pointer::button : serial, time, button, state.
            let serial = u32::from_le_bytes(args[0..4].try_into().unwrap());
            let button = u32::from_le_bytes(args[8..12].try_into().unwrap());
            let state = u32::from_le_bytes(args[12..16].try_into().unwrap());
            // Retenu AVANT d'appeler `on_event` : un deplacement interactif
            // declenche depuis ce callback (voir `AzureWindow::run`,
            // `xdg_manager::move_toplevel`) a besoin du serial de CET
            // appui, pas d'un serial deja perime.
            connection.set_last_pointer_serial(serial);
            connection.set_last_input_serial(serial);
            if !on_event(connection, WindowEvent::WindowMouseButton(button, state == 1)) {
                return Ok(());
            }
        }
        if object_id == connection.pointer_id() && opcode == 6 && args.len() >= 4 {
            // wl_pointer::axis_source (avant les `axis` de la meme trame).
            axis_source = u32::from_le_bytes(args[0..4].try_into().unwrap());
        }
        if object_id == connection.pointer_id() && opcode == 5 {
            // wl_pointer::frame : fin de la trame, source oubliee.
            axis_source = 0;
        }
        if object_id == connection.pointer_id() && opcode == 4 && args.len() >= 12 {
            // wl_pointer::axis : time, axis (0 = vertical, 1 = horizontal),
            // value (fixed 24.8, en pixels de surface). Une molette avance par
            // crans (~10-15) : multiplies pour un defilement confortable. Un
            // pave tactile suit deja le doigt : valeur telle quelle.
            let axis = u32::from_le_bytes(args[4..8].try_into().unwrap());
            let value = i32::from_le_bytes(args[8..12].try_into().unwrap()) as f64 / 256.0;
            let value = if matches!(axis_source, 1 | 2) { value } else { value * WHEEL_SPEED };
            let event = if axis == 0 { WindowEvent::WindowScroll(value) } else { WindowEvent::WindowScrollH(value) };
            if !on_event(connection, event) {
                return Ok(());
            }
        }
        if object_id == connection.keyboard_id() && opcode == 3 {
            // wl_keyboard::key : serial, time, key, state. Le serial sert a
            // copier (voir `Window::set_clipboard`).
            connection.set_last_input_serial(u32::from_le_bytes(args[0..4].try_into().unwrap()));
            let key = u32::from_le_bytes(args[8..12].try_into().unwrap());
            let state = u32::from_le_bytes(args[12..16].try_into().unwrap());
            if !on_event(connection, WindowEvent::WindowKeyPress(key, state == 1)) {
                return Ok(());
            }
        }
        if object_id == 1 && opcode == 0 {
            let error_object_id = u32::from_le_bytes(args[0..4].try_into().unwrap());
            let error_code = u32::from_le_bytes(args[4..8].try_into().unwrap());
            let msg_len = u32::from_le_bytes(args[8..12].try_into().unwrap()) as usize;
            let message = String::from_utf8_lossy(&args[12..12 + msg_len - 1]).to_string();
            return Err(format!("Wayland error on object {}: code {} - {}", error_object_id, error_code, message));
        }
    }
}


