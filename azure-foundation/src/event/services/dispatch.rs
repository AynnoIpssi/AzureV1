// Le coeur du systeme d'evenements : deux fonctions publiques,
// `handle_event` (un evenement recu) et `handle_tick` (le temps qui passe,
// pour le clignotement et la repetition de touche) - c'est tout ce qu'une
// application a besoin d'appeler pour obtenir gratuitement le
// comportement d'un vrai champ de texte (focus, curseur, selection a la
// souris ou au clavier, defilement, presse-papiers, raccourcis Ctrl+...)
// sur son arbre de `UiNode`, quelle que soit sa propre boucle de fenetrage.
use crate::event::models::app_state::EventState;
use crate::event::models::keys::{is_ctrl_key, is_shift_key, BTN_LEFT, BTN_RIGHT};
use crate::ui::services::interact::{self, KeyInput, KeyboardLayout};
use azure_core::rules::window_event::WindowEvent;
use std::time::{Duration, Instant};

/// Periode du clignotement du curseur de saisie - valeur typique des
/// environnements de bureau (GTK/GNOME tournent autour de 500-600ms).
pub const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);
/// Delai avant que maintenir une touche commence a la repeter, puis
/// cadence de la repetition elle-meme - valeurs typiques d'un clavier, pas
/// lues du vrai `wl_keyboard::repeat_info` du compositeur (voir la
/// limitation documentee sur `ui::services::interact::key_to_input`).
pub const KEY_REPEAT_INITIAL_DELAY: Duration = Duration::from_millis(400);
pub const KEY_REPEAT_INTERVAL: Duration = Duration::from_millis(40);
/// Survol immobile avant d'afficher une infobulle.
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

/// Interprete un `WindowEvent` recu (deplacement de souris, clic, molette,
/// touche pressee/relachee) et met a jour `state` en consequence. `layout`
/// vient de `ui::services::interact::detect_keyboard_layout` - a detecter
/// une seule fois au demarrage de l'application, pas a chaque appel.
/// `content` est la boite racine ou positionner `state.ui_nodes` (voir
/// `layout::managers::layout_manager::resolve`) - PAS forcement toute la
/// fenetre depuis `(0, 0)` : un appelant qui reserve une bande de chrome
/// (une barre d'en-tete auto-dessinee, voir `window::models::header_bar::content_box`
/// cote `AzureWindow`) passe ici la boite qui reste EN DESSOUS de ce
/// chrome, pour que `ui_nodes` ne s'y dessine/n'y reagisse jamais.
///
/// Retourne `true` si quelque chose a visuellement change (focus, texte,
/// curseur, selection, defilement, survol...) - c'est le signal pour
/// l'appelant de redessiner/republier sa fenetre.
pub fn handle_event(state: &mut EventState, event: WindowEvent, layout: KeyboardLayout, content: (u32, u32, u32, u32)) -> bool {
    match event {
        WindowEvent::WindowMouseMove(x, y) => handle_mouse_move(state, x, y, content),
        // Panneau de mise en forme ouvert : il prend le clic (element ou ailleurs).
        WindowEvent::WindowMouseButton(button, true) if button == BTN_LEFT && state.format_menu.is_some() => click_format_menu(state),
        // Menu `/` ouvert : un clic sur une commande la choisit, ailleurs le ferme.
        WindowEvent::WindowMouseButton(button, true) if button == BTN_LEFT && state.command_menu.is_some() => click_command_menu(state, content),
        WindowEvent::WindowMouseButton(button, true) if button == BTN_LEFT => handle_mouse_down(state, content),
        WindowEvent::WindowMouseButton(button, true) if button == BTN_RIGHT => open_format_menu(state, content),
        WindowEvent::WindowMouseButton(button, false) if button == BTN_LEFT => {
            // Une toile tenue : son geste finit, l'app recoit son resultat.
            if let Some(evenement) = interact::release_toile(&mut state.ui_nodes, state.mouse_x, state.mouse_y, content) {
                state.clicked_id = Some(evenement);
                state.activated = true;
            }
            state.dragging = false;
            state.scroll_drag = None;
            let dropped = finish_drag(state);
            interact::release_all(&mut state.ui_nodes) || dropped
        }
        // La page defile : le menu `/` ne suivrait plus son curseur.
        WindowEvent::WindowScroll(_) if state.command_menu.take().is_some() => true,
        // Au-dessus d'une toile : sa vue bouge (Ctrl : zoom), pas la page.
        WindowEvent::WindowScroll(delta)
            if {
                let (ctrl, maj) = (state.ctrl_held(), state.shift_held());
                interact::scroll_toile(&mut state.ui_nodes, state.mouse_x, state.mouse_y, delta, ctrl, maj, content).is_some()
            } =>
        {
            true
        }
        // Maj + molette : defilement horizontal, comme dans un navigateur.
        WindowEvent::WindowScroll(delta) if state.shift_held() => {
            interact::scroll_x_at(&mut state.ui_nodes, state.mouse_x, state.mouse_y, delta, content)
        }
        WindowEvent::WindowScroll(delta) => {
            interact::scroll_at(&mut state.ui_nodes, state.mouse_x, state.mouse_y, delta, content)
        }
        // Pave tactile vers la gauche ou la droite au-dessus d'une toile : sa
        // vue glisse de cote.
        WindowEvent::WindowScrollH(delta)
            if {
                let ctrl = state.ctrl_held();
                interact::scroll_toile(&mut state.ui_nodes, state.mouse_x, state.mouse_y, delta, ctrl, true, content).is_some()
            } =>
        {
            true
        }
        WindowEvent::WindowScrollH(delta) => {
            interact::scroll_x_at(&mut state.ui_nodes, state.mouse_x, state.mouse_y, delta, content)
        }
        WindowEvent::WindowKeyPress(key, true) if is_shift_key(key) || is_ctrl_key(key) || layout.is_level3(key) => {
            state.held_modifiers.insert(key);
            false
        }
        WindowEvent::WindowKeyPress(key, true) => handle_key_down(state, key, layout, content),
        WindowEvent::WindowKeyPress(key, false) => {
            handle_key_up(state, key);
            false
        }
        _ => false,
    }
}

/// A appeler regulierement (independamment de tout `WindowEvent` recu) -
/// par exemple a chaque "tic" d'une minuterie a ~60Hz - pour faire avancer
/// le clignotement du curseur de saisie et la repetition d'une touche
/// maintenue, qui ne dependent que du temps ecoule, pas d'un evenement.
/// Memes parametres/retour que `handle_event`.
pub fn handle_tick(state: &mut EventState, layout: KeyboardLayout, content: (u32, u32, u32, u32)) -> bool {
    // Defilement fluide des conteneurs (voir `interact::animate_scroll`) :
    // le contenu bouge sous une souris immobile, donc ce qu'elle survole
    // (la forme du curseur) doit etre recalcule.
    let mut changed = interact::animate_scroll(&mut state.ui_nodes);
    // Une toile a une nouvelle vue : cadree sur son dessin.
    changed |= interact::cadrer_toiles(&mut state.ui_nodes, content);

    // Infobulle : apres un instant sans bouger au-dessus d'un element qui en
    // a une. Cherchee une seule fois par pause de la souris (et de nouveau
    // si le contenu a defile dessous) : sinon toute la mise en page serait
    // refaite a chaque tic tant que la souris ne bouge pas.
    if changed {
        state.tooltip_sought = false;
    }
    if state.tooltip.is_none() && !state.tooltip_sought && state.still_since.elapsed() >= TOOLTIP_DELAY {
        state.tooltip_sought = true;
        if let Some(text) = interact::tooltip_at(&state.ui_nodes, state.mouse_x, state.mouse_y, content) {
            state.tooltip = Some((text, state.mouse_x, state.mouse_y));
            changed = true;
        }
    }
    if changed {
        state.hover = interact::hover_kind_at(&state.ui_nodes, state.mouse_x, state.mouse_y, content);
    }

    // Clignotement : seulement s'il y a effectivement une textarea
    // focalisee a afficher, sinon on redessinerait pour rien.
    if interact::any_focused(&state.ui_nodes) && state.last_blink.elapsed() >= CARET_BLINK_INTERVAL {
        state.caret_visible = !state.caret_visible;
        state.last_blink = Instant::now();
        changed = true;
    }

    // Repetition : la touche maintenue est re-appliquee comme si elle
    // venait d'etre pressee a nouveau, a intervalle regulier - evite
    // d'avoir a la marteler soi-meme.
    if let (Some(key), Some(fire_at)) = (state.held_key, state.next_repeat_at)
        && Instant::now() >= fire_at {
            let key_changed = interact::key_to_input_with(key, layout, state.modifiers(layout))
                .map(|input| match interact::key_on_focused(&mut state.ui_nodes, input, content) {
                    Some(id) => {
                        state.clicked_id = id;
                        state.activated = true;
                        true
                    }
                    None => edit_focused(state, input, content),
                })
                .unwrap_or(false);
            state.next_repeat_at = Some(Instant::now() + KEY_REPEAT_INTERVAL);
            if key_changed {
                interact::ensure_cursor_visible(&mut state.ui_nodes, content);
            }
            changed = changed || key_changed;
        }

    changed
}

/// Clic droit : du texte est selectionne dans une zone de texte riche, le
/// panneau de mise en forme s'ouvre sous la souris.
fn open_format_menu(state: &mut EventState, content: (u32, u32, u32, u32)) -> bool {
    let had = state.format_menu.take().is_some();
    match interact::focused_area_info(&state.ui_nodes) {
        Some(f) if f.rich && f.has_selection => {
            state.format_menu = Some(interact::FormatMenu::at(&f.id, state.mouse_x, state.mouse_y, content));
            state.tooltip = None;
            true
        }
        _ => had,
    }
}

/// Clic gauche avec le panneau ouvert : applique l'element touche (la zone
/// garde sa selection), puis ferme.
fn click_format_menu(state: &mut EventState) -> bool {
    let Some(menu) = state.format_menu.take() else { return false };
    if let Some(mark) = menu.mark_at(state.mouse_x, state.mouse_y) {
        interact::apply_rich_button(&mut state.ui_nodes, &format!("rt-{}-{mark}", menu.area), None);
        crate::layout::managers::web_layout::forget_sizes(&state.ui_nodes);
        state.caret_visible = true;
    }
    true
}

/// Clic avec le menu `/` ouvert : une commande est choisie ; ailleurs, le
/// menu se ferme et le clic suit son cours.
fn click_command_menu(state: &mut EventState, content: (u32, u32, u32, u32)) -> bool {
    let row = state.command_menu.as_ref().and_then(|m| m.row_at(state.mouse_x, state.mouse_y));
    let inside = state.command_menu.as_ref().is_some_and(|m| contains_rect(m.rect(), state.mouse_x, state.mouse_y));
    match (row, inside) {
        (Some(i), _) => {
            if let Some(m) = &mut state.command_menu {
                m.selected = i;
            }
            pick_command(state)
        }
        (None, true) => false,
        (None, false) => {
            state.command_menu = None;
            handle_mouse_down(state, content);
            true
        }
    }
}

fn contains_rect(r: crate::layout::managers::layout_manager::Rect, x: i32, y: i32) -> bool {
    crate::layout::managers::layout_manager::contains(r, x, y)
}

/// Valide la commande choisie : `/recherche` quitte le texte, l'app recoit
/// `commande-<zone>@<code>@<position>`.
fn pick_command(state: &mut EventState) -> bool {
    let Some(menu) = state.command_menu.take() else { return false };
    let Some(item) = menu.current() else { return false };
    let Some(f) = interact::focused_area_info(&state.ui_nodes).filter(|f| f.id == menu.area) else { return true };
    interact::delete_focused_range(&mut state.ui_nodes, menu.start, f.cursor.max(menu.start));
    crate::layout::managers::web_layout::forget_sizes(&state.ui_nodes);
    state.clicked_id = Some(format!("commande-{}@{}@{}", menu.area, item.code, menu.start));
    state.activated = true;
    state.held_key = None;
    true
}

/// Apres une touche : le menu `/` suit ce qui est tape apres le `/`, et se
/// ferme si le curseur en sort, si le `/` est efface, si un espace suit
/// une recherche sans resultat, ou si c'est un espace ou un autre `/` qui
/// suit le `/` (une division, un commentaire `//` : pas une commande).
fn refresh_command_menu(state: &mut EventState) {
    let Some(menu) = &mut state.command_menu else { return };
    let keep = match interact::focused_area_info(&state.ui_nodes) {
        Some(f) if f.id == menu.area && f.cursor > menu.start && f.text.chars().nth(menu.start) == Some('/') => {
            let q: String = f.text.chars().skip(menu.start + 1).take(f.cursor - menu.start - 1).collect();
            if q.contains('\n') || q.chars().count() > 30 || q.starts_with([' ', '/']) {
                false
            } else {
                menu.filter(&q);
                !(menu.shown.is_empty() && q.ends_with(' '))
            }
        }
        _ => false,
    };
    if !keep {
        state.command_menu = None;
    }
}

fn handle_mouse_move(state: &mut EventState, x: i32, y: i32, content: (u32, u32, u32, u32)) -> bool {
    state.mouse_x = x;
    state.mouse_y = y;
    state.still_since = Instant::now();
    state.tooltip_sought = false;
    // Le panneau de mise en forme et le menu `/` suivent le survol.
    if state.format_menu.is_some() || state.command_menu.as_ref().is_some_and(|m| contains_rect(m.rect(), x, y)) {
        return true;
    }
    // La souris bouge : l'infobulle disparait.
    let mut changed = state.tooltip.take().is_some();

    // Ne redessine que si le "mode souris" a reellement change (entree/
    // sortie d'un bouton ou d'une textarea), pas a chaque pixel de
    // deplacement.
    let new_hover = interact::hover_kind_at(&state.ui_nodes, x, y, content);
    if new_hover != state.hover {
        state.hover = new_hover;
        changed = true;
    }
    let group = interact::hover_group_at(&state.ui_nodes, x, y, content);
    if group != state.hover_group {
        state.hover_group = group;
        changed = true;
    }

    // Glisser-deposer : apres le seuil, la copie suit la souris et la zone
    // survolee se calcule a chaque pas.
    if state.dragging
        && let Some(mut drag) = state.drag.take()
    {
        if !drag.active && interact::drag_moved(&drag, x, y) {
            drag.active = true;
            state.text_selection = None;
            interact::clear_text_selection(&mut state.ui_nodes);
            interact::release_all(&mut state.ui_nodes);
        }
        let active = drag.active;
        if active {
            drag.target = interact::drag_target(&state.ui_nodes, &drag, x, y, content);
            state.hover = interact::HoverKind::Styled(crate::cursor::models::cursor_kind::CursorKind::Grab);
        }
        state.drag = Some(drag);
        if active {
            return true;
        }
    }

    // Barre de defilement tenue : le contenu suit la souris.
    if let Some(drag) = &state.scroll_drag {
        let drag = drag.clone();
        return interact::scrollbar_drag(&mut state.ui_nodes, &drag, y, content) || changed;
    }

    // Curseur (slider) tenu : suit la souris.
    if state.dragging && interact::drag_slider(&mut state.ui_nodes, x, content) {
        changed = true;
    }
    // Toile tenue : la boite, la vue ou le trait suivent.
    if state.dragging && interact::drag_toile(&mut state.ui_nodes, x, y, content) {
        changed = true;
    }
    // Sans bouton : la boite survolee d'une toile a focus ressort.
    if !state.dragging && interact::survol_toile(&mut state.ui_nodes, x, y, content) {
        changed = true;
    }

    // Glisser depuis un texte : la selection suit la souris.
    if state.dragging && state.text_selection.is_some() {
        if let Some(point) = interact::nearest_text_point(&state.ui_nodes, x, y, content)
            && let Some(selection) = state.text_selection.as_mut()
            && selection.focus != point
        {
            selection.focus = point;
            let selection = selection.clone();
            interact::apply_text_selection(&mut state.ui_nodes, &selection);
            changed = true;
        }
        return changed;
    }

    // Glisser en cours (bouton gauche maintenu depuis un clic dans une
    // textarea) : etend sa selection jusqu'au point courant, comme un vrai
    // champ de texte.
    if state.dragging && interact::extend_selection_to(&mut state.ui_nodes, x, y, content) {
        state.caret_visible = true;
        state.last_blink = Instant::now();
        changed = true;
    }

    changed
}

fn handle_mouse_down(state: &mut EventState, content: (u32, u32, u32, u32)) -> bool {
    let (x, y) = (state.mouse_x, state.mouse_y);
    state.tooltip = None;
    // Barre de defilement saisie : rien d'autre ne recoit ce clic.
    if let Some(drag) = interact::scrollbar_grab(&state.ui_nodes, x, y, content) {
        state.scroll_drag = Some(drag);
        return true;
    }
    // Un appui efface la selection de texte (sauf Maj+clic, qui l'etend).
    let previous_selection = state.text_selection.take();
    let had_selection = interact::clear_text_selection(&mut state.ui_nodes);
    let click_rank = click_rank(state, x, y);
    // La zone de texte riche en cours d'edition (pour `<richbar>` sans `pour`).
    let rich_before = interact::focused_rich_id(&state.ui_nodes);
    // Le bouton / champ clique prend le focus (sans contour : c'est la souris).
    interact::focus_clicked(&mut state.ui_nodes, x, y, content);
    // Champs (cases, curseurs, listes...) : une liste ouverte prend le clic.
    let controls = interact::click_controls(&mut state.ui_nodes, x, y, content);
    if controls.consumed {
        state.clicked_id = controls.id;
        return true;
    }
    state.clicked_id = interact::button_id_at(&state.ui_nodes, x, y, content);
    // Bouton `#ancre-<id>` : defile jusqu'a l'element `#<id>`.
    let anchored = follow_anchor(state, content);
    let toggled = interact::toggle_button_at(&mut state.ui_nodes, x, y, content) || controls.changed;
    let focus_changed = interact::focus_textarea_at(&mut state.ui_nodes, x, y, content);
    // Barre d'outils du texte riche : la zone reprend le focus (que le clic
    // vient de lui retirer), avec sa selection (voir `<richbar>`).
    let rich_button = state.clicked_id.as_deref().is_some_and(|id| interact::apply_rich_button(&mut state.ui_nodes, id, rich_before.as_deref()));
    let focus_changed = focus_changed || rich_button;
    // Appui sur un element saisissable (hors champ et zone de texte) : un
    // glisser-deposer peut commencer. Le clic d'un bouton dedans attend le
    // relachement (voir `finish_drag`).
    state.drag = None;
    if !controls.changed && !focus_changed
        && let Some(mut drag) = interact::drag_grab(&state.ui_nodes, x, y, content)
    {
        drag.click = state.clicked_id.take();
        state.drag = Some(drag);
    }
    // Point de depart d'un glisser eventuel - sans effet si le clic n'a
    // touche aucune textarea (voir `interact::extend_selection_to`, qui ne
    // fait rien sans textarea focalisee).
    state.dragging = true;

    // Appui sur un texte (hors bouton et champ) : debut d'une selection.
    if state.clicked_id.is_none() && !toggled && !interact::any_focused(&state.ui_nodes) {
        let selected = start_text_selection(state, previous_selection, click_rank, content);
        if !focus_changed {
            return anchored || had_selection || selected;
        }
    }
    if !toggled && !focus_changed {
        return anchored || had_selection;
    }
    // Un clic remet le curseur bien visible tout de suite, plutot que de
    // laisser l'utilisateur tomber sur une phase "invisible" du
    // clignotement.
    state.caret_visible = true;
    state.last_blink = Instant::now();
    interact::ensure_cursor_visible(&mut state.ui_nodes, content);
    true
}

/// Bouton relache pendant un glisser-deposer : lache dans une zone
/// (`state.dropped`), ou simple clic rendu si rien n'a bouge. `true` si
/// l'ecran change (la copie disparait).
fn finish_drag(state: &mut EventState) -> bool {
    let Some(drag) = state.drag.take() else { return false };
    if !drag.active {
        if let Some(id) = drag.click {
            state.clicked_id = Some(id);
            state.activated = true;
        }
        return false;
    }
    if let Some(target) = drag.target {
        state.dropped = Some(interact::Dropped { source: drag.source, target: target.zone, position: target.position });
    }
    true
}

/// Double-clic : deux appuis a moins de ce delai, au meme endroit.
pub const DOUBLE_CLICK_DELAY: Duration = Duration::from_millis(400);

/// Rang de cet appui : 1, puis 2 (double-clic), 3 (triple-clic), puis on
/// recommence.
fn click_rank(state: &mut EventState, x: i32, y: i32) -> u8 {
    let rank = match state.last_click {
        Some((at, px, py, rank)) if at.elapsed() <= DOUBLE_CLICK_DELAY && (px - x).abs() <= 4 && (py - y).abs() <= 4 => rank % 3 + 1,
        _ => 1,
    };
    state.last_click = Some((Instant::now(), x, y, rank));
    rank
}

/// Appui sur un texte : point de depart (simple clic), mot (double), tout
/// le texte (triple) ; Maj+clic etend la selection precedente.
fn start_text_selection(state: &mut EventState, previous: Option<interact::TextSelection>, rank: u8, content: (u32, u32, u32, u32)) -> bool {
    let Some(point) = interact::text_point_at(&state.ui_nodes, state.mouse_x, state.mouse_y, content) else { return false };
    let selection = match (previous, rank) {
        (Some(previous), 1) if state.shift_held() => interact::TextSelection { anchor: previous.anchor, focus: point },
        (_, 1) => interact::TextSelection::at(point),
        (_, rank) => match interact::text_unit_at(&state.ui_nodes, &point, rank == 3) {
            Some(unit) => unit,
            None => interact::TextSelection::at(point),
        },
    };
    interact::apply_text_selection(&mut state.ui_nodes, &selection);
    let visible = selection.anchor != selection.focus;
    state.text_selection = Some(selection);
    visible
}

/// Le dernier bouton active est un lien d'ancre (`#ancre-<id>`) : defile
/// jusqu'a sa cible.
fn follow_anchor(state: &mut EventState, content: (u32, u32, u32, u32)) -> bool {
    match state.clicked_id.as_deref().and_then(|id| id.strip_prefix("ancre-")) {
        Some(target) => {
            let target = target.to_string();
            interact::scroll_to_anchor(&mut state.ui_nodes, &target, content)
        }
        None => false,
    }
}

fn handle_key_down(state: &mut EventState, key: u32, layout: KeyboardLayout, content: (u32, u32, u32, u32)) -> bool {
    let input = interact::key_to_input_with(key, layout, state.modifiers(layout));
    state.tooltip = None;

    // Echap pendant un glisser-deposer : l'annule.
    if matches!(input, Some(KeyInput::Escape)) && state.drag_active() {
        state.drag = None;
        return true;
    }
    // Echap (ou toute touche) ferme le panneau de mise en forme.
    if state.format_menu.take().is_some() && matches!(input, Some(KeyInput::Escape)) {
        return true;
    }

    // Touche morte : retenue, puis composee avec la lettre suivante (`^`
    // puis `e` -> `ê`). Rien a composer : l'accent puis la lettre, comme
    // dans les autres apps.
    let input = match (input, state.dead_key.take()) {
        (Some(KeyInput::Dead(a)), Some(b)) if a == b => Some(KeyInput::Char(a)),
        (Some(KeyInput::Dead(a)), _) => {
            state.dead_key = Some(a);
            return false;
        }
        (Some(KeyInput::Char(c)), Some(a)) => match interact::keymap::compose(a, c) {
            Some(composed) => Some(KeyInput::Char(composed)),
            None => {
                interact::type_into_focused(&mut state.ui_nodes, KeyInput::Char(a), &mut state.clipboard);
                Some(KeyInput::Char(c))
            }
        },
        (other, _) => other,
    };

    // Menu `/` ouvert : ↑/↓ choisissent, Entree ou Tab valident, Echap ferme.
    if let Some(menu) = &mut state.command_menu {
        match input {
            Some(KeyInput::Up(_)) => {
                menu.step(false);
                return true;
            }
            Some(KeyInput::Down(_)) => {
                menu.step(true);
                return true;
            }
            Some(KeyInput::Enter | KeyInput::Tab(_)) if menu.current().is_some() => return pick_command(state),
            Some(KeyInput::Escape) => {
                state.command_menu = None;
                return true;
            }
            _ => {}
        }
    }

    // Aucun champ en saisie : Ctrl+C copie le texte selectionne a la
    // souris, Ctrl+A selectionne toute la page.
    if !interact::any_focused(&state.ui_nodes) {
        match input {
            Some(KeyInput::Copy) => {
                if let Some(text) = interact::selected_text(&state.ui_nodes, content) {
                    state.clipboard = text;
                    state.clipboard_changed = true;
                }
                return false;
            }
            Some(KeyInput::SelectAll) => {
                state.text_selection = interact::select_all_text(&mut state.ui_nodes);
                return state.text_selection.is_some();
            }
            _ => {}
        }
    }

    // Clavier sans souris : Tab, Echap, et les touches d'un bouton ou d'un
    // champ focalise (Entree, Espace, fleches).
    match input {
        Some(KeyInput::Tab(backwards)) => {
            state.caret_visible = true;
            state.last_blink = Instant::now();
            return interact::focus_next(&mut state.ui_nodes, backwards, content);
        }
        Some(KeyInput::Escape) => {
            return match interact::escape(&mut state.ui_nodes, content) {
                Some(Some(id)) => {
                    state.clicked_id = Some(id);
                    state.activated = true;
                    true
                }
                Some(None) => true,
                None => {
                    interact::clear_focus(&mut state.ui_nodes);
                    true
                }
            };
        }
        Some(i) => {
            if let Some(id) = interact::key_on_focused(&mut state.ui_nodes, i, content) {
                state.clicked_id = id;
                state.activated = true;
                follow_anchor(state, content);
                state.held_key = if i.is_repeatable() { Some(key) } else { None };
                state.next_repeat_at = state.held_key.map(|_| Instant::now() + KEY_REPEAT_INITIAL_DELAY);
                return true;
            }
        }
        None => {}
    }
    // Zone `entree` : Entree (sans Maj) est pour l'app, avec la position du
    // curseur (pour couper le bloc la).
    let focused = interact::focused_area_info(&state.ui_nodes);
    if let (Some(KeyInput::Enter), Some(f)) = (input, &focused)
        && f.enter_submits
        && !state.shift_held()
    {
        state.clicked_id = Some(format!("entree-{}@{}", f.id, f.cursor));
        state.activated = true;
        state.held_key = None;
        return true;
    }
    // Retour arriere tout au debut (sans selection) : pour l'app aussi
    // (supprimer le bloc vide, le coller au precedent).
    if let (Some(KeyInput::Backspace), Some(f)) = (input, &focused)
        && f.enter_submits
        && f.cursor == 0
        && !f.has_selection
    {
        state.clicked_id = Some(format!("retour-{}", f.id));
        state.activated = true;
        state.held_key = None;
        return true;
    }
    // Maj+Entree dans une zone riche : un simple retour a la ligne.
    let input = match (input, &focused) {
        (Some(KeyInput::Enter), Some(f)) if f.enter_submits => Some(KeyInput::Char('\n')),
        _ => input,
    };
    let changed = input.map(|i| edit_focused(state, i, content)).unwrap_or(false);
    // Zone `commandes` : `/` tape en debut de mot ouvre le menu au clavier
    // (liste donnee), ou previent l'app (`slash-<id>`).
    if state.command_menu.is_some() {
        refresh_command_menu(state);
    }
    if changed
        && matches!(input, Some(KeyInput::Char('/')))
        && let Some(f) = &focused
        && f.commands
        && f.before.is_none_or(char::is_whitespace)
    {
        if f.command_list.is_empty() {
            state.clicked_id = Some(format!("slash-{}", f.id));
            state.activated = true;
        } else if let Some(caret) = interact::focused_caret(&mut state.ui_nodes, content) {
            state.command_menu = Some(interact::CommandMenu::open(&f.id, f.cursor, interact::parse_commands(&f.command_list), caret, content));
        }
    }
    // Copie depuis un champ : aussi pour les autres apps.
    if matches!(input, Some(KeyInput::Copy | KeyInput::Cut)) && !state.clipboard.is_empty() {
        state.clipboard_changed = true;
    }

    // Maintenir la touche la fera repeter au prochain tic une fois
    // `KEY_REPEAT_INITIAL_DELAY` ecoule - meme si cette premiere frappe
    // n'a rien change (ex: Backspace sur un champ vide), au cas ou du
    // texte apparaisse avant la repetition. Sauf pour les touches
    // marquees non-repetables (voir `KeyInput::is_repeatable` -
    // Selectionner tout / Copier / Couper / Coller, contrairement a
    // Ctrl+Z qui doit au contraire pouvoir se repeter).
    if input.is_some_and(|i| i.is_repeatable()) {
        state.held_key = Some(key);
        state.next_repeat_at = Some(Instant::now() + KEY_REPEAT_INITIAL_DELAY);
    } else {
        state.held_key = None;
        state.next_repeat_at = None;
    }
    state.caret_visible = true;
    state.last_blink = Instant::now();

    if changed {
        interact::ensure_cursor_visible(&mut state.ui_nodes, content);
    }
    changed
}

/// Une touche pour la zone de texte focalisee. Haut / bas vont a la ligne
/// voisine, ce qui demande les boites (voir `interact::move_vertical`).
fn edit_focused(state: &mut EventState, input: KeyInput, content: (u32, u32, u32, u32)) -> bool {
    match input {
        KeyInput::Up(extend) => interact::move_vertical(&mut state.ui_nodes, false, extend, content),
        KeyInput::Down(extend) => interact::move_vertical(&mut state.ui_nodes, true, extend, content),
        other => {
            let changed = interact::type_into_focused(&mut state.ui_nodes, other, &mut state.clipboard);
            // Un texte riche grandit avec ses lignes : la page se remet en page.
            if changed && interact::rich_focused(&state.ui_nodes) {
                crate::layout::managers::web_layout::forget_sizes(&state.ui_nodes);
            }
            changed
        }
    }
}

fn handle_key_up(state: &mut EventState, key: u32) {
    if state.held_modifiers.remove(&key) {
        // Maj, Ctrl ou AltGr relache.
    } else if state.held_key == Some(key) {
        state.held_key = None;
        state.next_repeat_at = None;
    }
}
