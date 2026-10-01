use crate::cursor::models::cursor_kind::CursorKind;
use crate::cursor::services::cursor_icon;
use azure_engine::platform::wayland::models::cursor::CursorId;
use azure_engine::platform::wayland::models::window::Window as WaylandWindow;

/// Les icones generees, deja envoyees au compositeur pour UNE fenetre (voir
/// `WaylandWindow::register_cursor`) : changer de curseur ensuite ne recopie
/// jamais de pixels, ce n'est qu'un `set_cursor` - et encore, seulement si
/// la forme change reellement (voir `WaylandWindow::show_cursor`).
pub struct CursorSet {
    /// Une par forme, dans l'ordre de `CursorKind::ALL`.
    ids: Vec<CursorId>,
}

impl CursorSet {
    /// Genere toutes les icones (voir `cursor_icon::generate`) et les enregistre
    /// aupres de `window`, puis affiche la fleche par defaut.
    pub fn new(window: &mut WaylandWindow) -> Result<CursorSet, String> {
        let mut ids = Vec::with_capacity(CursorKind::ALL.len());
        for kind in CursorKind::ALL {
            ids.push(window.register_cursor(&cursor_icon::generate(kind))?);
        }
        let set = CursorSet { ids };
        set.show(window, CursorKind::Default)?;
        Ok(set)
    }

    pub fn show(&self, window: &mut WaylandWindow, kind: CursorKind) -> Result<(), String> {
        let index = CursorKind::ALL.iter().position(|k| *k == kind).expect("CursorKind::ALL couvre toutes les formes");
        window.show_cursor(self.ids[index])
    }
}
