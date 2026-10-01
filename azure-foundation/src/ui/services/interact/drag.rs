// Glisser-deposer : un element `<draggable id="...">` se saisit a la souris,
// une copie translucide suit le pointeur, et le lacher au-dessus d'une
// `<dropzone id="...">` donne `(source, cible, position)` a l'app (voir
// `AzureWindow::on_drop`).
//
// - La saisie ne devient un glisser qu'apres quelques pixels : un simple
//   clic sur l'element (ou un bouton dedans) reste un clic, rendu au
//   relachement.
// - `position` : le rang ou la source irait parmi les elements saisissables
//   de la zone, la source elle-meme non comptee.
// - Une zone a l'interieur de l'element tenu ne recoit rien (une page ne se
//   range pas dans sa propre sous-page).
use crate::layout::managers::layout_manager::{contains, Rect};
use crate::ui::models::ui_node::UiNode;
use super::tree::{intersect, walk_with_paths};

/// Distance (px) a parcourir, bouton enfonce, avant que le glisser commence.
pub const DRAG_THRESHOLD: i32 = 5;

/// Un glisser en cours (voir `EventState::drag`).
#[derive(Debug, Clone, PartialEq)]
pub struct Drag {
    /// L'id de l'element tenu (`<draggable id="...">`).
    pub source: String,
    /// Son chemin dans l'arbre (voir `interact::node_at`), pour dessiner sa copie.
    pub path: Vec<usize>,
    /// Point d'appui.
    pub start: (i32, i32),
    /// Position de l'appui dans l'element.
    pub grab: (i32, i32),
    /// Taille de l'element.
    pub size: (u32, u32),
    /// Le seuil est passe : la copie suit la souris.
    pub active: bool,
    /// Le bouton appuye dans l'element : son clic est rendu au relachement
    /// si rien n'a ete glisse.
    pub click: Option<String>,
    /// La zone survolee.
    pub target: Option<DropTarget>,
}

/// Ou l'element tenu tomberait.
#[derive(Debug, Clone, PartialEq)]
pub struct DropTarget {
    pub zone: String,
    pub position: usize,
    pub zone_box: Rect,
    /// Le trait qui montre l'emplacement.
    pub line: Rect,
}

/// Un element lache dans une zone.
#[derive(Debug, Clone, PartialEq)]
pub struct Dropped {
    pub source: String,
    pub target: String,
    pub position: usize,
}

/// L'element saisissable le plus profond sous `(x, y)`.
pub fn drag_grab(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<Drag> {
    let mut found = None;
    // `clip` : ce que les conteneurs au-dessus laissent voir.
    walk_with_paths(nodes, parent, &mut |node, own_box, clip, path| {
        let d = node.decoration();
        if !d.drag.is_empty() && d.visible && contains(intersect(own_box, clip), x, y) {
            found = Some(Drag { source: d.drag.clone(), path: path.to_vec(), start: (x, y), grab: (x - own_box.0, y - own_box.1), size: (own_box.2, own_box.3), active: false, click: None, target: None });
        }
    });
    found
}

/// Le seuil est-il passe ?
pub fn drag_moved(drag: &Drag, x: i32, y: i32) -> bool {
    (x - drag.start.0).abs() > DRAG_THRESHOLD || (y - drag.start.1).abs() > DRAG_THRESHOLD
}

/// La zone sous `(x, y)` et le rang ou `drag` y tomberait.
pub fn drag_target(nodes: &[UiNode], drag: &Drag, x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<DropTarget> {
    // Zones et elements saisissables, avec leurs chemins.
    let mut zones: Vec<(String, Rect, Vec<usize>)> = Vec::new();
    let mut items: Vec<(Rect, Vec<usize>)> = Vec::new();
    walk_with_paths(nodes, parent, &mut |node, own_box, clip, path| {
        let d = node.decoration();
        if !d.visible {
            return;
        }
        if !d.drop.is_empty() && contains(intersect(own_box, clip), x, y) && !path.starts_with(&drag.path) {
            zones.push((d.drop.clone(), own_box, path.to_vec()));
        }
        if !d.drag.is_empty() && path != drag.path.as_slice() {
            items.push((own_box, path.to_vec()));
        }
    });
    // La plus profonde.
    let (zone, zone_box, zone_path) = zones.into_iter().max_by_key(|z| z.2.len())?;
    // Les elements de la zone, pas ceux ranges dans un autre element de la zone.
    let inside: Vec<&(Rect, Vec<usize>)> = items.iter().filter(|(_, p)| p.len() > zone_path.len() && p.starts_with(&zone_path)).collect();
    let direct: Vec<Rect> = inside.iter().filter(|(_, p)| !inside.iter().any(|(_, q)| q.len() < p.len() && p.starts_with(q))).map(|(b, _)| *b).collect();
    let position = direct.iter().filter(|b| before(**b, x, y)).count();
    let line = insertion_line(&direct, position, zone_box);
    Some(DropTarget { zone, position, zone_box, line })
}

/// L'element `b` est-il avant le point ? Au-dessus : oui ; en dessous :
/// non ; a sa hauteur, on coupe a mi-hauteur pour un element large (ligne
/// d'une liste), a mi-largeur sinon (carte d'une rangee, d'une galerie).
fn before(b: Rect, x: i32, y: i32) -> bool {
    let (top, bottom) = (b.1, b.1 + b.3 as i32);
    if y < top {
        return false;
    }
    if y >= bottom {
        return true;
    }
    if b.2 >= b.3 * 4 { y >= top + b.3 as i32 / 2 } else { x >= b.0 + b.2 as i32 / 2 }
}

fn insertion_line(items: &[Rect], position: usize, zone: Rect) -> Rect {
    const T: u32 = 3;
    match (items.get(position), position.checked_sub(1).and_then(|i| items.get(i))) {
        (Some(next), _) => (next.0, next.1 - 3, next.2, T),
        (None, Some(last)) => (last.0, last.1 + last.3 as i32 + 1, last.2, T),
        (None, None) => (zone.0 + 8, zone.1 + 8, zone.2.saturating_sub(16), T),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rang_dans_une_liste_verticale() {
        let cards = [(0, 0, 200, 40), (0, 50, 200, 40), (0, 100, 200, 40)];
        let rank = |y| cards.iter().filter(|b| before(**b, 10, y)).count();
        assert_eq!(rank(5), 0);
        assert_eq!(rank(30), 1);
        assert_eq!(rank(75), 2);
        assert_eq!(rank(200), 3);
        assert_eq!(insertion_line(&cards, 1, (0, 0, 200, 300)).1, 47);
        assert_eq!(insertion_line(&cards, 3, (0, 0, 200, 300)).1, 141);
        assert_eq!(insertion_line(&[], 0, (10, 10, 200, 300)), (18, 18, 184, 3));
    }
}
