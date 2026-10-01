// Image de curseur souris fournie par l'application (voir
// `Window::register_cursor`) - le moteur ne sait PAS dessiner de curseur
// lui-meme, il ne fait que transporter des pixels deja generes jusqu'au
// compositeur (voir `azure_foundation::cursor` pour la generation).

/// Pixels ARGB8888 premultiplies, stockes en little-endian comme le reste
/// des buffers `wl_shm` de ce moteur (voir `shm_manager::create_buffer`,
/// format 0) : 4 octets par pixel dans l'ordre B, G, R, A, ligne par ligne,
/// `width * height * 4` octets au total. `hotspot_x`/`hotspot_y` = le pixel
/// de l'image qui correspond a la position reelle de la souris (la pointe
/// de la fleche, le bout du doigt, le centre de la barre de texte).
#[derive(Debug, Clone)]
pub struct CursorImage {
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub pixels: Vec<u8>,
}

/// Identifiant d'un curseur deja envoye au compositeur par
/// `Window::register_cursor` - valable uniquement pour la `Window` qui l'a
/// cree (chaque fenetre a sa propre connexion Wayland, donc ses propres
/// `wl_buffer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorId(pub(crate) usize);

// Un curseur enregistre, cote compositeur : les pixels ont ete copies une
// fois pour toutes dans un `wl_buffer` (voir `Window::register_cursor`),
// seul son id et son point chaud restent necessaires pour l'afficher.
pub(crate) struct CursorBuffer {
    pub(crate) buffer_id: u32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) hotspot_x: i32,
    pub(crate) hotspot_y: i32,
}

// Tout ce qu'il faut a une `Window` pour changer de curseur a la volee.
// `enter_serial` : le protocole exige, pour `wl_pointer::set_cursor`, le
// serial du DERNIER `wl_pointer::enter` recu - `None` tant que la souris
// n'est jamais entree dans la fenetre (un `set_cursor` serait alors ignore :
// le curseur choisi est retenu dans `current` et applique a la prochaine
// entree, voir `Window::pointer_entered`).
pub(crate) struct CursorState {
    pub(crate) surface_id: u32,
    pub(crate) buffers: Vec<CursorBuffer>,
    pub(crate) current: Option<CursorId>,
    pub(crate) enter_serial: Option<u32>,
}

impl CursorState {
    pub(crate) fn new(surface_id: u32) -> CursorState {
        CursorState { surface_id, buffers: Vec::new(), current: None, enter_serial: None }
    }
}
