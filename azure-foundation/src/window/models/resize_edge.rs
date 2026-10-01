// Detection du bord/coin de fenetre sous le pointeur, pour le redimensionnement
// interactif au clic-glisser (voir `AzureWindow::run`,
// `xdg_manager::resize_toplevel`) - independant de `header_bar` : contrairement
// au deplacement (poignee = la barre d'en-tete uniquement), le redimensionnement
// se declenche depuis N'IMPORTE QUEL bord de la fenetre, header inclus.

/// Epaisseur (en pixels, cote fenetre) de la zone sensible le long de chaque
/// bord - suffisamment fine pour ne pas empieter sur le contenu normal, assez
/// large pour rester facile a attraper a la souris.
pub const RESIZE_BORDER: u32 = 8;

/// Correspond exactement aux valeurs de l'enum `xdg_toplevel::resize_edge`
/// (protocole xdg-shell) - transmises telles quelles comme argument `edges`
/// de la requete `resize` (voir `xdg_manager::resize_toplevel`), jamais
/// reinterpretees cote client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Top = 1,
    Bottom = 2,
    Left = 4,
    TopLeft = 5,
    BottomLeft = 6,
    Right = 8,
    TopRight = 9,
    BottomRight = 10,
}

impl ResizeEdge {
    pub fn to_wayland(self) -> u32 {
        self as u32
    }
}

/// Le bord/coin de la fenetre `(window_width, window_height)` sous le point
/// `(x, y)` (coordonnees fenetre absolues, comme `EventState::mouse_x/y`) -
/// `None` si `(x, y)` est hors fenetre ou trop loin de tout bord (au-dela de
/// `RESIZE_BORDER`). Un coin (ex. `TopLeft`) est retourne des que le point
/// est proche des DEUX bords qui le forment, pas seulement d'un - sinon les
/// coins, plus difficiles a viser qu'un bord droit, seraient impossibles a
/// attraper.
pub fn resize_edge_at(window_width: u32, window_height: u32, x: i32, y: i32) -> Option<ResizeEdge> {
    if x < 0 || y < 0 {
        return None;
    }
    let (x, y) = (x as u32, y as u32);
    if x >= window_width || y >= window_height {
        return None;
    }

    let near_left = x < RESIZE_BORDER;
    let near_right = x >= window_width.saturating_sub(RESIZE_BORDER);
    let near_top = y < RESIZE_BORDER;
    let near_bottom = y >= window_height.saturating_sub(RESIZE_BORDER);

    match (near_top, near_bottom, near_left, near_right) {
        (true, _, true, _) => Some(ResizeEdge::TopLeft),
        (true, _, _, true) => Some(ResizeEdge::TopRight),
        (_, true, true, _) => Some(ResizeEdge::BottomLeft),
        (_, true, _, true) => Some(ResizeEdge::BottomRight),
        (true, false, false, false) => Some(ResizeEdge::Top),
        (false, true, false, false) => Some(ResizeEdge::Bottom),
        (false, false, true, false) => Some(ResizeEdge::Left),
        (false, false, false, true) => Some(ResizeEdge::Right),
        _ => None,
    }
}
