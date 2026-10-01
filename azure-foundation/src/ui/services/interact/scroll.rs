// Defilement : molette et pave tactile (vertical et horizontal),
// animation, barre saisie a la souris, liens d'ancre.
use crate::layout::managers::web_layout::{fixed_box, is_fixed, layout_roots};
use crate::layout::managers::layout_manager::{container_layout, contains, Rect};
use crate::ui::models::ui_node::UiNode;
use super::text::scroll_textarea;
use super::tree::{at_point, for_each_layer, hit_order, layer_paths, node_at, node_at_mut};

// Molette sur un `Container` defilable : la distance vient de `delta`
// lui-meme (la valeur de `wl_pointer.axis`, en pixels de surface - un cran
// de molette vaut ~10-15, un pave tactile envoie de petites valeurs
// continues), multipliee par ce facteur. Elle deplace la CIBLE du
// defilement (`Container::scroll_target`) ; `animate_scroll`, appele a
// chaque tic, fait glisser `scroll_offset` vers elle.
// (La molette est deja acceleree par le moteur, pas le pave tactile : voir
// `surface_manager::WHEEL_SPEED`.)
const CONTAINER_SCROLL_SPEED: f64 = 1.0;

// Part de la distance restante parcourue a chaque tic (~16ms) : ~90% du
// chemin en ~100ms, sans a-coup.
const SCROLL_EASING: f32 = 0.35;

pub fn scroll_at(nodes: &mut [UiNode], x: i32, y: i32, delta: f64, parent: (u32, u32, u32, u32)) -> bool {
    at_point(nodes, x, y, parent, |n| scroll_at_base(n, x, y, delta, parent))
}

/// Fait defiler ce qui est survole par `(x, y)` - pas necessairement ce qui
/// a le focus (comme dans un vrai navigateur : survoler suffit). Le plus
/// profond d'abord : une `TextArea` (une ligne par cran) ou un `Container`
/// defilable imbrique absorbe la molette avant son parent, qui ne defile
/// que si l'interieur est deja en butee. Retourne `true` si quelque chose
/// va bouger (donc s'il faut redessiner).
fn scroll_at_base(nodes: &mut [UiNode], x: i32, y: i32, delta: f64, parent: (u32, u32, u32, u32)) -> bool {
    let roots = layout_roots(nodes, parent);
    for (node, own_box) in nodes.iter_mut().zip(roots) {
        if scroll_at_node(node, x, y, delta, own_box) {
            return true;
        }
    }
    false
}

fn scroll_at_node(node: &mut UiNode, x: i32, y: i32, delta: f64, own_box: Rect) -> bool {
    match node {
        UiNode::TextArea(area) => contains(own_box, x, y) && scroll_textarea(area, own_box, delta),
        UiNode::Container(container) => {
            let layout = container_layout(container, own_box);
            if !contains(layout.clip, x, y) {
                return false;
            }
            for (child, child_box) in container.children.iter_mut().zip(layout.children) {
                if scroll_at_node(child, x, y, delta, child_box) {
                    return true;
                }
            }

            if !container.layout.scrollable() || layout.max_scroll == 0 {
                return false;
            }
            let target = (container.scroll_target as f64 + delta * CONTAINER_SCROLL_SPEED).clamp(0.0, layout.max_scroll as f64).round() as u32;
            if target == container.scroll_target {
                return false;
            }
            container.scroll_target = target;
            true
        }
        _ => false,
    }
}

/// Defilement horizontal (`overflow-x`) de ce qui est survole : le
/// conteneur le plus profond qui peut encore bouger dans ce sens.
pub fn scroll_x_at(nodes: &mut [UiNode], x: i32, y: i32, delta: f64, parent: (u32, u32, u32, u32)) -> bool {
    fn node_x(node: &mut UiNode, x: i32, y: i32, delta: f64, own_box: Rect) -> bool {
        let UiNode::Container(container) = node else { return false };
        let layout = container_layout(container, own_box);
        if !contains(layout.clip, x, y) {
            return false;
        }
        for (i, child_box) in hit_order(&container.children, layout.children.clone()) {
            if node_x(&mut container.children[i], x, y, delta, child_box) {
                return true;
            }
        }
        if !container.layout.scrollable_x() || layout.max_scroll_x == 0 {
            return false;
        }
        let target = (container.scroll_x_target as f64 + delta * CONTAINER_SCROLL_SPEED).clamp(0.0, layout.max_scroll_x as f64).round() as u32;
        if target == container.scroll_x_target {
            return false;
        }
        container.scroll_x_target = target;
        true
    }
    at_point(nodes, x, y, parent, |slice| {
        let roots = layout_roots(slice, parent);
        slice.iter_mut().zip(roots).any(|(node, b)| node_x(node, x, y, delta, b))
    })
}

/// Reporte le defilement de l'ancien ecran sur le nouveau : chaque
/// `Container` garde la position de celui qui etait au meme endroit de
/// l'arbre. Quand un element apparait ou disparait (un bandeau de message
/// au-dessus, un panneau en dessous), les enfants sont apparies par le debut
/// ou par la fin, selon ce qui se ressemble le plus. Pour redessiner la
/// meme page sans la faire remonter (voir `WindowContext::refresh`) ; un
/// decalage trop grand est ramene au bout au rendu.
pub fn carry_scroll(old: &[UiNode], new: &mut [UiNode]) {
    let decalage = if old.len() == new.len() {
        0
    } else {
        let n = old.len().min(new.len());
        let (o_fin, n_fin) = (old.len() - n, new.len() - n);
        let debut: usize = (0..n).map(|i| likeness(&old[i], &new[i])).sum();
        let fin: usize = (0..n).map(|i| likeness(&old[o_fin + i], &new[n_fin + i])).sum();
        if fin > debut { o_fin as isize - n_fin as isize } else { 0 }
    };
    for (i, n) in new.iter_mut().enumerate() {
        let Some(o) = usize::try_from(i as isize + decalage).ok().and_then(|j| old.get(j)) else { continue };
        if let (UiNode::Container(o), UiNode::Container(n)) = (o, n) {
            n.scroll_offset = o.scroll_offset;
            n.scroll_target = o.scroll_target;
            n.scroll_x = o.scroll_x;
            n.scroll_x_target = o.scroll_x_target;
            carry_scroll(&o.children, &mut n.children);
        }
    }
}

/// A quel point deux noeuds se ressemblent : nombre de noeuds de meme
/// sorte aux memes places dans leurs sous-arbres.
fn likeness(a: &UiNode, b: &UiNode) -> usize {
    match (a, b) {
        (UiNode::Container(a), UiNode::Container(b)) => 1 + a.children.iter().zip(&b.children).map(|(x, y)| likeness(x, y)).sum::<usize>(),
        _ if std::mem::discriminant(a) == std::mem::discriminant(b) => 1,
        _ => 0,
    }
}

/// Avance d'un pas l'animation de defilement de chaque `Container` dont
/// `scroll_offset` n'a pas encore rejoint `scroll_target` (voir
/// `scroll_at`) - a appeler a chaque tic. Retourne `true` si au moins un
/// conteneur a bouge.
pub fn animate_scroll(nodes: &mut [UiNode]) -> bool {
    let ease = |offset: &mut u32, target: u32| -> bool {
        if *offset == target {
            return false;
        }
        let (from, to) = (*offset as f32, target as f32);
        let step = (to - from) * SCROLL_EASING;
        let step = if step.abs() < 1.0 { (to - from).signum() } else { step };
        *offset = (from + step).round().max(0.0) as u32;
        true
    };
    let mut moved = false;
    for node in nodes {
        if let UiNode::Container(container) = node {
            moved |= ease(&mut container.scroll_offset, container.scroll_target);
            moved |= ease(&mut container.scroll_x, container.scroll_x_target);
            moved |= animate_scroll(&mut container.children);
        }
    }
    moved
}

/// Une barre de defilement saisie a la souris : quel conteneur (chemin dans
/// l'arbre) et ou dans la poignee (pour qu'elle ne saute pas sous la souris).
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollDrag {
    path: Vec<usize>,
    grab: i32,
}

/// La poignee de barre de defilement sous `(x, y)`, s'il y en a une.
pub fn scrollbar_grab(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<ScrollDrag> {
    fn find(node: &UiNode, own_box: Rect, x: i32, y: i32, path: Vec<usize>) -> Option<ScrollDrag> {
        let UiNode::Container(c) = node else { return None };
        let layout = container_layout(c, own_box);
        if !contains(layout.clip, x, y) {
            return None;
        }
        for (i, child_box) in hit_order(&c.children, layout.children.clone()) {
            if is_fixed(&c.children[i]) {
                continue;
            }
            let mut p = path.clone();
            p.push(i);
            if let Some(d) = find(&c.children[i], child_box, x, y, p) {
                return Some(d);
            }
        }
        if !c.layout.scrollable() || layout.max_scroll == 0 {
            return None;
        }
        let track = (layout.clip.0, layout.visible.1, layout.clip.2, layout.visible.3);
        let (thumb, _) = crate::ui::services::draw_ui::scrollbar_thumb(track, layout.content_height, c.scroll_offset.min(layout.max_scroll))?;
        contains(thumb, x, y).then(|| ScrollDrag { path, grab: y - thumb.1 })
    }
    for path in layer_paths(nodes).into_iter().rev() {
        let node = node_at(nodes, &path);
        if let Some(d) = find(node, fixed_box(node, parent), x, y, path.clone()) {
            return Some(d);
        }
    }
    let roots = layout_roots(nodes, parent);
    nodes.iter().zip(roots).enumerate().filter(|(_, (n, _))| !is_fixed(n)).find_map(|(i, (n, b))| find(n, b, x, y, vec![i]))
}

/// La poignee tenue suit la souris (`y`) : le contenu defile d'autant.
pub fn scrollbar_drag(nodes: &mut [UiNode], drag: &ScrollDrag, y: i32, parent: (u32, u32, u32, u32)) -> bool {
    // Boite du conteneur, niveau par niveau depuis la racine.
    let top = drag.path[0];
    let mut own_box = if is_fixed(&nodes[top]) { fixed_box(&nodes[top], parent) } else { layout_roots(nodes, parent)[top] };
    let mut node = &nodes[top];
    for &i in &drag.path[1..] {
        let UiNode::Container(c) = node else { return false };
        let layout = container_layout(c, own_box);
        own_box = layout.children[i];
        node = &c.children[i];
    }
    let UiNode::Container(c) = node else { return false };
    let layout = container_layout(c, own_box);
    let track = (layout.clip.0, layout.visible.1, layout.clip.2, layout.visible.3);
    let Some((_, travel)) = crate::ui::services::draw_ui::scrollbar_thumb(track, layout.content_height, c.scroll_offset) else { return false };
    let thumb_top = y - drag.grab - track.1 - crate::ui::services::draw_ui::SCROLLBAR_MARGIN as i32;
    let offset = if travel == 0 { 0 } else { ((thumb_top.max(0) as u64 * layout.max_scroll as u64) / travel as u64).min(layout.max_scroll as u64) as u32 };
    let UiNode::Container(c) = node_at_mut(nodes, &drag.path) else { return false };
    if c.scroll_offset == offset && c.scroll_target == offset {
        return false;
    }
    c.scroll_offset = offset;
    c.scroll_target = offset;
    true
}

/// Lien d'ancre : fait defiler (en douceur) les conteneurs qui contiennent
/// l'element d'`#id` `anchor` pour l'amener en haut de leur partie
/// visible. `false` si aucun element n'a cet id.
pub fn scroll_to_anchor(nodes: &mut [UiNode], anchor: &str, parent: (u32, u32, u32, u32)) -> bool {
    fn go(node: &mut UiNode, own_box: Rect, anchor: &str) -> Option<Rect> {
        if node.decoration().anchor == anchor {
            return Some(own_box);
        }
        let UiNode::Container(c) = node else { return None };
        let layout = container_layout(c, own_box);
        let found = c.children.iter_mut().zip(layout.children).filter(|(ch, _)| !is_fixed(ch)).find_map(|(ch, b)| go(ch, b, anchor))?;
        if c.layout.scrollable() && layout.max_scroll > 0 {
            // Un peu d'air au-dessus de la cible.
            let delta = found.1 - layout.visible.1 - 8;
            let target = (c.scroll_offset as i32 + delta).clamp(0, layout.max_scroll as i32) as u32;
            let moved = target as i32 - c.scroll_offset as i32;
            c.scroll_target = target;
            // Position de la cible une fois ce conteneur defile (pour ceux
            // qui l'englobent).
            return Some((found.0, found.1 - moved, found.2, found.3));
        }
        Some(found)
    }
    if anchor.is_empty() {
        return false;
    }
    let mut hit = false;
    for_each_layer(nodes, parent, &mut |slice| {
        let roots = layout_roots(slice, parent);
        for (node, b) in slice.iter_mut().zip(roots) {
            hit |= go(node, b, anchor).is_some();
        }
    });
    hit
}
