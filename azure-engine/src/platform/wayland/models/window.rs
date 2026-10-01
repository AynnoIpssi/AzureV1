use azure_core::rules::window_provider::AzureWindowProvider;
use azure_core::rules::window_event::WindowEvent;
use crate::platform::wayland::managers::shared_memory_manager::{grow_shared_memory, map_memory, unmap_memory};
use crate::platform::wayland::managers::shm_manager::{create_buffer, destroy_buffer, resize_pool};
use crate::platform::wayland::models::connection::WaylandConnection;
use crate::platform::wayland::models::object_id_allocator::ObjectIdAllocator;
use crate::platform::wayland::models::shared_memory::WaylandMemory;
use crate::platform::wayland::managers::decoration_manager::DecorationMode;
use crate::platform::wayland::managers::cursor_manager::{destroy_pool, set_cursor};
use crate::platform::wayland::managers::shared_memory_manager::create_shared_memory;
use crate::platform::wayland::managers::shm_manager::create_shm_pool;
use crate::platform::wayland::managers::surface_manager::{commit, damage_buffer};
use crate::platform::wayland::managers::xdg_manager::attach;
use crate::platform::wayland::models::cursor::{CursorBuffer, CursorId, CursorImage, CursorState};
use crate::platform::wayland::models::clipboard::Clipboard;
use crate::platform::wayland::managers::clipboard_manager;
pub struct Window{
    surface_id: u32,
    buffer_id: u32,
    pool_id: u32,
    xdg_toplevel_id: u32,
    xdg_wm_id: u32,
    xdg_surface_id: u32,
    keyboard_id: u32,
    pointer_id: u32,
    // `wl_seat` (pas le sous-objet `wl_pointer`/`wl_keyboard`) - requis tel
    // quel par certaines requetes qui portent sur le SIEGE plutot que sur un
    // de ses peripheriques, ex. `xdg_toplevel::move` (voir
    // `xdg_manager::move_toplevel`, appelee par `AzureWindow::run` au
    // deplacement de la fenetre par sa barre d'en-tete maison).
    seat_id: u32,
    // Serial du DERNIER `wl_pointer::button` recu (voir
    // `surface_manager::run_event_loop_interactive`, mis a jour avant
    // d'appeler `on_event`) - c'est celui-ci, jamais un serial de
    // `xdg_surface::configure` ou perime, qu'exige le compositeur pour
    // honorer une requete interactive comme `xdg_toplevel::move`
    // (protocole xdg-shell : un serial qui ne correspond pas au dernier
    // evenement d'entree est ignore silencieusement).
    last_pointer_serial: u32,
    width: i32,
    height: i32,
    ptr: *mut u8,
    // La memoire partagee sous-jacente au buffer actuel - gardee (pas
    // juste mappee puis oubliee comme avant) pour pouvoir l'agrandir a la
    // volee sur un redimensionnement (voir `resize`), et savoir combien
    // en demapper a la fermeture (voir `mapped_size`).
    memory: WaylandMemory,
    // Pour minter de nouveaux id d'objet Wayland APRES la creation de la
    // fenetre (un nouveau `wl_buffer` a chaque redimensionnement, voir
    // `resize`) - l'allocateur utilise a la creation ne doit donc pas
    // etre abandonne une fois `Window` construit.
    allocator: ObjectIdAllocator,
    connection: WaylandConnection,
    // Mode reellement applique par le compositeur (voir
    // `window_manager::window_create` et
    // `surface_manager::wait_for_configure_and_decoration`) - `ClientSide`
    // aussi bien quand le compositeur l'a explicitement choisi que quand le
    // protocole `xdg-decoration` est absent du registre (GNOME/Mutter,
    // notamment). C'est CE champ, via `is_server_side_decorated`, qui dit a
    // `AzureWindow` s'il doit dessiner sa propre barre d'en-tete ou laisser
    // la place a celle du compositeur.
    decoration_mode: DecorationMode,
    // `wl_shm` lie a la creation (voir `window_manager::window_create`) -
    // garde pour pouvoir creer, apres coup, un pool par image de curseur
    // (voir `register_cursor`).
    shm_id: u32,
    // Surface dediee au curseur souris + les images deja envoyees au
    // compositeur (voir `register_cursor`/`show_cursor`).
    cursor: CursorState,
    // Presse-papiers du systeme (voir `set_clipboard`/`clipboard_text`).
    clipboard: Clipboard,
    // Serial du dernier evenement clavier ou bouton de souris : exige par
    // `wl_data_device::set_selection` (voir `set_clipboard`).
    last_input_serial: u32,
    // `xdg_activation_v1`, si le compositeur l'expose (voir
    // `activation_token`/`activate`).
    activation_id: Option<u32>,
}

impl Window {
    pub fn surface_id(&self) -> u32 { self.surface_id }
    pub fn buffer_id(&self) -> u32 { self.buffer_id }
    pub fn xdg_toplevel_id(&self) -> u32 { self.xdg_toplevel_id }
    pub fn xdg_wm_id(&self) -> u32 { self.xdg_wm_id }
    pub fn xdg_surface_id(&self) -> u32 { self.xdg_surface_id }
    pub fn keyboard_id(&self) -> u32 { self.keyboard_id }
    pub fn pointer_id(&self) -> u32 { self.pointer_id }
    pub fn seat_id(&self) -> u32 { self.seat_id }
    pub fn last_pointer_serial(&self) -> u32 { self.last_pointer_serial }
    /// Enregistre le serial d'un `wl_pointer::button` fraichement recu -
    /// voir `last_pointer_serial` pour pourquoi. Appelee depuis
    /// `surface_manager::run_event_loop_interactive`, jamais directement par
    /// une application.
    pub fn set_last_pointer_serial(&mut self, serial: u32) { self.last_pointer_serial = serial; }
    pub fn width(&self) -> i32 { self.width }
    pub fn height(&self) -> i32 { self.height }
    pub fn ptr(&self) -> *mut u8 { self.ptr }

    /// `true` si le compositeur dessine lui-meme la decoration (barre de
    /// titre + boutons) de cette fenetre - voir `decoration_mode`.
    /// `AzureWindow::run` s'appuie dessus pour savoir s'il doit dessiner sa
    /// propre barre d'en-tete (voir `window::models::header_bar`) ou s'en
    /// remettre entierement au compositeur.
    pub fn is_server_side_decorated(&self) -> bool {
        matches!(self.decoration_mode, DecorationMode::ServerSide)
    }

    /// Taille reelle (en octets) de la memoire partagee actuellement
    /// mappee derriere `ptr()` - peut etre plus grande que
    /// `width() * height() * 4` (voir `resize`, qui ne retrecit jamais
    /// cette allocation) : c'est CETTE taille, pas un calcul a partir de
    /// `width()`/`height()`, qu'il faut passer a `unmap_memory` en fin de
    /// vie de la fenetre pour ne demapper ni trop ni trop peu.
    pub fn mapped_size(&self) -> usize { self.memory.size() }

    pub fn connection_mut(&mut self) -> &mut WaylandConnection {
        &mut self.connection
    }

    #[allow(clippy::too_many_arguments)] // position, taille, style... : lus d'un coup, comme le reste de l'API
    pub fn new(surface_id: u32, buffer_id: u32, pool_id: u32, xdg_toplevel_id: u32, xdg_wm_id: u32, xdg_surface_id: u32,keyboard_id :u32, pointer_id:u32, seat_id: u32, width: i32, height: i32, ptr: *mut u8, memory: WaylandMemory, allocator: ObjectIdAllocator, connection: WaylandConnection, decoration_mode: DecorationMode, shm_id: u32, cursor_surface_id: u32) -> Window{
        Window{
            surface_id,
            buffer_id,
            pool_id,
            xdg_toplevel_id,
            xdg_wm_id,
            xdg_surface_id,
            keyboard_id,
            pointer_id,
            seat_id,
            last_pointer_serial: 0,
            width,
            height,
            ptr,
            memory,
            allocator,
            connection,
            decoration_mode,
            shm_id,
            cursor: CursorState::new(cursor_surface_id),
            clipboard: Clipboard::default(),
            last_input_serial: 0,
            activation_id: None,
        }
    }

    /// Un nouvel id d'objet Wayland (pour lier un global apres coup).
    pub fn next_object_id(&mut self) -> u32 { self.allocator.next_id() }

    /// Branche le presse-papiers du systeme (voir `window_manager::window_create`).
    pub fn attach_data_device(&mut self, manager_id: u32) -> Result<(), String> {
        let device_id = clipboard_manager::get_data_device(&mut self.connection, manager_id, self.seat_id, self.allocator.next_id())?;
        self.clipboard.manager_id = manager_id;
        self.clipboard.device_id = Some(device_id);
        Ok(())
    }

    /// `true` si le presse-papiers du systeme est disponible.
    pub fn has_system_clipboard(&self) -> bool {
        self.clipboard.device_id.is_some()
    }

    /// Branche `xdg_activation_v1` (voir `window_manager::window_create`).
    pub fn attach_activation(&mut self, activation_id: u32) {
        self.activation_id = Some(activation_id);
    }

    /// Un jeton pour mettre une AUTRE fenetre (d'une autre app) au premier
    /// plan, lie a la derniere action de l'utilisateur dans celle-ci (voir
    /// `activation_manager`). A passer a l'autre app, qui appelle
    /// `activate`. `None` sans protocole, ou si le compositeur ne repond
    /// pas a temps. Attend la reponse ; les autres messages recus entre-temps
    /// sont gardes pour la boucle d'evenements.
    pub fn activation_token(&mut self) -> Option<String> {
        use crate::platform::wayland::managers::activation_manager::{destroy_token, read_token, request_token, TOKEN_DONE};
        let activation_id = self.activation_id?;
        let token_id = self.allocator.next_id();
        request_token(&mut self.connection, activation_id, token_id, self.last_input_serial, self.seat_id, self.surface_id).ok()?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        let mut kept = Vec::new();
        let token = loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() || !matches!(self.connection.wait_readable(left.as_millis().max(1) as i32), Ok(true)) {
                break None;
            }
            let Ok(message) = self.connection.read_message() else { break None };
            if message.0 == token_id && message.1 == TOKEN_DONE {
                break read_token(&message.2);
            }
            kept.push(message);
        };
        for message in kept {
            self.connection.defer(message);
        }
        let _ = destroy_token(&mut self.connection, token_id);
        token
    }

    /// Met cette fenetre au premier plan avec un jeton recu d'une autre app
    /// (voir `activation_token`). Sans protocole : rien.
    pub fn activate(&mut self, token: &str) -> Result<(), String> {
        let Some(activation_id) = self.activation_id else { return Ok(()) };
        crate::platform::wayland::managers::activation_manager::activate(&mut self.connection, activation_id, token, self.surface_id)
    }

    /// Voir `last_input_serial`. Appelee par la boucle d'evenements.
    pub fn set_last_input_serial(&mut self, serial: u32) { self.last_input_serial = serial; }

    /// Copie `text` dans le presse-papiers du systeme : les autres apps
    /// peuvent le coller.
    pub fn set_clipboard(&mut self, text: &str) -> Result<(), String> {
        let id = self.allocator.next_id();
        clipboard_manager::set_selection(&mut self.connection, &mut self.clipboard, text, self.last_input_serial, id)
    }

    /// La carte du clavier recue du compositeur depuis le dernier appel
    /// (voir `WaylandConnection::take_keymap`).
    pub fn take_keymap(&mut self) -> Option<String> {
        self.connection.take_keymap()
    }

    /// `(modificateurs verrouilles, groupe)` du clavier (voir
    /// `WaylandConnection::keyboard_modifiers`).
    pub fn keyboard_modifiers(&self) -> (u32, u32) {
        self.connection.keyboard_modifiers()
    }

    /// Le texte du presse-papiers du systeme (copie par n'importe quelle
    /// app), `None` s'il est vide ou n'est pas du texte.
    pub fn clipboard_text(&mut self) -> Option<String> {
        clipboard_manager::selection_text(&mut self.connection, &self.clipboard)
    }

    /// Traite un evenement du presse-papiers ; `false` s'il ne le concerne pas.
    pub fn handle_clipboard_event(&mut self, object_id: u32, opcode: u16, args: &[u8]) -> bool {
        clipboard_manager::handle_event(&mut self.connection, &mut self.clipboard, object_id, opcode, args)
    }

    /// Envoie UNE fois les pixels de `image` au compositeur (memoire
    /// partagee + pool + `wl_buffer`, comme le buffer de la fenetre) et
    /// retourne un id a passer ensuite a `show_cursor` autant de fois que
    /// voulu, sans jamais recopier les pixels. Le pool et notre copie de la
    /// memoire sont liberes aussitot : le `wl_buffer` suffit au compositeur.
    pub fn register_cursor(&mut self, image: &CursorImage) -> Result<CursorId, String> {
        let size = (image.width * image.height * 4) as usize;
        if image.width == 0 || image.height == 0 || image.pixels.len() != size {
            return Err(format!("CursorImage invalide: {}x{} pour {} octets", image.width, image.height, image.pixels.len()));
        }
        let memory = create_shared_memory(size)?;
        let ptr = map_memory(&memory)?;
        unsafe { std::ptr::copy_nonoverlapping(image.pixels.as_ptr(), ptr, size) };
        unmap_memory(ptr, size)?;

        let pool_id = create_shm_pool(&mut self.connection, self.shm_id, memory.fd(), size, self.allocator.next_id())?;
        let buffer_id = create_buffer(&mut self.connection, pool_id, image.width as i32, image.height as i32, self.allocator.next_id())?;
        destroy_pool(&mut self.connection, pool_id)?;
        // Le fd a deja ete transmis au compositeur (`sendmsg` synchrone,
        // voir `WaylandConnection::send_with_fd`) : notre copie est inutile.
        unsafe { libc::close(memory.fd()) };

        self.cursor.buffers.push(CursorBuffer {
            buffer_id,
            width: image.width as i32,
            height: image.height as i32,
            hotspot_x: image.hotspot_x as i32,
            hotspot_y: image.hotspot_y as i32,
        });
        Ok(CursorId(self.cursor.buffers.len() - 1))
    }

    /// Affiche le curseur `id` (voir `register_cursor`) tant que la souris
    /// est sur cette fenetre - sans effet si c'est deja lui. Si la souris
    /// n'est pas (encore) entree dans la fenetre, le choix est retenu et
    /// applique a la prochaine entree (voir `pointer_entered`).
    pub fn show_cursor(&mut self, id: CursorId) -> Result<(), String> {
        if self.cursor.current == Some(id) {
            return Ok(());
        }
        self.cursor.current = Some(id);
        self.apply_cursor()
    }

    /// A appeler a chaque `wl_pointer::enter` (voir
    /// `surface_manager::run_event_loop_interactive`/`poll_event`) : le
    /// compositeur oublie le curseur d'un client quand la souris sort de sa
    /// fenetre, il faut donc le redonner a chaque entree, avec le serial de
    /// CETTE entree.
    pub fn pointer_entered(&mut self, serial: u32) -> Result<(), String> {
        self.cursor.enter_serial = Some(serial);
        self.apply_cursor()
    }

    fn apply_cursor(&mut self) -> Result<(), String> {
        let (Some(serial), Some(CursorId(index))) = (self.cursor.enter_serial, self.cursor.current) else {
            return Ok(());
        };
        let Some(buffer) = self.cursor.buffers.get(index) else {
            return Err(format!("CursorId {index} inconnu pour cette fenetre"));
        };
        let surface_id = self.cursor.surface_id;
        let (buffer_id, width, height, hotspot_x, hotspot_y) = (buffer.buffer_id, buffer.width, buffer.height, buffer.hotspot_x, buffer.hotspot_y);
        attach(&mut self.connection, surface_id, buffer_id)?;
        damage_buffer(&mut self.connection, surface_id, 0, 0, width, height)?;
        commit(&mut self.connection, surface_id)?;
        set_cursor(&mut self.connection, self.pointer_id, serial, surface_id, hotspot_x, hotspot_y)
    }

    /// Reagit a un `xdg_toplevel::configure` suggerant une nouvelle taille
    /// de fenetre (voir `WindowEvent::WindowResize`) : recree le
    /// `wl_buffer` a la nouvelle taille - les buffers Wayland sont de
    /// taille FIXE, on ne peut pas "redimensionner" celui qui existe deja,
    /// seulement en creer un nouveau puis detruire l'ancien - et agrandit
    /// (jamais ne retrecit) la memoire partagee sous-jacente si le nouveau
    /// buffer ne tient plus dans l'allocation actuelle.
    ///
    /// Ne fait rien si `new_width`/`new_height` est nul ou negatif (un
    /// `configure` a (0,0) delegue entierement le choix de la taille au
    /// client - garder la taille actuelle plutot que tenter un buffer
    /// vide) ni si la taille demandee est deja la taille courante.
    pub fn resize(&mut self, new_width: i32, new_height: i32) -> Result<(), String> {
        if new_width <= 0 || new_height <= 0 || (new_width == self.width && new_height == self.height) {
            return Ok(());
        }

        let new_size = (new_width as usize) * (new_height as usize) * 4;
        if new_size > self.memory.size() {
            grow_shared_memory(self.memory.fd(), new_size)?;
            unmap_memory(self.ptr, self.memory.size())?;
            self.memory = WaylandMemory::new(self.memory.fd(), new_size);
            self.ptr = map_memory(&self.memory)?;
            resize_pool(&mut self.connection, self.pool_id, new_size)?;
        }

        let new_buffer_id = self.allocator.next_id();
        create_buffer(&mut self.connection, self.pool_id, new_width, new_height, new_buffer_id)?;
        destroy_buffer(&mut self.connection, self.buffer_id)?;

        self.buffer_id = new_buffer_id;
        self.width = new_width;
        self.height = new_height;
        Ok(())
    }
}

impl AzureWindowProvider for Window {

    fn width(&self) -> i32 {self.width}
    fn height(&self) -> i32 {self.height}
    fn render(&mut self, pixels: &[u8]) -> Result<(), String> {
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(self.ptr, (self.width * self.height * 4) as usize)
        };
        buffer.copy_from_slice(pixels);
        Ok(())
    }
    fn poll_event(&mut self) -> Option<WindowEvent> {
        match self.connection.has_data() {
            Ok(true) => {}
            _ => return None,
        }

        let Ok((object_id, opcode, args)) = self.connection.read_message() else { return None };
        if self.handle_clipboard_event(object_id, opcode, &args) {
            return None;
        }

        let toplevel_id = self.xdg_toplevel_id;

        if object_id == toplevel_id && opcode == 1 {
            return Some(WindowEvent::WindowClose);
        }

        if object_id == toplevel_id && opcode == 0 {
            let new_width = i32::from_le_bytes(args[0..4].try_into().unwrap_or([0;4]));
            let new_height = i32::from_le_bytes(args[4..8].try_into().unwrap_or([0;4]));
            return Some(WindowEvent::WindowResize(new_width, new_height));
        }

        if object_id == self.keyboard_id && opcode == 3 {
            self.last_input_serial = u32::from_le_bytes(args[0..4].try_into().unwrap_or([0;4]));
            let key = u32::from_le_bytes(args[8..12].try_into().unwrap_or([0;4]));
            let state = u32::from_le_bytes(args[12..16].try_into().unwrap_or([0;4]));
            return Some(WindowEvent::WindowKeyPress(key, state == 1));
        }

        if object_id == self.pointer_id && opcode == 0 {
            // wl_pointer::enter : serial, surface, x, y (fixed 24.8) - voir
            // `pointer_entered`. Remonte comme un deplacement pour que le
            // survol soit calcule des l'entree, sans attendre un `motion`.
            let serial = u32::from_le_bytes(args[0..4].try_into().unwrap_or([0;4]));
            let _ = self.pointer_entered(serial);
            let x_fixed = i32::from_le_bytes(args[8..12].try_into().unwrap_or([0;4]));
            let y_fixed = i32::from_le_bytes(args[12..16].try_into().unwrap_or([0;4]));
            return Some(WindowEvent::WindowMouseMove(x_fixed / 256, y_fixed / 256));
        }

        if object_id == self.pointer_id && opcode == 3 {
            self.last_input_serial = u32::from_le_bytes(args[0..4].try_into().unwrap_or([0;4]));
            let button = u32::from_le_bytes(args[8..12].try_into().unwrap_or([0;4]));
            let state = u32::from_le_bytes(args[12..16].try_into().unwrap_or([0;4]));
            return Some(WindowEvent::WindowMouseButton(button, state == 1));
        }

        if object_id == self.pointer_id && opcode == 2 {
            let x_fixed = i32::from_le_bytes(args[4..8].try_into().unwrap_or([0;4]));
            let y_fixed = i32::from_le_bytes(args[8..12].try_into().unwrap_or([0;4]));
            let x = x_fixed / 256; // conversion fixed (24.8) vers entier
            let y = y_fixed / 256;
            return Some(WindowEvent::WindowMouseMove(x, y));
        }

        // xdg_wm_base.ping : si on ne repond pas par un pong, le compositeur
        // affiche la fenetre comme "ne repond pas" apres quelques secondes.
        if object_id == self.xdg_wm_id && opcode == 0 {
            let serial = u32::from_le_bytes(args[0..4].try_into().unwrap_or([0;4]));
            let mut msg = Vec::new();
            msg.extend_from_slice(&self.xdg_wm_id.to_le_bytes());
            msg.extend_from_slice(&((12u32 << 16 | 3u32).to_le_bytes()));
            msg.extend_from_slice(&serial.to_le_bytes());
            self.connection.send(&msg).ok();
            return None;
        }
        None
    }
}