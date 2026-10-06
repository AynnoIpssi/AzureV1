// Interaction en temps reel avec l'arbre de `UiNode` deja construit : hit-
// testing souris (clic sur un bouton, focus + positionnement du curseur
// dans une textarea, glisser pour selectionner) et saisie clavier (frappe
// dans la textarea focalisee). Appele depuis la boucle d'evenements de
// `AzureWindow::run` a chaque clic/deplacement/touche recu.
//
// tree      parcours de l'arbre avec les boites, couches `position: fixed`
// pointer   survol, boutons, champs, infobulles
// drag      glisser-deposer (`<draggable>`, `<dropzone>`)
// scroll    molette, defilement horizontal, barre, ancres
// text      zones de texte (focus au clic, selection, frappe)
// select    selection du texte affiche a la souris, texte a copier
// focus     focus au clavier, Tab, Echap
// keyboard  touches -> `KeyInput`, dispositions de clavier
mod command_menu;
pub mod command_menu_consts {
    pub use super::command_menu::{HEAD_H, MAX_ROWS, MENU_W, PAD, ROW_H};
}
mod drag;
mod focus;
mod format_menu;
mod keyboard;
pub mod keymap;
mod keysyms;
mod pointer;
mod scroll;
mod select;
mod text;
mod tree;

pub use command_menu::{CommandItem, CommandMenu, parse_items as parse_commands};
pub use drag::{DRAG_THRESHOLD, Drag, DropTarget, Dropped, drag_grab, drag_moved, drag_target};
pub use format_menu::{FormatItem, FormatMenu};
pub use focus::{clear_focus, escape, focus_clicked, focus_next, key_on_focused};
pub use keyboard::{KeyInput, KeyboardLayout, detect_keyboard_layout, key_to_input, key_to_input_with};
pub use keymap::{Keymap, Modifiers};
pub use pointer::{cadrer_toiles, carry_toiles, survol_toile, drag_toile, release_toile, scroll_toile, ControlClick, HoverKind, any_dragging, button_id_at, click_controls, drag_slider, hover_group_at, hover_kind_at, release_all, set_button_text, set_field_text, toggle_button_at, tooltip_at};
pub use scroll::{ScrollDrag, animate_scroll, carry_scroll, carry_scroll_anchored, scroll_at, scroll_to_anchor, scroll_x_at, scrollbar_drag, scrollbar_grab};
pub use select::{TextPoint, TextSelection, apply_text_selection, clear_text_selection, nearest_text_point, select_all_text, selected_text, text_point_at, text_unit_at};
pub use text::{FocusedArea, delete_focused_range, focused_area_info, focused_caret, focused_rich_id, rich_focused, move_vertical, apply_rich_button, line_height, any_focused, ensure_cursor_visible, extend_selection_to, focus_textarea_at, type_into_focused};
pub use tree::{layer_paths, node_at, node_at_path, walk};
pub(crate) use tree::walk_with_paths;

/// `true` si `(x, y)` (coordonnees fenetre, comme `WindowMouseMove`) tombe
/// dans la boite `[box_x, box_x+box_w) x [box_y, box_y+box_h)` - la meme
/// regle de survol/clic partout (hit-testing, focus, mode souris au survol
/// dans `ui::services::draw_ui`).
pub fn hit(box_x: u32, box_y: u32, box_w: u32, box_h: u32, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && (x as u32) >= box_x
        && (x as u32) < box_x + box_w
        && (y as u32) >= box_y
        && (y as u32) < box_y + box_h
}
