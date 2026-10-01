use crate::ui::services::interact::HoverKind;

/// Les formes de curseur que sait dessiner `cursor::services::cursor_icon`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorKind {
    /// Fleche classique : rien d'interactif sous la souris.
    Default,
    /// Main a l'index tendu : un clic ici declenche quelque chose.
    Pointer,
    /// Barre verticale (I-beam) : on peut taper du texte ici.
    Text,
    /// Rond barre (`cursor: not-allowed`).
    NotAllowed,
    /// Croix fine (`crosshair`).
    Crosshair,
    /// Quatre fleches (`move`).
    Move,
    /// Anneau (`wait` / `progress`).
    Wait,
    /// Main (`grab` / `grabbing`).
    Grab,
    /// Fleche et point d'interrogation (`help`).
    Help,
    /// Double fleche horizontale (`ew-resize`, `col-resize`).
    ResizeHorizontal,
    /// Double fleche verticale (`ns-resize`, `row-resize`).
    ResizeVertical,
}

impl CursorKind {
    /// Toutes les formes, dans un ordre fixe - voir `CursorSet::new`.
    pub const ALL: [CursorKind; 11] = [
        CursorKind::Default,
        CursorKind::Pointer,
        CursorKind::Text,
        CursorKind::NotAllowed,
        CursorKind::Crosshair,
        CursorKind::Move,
        CursorKind::Wait,
        CursorKind::Grab,
        CursorKind::Help,
        CursorKind::ResizeHorizontal,
        CursorKind::ResizeVertical,
    ];

    /// La valeur CSS de `cursor` (`None` pour `auto` ou inconnue : le
    /// curseur est alors choisi selon l'element survole).
    pub fn from_css(name: &str) -> Option<CursorKind> {
        Some(match name {
            "default" => CursorKind::Default,
            "pointer" => CursorKind::Pointer,
            "text" | "vertical-text" => CursorKind::Text,
            "not-allowed" | "no-drop" => CursorKind::NotAllowed,
            "crosshair" | "cell" => CursorKind::Crosshair,
            "move" | "all-scroll" => CursorKind::Move,
            "wait" | "progress" => CursorKind::Wait,
            "grab" | "grabbing" => CursorKind::Grab,
            "help" => CursorKind::Help,
            "ew-resize" | "col-resize" | "e-resize" | "w-resize" => CursorKind::ResizeHorizontal,
            "ns-resize" | "row-resize" | "n-resize" | "s-resize" => CursorKind::ResizeVertical,
            _ => return None,
        })
    }

    /// La forme a afficher pour un survol du contenu (voir
    /// `ui::services::interact::hover_kind_at`) : la barre de texte sur un
    /// champ et sur un texte affiche (il se selectionne).
    pub fn from_hover(hover: HoverKind) -> CursorKind {
        match hover {
            HoverKind::Button => CursorKind::Pointer,
            HoverKind::TextArea | HoverKind::Text => CursorKind::Text,
            HoverKind::None => CursorKind::Default,
            HoverKind::Styled(kind) => kind,
        }
    }
}
