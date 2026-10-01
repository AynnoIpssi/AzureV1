// Selection du texte affiche (les `Label`) a la souris, comme dans un
// navigateur : on appuie sur un texte, on glisse, et la selection suit le
// pointeur a travers autant de textes que l'on veut, dans l'ordre du
// document. Double-clic : un mot ; triple-clic : tout le texte. Ctrl+C
// copie (voir `event::services::dispatch`), Ctrl+A selectionne toute la
// page quand aucun champ n'a le focus.
//
// La selection vit dans les noeuds eux-memes (`Label::selection`) : le
// dessin la surligne sans rien savoir d'autre, et une page reconstruite
// repart sans selection.
use super::tree::{for_each_mut, intersect, node_at_mut, walk_with_paths};
use crate::layout::managers::layout_manager::{contains, Rect};
use crate::ui::models::label::Label;
use crate::ui::models::ui_node::UiNode;
use crate::ui::services::draw_label::{label_lines, line_positions};

/// Un endroit dans le texte : le `Label` (son chemin dans l'arbre) et un
/// indice de caractere. L'ordre des points est l'ordre du document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextPoint {
    pub path: Vec<usize>,
    pub index: usize,
}

/// Du point ou l'on a appuye (`anchor`) au point ou est la souris (`focus`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: TextPoint,
    pub focus: TextPoint,
}

impl TextSelection {
    pub fn at(point: TextPoint) -> TextSelection {
        TextSelection { anchor: point.clone(), focus: point }
    }

    fn ordered(&self) -> (&TextPoint, &TextPoint) {
        if self.anchor <= self.focus { (&self.anchor, &self.focus) } else { (&self.focus, &self.anchor) }
    }
}

fn len(label: &Label) -> usize {
    label.text.chars().count()
}

/// L'indice du caractere de `label` le plus proche de `(x, y)`. Au-dessus
/// du texte : son debut ; en dessous : sa fin.
fn char_at(label: &Label, own_box: Rect, x: i32, y: i32) -> usize {
    let lines = label_lines(label, own_box);
    let (Some(first), Some(last)) = (lines.first(), lines.last()) else { return 0 };
    let (x, y) = (x as f32, y as f32);
    if y < first.top {
        return 0;
    }
    if y >= last.top + last.height {
        return len(label);
    }
    let line = lines.iter().find(|l| y < l.top + l.height).unwrap_or(last);
    let positions = line_positions(label, line);
    let rel = x - line.x;
    // Le caractere dont le milieu est le plus proche.
    let mut index = positions.len().saturating_sub(1);
    for (i, pair) in positions.windows(2).enumerate() {
        if rel < (pair[0] + pair[1]) / 2.0 {
            index = i;
            break;
        }
    }
    (line.start + index).min(line.end)
}

/// Le texte sous `(x, y)`, s'il y en a un de visible.
pub fn text_point_at(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<TextPoint> {
    let mut found = None;
    walk_with_paths(nodes, parent, &mut |node, own_box, clip, path| {
        // Le dernier touche est au-dessus des autres (couches comprises).
        if let UiNode::Label(label) = node
            && contains(own_box, x, y)
            && contains(clip, x, y)
        {
            found = Some(TextPoint { path: path.to_vec(), index: char_at(label, own_box, x, y) });
        }
    });
    found
}

/// Le point de texte le plus proche de `(x, y)`, meme hors de tout texte :
/// pendant un glisser, la selection suit la souris entre les lignes et au
/// bord des blocs. La meme ligne d'abord, puis la distance horizontale.
pub fn nearest_text_point(nodes: &[UiNode], x: i32, y: i32, parent: (u32, u32, u32, u32)) -> Option<TextPoint> {
    let mut best: Option<(i64, TextPoint)> = None;
    walk_with_paths(nodes, parent, &mut |node, own_box, clip, path| {
        let UiNode::Label(label) = node else { return };
        let visible = intersect(own_box, clip);
        if visible.2 == 0 || visible.3 == 0 {
            return;
        }
        let dist = |v: i32, start: i32, size: u32| {
            if v < start { (start - v) as i64 } else if v >= start + size as i32 { (v - start - size as i32 + 1) as i64 } else { 0 }
        };
        let score = dist(y, visible.1, visible.3) * 100_000 + dist(x, visible.0, visible.2);
        if best.as_ref().is_none_or(|(s, _)| score <= *s) {
            best = Some((score, TextPoint { path: path.to_vec(), index: char_at(label, own_box, x, y) }));
        }
    });
    best.map(|(_, p)| p)
}

/// Les chemins de tous les `Label`, dans l'ordre du document.
fn label_paths(nodes: &[UiNode]) -> Vec<(Vec<usize>, usize)> {
    fn visit(nodes: &[UiNode], path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, usize)>) {
        for (i, node) in nodes.iter().enumerate() {
            path.push(i);
            match node {
                UiNode::Label(label) => out.push((path.clone(), len(label))),
                UiNode::Container(c) => visit(&c.children, path, out),
                _ => {}
            }
            path.pop();
        }
    }
    let mut out = Vec::new();
    visit(nodes, &mut Vec::new(), &mut out);
    out
}

/// Retire toute selection de texte. `true` s'il y en avait une.
pub fn clear_text_selection(nodes: &mut [UiNode]) -> bool {
    let mut had = false;
    for_each_mut(nodes, &mut |node| {
        if let UiNode::Label(label) = node {
            had |= label.selection.take().is_some();
        }
    });
    had
}

/// Surligne `selection` dans les textes (et retire l'ancienne).
pub fn apply_text_selection(nodes: &mut [UiNode], selection: &TextSelection) {
    clear_text_selection(nodes);
    let (start, end) = selection.ordered();
    for (path, n) in label_paths(nodes) {
        if path < start.path || path > end.path {
            continue;
        }
        let from = if path == start.path { start.index.min(n) } else { 0 };
        let to = if path == end.path { end.index.min(n) } else { n };
        if from < to
            && let UiNode::Label(label) = node_at_mut(nodes, &path)
        {
            label.selection = Some((from, to));
        }
    }
}

/// Selectionne tout le texte de la page.
pub fn select_all_text(nodes: &mut [UiNode]) -> Option<TextSelection> {
    let paths = label_paths(nodes);
    let (first, last) = (paths.first()?, paths.last()?);
    let selection = TextSelection { anchor: TextPoint { path: first.0.clone(), index: 0 }, focus: TextPoint { path: last.0.clone(), index: last.1 } };
    apply_text_selection(nodes, &selection);
    Some(selection)
}

/// Le mot autour de `point` (double-clic), ou tout son texte (`whole`,
/// triple-clic).
pub fn text_unit_at(nodes: &[UiNode], point: &TextPoint, whole: bool) -> Option<TextSelection> {
    let node = label_paths(nodes).into_iter().find(|(p, _)| *p == point.path)?;
    let UiNode::Label(label) = super::tree::node_at(nodes, &node.0) else { return None };
    let chars: Vec<char> = label.text.chars().collect();
    let (start, end) = if whole {
        (0, chars.len())
    } else {
        let word = |c: char| c.is_alphanumeric() || c == '_';
        let i = point.index.min(chars.len());
        // Sur un espace ou une ponctuation : ce seul caractere.
        let on = |j: usize| chars.get(j).is_some_and(|&c| word(c));
        if !(on(i) || (i > 0 && on(i - 1))) {
            (i, (i + 1).min(chars.len()))
        } else {
            let mut s = i;
            while s > 0 && word(chars[s - 1]) {
                s -= 1;
            }
            let mut e = i;
            while e < chars.len() && word(chars[e]) {
                e += 1;
            }
            (s, e)
        }
    };
    Some(TextSelection { anchor: TextPoint { path: point.path.clone(), index: start }, focus: TextPoint { path: point.path.clone(), index: end } })
}

/// Le texte selectionne, pret a copier : deux textes sur la meme ligne
/// sont colles s'ils se touchent (les morceaux d'une ligne de code
/// coloree) ou separes d'une espace ; un texte plus bas commence une
/// nouvelle ligne.
pub fn selected_text(nodes: &[UiNode], parent: (u32, u32, u32, u32)) -> Option<String> {
    let mut parts: Vec<(Vec<usize>, Rect, String)> = Vec::new();
    walk_with_paths(nodes, parent, &mut |node, own_box, _, path| {
        if let UiNode::Label(label) = node
            && let Some((s, e)) = label.selection
        {
            let text: String = label.text.chars().skip(s).take(e - s).collect();
            parts.push((path.to_vec(), own_box, text));
        }
    });
    parts.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = String::new();
    let mut previous: Option<Rect> = None;
    for (_, rect, text) in parts {
        if let Some(p) = previous {
            let same_line = rect.1 < p.1 + p.3 as i32 && rect.1 + rect.3 as i32 > p.1;
            if !same_line {
                out.push('\n');
            } else if rect.0 > p.0 + p.2 as i32 + 1 {
                out.push(' ');
            }
        }
        out.push_str(&text);
        previous = Some(rect);
    }
    (!out.is_empty()).then_some(out)
}
