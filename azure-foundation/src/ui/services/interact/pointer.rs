// Souris : forme du curseur au survol, boutons, champs (`Control`),
// infobulles.
use crate::layout::managers::web_layout::{is_fixed, layout_roots};
use crate::layout::managers::layout_manager::{contains, Rect};
use crate::ui::models::control::ControlKind;
use crate::ui::models::ui_node::UiNode;
use super::tree::{at_point, children_under, for_each_mut, hit_order, layer_at, node_at, walk, walk_mut, walk_node};

/// Le "mode" dans lequel la souris se trouve actuellement, d'apres ce
/// qu'elle survole - le systeme demande explicitement par l'utilisateur :
/// mode "clic" au-dessus d'un bouton, mode "edition de texte" au-dessus
/// d'une textarea. Sert a choisir la forme du vrai curseur systeme (voir
/// `crate::cursor::models::cursor_kind::CursorKind::from_hover`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverKind {
    None,
    Button,
    TextArea,
    /// Texte affiche : il se selectionne a la souris (voir `select`).
    Text,
    /// Forme demandee par `cursor:` en rsC (voir `Decoration::cursor`).
    Styled(crate::cursor::models::cursor_kind::CursorKind),
}

/// La boite du groupe de survol le plus profond sous `(x, y)` (voir
/// `Decoration::hover_group`) : quand elle change, il faut redessiner.
pub fn hover_group_at(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<Rect> {
    // Appele a chaque mouvement de souris : sans groupe dans l'ecran (le cas
    // courant), pas de mise en page pour rien.
    if !has_hover_group(nodes) {
        return None;
    }
    let mut found = None;
    walk(nodes, parent, &mut |node, own_box| {
        if node.decoration().hover_group && contains(own_box, x, y) {
            found = Some(own_box);
        }
    });
    found
}

fn has_hover_group(nodes: &[UiNode]) -> bool {
    nodes.iter().any(|n| n.decoration().hover_group || matches!(n, UiNode::Container(c) if has_hover_group(&c.children)))
}

pub fn hover_kind_at(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> HoverKind {
    match layer_at(nodes, x, y, parent) {
        Some(path) => hover_kind_at_base(std::slice::from_ref(node_at(nodes, &path)), x, y, parent),
        None => hover_kind_at_base(nodes, x, y, parent),
    }
}

/// Calcule le mode souris courant a `(x, y)`, sans rien modifier - appele a
/// chaque `WindowMouseMove` pour savoir si l'affichage doit changer (voir
/// `AzureWindow::run`), independamment du hit-testing "actif" (clic,
/// focus) qui mute l'arbre.
fn hover_kind_at_base(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> HoverKind {
    let roots = layout_roots(nodes, parent);
    for (node, own_box) in nodes.iter().zip(roots) {
        let kind = hover_kind_at_node(node, x, y, own_box);
        if kind != HoverKind::None {
            return kind;
        }
    }
    HoverKind::None
}

// `own_box` est la boite DEJA resolue de `node` (voir `hover_kind_at` pour
// un nœud racine, ou l'enfant d'un `Container` ci-dessous via
// `resolve_children` - le MEME calcul que celui utilise pour le dessin,
// flex/grid compris, voir `ui::services::draw_ui::draw_container`).
fn hover_kind_at_node(node: &UiNode, x: i32, y: i32, own_box: Rect) -> HoverKind {
    let decoration = node.decoration();
    // Element cache (`visibility: hidden`) : ni survol ni clic, mais ses
    // enfants peuvent etre visibles.
    let shown = decoration.visible;
    let styled = decoration.cursor.filter(|_| shown && contains(own_box, x, y)).map(HoverKind::Styled);
    match node {
        UiNode::Button(_) if shown && contains(own_box, x, y) => styled.unwrap_or(HoverKind::Button),
        UiNode::Control(c) if shown && c.kind.interactive() && !c.disabled && contains(own_box, x, y) => styled.unwrap_or(HoverKind::Button),
        UiNode::TextArea(_) if shown && contains(own_box, x, y) => styled.unwrap_or(HoverKind::TextArea),
        UiNode::Label(_) if shown && contains(own_box, x, y) => styled.unwrap_or(HoverKind::Text),
        UiNode::Container(container) => {
            let Some(child_boxes) = children_under(container, own_box, x, y) else { return HoverKind::None };
            for (i, child_box) in hit_order(&container.children, child_boxes) {
                let child = &container.children[i];
                let kind = hover_kind_at_node(child, x, y, child_box);
                if kind != HoverKind::None {
                    return kind;
                }
            }
            styled.unwrap_or(HoverKind::None)
        }
        _ => styled.unwrap_or(HoverKind::None),
    }
}

/// Remplace le texte du bouton `#id` et rend l'ancien, `None` si aucun
/// bouton ne porte cet id.
pub fn set_button_text(nodes: &mut [UiNode], id: &str, text: &str) -> Option<String> {
    let mut old = None;
    for_each_mut(nodes, &mut |node| {
        if let UiNode::Button(b) = node
            && old.is_none()
            && b.id == id
        {
            old = Some(std::mem::replace(&mut b.text, text.to_string()));
        }
    });
    old
}

/// Remplace le texte du champ `#id` (`<input>`, `<textarea>`), curseur a
/// la fin ; `false` si aucun champ ne porte cet id.
pub fn set_field_text(nodes: &mut [UiNode], id: &str, text: &str) -> bool {
    let mut found = false;
    for_each_mut(nodes, &mut |node| {
        if let UiNode::TextArea(t) = node
            && !found
            && t.id == id
        {
            t.text = text.to_string();
            t.cursor = t.text.chars().count();
            t.selection_anchor = None;
            t.scroll_offset = 0;
            found = true;
        }
    });
    found
}

pub fn button_id_at(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<String> {
    match layer_at(nodes, x, y, parent) {
        Some(path) => button_id_at_base(std::slice::from_ref(node_at(nodes, &path)), x, y, parent),
        None => button_id_at_base(nodes, x, y, parent),
    }
}

/// L'`#id` du bouton sous `(x, y)` (voir `Button::id`), sans rien modifier :
/// `None` si aucun bouton n'est touche ou s'il n'a pas d'id. Meme parcours
/// que `toggle_button_at`, appele juste avant lui (voir
/// `event::services::dispatch`).
fn button_id_at_base(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<String> {
    nodes.iter().zip(layout_roots(nodes, parent)).find_map(|(node, own_box)| button_id_at_node(node, x, y, own_box))
}

fn button_id_at_node(node: &UiNode, x: i32, y: i32, own_box: Rect) -> Option<String> {
    match node {
        UiNode::Button(button) if button.decoration.visible && contains(own_box, x, y) => Some(button.id.clone()).filter(|id| !id.is_empty()),
        // Une toile ne dit que ses propres evenements (voir `click_controls`).
        UiNode::Control(control) if control.kind == ControlKind::Toile => None,
        UiNode::Control(control) if control.kind.interactive() && contains(own_box, x, y) => Some(control.id.clone()).filter(|id| !id.is_empty()),
        UiNode::Container(container) => {
            let child_boxes = children_under(container, own_box, x, y)?;
            hit_order(&container.children, child_boxes).into_iter().find_map(|(i, child_box)| button_id_at_node(&container.children[i], x, y, child_box))
        }
        _ => None,
    }
}

pub fn toggle_button_at(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> bool {
    at_point(nodes, x, y, parent, |n| toggle_button_at_base(n, x, y, parent))
}

/// Bascule l'etat (`state`) du premier `Button` (parcours en profondeur)
/// dont la boite resolue contient `(x, y)`. Retourne `true` si un bouton a
/// effectivement ete trouve et bascule - permet a l'appelant de savoir s'il
/// faut redessiner/republier la fenetre.
fn toggle_button_at_base(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> bool {
    let roots = layout_roots(nodes, parent);
    for (node, own_box) in nodes.iter_mut().zip(roots) {
        if toggle_button_at_node(node, x, y, own_box) {
            return true;
        }
    }
    false
}

fn toggle_button_at_node(node: &mut UiNode, x: i32, y: i32, own_box: Rect) -> bool {
    match node {
        // Enfonce pendant l'appui seulement (relache par `release_all`).
        UiNode::Button(button) if button.decoration.visible && contains(own_box, x, y) => {
            button.state = true;
            true
        }
        UiNode::Container(container) => {
            let Some(child_boxes) = children_under(container, own_box, x, y) else { return false };
            for (i, child_box) in hit_order(&container.children, child_boxes) {
                let child = &mut container.children[i];
                if toggle_button_at_node(child, x, y, child_box) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Resultat d'un clic sur les champs.
pub struct ControlClick {
    pub changed: bool,
    /// L'id du champ touche (ou de la liste deroulante dont une ligne a ete
    /// choisie).
    pub id: Option<String>,
    /// Le clic a ete pris par une liste ouverte : rien d'autre ne le recoit.
    pub consumed: bool,
}

/// La toile qui recoit la souris en `(x, y)` : celle sous le point, si rien
/// n'est dessine par-dessus a cet endroit (barre d'outils flottante,
/// panneau, champ...). L'ordre du parcours est celui du dessin : un element
/// visite apres la toile et sous le point la recouvre.
pub fn toile_sous(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<String> {
    let dans = |own: Rect, clip: Rect| contains(own, x, y) && contains(clip, x, y);
    let mut toile: Option<String> = None;
    let mut couverte = false;
    super::tree::walk_with_paths(nodes, parent, &mut |n, own, clip, _| {
        if !dans(own, clip) || !n.decoration().visible {
            return;
        }
        match n {
            UiNode::Control(c) if c.toile.is_some() => {
                toile = Some(c.id.clone());
                couverte = false;
            }
            _ if toile.is_some() => couverte = true,
            _ => {}
        }
    });
    toile.filter(|_| !couverte)
}

/// Un clic a `(x, y)` sur les champs : liste deroulante ouverte d'abord
/// (choisir une ligne, ou la fermer), puis le champ sous le pointeur.
pub fn click_controls(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> ControlClick {
    use crate::ui::services::draw_control::{option_rows, rating_dot_x, segments, slider_value_at};
    let mut result = ControlClick { changed: false, id: None, consumed: false };

    // 1. Une liste ouverte.
    walk_mut(nodes, parent, &mut |node, own_box| {
        let UiNode::Control(control) = node else { return false };
        if !control.open {
            return false;
        }
        control.open = false;
        result.changed = true;
        if let Some(i) = option_rows(control, own_box).iter().position(|row| contains(*row, x, y)) {
            control.selected = i;
            result.id = Some(control.id.clone()).filter(|id| !id.is_empty());
            result.consumed = true;
        } else if contains(own_box, x, y) {
            // Clic sur la liste elle-meme : juste la refermer.
            result.consumed = true;
        }
        true
    });
    if result.consumed {
        return result;
    }

    // 2. Le champ sous le pointeur (dans la couche touchee seulement).
    let toile_libre = toile_sous(nodes, x, y, parent);
    let mut radio: Option<(*const crate::ui::models::control::Control, String)> = None;
    at_point(nodes, x, y, parent, |slice| walk_mut(slice, parent, &mut |node, own_box| {
        let UiNode::Control(control) = node else { return false };
        if !control.kind.interactive() || control.disabled || !contains(own_box, x, y) {
            return false;
        }
        // Une toile recouverte (barre flottante, panneau...) : pas pour elle.
        if control.kind == ControlKind::Toile && toile_libre.as_deref() != Some(control.id.as_str()) {
            return false;
        }
        result.id = Some(control.id.clone()).filter(|id| !id.is_empty());
        result.changed = true;
        match control.kind {
            ControlKind::Checkbox | ControlKind::Switch => control.checked = !control.checked,
            ControlKind::Radio => radio = Some((control as *const _, control.name.clone())),
            ControlKind::Slider => {
                control.set_value(slider_value_at(control, own_box, x));
                control.dragging = true;
            }
            ControlKind::Rating => {
                let count = (control.max.round() as usize).clamp(1, 10);
                let nearest = (0..count).min_by_key(|i| (rating_dot_x(own_box, *i) - x as f32).abs() as i64).unwrap_or(0);
                control.value = (nearest + 1) as f64;
            }
            ControlKind::Select => control.open = true,
            ControlKind::Segmented => {
                if let Some(i) = segments(control, own_box.2).iter().position(|(sx, sw)| (x as f32) >= own_box.0 as f32 + sx && (x as f32) < own_box.0 as f32 + sx + sw) {
                    control.selected = i;
                }
            }
            ControlKind::Toile => {
                if let Some(t) = control.toile.as_mut() {
                    let evenement = t.appuyer(own_box, x, y, &crate::ui::models::toile::mesure_par_defaut);
                    control.dragging = t.geste.is_some();
                    // Un evenement des l'appui (double-clic) : il prend le clic,
                    // sinon le bouton cherche ensuite (aucun) l'effacerait.
                    result.consumed = evenement.is_some();
                    result.id = evenement.map(|e| format!("{}@{e}", control.id));
                }
            }
            ControlKind::Progress | ControlKind::Graphe => {}
        }
        true
    }));
    // Un seul radio coche par groupe.
    if let Some((target, name)) = radio {
        for_each_mut(nodes, &mut |node| {
            if let UiNode::Control(c) = node
                && c.kind == ControlKind::Radio && c.name == name {
                    c.checked = std::ptr::eq(c as *const _, target);
                }
        });
    }
    result
}

/// Glisser d'un curseur en cours : suit la souris.
pub fn drag_slider(nodes: &mut [UiNode], x: i32, parent: (u32, u32, u32, u32)) -> bool {
    use crate::ui::services::draw_control::slider_value_at;
    let mut changed = false;
    walk_mut(nodes, parent, &mut |node, own_box| {
        let UiNode::Control(control) = node else { return false };
        if !control.dragging {
            return false;
        }
        changed = control.set_value(slider_value_at(control, own_box, x));
        true
    });
    changed
}

/// Glisser sur une toile tenue (boite, vue, trait a relier).
pub fn drag_toile(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> bool {
    let mut changed = false;
    walk_mut(nodes, parent, &mut |node, own_box| {
        let UiNode::Control(control) = node else { return false };
        let (true, Some(t)) = (control.dragging, control.toile.as_mut()) else { return false };
        changed = t.glisser(own_box, x, y);
        true
    });
    changed
}

/// Bouton relache sur une toile tenue : ce que l'app doit recevoir
/// (`<id>@deplacer@...`, `<id>@choisir@...`, `<id>@relier@...`).
pub fn release_toile(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<String> {
    let mut evenement = None;
    walk_mut(nodes, parent, &mut |node, own_box| {
        let UiNode::Control(control) = node else { return false };
        let (true, Some(t)) = (control.dragging, control.toile.as_mut()) else { return false };
        evenement = t.relacher(own_box, x, y, &crate::ui::models::toile::mesure_par_defaut).map(|e| format!("{}@{e}", control.id));
        control.dragging = false;
        true
    });
    evenement
}

/// La molette au-dessus d'une toile : sa vue bouge (Ctrl : zoom). `None` :
/// pas de toile sous la souris (la page defile).
pub fn scroll_toile(nodes: &mut [UiNode], x: i32, y: i32, delta: f64, ctrl: bool, maj: bool, parent: (u32, u32, u32, u32)) -> Option<bool> {
    // Au-dessus d'un panneau flottant, c'est lui qui defile.
    let libre = toile_sous(nodes, x, y, parent)?;
    let mut fait = None;
    walk_mut(nodes, parent, &mut |node, own_box| {
        let UiNode::Control(control) = node else { return false };
        let Some(t) = control.toile.as_mut() else { return false };
        if !contains(own_box, x, y) || control.id != libre {
            return false;
        }
        fait = Some(t.molette(own_box, x, y, delta, ctrl, maj));
        true
    });
    fait
}

/// Garde la vue (decalage, zoom) des toiles quand l'ecran est reconstruit :
/// une toile reprend celle de meme id de l'ecran d'avant.
pub fn carry_toiles(old: &[UiNode], new: &mut [UiNode]) {
    let mut vues = std::collections::HashMap::new();
    fn lire<'a>(nodes: &'a [UiNode], vues: &mut std::collections::HashMap<&'a str, &'a crate::ui::models::toile::Toile>) {
        for n in nodes {
            match n {
                UiNode::Control(c) if !c.id.is_empty() => {
                    if let Some(t) = c.toile.as_deref() {
                        vues.insert(c.id.as_str(), t);
                    }
                }
                UiNode::Container(c) => lire(&c.children, vues),
                _ => {}
            }
        }
    }
    lire(old, &mut vues);
    if vues.is_empty() {
        return;
    }
    for_each_mut(new, &mut |n| {
        if let UiNode::Control(c) = n
            && let (Some(t), Some(avant)) = (c.toile.as_mut(), vues.get(c.id.as_str()))
        {
            t.reprendre(avant);
        }
    });
}

/// Bouton de la souris relache : les boutons remontent, le glisser
/// s'arrete. `true` si quelque chose change a l'ecran.
pub fn release_all(nodes: &mut [UiNode]) -> bool {
    let mut changed = false;
    for_each_mut(nodes, &mut |node| match node {
        UiNode::Button(button) if button.state => {
            button.state = false;
            changed = true;
        }
        UiNode::Control(control) if control.dragging => {
            control.dragging = false;
            changed = true;
        }
        _ => {}
    });
    changed
}

/// Un curseur est-il en cours de glissement ?
pub fn any_dragging(nodes: &[UiNode]) -> bool {
    nodes.iter().any(|node| match node {
        UiNode::Control(c) => c.dragging,
        UiNode::Container(container) => any_dragging(&container.children),
        _ => false,
    })
}

/// L'infobulle sous `(x, y)` (element le plus profond qui en a une).
pub fn tooltip_at(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<String> {
    let target = layer_at(nodes, x, y, parent);
    let mut found = None;
    let mut visit = |node: &UiNode, own_box: Rect| {
        if !node.decoration().tooltip.is_empty() && contains(own_box, x, y) {
            found = Some(node.decoration().tooltip.clone());
        }
    };
    match target {
        Some(path) => walk(std::slice::from_ref(node_at(nodes, &path)), parent, &mut visit),
        None => {
            for (node, own_box) in nodes.iter().zip(layout_roots(nodes, parent)) {
                if !is_fixed(node) {
                    walk_node(node, own_box, &mut visit);
                }
            }
        }
    }
    found
}

/// Cadre les toiles qui l'attendent (nouvelle `vue`), maintenant que leur
/// boite est connue. `true` : a redessiner.
pub fn cadrer_toiles(nodes: &mut [UiNode], parent: (u32, u32, u32, u32)) -> bool {
    let mut boites = std::collections::HashMap::new();
    walk(nodes, parent, &mut |node, own_box| {
        if let UiNode::Control(c) = node
            && c.toile.as_deref().is_some_and(|t| t.a_cadrer)
        {
            boites.insert(c.id.clone(), own_box);
        }
    });
    let mut fait = false;
    if boites.is_empty() {
        return false;
    }
    for_each_mut(nodes, &mut |n| {
        if let UiNode::Control(c) = n
            && let (Some(t), Some(b)) = (c.toile.as_mut(), boites.get(&c.id))
            && b.2 > 0
            && b.3 > 0
        {
            t.cadrer(*b, &crate::ui::models::toile::mesure_par_defaut);
            t.a_cadrer = false;
            fait = true;
        }
    });
    fait
}

/// La souris bouge sans bouton : les toiles a `focus` suivent la boite
/// survolee. `true` : a redessiner.
pub fn survol_toile(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> bool {
    fn a_focus(nodes: &[UiNode]) -> bool {
        nodes.iter().any(|n| match n {
            UiNode::Control(c) => c.toile.as_deref().is_some_and(|t| t.focus),
            UiNode::Container(c) => a_focus(&c.children),
            _ => false,
        })
    }
    // Appele a chaque mouvement : sans toile a focus, aucune mise en page.
    if !a_focus(nodes) {
        return false;
    }
    let mut change = false;
    walk_mut(nodes, parent, &mut |node, own_box| {
        if let UiNode::Control(c) = node
            && let Some(t) = c.toile.as_mut()
        {
            change |= t.survoler(own_box, x, y, &crate::ui::models::toile::mesure_par_defaut);
        }
        false
    });
    change
}
