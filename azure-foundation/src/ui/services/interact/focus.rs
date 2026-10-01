// Focus au clavier : Tab / Maj+Tab, clic, touches sur le bouton ou le
// champ focalise, Echap.
use crate::layout::managers::web_layout::{is_fixed, layout_roots};
use crate::layout::managers::layout_manager::{container_layout, contains, Rect};
use crate::ui::models::control::ControlKind;
use crate::ui::models::ui_node::UiNode;
use super::keyboard::KeyInput;
use super::tree::{at_point, for_each_layer, for_each_mut, layer_paths, modal_layer, node_at, node_at_mut, walk_mut};

fn focusable(node: &UiNode) -> bool {
    if !node.decoration().visible {
        return false;
    }
    match node {
        UiNode::Button(_) | UiNode::TextArea(_) => true,
        UiNode::Control(c) => c.kind.interactive() && !c.disabled,
        _ => false,
    }
}

fn is_focused(node: &UiNode) -> bool {
    match node {
        UiNode::Button(b) => b.focused,
        UiNode::TextArea(t) => t.focused,
        UiNode::Control(c) => c.focused,
        _ => false,
    }
}

fn set_focus(node: &mut UiNode, on: bool, ring: bool) {
    match node {
        UiNode::Button(b) => {
            b.focused = on;
            b.focus_ring = on && ring;
        }
        UiNode::TextArea(t) => t.focused = on,
        UiNode::Control(c) => {
            c.focused = on;
            c.focus_ring = on && ring;
        }
        _ => {}
    }
}

/// Les elements focalisables dans l'ordre du document (page puis couches) ;
/// si une modale est ouverte, seulement les siens.
fn focus_order(nodes: &[UiNode], parent: (u32, u32, u32, u32)) -> Vec<Vec<usize>> {
    fn collect(nodes: &[UiNode], prefix: &mut Vec<usize>, out: &mut Vec<Vec<usize>>, root_is_layer: bool) {
        for (i, node) in nodes.iter().enumerate() {
            if is_fixed(node) && !(root_is_layer && prefix.is_empty()) {
                continue;
            }
            prefix.push(i);
            if focusable(node) {
                out.push(prefix.clone());
            }
            if let UiNode::Container(c) = node {
                let mut inner = Vec::new();
                collect(&c.children, &mut Vec::new(), &mut inner, false);
                out.extend(inner.into_iter().map(|p| [prefix.clone(), p].concat()));
            }
            prefix.pop();
        }
    }
    let within = |path: &[usize]| -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        collect(std::slice::from_ref(node_at(nodes, path)), &mut Vec::new(), &mut out, true);
        out.into_iter().map(|p| [path.to_vec(), p[1..].to_vec()].concat()).collect()
    };
    if let Some(modal) = modal_layer(nodes, parent) {
        return within(&modal);
    }
    let mut order = Vec::new();
    collect(nodes, &mut Vec::new(), &mut order, false);
    for path in layer_paths(nodes) {
        order.extend(within(&path));
    }
    order
}

/// Tab / Maj+Tab : passe au champ suivant / precedent (en boucle). Retourne
/// `true` si le focus a change.
pub fn focus_next(nodes: &mut [UiNode], backwards: bool, parent: (u32, u32, u32, u32)) -> bool {
    let order = focus_order(nodes, parent);
    if order.is_empty() {
        return false;
    }
    let current = order.iter().position(|p| is_focused(node_at(nodes, p)));
    let next = match (current, backwards) {
        (None, false) => 0,
        (None, true) => order.len() - 1,
        (Some(i), false) => (i + 1) % order.len(),
        (Some(i), true) => (i + order.len() - 1) % order.len(),
    };
    clear_focus(nodes);
    set_focus(node_at_mut(nodes, &order[next]), true, true);
    reveal_focused(nodes, parent);
    true
}

/// Plus rien de focalise (et les listes ouvertes se ferment).
pub fn clear_focus(nodes: &mut [UiNode]) {
    for_each_mut(nodes, &mut |n| {
        set_focus(n, false, false);
        if let UiNode::Control(c) = n {
            c.open = false;
        }
    });
}

/// Un clic souris sur un bouton / un champ le focalise (sans contour).
pub fn focus_clicked(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) {
    for_each_mut(nodes, &mut |n| {
        if !matches!(n, UiNode::TextArea(_)) {
            set_focus(n, false, false)
        }
    });
    at_point(nodes, x, y, parent, |slice| {
        walk_mut(slice, parent, &mut |node, own_box| {
            if matches!(node, UiNode::Button(_) | UiNode::Control(_)) && focusable(node) && contains(own_box, x, y) {
                set_focus(node, true, false);
                return true;
            }
            false
        })
    });
}

/// Fait defiler les conteneurs pour que l'element focalise soit visible.
fn reveal_focused(nodes: &mut [UiNode], parent: (u32, u32, u32, u32)) {
    fn reveal(node: &mut UiNode, own_box: Rect) -> Option<Rect> {
        if is_focused(node) {
            return Some(own_box);
        }
        let UiNode::Container(c) = node else { return None };
        let layout = container_layout(c, own_box);
        let found = c.children.iter_mut().zip(layout.children).filter(|(ch, _)| !is_fixed(ch)).find_map(|(ch, b)| reveal(ch, b))?;
        if c.layout.scrollable() && layout.max_scroll > 0 {
            let (top, bottom) = (layout.visible.1, layout.visible.1 + layout.visible.3 as i32);
            let delta = if found.1 < top { found.1 - top - 8 } else if found.1 + found.3 as i32 > bottom { found.1 + found.3 as i32 - bottom + 8 } else { 0 };
            if delta != 0 {
                let offset = (c.scroll_offset as i32 + delta).clamp(0, layout.max_scroll as i32) as u32;
                c.scroll_offset = offset;
                c.scroll_target = offset;
                return Some((found.0, found.1 - delta, found.2, found.3));
            }
        }
        Some(found)
    }
    for_each_layer(nodes, parent, &mut |slice| {
        let roots = layout_roots(slice, parent);
        for (node, b) in slice.iter_mut().zip(roots) {
            let _ = reveal(node, b);
        }
    });
}

/// Une touche pour le bouton ou le champ focalise (pas une zone de texte).
/// Retourne `Some(id)` si quelque chose a ete active ou a change.
pub fn key_on_focused(nodes: &mut [UiNode], input: KeyInput, parent: (u32, u32, u32, u32)) -> Option<Option<String>> {
    let mut result = None;
    let mut radio: Option<(String, i32)> = None;
    for_each_mut(nodes, &mut |node| {
        if result.is_some() || !is_focused(node) {
            return;
        }
        match node {
            UiNode::Button(b) if matches!(input, KeyInput::Enter | KeyInput::Char(' ')) => result = Some(Some(b.id.clone()).filter(|id| !id.is_empty())),
            // Entree dans un champ d'une ligne : le valide (on_click avec
            // l'id du champ), comme un formulaire web.
            UiNode::TextArea(t) if t.single_line && input == KeyInput::Enter && !t.id.is_empty() => result = Some(Some(t.id.clone())),
            UiNode::Control(c) => {
                let id = Some(c.id.clone()).filter(|id| !id.is_empty());
                let step = if c.step > 0.0 { c.step } else { 1.0 };
                let changed = match (c.kind, input) {
                    (ControlKind::Checkbox | ControlKind::Switch, KeyInput::Enter | KeyInput::Char(' ')) => {
                        c.checked = !c.checked;
                        true
                    }
                    (ControlKind::Radio, KeyInput::Enter | KeyInput::Char(' ')) => {
                        radio = Some((c.name.clone(), 0));
                        true
                    }
                    (ControlKind::Radio, KeyInput::MoveLeft(_) | KeyInput::Up(_)) => {
                        radio = Some((c.name.clone(), -1));
                        true
                    }
                    (ControlKind::Radio, KeyInput::MoveRight(_) | KeyInput::Down(_)) => {
                        radio = Some((c.name.clone(), 1));
                        true
                    }
                    (ControlKind::Slider, KeyInput::MoveLeft(_) | KeyInput::Down(_)) => c.set_value(c.value - step),
                    (ControlKind::Slider, KeyInput::MoveRight(_) | KeyInput::Up(_)) => c.set_value(c.value + step),
                    (ControlKind::Slider, KeyInput::Home(_)) => c.set_value(c.min),
                    (ControlKind::Slider, KeyInput::End(_)) => c.set_value(c.max),
                    (ControlKind::Rating, KeyInput::MoveLeft(_) | KeyInput::Down(_)) => {
                        let before = c.value;
                        c.value = (c.value - 1.0).max(1.0);
                        before != c.value
                    }
                    (ControlKind::Rating, KeyInput::MoveRight(_) | KeyInput::Up(_)) => {
                        let before = c.value;
                        c.value = (c.value + 1.0).min(c.max.round().clamp(1.0, 10.0));
                        before != c.value
                    }
                    (ControlKind::Segmented, KeyInput::MoveLeft(_)) | (ControlKind::Select, KeyInput::Up(_)) => {
                        let before = c.selected;
                        c.selected = c.selected.saturating_sub(1);
                        before != c.selected
                    }
                    (ControlKind::Segmented, KeyInput::MoveRight(_)) | (ControlKind::Select, KeyInput::Down(_)) => {
                        let before = c.selected;
                        c.selected = (c.selected + 1).min(c.options.len().saturating_sub(1));
                        before != c.selected
                    }
                    (ControlKind::Select, KeyInput::Enter | KeyInput::Char(' ')) => {
                        c.open = !c.open;
                        true
                    }
                    (ControlKind::Select, KeyInput::Escape) if c.open => {
                        c.open = false;
                        true
                    }
                    _ => false,
                };
                if changed {
                    result = Some(id);
                }
            }
            _ => {}
        }
    });
    // Radio : coche celui-ci (Espace) ou le voisin du groupe (fleches), qui
    // prend aussi le focus.
    if let Some((name, direction)) = radio {
        let mut group: Vec<Vec<usize>> = focus_order(nodes, parent).into_iter().filter(|p| matches!(node_at(nodes, p), UiNode::Control(c) if c.kind == ControlKind::Radio && c.name == name)).collect();
        if group.is_empty() {
            return result;
        }
        let current = group.iter().position(|p| is_focused(node_at(nodes, p))).unwrap_or(0) as i32;
        let target = (current + direction).rem_euclid(group.len() as i32) as usize;
        let target_path = group.swap_remove(target);
        for_each_mut(nodes, &mut |n| {
            if let UiNode::Control(c) = n
                && c.kind == ControlKind::Radio && c.name == name {
                    c.checked = false;
                }
        });
        clear_focus(nodes);
        let node = node_at_mut(nodes, &target_path);
        set_focus(node, true, true);
        if let UiNode::Control(c) = node {
            c.checked = true;
            result = Some(Some(c.id.clone()).filter(|id| !id.is_empty()));
        }
    }
    result
}

/// Echap : ferme la liste ouverte, sinon active le bouton de fermeture de
/// la modale du dessus (un bouton dont l'id finit par `-fermer`). Retourne
/// l'id active, ou `Some(None)` si seulement une liste a ete fermee.
pub fn escape(nodes: &mut [UiNode], parent: (u32, u32, u32, u32)) -> Option<Option<String>> {
    let mut closed = false;
    for_each_mut(nodes, &mut |n| {
        if let UiNode::Control(c) = n
            && c.open {
                c.open = false;
                closed = true;
            }
    });
    if closed {
        return Some(None);
    }
    let layer = modal_layer(nodes, parent).or_else(|| layer_paths(nodes).pop())?;
    let mut found = None;
    fn find(node: &UiNode, out: &mut Option<String>) {
        match node {
            UiNode::Button(b) if b.id.ends_with("-fermer") && out.is_none() => *out = Some(b.id.clone()),
            UiNode::Container(c) => c.children.iter().for_each(|ch| find(ch, out)),
            _ => {}
        }
    }
    find(node_at(nodes, &layer), &mut found);
    found.map(Some)
}
