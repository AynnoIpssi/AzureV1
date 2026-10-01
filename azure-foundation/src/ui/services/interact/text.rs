// Zones de texte : focus et curseur au clic, selection a la souris,
// defilement pour garder le curseur visible, frappe.
use crate::layout::managers::web_layout::layout_roots;
use crate::layout::managers::layout_manager::{container_layout, contains, Rect};
use crate::ui::models::textarea::TextArea;
use crate::ui::models::ui_node::UiNode;
use crate::ui::services::draw_ui::{FONT_PATH, TEXTAREA_TEXT_PADDING};
use crate::ui::services::text_layout;
use azure_engine::rendering::managers::renderer::char_index_at;
use super::keyboard::KeyInput;
use super::tree::{for_each_layer, hit_order, layer_at, layer_paths, node_at_mut};

/// `true` si au moins une `TextArea` de l'arbre a le focus - permet
/// d'eviter de redessiner a chaque tic du clignotement du curseur de
/// saisie (voir `AzureWindow::run`) quand rien n'est de toute facon
/// focalise pour l'afficher.
pub fn any_focused(nodes: &[UiNode]) -> bool {
    nodes.iter().any(|node| match node {
        UiNode::TextArea(area) => area.focused,
        UiNode::Container(container) => any_focused(&container.children),
        _ => false,
    })
}

/// La zone focalisee est-elle un texte riche ?
pub fn rich_focused(nodes: &[UiNode]) -> bool {
    nodes.iter().any(|node| match node {
        UiNode::TextArea(area) => area.focused && area.rich.is_some(),
        UiNode::Container(container) => rich_focused(&container.children),
        _ => false,
    })
}

// Geometrie de mise en page partagee entre hit-testing (clic, molette) et
// auto-scroll : les lignes affichables (voir `text_layout::wrap_lines`),
// combien tiennent a la fois dans la boite, et le decalage de defilement
// courant borne a ce que le texte contient reellement.
struct TextGeometry {
    lines: Vec<(usize, usize)>,
    /// Texte riche : abscisse de chaque caractere (voir `rich_layout`).
    positions: Option<Vec<f32>>,
    visible_lines: usize,
    scroll_offset: usize,
    line_height: u32,
    /// Hauteur de chaque ligne (texte riche : selon ses tailles de police).
    heights: Vec<u32>,
}

impl TextGeometry {
    /// Haut de la ligne `line` depuis le haut du texte defile.
    fn top_of(&self, line: usize) -> u32 {
        self.heights.iter().take(line).skip(self.scroll_offset).sum()
    }

    /// La ligne visible a `y` (depuis le haut du texte defile).
    fn visible_at(&self, y: u32) -> usize {
        let mut top = 0;
        for (k, h) in self.heights.iter().skip(self.scroll_offset).enumerate().take(self.visible_lines) {
            top += h;
            if y < top {
                return k;
            }
        }
        self.visible_lines.saturating_sub(1)
    }
}

// Combien de lignes, depuis `offset`, tiennent dans `height` (au moins une).
fn fitting(heights: &[u32], offset: usize, height: u32) -> usize {
    let mut used = 0;
    let mut n = 0;
    for h in heights.iter().skip(offset) {
        used += h;
        if used > height {
            break;
        }
        n += 1;
    }
    n.max(1)
}

fn text_geometry(area: &TextArea, box_width: u32, box_height: u32) -> TextGeometry {
    let content_width = box_width.saturating_sub(2 * TEXTAREA_TEXT_PADDING).max(1);
    let content_height = box_height.saturating_sub(2 * TEXTAREA_TEXT_PADDING).max(1);
    let line_height = line_height(area);

    if area.single_line {
        return TextGeometry { lines: vec![(0, area.text.chars().count())], positions: None, visible_lines: 1, scroll_offset: 0, line_height, heights: vec![line_height] };
    }
    if let Some(rich) = &area.rich {
        let (positions, lines) = crate::ui::services::rich_layout::layout(&area.text, &rich.styles, area.font_size, area.font_weight, content_width as f32);
        let heights = crate::ui::services::rich_layout::line_heights(&area.text, &rich.styles, area.font_size, &lines);
        let scroll_offset = area.scroll_offset.min(lines.len().saturating_sub(1));
        let visible_lines = fitting(&heights, scroll_offset, content_height);
        return TextGeometry { lines, positions: Some(positions), visible_lines, scroll_offset, line_height, heights };
    }
    let lines = text_layout::wrap_lines(&area.text, FONT_PATH, area.font_size, area.font_weight, content_width as f32);
    let visible_lines = (content_height / line_height.max(1)).max(1) as usize;
    let scroll_offset = area.scroll_offset.min(lines.len().saturating_sub(1));

    let heights = vec![line_height; lines.len()];
    TextGeometry { lines, positions: None, visible_lines, scroll_offset, line_height, heights }
}

/// Hauteur d'une ligne affichee (meme valeur au dessin, voir `draw_ui`).
pub fn line_height(area: &TextArea) -> u32 {
    match area.rich {
        Some(_) => (area.font_size * crate::ui::services::rich_layout::RICH_LINE_SPACING) as u32,
        None => (area.font_size * 1.1) as u32,
    }
}

/// Molette sur une zone de texte : une ligne par cran. `true` si le texte
/// a defile.
pub(super) fn scroll_textarea(area: &mut TextArea, own_box: Rect, delta: f64) -> bool {
    let geo = text_geometry(area, own_box.2, own_box.3);
    let max_offset = geo.lines.len().saturating_sub(geo.visible_lines.min(geo.lines.len()));
    let step: i64 = if delta > 0.0 { 1 } else if delta < 0.0 { -1 } else { 0 };
    let new_offset = (area.scroll_offset as i64 + step).clamp(0, max_offset as i64) as usize;
    if new_offset == area.scroll_offset {
        return false;
    }
    area.scroll_offset = new_offset;
    true
}

// Convertit un clic en position-fenetre `(click_x, click_y)` en index de
// caractere dans le texte de `area`, dont la boite commence a
// `(box_x, box_y)` et fait `box_width` x `box_height` - meme logique de
// placement que le rendu (retour a la ligne ET defilement compris, voir
// `text_geometry`/`ui::services::text_layout::wrap_lines` et
// `azure_engine::rendering::managers::renderer`, partagee entre
// dessin/mesure/hit-test) justement pour que cliquer place vraiment le
// curseur sous le pointeur, jamais a cote, meme sur une ligne repliee ou
// apres avoir fait defiler le texte.
fn char_index_for_click(area: &TextArea, box_x: i32, box_y: i32, box_width: u32, box_height: u32, click_x: i32, click_y: i32) -> usize {
    let text_x = box_x + TEXTAREA_TEXT_PADDING as i32;
    let text_y = box_y + TEXTAREA_TEXT_PADDING as i32;
    let geo = text_geometry(area, box_width, box_height);

    // Quelle ligne affichee (dans la fenetre de defilement courante) le
    // clic touche - la premiere visible si le clic est au-dessus du
    // texte, la derniere visible s'il est en dessous.
    let relative_y = click_y - text_y;
    let visible_idx = if relative_y < 0 {
        0
    } else {
        geo.visible_at(relative_y as u32)
    };
    let line_idx = (geo.scroll_offset + visible_idx).min(geo.lines.len() - 1);

    let (ls, le) = geo.lines[line_idx];
    if let Some(positions) = &geo.positions {
        return crate::ui::services::rich_layout::index_at(positions, ls, le, (click_x - text_x).max(0) as f32);
    }
    let shown = area.display_text();
    // Champ d'une ligne : le debut peut etre defile hors de vue (voir
    // `text_layout::single_line_start`, le meme calcul que le dessin).
    let ls = if area.single_line { text_layout::single_line_start(&shown, area.cursor, box_width.saturating_sub(2 * TEXTAREA_TEXT_PADDING), area.focused, area.font_size, area.font_weight) } else { ls };
    let line_text = text_layout::char_slice(&shown, ls, le);
    let pixel_offset = (click_x - text_x).max(0) as f32;
    let local_idx = char_index_at(line_text, FONT_PATH, area.font_size, area.font_weight, pixel_offset)
        .unwrap_or_else(|_| line_text.chars().count());
    ls + local_idx
}

/// Focus d'une zone de texte : celle sous le point (dans la couche touchee),
/// et plus aucune ailleurs.
pub fn focus_textarea_at(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> bool {
    let target = layer_at(nodes, x, y, parent);
    // La page d'abord (son parcours traverse aussi les couches, sans les
    // toucher), puis les couches, celle qui est touchee en dernier.
    let (px, py) = if target.is_none() { (x, y) } else { (-1, -1) };
    let mut changed = focus_textarea_at_base(nodes, px, py, parent);
    let mut layers = layer_paths(nodes);
    if let Some(t) = &target {
        layers.retain(|p| p != t);
        layers.push(t.clone());
    }
    for path in layers {
        let hit = target.as_ref() == Some(&path);
        let node = node_at_mut(nodes, &path);
        let (px, py) = if hit { (x, y) } else { (-1, -1) };
        changed |= focus_textarea_at_base(std::slice::from_mut(node), px, py, parent);
    }
    changed
}

/// Focalise la premiere `TextArea` (ordre de parcours en profondeur) dont la
/// boite contient `(x, y)`, et defocalise toutes les autres (une seule
/// textarea focalisee a la fois, comme un champ de formulaire classique) -
/// y compris si plusieurs boites se chevauchent au point du clic (un layout
/// degenere, mais que le moteur de `layout` n'interdit pas) : seule la
/// PREMIERE rencontree gagne le focus, les autres sont traitees comme un
/// clic en dehors d'elles, jamais comme un hit concurrent. Un clic dans une
/// textarea - deja focalisee ou pas - y place aussi le curseur exactement
/// sous le pointeur et efface toute selection existante, comme dans un vrai
/// champ de texte (c'est aussi le point de depart d'une selection a la
/// souris, voir `extend_selection_to`). Retourne `true` si quelque chose a
/// visuellement change (focus ou position du curseur).
fn focus_textarea_at_base(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> bool {
    let mut changed = false;
    let mut hit_found = false;
    let roots = layout_roots(nodes, parent);
    for (node, own_box) in nodes.iter_mut().zip(roots) {
        set_focus_node(node, x, y, own_box, true, &mut changed, &mut hit_found);
    }
    changed
}

// `reachable` : `(x, y)` est dans la partie visible de tous les conteneurs
// ancetres. Sinon le clic ne peut focaliser aucune textarea de ce
// sous-arbre (elle est defilee hors de vue ou coupee) - mais le parcours
// continue quand meme, pour retirer le focus a celle qui l'avait.
fn set_focus_node(node: &mut UiNode, x: i32, y: i32, own_box: Rect, reachable: bool, changed: &mut bool, hit_found: &mut bool) {
    let (bx, by, bw, bh) = own_box;
    match node {
        UiNode::TextArea(area) => {
            // `!*hit_found` : une fois la premiere textarea touchee trouvee,
            // toute autre boite qui contiendrait AUSSI (x, y) (chevauchement)
            // n'est plus consideree comme un hit - elle suit la branche
            // `else` ci-dessous comme un simple clic en dehors d'elle,
            // exactement comme une textarea non touchee du tout.
            if !*hit_found && reachable && contains(own_box, x, y) {
                *hit_found = true;
                let idx = char_index_for_click(area, bx, by, bw, bh, x, y);
                if !area.focused || area.cursor != idx || area.selection_anchor.is_some() {
                    *changed = true;
                }
                area.focused = true;
                area.set_cursor(idx);
                area.selection_anchor = None;
            } else if area.focused {
                area.focused = false;
                *changed = true;
            }
        }
        UiNode::Container(container) => {
            let layout = container_layout(container, own_box);
            let reachable = reachable && contains(layout.clip, x, y);
            for (child, child_box) in container.children.iter_mut().zip(layout.children) {
                set_focus_node(child, x, y, child_box, reachable, changed, hit_found);
            }
        }
        _ => {}
    }
}

pub fn extend_selection_to(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> bool {
    let mut changed = false;
    for_each_layer(nodes, parent, &mut |n| changed |= extend_selection_to_base(n, x, y, parent));
    changed
}

/// Etend la selection de la (au plus une) `TextArea` focalisee jusqu'a la
/// position de caractere sous `(x, y)` - appele a chaque deplacement de
/// souris pendant un glisser demarre par un clic (voir `AzureWindow::run`,
/// `focus_textarea_at` pour le point de depart de l'ancre). `y` permet au
/// glisser de traverser plusieurs lignes affichees (voir
/// `ui::services::text_layout::wrap_lines`), pas seulement de bouger
/// horizontalement. Retourne `true` si le curseur a effectivement bouge
/// (donc s'il faut redessiner).
fn extend_selection_to_base(nodes: &mut [UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> bool {
    let roots = layout_roots(nodes, parent);
    for (node, own_box) in nodes.iter_mut().zip(roots) {
        if extend_selection_to_node(node, x, y, own_box) {
            return true;
        }
    }
    false
}

fn extend_selection_to_node(node: &mut UiNode, x: i32, y: i32, own_box: Rect) -> bool {
    match node {
        UiNode::TextArea(area) if area.focused => {
            let (bx, by, bw, bh) = own_box;
            let idx = char_index_for_click(area, bx, by, bw, bh, x, y);
            if area.selection_anchor.is_none() && idx != area.cursor {
                area.selection_anchor = Some(area.cursor);
            }
            if area.cursor == idx {
                return false;
            }
            area.cursor = idx;
            true
        }
        UiNode::Container(container) => {
            let child_boxes = container_layout(container, own_box).children;
            for (i, child_box) in hit_order(&container.children, child_boxes) {
                let child = &mut container.children[i];
                if extend_selection_to_node(child, x, y, child_box) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Fleche haut / bas dans la zone de texte focalisee : ligne affichee
/// precedente / suivante, a la meme abscisse (Maj : etend la selection).
/// `true` si le curseur a bouge.
pub fn move_vertical(nodes: &mut [UiNode], down: bool, extend: bool, parent: (u32, u32, u32, u32)) -> bool {
    let mut moved = false;
    super::tree::walk_mut(nodes, parent, &mut |node, own_box| {
        let UiNode::TextArea(area) = node else { return false };
        if !area.focused || area.single_line {
            return false;
        }
        let geo = text_geometry(area, own_box.2, own_box.3);
        let line = text_layout::line_containing(&geo.lines, area.cursor);
        let target = if down { line + 1 } else { line.wrapping_sub(1) };
        let Some(&(ts, te)) = geo.lines.get(target) else { return true };
        let (ls, _) = geo.lines[line];
        let x = match &geo.positions {
            Some(p) => p[area.cursor.min(p.len() - 1)] - p[ls],
            None => {
                let shown = area.display_text();
                let line_text = text_layout::char_slice(&shown, ls, geo.lines[line].1);
                azure_engine::rendering::managers::renderer::char_position(line_text, FONT_PATH, area.font_size, area.font_weight, area.cursor - ls).unwrap_or(0.0)
            }
        };
        let idx = match &geo.positions {
            Some(p) => crate::ui::services::rich_layout::index_at(p, ts, te, x),
            None => {
                let shown = area.display_text();
                let line_text = text_layout::char_slice(&shown, ts, te);
                ts + char_index_at(line_text, FONT_PATH, area.font_size, area.font_weight, x).unwrap_or(te - ts)
            }
        };
        if extend {
            area.selection_anchor.get_or_insert(area.cursor);
        } else {
            area.selection_anchor = None;
        }
        area.set_cursor(idx);
        moved = true;
        true
    });
    moved
}

pub fn ensure_cursor_visible(nodes: &mut [UiNode], parent: (u32, u32, u32, u32)) {
    for_each_layer(nodes, parent, &mut |n| ensure_cursor_visible_base(n, parent));
}

/// Ajuste `scroll_offset` de la (au plus une) `TextArea` focalisee pour que
/// la ligne du curseur reste visible dans sa boite - a appeler apres toute
/// operation qui peut avoir deplace le curseur (frappe, clic, annulation...)
/// pour que taper/naviguer jusqu'au bord de la zone visible fasse defiler
/// automatiquement, comme dans un vrai champ de texte.
fn ensure_cursor_visible_base(nodes: &mut [UiNode], parent: (u32, u32, u32, u32)) {
    let roots = layout_roots(nodes, parent);
    for (node, own_box) in nodes.iter_mut().zip(roots) {
        if ensure_cursor_visible_node(node, own_box) {
            return;
        }
    }
}

fn ensure_cursor_visible_node(node: &mut UiNode, own_box: Rect) -> bool {
    match node {
        UiNode::TextArea(area) if area.focused => {
            let (_, _, bw, bh) = own_box;
            let geo = text_geometry(area, bw, bh);
            let cursor_line = text_layout::line_containing(&geo.lines, area.cursor);

            if cursor_line < area.scroll_offset {
                area.scroll_offset = cursor_line;
            } else if cursor_line >= area.scroll_offset + geo.visible_lines {
                area.scroll_offset = cursor_line + 1 - geo.visible_lines;
            }
            area.scroll_offset = area.scroll_offset.min(geo.lines.len().saturating_sub(1));
            true
        }
        UiNode::Container(container) => {
            let child_boxes = container_layout(container, own_box).children;
            for (i, child_box) in hit_order(&container.children, child_boxes) {
                let child = &mut container.children[i];
                if ensure_cursor_visible_node(child, child_box) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Applique `input` a la (au plus une) `TextArea` actuellement focalisee,
/// avec `clipboard` comme presse-papiers (voir `KeyInput::Copy` pour sa
/// limitation). Retourne `true` si quelque chose a visuellement change
/// (texte, curseur ou selection) - donc s'il faut redessiner. Ignore
/// l'appui si aucune textarea n'a le focus.
/// La zone de texte focalisee, vue par `dispatch`.
#[derive(Debug, Clone, PartialEq)]
pub struct FocusedArea {
    pub id: String,
    pub cursor: usize,
    /// `commandes` : `/` previent l'app.
    pub commands: bool,
    /// `entree` : Entree (et Retour arriere en tete) previent l'app.
    pub enter_submits: bool,
    /// Le caractere avant le curseur.
    pub before: Option<char>,
    pub has_selection: bool,
    pub rich: bool,
    pub text: String,
    pub command_list: String,
}

pub fn focused_area_info(nodes: &[UiNode]) -> Option<FocusedArea> {
    nodes.iter().find_map(|node| match node {
        UiNode::TextArea(area) if area.focused => Some(FocusedArea {
            id: area.id.clone(),
            cursor: area.cursor,
            commands: area.commands,
            enter_submits: area.enter_submits,
            before: area.cursor.checked_sub(1).and_then(|i| area.text.chars().nth(i)),
            has_selection: area.selection_range().is_some_and(|(s, e)| s < e),
            rich: area.rich.is_some(),
            text: area.text.clone(),
            command_list: area.command_list.clone(),
        }),
        UiNode::Container(container) => focused_area_info(&container.children),
        _ => None,
    })
}

/// Le curseur de la zone focalisee a l'ecran : (x, haut, bas) de sa ligne.
pub fn focused_caret(nodes: &mut [UiNode], parent: (u32, u32, u32, u32)) -> Option<(i32, i32, i32)> {
    let mut out = None;
    super::tree::walk_mut(nodes, parent, &mut |node, own_box| {
        let UiNode::TextArea(area) = node else { return false };
        if !area.focused {
            return false;
        }
        let geo = text_geometry(area, own_box.2, own_box.3);
        let line = text_layout::line_containing(&geo.lines, area.cursor);
        let (ls, le) = geo.lines[line];
        let dx = match &geo.positions {
            Some(p) => p[area.cursor.min(p.len() - 1)] - p[ls],
            None => {
                let shown = area.display_text();
                azure_engine::rendering::managers::renderer::char_position(text_layout::char_slice(&shown, ls, le), FONT_PATH, area.font_size, area.font_weight, area.cursor - ls).unwrap_or(0.0)
            }
        };
        let pad = TEXTAREA_TEXT_PADDING as i32;
        let top = own_box.1 + pad + geo.top_of(line) as i32;
        out = Some((own_box.0 + pad + dx as i32, top, top + geo.heights.get(line).copied().unwrap_or(geo.line_height) as i32));
        true
    });
    out
}

/// Efface les caracteres `start..end` de la zone focalisee (le `/commande`
/// tape), curseur a `start`.
pub fn delete_focused_range(nodes: &mut [UiNode], start: usize, end: usize) -> bool {
    let mut done = false;
    super::tree::for_each_mut(nodes, &mut |node| {
        if let UiNode::TextArea(area) = node
            && area.focused
            && !done
        {
            let end = end.min(area.char_count());
            area.selection_anchor = Some(start.min(end));
            area.set_cursor(end);
            area.backspace();
            area.selection_anchor = None;
            done = true;
        }
    });
    done
}

/// L'id de la zone de texte riche focalisee.
pub fn focused_rich_id(nodes: &[UiNode]) -> Option<String> {
    nodes.iter().find_map(|node| match node {
        UiNode::TextArea(area) if area.focused && area.rich.is_some() => Some(area.id.clone()),
        UiNode::Container(container) => focused_rich_id(&container.children),
        _ => None,
    })
}

/// Bouton de barre d'outils `#rt-<id>-<marque>` (voir `<richbar>`) : la
/// marque s'applique a la zone de texte riche `#<id>`, qui reprend le focus
/// (sa selection n'a pas bouge). Sans id (`#rt--<marque>`, `<richbar>` sans
/// `pour`) : a `previous`, la zone qui avait le focus avant ce clic. `true`
/// si c'etait un tel bouton.
pub fn apply_rich_button(nodes: &mut [UiNode], button_id: &str, previous: Option<&str>) -> bool {
    let Some(rest) = button_id.strip_prefix("rt-") else { return false };
    let owned;
    let rest = match rest.strip_prefix('-') {
        Some(mark) => {
            let Some(target) = previous else { return false };
            owned = format!("{target}-{mark}");
            owned.as_str()
        }
        None => rest,
    };
    let mut done = false;
    super::tree::for_each_mut(nodes, &mut |node| {
        if let UiNode::TextArea(area) = node
            && !done && area.rich.is_some() && !area.id.is_empty()
            && let Some(mark) = rest.strip_prefix(area.id.as_str()).and_then(|m| m.strip_prefix('-')).and_then(crate::ui::models::rich::Mark::parse)
        {
            area.focused = true;
            area.toggle_mark(&mark);
            done = true;
        }
    });
    done
}

pub fn type_into_focused(nodes: &mut [UiNode], input: KeyInput, clipboard: &mut String) -> bool {
    for node in nodes {
        match node {
            UiNode::TextArea(area) if area.focused => return apply_key(area, input, clipboard),
            UiNode::Container(container) => {
                if type_into_focused(&mut container.children, input, clipboard) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn apply_key(area: &mut TextArea, input: KeyInput, clipboard: &mut String) -> bool {
    match input {
        KeyInput::Char(c) => {
            if !area.accepts(c) {
                return false;
            }
            area.insert_char(c);
            true
        }
        KeyInput::Backspace => area.backspace(),
        KeyInput::Delete => area.delete_forward(),
        KeyInput::Enter => {
            if area.single_line {
                return false;
            }
            area.insert_char('\n');
            true
        }
        KeyInput::MoveLeft(extend) => {
            area.move_left(extend);
            true
        }
        KeyInput::MoveRight(extend) => {
            area.move_right(extend);
            true
        }
        KeyInput::Home(extend) => {
            area.move_home(extend);
            true
        }
        KeyInput::End(extend) => {
            area.move_end(extend);
            true
        }
        KeyInput::SelectAll => {
            area.select_all();
            true
        }
        // `clipboard` : celui de la fenetre, echange avec le presse-papiers
        // du systeme par `AzureWindow` (voir `EventState::clipboard`).
        KeyInput::Copy => {
            if let Some(text) = area.selected_text() { *clipboard = text }
            false
        }
        KeyInput::Cut => match area.cut_selection() {
            Some(text) => {
                *clipboard = text;
                true
            }
            None => false,
        },
        KeyInput::Paste => {
            let text: String = clipboard.chars().filter(|c| area.accepts(*c)).collect();
            if text.is_empty() {
                false
            } else {
                area.insert_str(&text);
                true
            }
        }
        KeyInput::Undo => area.undo(),
        KeyInput::Format(c) => crate::ui::models::rich::Mark::parse(&c.to_string()).is_some_and(|mark| area.toggle_mark(&mark)),
        // Geres avant d'arriver ici (focus, fermeture, touche morte composee
        // par `dispatch`) ou sans effet.
        KeyInput::Tab(_) | KeyInput::Escape | KeyInput::Up(_) | KeyInput::Down(_) | KeyInput::Dead(_) => false,
    }
}
