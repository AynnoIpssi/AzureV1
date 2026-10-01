// Parcours de l'arbre de `UiNode` avec les boites resolues (le meme
// calcul que le dessin), et couches `position: fixed` (modales, panneaux,
// notifications) : ce qui est sous un point, dans quel ordre.
use crate::layout::managers::web_layout::{fixed_box, is_fixed, layout_roots, z_index};
use crate::layout::managers::layout_manager::{container_layout, contains, Rect};
use crate::ui::models::ui_node::UiNode;

/// Appelle `f` pour chaque noeud avec sa boite (enfants des conteneurs
/// compris, defilement pris en compte).
/// La page d'abord, puis chaque couche `position: fixed` (du dessous au
/// dessus).
pub fn walk(nodes: &[UiNode], parent: (u32, u32, u32, u32), f: &mut dyn FnMut(&UiNode, Rect)) {
    for (node, own_box) in nodes.iter().zip(layout_roots(nodes, parent)) {
        if !is_fixed(node) {
            walk_node(node, own_box, f);
        }
    }
    for path in layer_paths(nodes) {
        let node = node_at(nodes, &path);
        walk_node(node, fixed_box(node, parent), f);
    }
}

/// Comme `walk`, avec la partie visible de chaque noeud (coupee par les
/// conteneurs qui defilent ou masquent leur debordement) et son chemin
/// (indices des enfants depuis la racine) : l'ordre des chemins est l'ordre
/// du document.
/// `(noeud, boite, partie visible, chemin)`, voir `walk_with_paths`.
pub(super) type PathVisitor<'a> = dyn FnMut(&UiNode, Rect, Rect, &[usize]) + 'a;

pub(super) fn walk_with_paths(nodes: &[UiNode], parent: (u32, u32, u32, u32), f: &mut PathVisitor) {
    fn visit(node: &UiNode, own_box: Rect, clip: Rect, path: &mut Vec<usize>, f: &mut PathVisitor) {
        f(node, own_box, clip, path);
        if let UiNode::Container(container) = node {
            let layout = container_layout(container, own_box);
            let clip = intersect(clip, layout.clip);
            for (i, (child, child_box)) in container.children.iter().zip(layout.children).enumerate() {
                if !is_fixed(child) {
                    path.push(i);
                    visit(child, child_box, clip, path, f);
                    path.pop();
                }
            }
        }
    }
    let window = (parent.0 as i32, parent.1 as i32, parent.2, parent.3);
    for (i, (node, own_box)) in nodes.iter().zip(layout_roots(nodes, parent)).enumerate() {
        if !is_fixed(node) {
            visit(node, own_box, window, &mut vec![i], f);
        }
    }
    for path in layer_paths(nodes) {
        let node = node_at(nodes, &path);
        visit(node, fixed_box(node, parent), window, &mut path.clone(), f);
    }
}

/// Intersection de deux boites (largeur ou hauteur nulle si disjointes).
pub(super) fn intersect(a: Rect, b: Rect) -> Rect {
    let x0 = a.0.max(b.0);
    let y0 = a.1.max(b.1);
    let x1 = (a.0 + a.2 as i32).min(b.0 + b.2 as i32);
    let y1 = (a.1 + a.3 as i32).min(b.1 + b.3 as i32);
    (x0, y0, (x1 - x0).max(0) as u32, (y1 - y0).max(0) as u32)
}

pub(super) fn walk_node(node: &UiNode, own_box: Rect, f: &mut dyn FnMut(&UiNode, Rect)) {
    f(node, own_box);
    if let UiNode::Container(container) = node {
        let layout = container_layout(container, own_box);
        for (child, child_box) in container.children.iter().zip(layout.children) {
            // Une couche imbriquee est parcourue a part (voir `walk`).
            if !is_fixed(child) {
                walk_node(child, child_box, f);
            }
        }
    }
}

/// Comme `walk`, en modifiant : `f` retourne `true` pour arreter.
pub(super) fn walk_mut(nodes: &mut [UiNode], parent: (u32, u32, u32, u32), f: &mut dyn FnMut(&mut UiNode, Rect) -> bool) -> bool {
    // Couches d'abord (du dessus au dessous), puis la page.
    for path in layer_paths(nodes).into_iter().rev() {
        let node = node_at_mut(nodes, &path);
        let own_box = fixed_box(node, parent);
        if walk_node_mut(node, own_box, f) {
            return true;
        }
    }
    let roots = layout_roots(nodes, parent);
    for (node, own_box) in nodes.iter_mut().zip(roots) {
        if !is_fixed(node) && walk_node_mut(node, own_box, f) {
            return true;
        }
    }
    false
}

fn walk_node_mut(node: &mut UiNode, own_box: Rect, f: &mut dyn FnMut(&mut UiNode, Rect) -> bool) -> bool {
    if f(node, own_box) {
        return true;
    }
    if let UiNode::Container(container) = node {
        let layout = container_layout(container, own_box);
        for (i, child_box) in hit_order(&container.children, layout.children) {
            let child = &mut container.children[i];
            if !is_fixed(child) && walk_node_mut(child, child_box, f) {
                return true;
            }
        }
    }
    false
}

/// Tous les noeuds, sans calcul de boite.
pub(super) fn for_each_mut(nodes: &mut [UiNode], f: &mut dyn FnMut(&mut UiNode)) {
    for node in nodes {
        f(node);
        if let UiNode::Container(container) = node {
            for_each_mut(&mut container.children, f);
        }
    }
}

/// La page puis chaque couche, chacune avec sa propre racine.
pub(super) fn for_each_layer(nodes: &mut [UiNode], _parent: (u32, u32, u32, u32), f: &mut dyn FnMut(&mut [UiNode])) {
    f(nodes);
    for path in layer_paths(nodes) {
        f(std::slice::from_mut(node_at_mut(nodes, &path)));
    }
}

// Boites des enfants de `container` si `(x, y)` tombe dans sa partie
// visible (boite de contenu), `None` sinon : un enfant defile hors de vue ou
// coupe par le conteneur ne peut etre ni survole ni clique, meme si sa
// boite contient le point.
pub(super) fn children_under(container: &crate::ui::models::container::Container, own_box: Rect, x: i32, y: i32) -> Option<Vec<Rect>> {
    let layout = container_layout(container, own_box);
    contains(layout.clip, x, y).then_some(layout.children)
}

/// Enfants dans l'ordre ou la souris les rencontre : le plus haut d'abord
/// (l'inverse de l'ordre de dessin, voir `draw_ui::paint_order`). Un element
/// cache (`visibility: hidden`) n'a plus de surface cliquable ; un conteneur
/// cache garde la sienne pour ses enfants visibles.
pub(super) fn hit_order(children: &[UiNode], boxes: Vec<Rect>) -> Vec<(usize, Rect)> {
    crate::ui::services::draw_ui::paint_order(children)
        .into_iter()
        .rev()
        .map(|i| {
            let hidden = !children[i].decoration().visible && !matches!(children[i], UiNode::Container(_));
            (i, if hidden { (boxes[i].0, boxes[i].1, 0, 0) } else { boxes[i] })
        })
        .collect()
}

/// Chemins (indices depuis la racine) des elements `position: fixed`, du
/// dessous au dessus : `z-index`, puis ordre du document.
pub fn layer_paths(nodes: &[UiNode]) -> Vec<Vec<usize>> {
    fn collect(nodes: &[UiNode], prefix: &mut Vec<usize>, out: &mut Vec<(i32, Vec<usize>)>) {
        for (i, node) in nodes.iter().enumerate() {
            prefix.push(i);
            if is_fixed(node) {
                out.push((z_index(node), prefix.clone()));
            }
            if let UiNode::Container(c) = node {
                collect(&c.children, prefix, out);
            }
            prefix.pop();
        }
    }
    let mut out = Vec::new();
    collect(nodes, &mut Vec::new(), &mut out);
    out.sort_by_key(|(z, _)| *z);
    out.into_iter().map(|(_, path)| path).collect()
}

pub fn node_at<'a>(nodes: &'a [UiNode], path: &[usize]) -> &'a UiNode {
    let mut node = &nodes[path[0]];
    for &i in &path[1..] {
        let UiNode::Container(c) = node else { break };
        node = &c.children[i];
    }
    node
}

/// Comme `node_at`, `None` si le chemin ne mene plus a rien (arbre
/// reconstruit entre-temps).
pub fn node_at_path<'a>(nodes: &'a [UiNode], path: &[usize]) -> Option<&'a UiNode> {
    let mut node = nodes.get(*path.first()?)?;
    for &i in &path[1..] {
        let UiNode::Container(c) = node else { return None };
        node = c.children.get(i)?;
    }
    Some(node)
}

pub(super) fn node_at_mut<'a>(nodes: &'a mut [UiNode], path: &[usize]) -> &'a mut UiNode {
    let mut node = &mut nodes[path[0]];
    for &i in &path[1..] {
        match node {
            UiNode::Container(c) => node = &mut c.children[i],
            _ => break,
        }
    }
    node
}

/// La couche la plus haute sous `(x, y)`.
pub(super) fn layer_at(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<Vec<usize>> {
    layer_paths(nodes).into_iter().rev().find(|path| contains(fixed_box(node_at(nodes, path), parent), x, y))
}

/// Une couche qui couvre toute la fenetre (fond d'une modale) : elle
/// bloque la page dessous, et le clavier y reste.
pub(super) fn modal_layer(nodes: &[UiNode], parent: (u32, u32, u32, u32)) -> Option<Vec<usize>> {
    layer_paths(nodes).into_iter().rev().find(|path| {
        let b = fixed_box(node_at(nodes, path), parent);
        b.0 <= parent.0 as i32 && b.1 <= parent.1 as i32 && b.2 >= parent.2 && b.3 >= parent.3
    })
}

/// Applique `f` a ce qui est sous le point : la couche la plus haute qui le
/// contient (elle seule), sinon la page.
pub(super) fn at_point<R>(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32), f: impl FnOnce(&mut [UiNode]) -> R) -> R {
    match layer_at(nodes, x, y, parent) {
        Some(path) => f(std::slice::from_mut(node_at_mut(nodes, &path))),
        None => f(nodes),
    }
}
