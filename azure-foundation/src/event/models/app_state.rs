use crate::event::models::keys::{KEY_LEFTCTRL, KEY_LEFTSHIFT, KEY_RIGHTCTRL, KEY_RIGHTSHIFT};
use crate::ui::models::ui_node::UiNode;
use crate::ui::services::interact::{HoverKind, KeyboardLayout, Modifiers};
use std::collections::HashSet;
use std::time::Instant;

/// Tout l'etat necessaire pour interpreter les evenements clavier/souris
/// sur un arbre de `UiNode` (survol, focus, curseur de saisie/selection,
/// presse-papiers, glisser, touche maintenue...) - independant de tout
/// systeme de fenetrage ou de rendu concret (pas de `Canvas`, pas d'id
/// Wayland). C'est la piece a construire et reutiliser directement (avec
/// `services::dispatch::handle_event`/`handle_tick`) si vous n'utilisez
/// pas `AzureWindow` mais voulez quand meme toute la logique d'interaction
/// (clic, saisie/edition de texte, selection, defilement...) toute faite.
pub struct EventState {
    /// L'arbre de widgets interactif - typiquement construit par
    /// `compiler::services::interpreter::build_ui`. Mute directement par
    /// `handle_event`/`handle_tick` au fil des evenements.
    pub ui_nodes: Vec<UiNode>,
    pub mouse_x: i32,
    pub mouse_y: i32,
    /// Ce que la souris survole actuellement - purement informatif, voir
    /// `ui::services::interact::HoverKind` (un consommateur peut s'en
    /// servir pour dessiner son propre indicateur de survol).
    pub hover: HoverKind,
    /// Le groupe de survol sous la souris (voir `interact::hover_group_at`).
    pub hover_group: Option<crate::layout::managers::layout_manager::Rect>,
    /// Visibilite courante du curseur de saisie clignotant (bascule
    /// toutes les `CARET_BLINK_INTERVAL`, voir `services::dispatch`).
    pub caret_visible: bool,
    pub last_blink: Instant,
    /// Touche actuellement maintenue (au plus une a la fois, comme un
    /// clavier physique) et l'instant ou elle doit se repeter ensuite -
    /// voir `services::dispatch::handle_tick`.
    pub held_key: Option<u32>,
    pub next_repeat_at: Option<Instant>,
    /// Touches modificatrices actuellement maintenues (Shift/Ctrl, gauche
    /// ou droite) - voir `shift_held`/`ctrl_held`.
    pub held_modifiers: HashSet<u32>,
    /// Verr. Maj, Verr. Num et la disposition active, tels que le
    /// compositeur les rapporte (voir `AzureWindow::run`).
    pub caps_lock: bool,
    pub num_lock: bool,
    pub group: u32,
    /// Touche morte en attente de la lettre suivante (voir
    /// `dispatch::handle_key_down`).
    pub dead_key: Option<char>,
    /// Dernier texte copie (textarea ou texte affiche). `AzureWindow` le
    /// donne au presse-papiers du systeme (voir `clipboard_changed`), et
    /// un collage lit d'abord celui du systeme.
    pub clipboard: String,
    /// `true` entre un clic gauche presse et relache : pendant ce temps,
    /// chaque deplacement de souris etend la selection de la textarea
    /// focalisee - un glisser pour selectionner.
    pub dragging: bool,
    /// `#id` du bouton touche par le DERNIER clic gauche (voir
    /// `interact::button_id_at`) - `None` si ce clic n'a touche aucun
    /// bouton identifie. Lu par `AzureWindow` pour `WindowContext::clicked`.
    pub clicked_id: Option<String>,
    /// Un bouton ou un champ a ete active AU CLAVIER (Entree, Espace,
    /// fleches) : `AzureWindow` appelle alors `on_click` comme pour un clic.
    /// Remis a `false` par qui le lit (voir `take_activation`).
    pub activated: bool,
    /// Infobulle affichee (texte, position de la souris), apres un instant
    /// de survol immobile (voir `dispatch::handle_tick`).
    pub tooltip: Option<(String, i32, i32)>,
    /// Depuis quand la souris n'a pas bouge.
    pub still_since: Instant,
    /// Barre de defilement tenue a la souris (voir `interact::scrollbar_grab`).
    pub scroll_drag: Option<crate::ui::services::interact::ScrollDrag>,
    /// Selection du texte affiche en cours (voir `interact::select`).
    pub text_selection: Option<crate::ui::services::interact::TextSelection>,
    /// Dernier appui gauche : instant, position et rang (1 simple, 2
    /// double, 3 triple clic).
    pub last_click: Option<(Instant, i32, i32, u8)>,
    /// `clipboard` vient d'etre rempli par une copie : `AzureWindow` le
    /// donne au presse-papiers du systeme (voir `take_clipboard_change`).
    pub clipboard_changed: bool,
    /// Element saisi a la souris (`<draggable>`), voir `interact::drag`.
    pub drag: Option<crate::ui::services::interact::Drag>,
    /// Element lache dans une zone : `AzureWindow` appelle `on_drop`
    /// (voir `take_dropped`).
    pub dropped: Option<crate::ui::services::interact::Dropped>,
    /// Panneau de mise en forme ouvert au clic droit (voir
    /// `interact::FormatMenu`).
    pub format_menu: Option<crate::ui::services::interact::FormatMenu>,
    /// Menu `/` au clavier ouvert (voir `interact::CommandMenu`).
    pub command_menu: Option<crate::ui::services::interact::CommandMenu>,
}

impl EventState {
    /// Cree un etat d'evenements frais pour `ui_nodes` - rien n'est
    /// focalise, aucune touche/bouton n'est maintenu.
    pub fn new(ui_nodes: Vec<UiNode>) -> EventState {
        EventState {
            ui_nodes,
            mouse_x: 0,
            mouse_y: 0,
            hover: HoverKind::None,
            caret_visible: true,
            last_blink: Instant::now(),
            held_key: None,
            next_repeat_at: None,
            held_modifiers: HashSet::new(),
            caps_lock: false,
            num_lock: false,
            group: 0,
            dead_key: None,
            clipboard: String::new(),
            dragging: false,
            clicked_id: None,
            activated: false,
            tooltip: None,
            scroll_drag: None,
            still_since: Instant::now(),
            text_selection: None,
            last_click: None,
            clipboard_changed: false,
            drag: None,
            dropped: None,
            format_menu: None,
            command_menu: None,
            hover_group: None,
        }
    }

    /// `true` une seule fois apres une activation au clavier.
    pub fn take_activation(&mut self) -> bool {
        std::mem::take(&mut self.activated)
    }

    /// `true` une seule fois apres une copie (voir `clipboard_changed`).
    pub fn take_clipboard_change(&mut self) -> bool {
        std::mem::take(&mut self.clipboard_changed)
    }

    /// L'element lache, une seule fois (voir `dropped`).
    pub fn take_dropped(&mut self) -> Option<crate::ui::services::interact::Dropped> {
        self.dropped.take()
    }

    /// Un glisser a passe le seuil (la copie suit la souris).
    pub fn drag_active(&self) -> bool {
        self.drag.as_ref().is_some_and(|d| d.active)
    }

    /// `true` si l'une des deux touches Shift est actuellement maintenue.
    pub fn shift_held(&self) -> bool {
        self.held_modifiers.contains(&KEY_LEFTSHIFT) || self.held_modifiers.contains(&KEY_RIGHTSHIFT)
    }

    /// `true` si l'une des deux touches Ctrl est actuellement maintenue.
    pub fn ctrl_held(&self) -> bool {
        self.held_modifiers.contains(&KEY_LEFTCTRL) || self.held_modifiers.contains(&KEY_RIGHTCTRL)
    }

    /// Tous les modificateurs du moment, pour traduire une touche avec
    /// `layout` (qui dit quelles touches donnent AltGr).
    pub fn modifiers(&self, layout: KeyboardLayout) -> Modifiers {
        Modifiers {
            shift: self.shift_held(),
            ctrl: self.ctrl_held(),
            altgr: self.held_modifiers.iter().any(|&k| layout.is_level3(k)),
            caps_lock: self.caps_lock,
            num_lock: self.num_lock,
            group: self.group,
        }
    }
}
