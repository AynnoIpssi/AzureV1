// Menu de commandes au clavier : dans une zone `<richtext commandes="...">`,
// `/` en debut de mot ouvre un petit menu sous le curseur. La suite tapee
// (`/tit`) filtre la liste, sans accents ni majuscules ; ↑/↓ choisit, Entree
// ou Tab valide, Echap ferme. Valider retire `/tit` du texte et previent
// l'app : activation `commande-<zone>@<code>@<position>`.
//
// La liste vient de l'attribut : `code|Nom|description|mots-cles` separes
// par `;` (voir `parse_items`).
use crate::layout::managers::layout_manager::{contains, Rect};

#[derive(Debug, Clone, PartialEq)]
pub struct CommandItem {
    pub code: String,
    pub label: String,
    pub desc: String,
    /// Mots de recherche en plus du nom (`h1`, `todo`...), normalises.
    pub keys: Vec<String>,
}

/// `code|Nom|description|mots cles;...` -> les commandes. Vide (ou
/// `commandes="true"`) : aucune, l'app recoit alors `slash-<id>`.
pub fn parse_items(spec: &str) -> Vec<CommandItem> {
    spec.split(';')
        .filter_map(|item| {
            let mut f = item.split('|').map(str::trim);
            let code = f.next().filter(|c| !c.is_empty())?;
            let label = f.next()?;
            Some(CommandItem {
                code: code.to_string(),
                label: label.to_string(),
                desc: f.next().unwrap_or("").to_string(),
                keys: f.next().unwrap_or("").split_whitespace().map(normalize).chain(std::iter::once(normalize(code))).collect(),
            })
        })
        .collect()
}

/// Minuscules sans accents : « Titre » et « tître » se valent.
pub fn normalize(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// Le menu ouvert (voir `EventState::command_menu`).
#[derive(Debug, Clone, PartialEq)]
pub struct CommandMenu {
    /// L'`#id` de la zone de texte.
    pub area: String,
    /// Position du `/` dans son texte.
    pub start: usize,
    pub query: String,
    pub items: Vec<CommandItem>,
    /// Les commandes qui correspondent, dans l'ordre affiche.
    pub shown: Vec<usize>,
    pub selected: usize,
    /// Premiere ligne visible (la liste defile avec le choix).
    pub first: usize,
    pub x: i32,
    pub y: i32,
}

pub const MENU_W: u32 = 300;
pub const HEAD_H: u32 = 28;
pub const ROW_H: u32 = 34;
pub const MAX_ROWS: usize = 8;
pub const PAD: u32 = 5;

impl CommandMenu {
    /// Ouvre le menu sous le curseur : `caret` = (x, haut, bas) de la ligne
    /// du curseur ; au-dessus s'il n'y a pas la place dessous.
    pub fn open(area: &str, start: usize, items: Vec<CommandItem>, caret: (i32, i32, i32), bounds: (u32, u32, u32, u32)) -> CommandMenu {
        let mut m = CommandMenu { area: area.to_string(), start, query: String::new(), shown: (0..items.len()).collect(), items, selected: 0, first: 0, x: 0, y: 0 };
        let h = m.height() as i32;
        let (bx, by, bw, bh) = (bounds.0 as i32, bounds.1 as i32, bounds.2 as i32, bounds.3 as i32);
        m.x = (caret.0 - 8).min(bx + bw - MENU_W as i32 - 6).max(bx);
        m.y = if caret.2 + 6 + h <= by + bh { caret.2 + 6 } else { (caret.1 - 6 - h).max(by) };
        m
    }

    fn height(&self) -> u32 {
        HEAD_H + self.shown.len().clamp(1, MAX_ROWS) as u32 * ROW_H + 2 * PAD
    }

    pub fn rect(&self) -> Rect {
        (self.x, self.y, MENU_W, self.height())
    }

    /// Filtre avec `query` (ce qui suit le `/`) : le nom qui commence par la
    /// recherche d'abord, puis un de ses mots, un mot-cle, et enfin le nom
    /// qui la contient.
    pub fn filter(&mut self, query: &str) {
        self.query = query.to_string();
        let q = normalize(query.trim());
        let mut ranked: Vec<(u8, usize)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, it)| {
                let label = normalize(&it.label);
                let rank = if q.is_empty() || label.starts_with(&q) {
                    0
                } else if label.split_whitespace().any(|w| w.starts_with(&q)) {
                    1
                } else if it.keys.iter().any(|k| k.starts_with(&q)) {
                    2
                } else if label.contains(&q) || normalize(&it.desc).contains(&q) {
                    3
                } else {
                    return None;
                };
                Some((rank, i))
            })
            .collect();
        ranked.sort_by_key(|r| r.0);
        self.shown = ranked.into_iter().map(|r| r.1).collect();
        self.selected = 0;
        self.first = 0;
    }

    pub fn current(&self) -> Option<&CommandItem> {
        self.shown.get(self.selected).map(|i| &self.items[*i])
    }

    /// ↑ / ↓ : le choix tourne, la liste suit.
    pub fn step(&mut self, down: bool) {
        let n = self.shown.len();
        if n == 0 {
            return;
        }
        self.selected = if down { (self.selected + 1) % n } else { (self.selected + n - 1) % n };
        if self.selected < self.first {
            self.first = self.selected;
        } else if self.selected >= self.first + MAX_ROWS {
            self.first = self.selected + 1 - MAX_ROWS;
        }
    }

    /// La fin du nom choisi, a completer apres ce qui est tape (`/tit` ->
    /// `re 1`) - vide si le nom ne commence pas par la recherche.
    pub fn completion(&self) -> String {
        let Some(it) = self.current() else { return String::new() };
        let typed = self.query.chars().count();
        let label: Vec<char> = it.label.chars().collect();
        let head: String = label.iter().take(typed).collect();
        if typed > 0 && normalize(&head) == normalize(&self.query) { label[typed..].iter().collect() } else { String::new() }
    }

    /// Les lignes visibles : (index dans `shown`, boite).
    pub fn rows(&self) -> Vec<(usize, Rect)> {
        let (x0, y0) = (self.x + PAD as i32, self.y + (PAD + HEAD_H) as i32);
        (self.first..self.shown.len().min(self.first + MAX_ROWS))
            .enumerate()
            .map(|(k, i)| (i, (x0, y0 + (k as u32 * ROW_H) as i32, MENU_W - 2 * PAD, ROW_H)))
            .collect()
    }

    /// La ligne sous `(x, y)` (index dans `shown`).
    pub fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        self.rows().into_iter().find(|r| contains(r.1, x, y)).map(|r| r.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> CommandMenu {
        let items = parse_items("texte|Texte|Un paragraphe|p;titre1|Titre 1|Grand titre|h1;puce|Liste à puces|Une liste|ul;numero|Liste numérotée||ol;tache|Tâche|Case à cocher|todo");
        CommandMenu::open("b-1", 0, items, (100, 100, 120), (0, 0, 1000, 800))
    }

    #[test]
    fn filtre_et_completion() {
        let mut m = menu();
        assert_eq!(m.shown.len(), 5);
        m.filter("text");
        assert_eq!(m.current().unwrap().code, "texte");
        assert_eq!(m.completion(), "e");
        m.filter("tach");
        assert_eq!(m.current().unwrap().code, "tache", "sans accent");
        m.filter("h1");
        assert_eq!(m.current().unwrap().code, "titre1", "mot-cle");
        m.filter("liste");
        assert_eq!(m.shown.len(), 2);
        m.step(true);
        assert_eq!(m.current().unwrap().code, "numero");
        m.step(true);
        assert_eq!(m.current().unwrap().code, "puce", "le choix tourne");
        m.filter("puces");
        assert_eq!(m.current().unwrap().code, "puce", "un mot du nom");
        m.filter("zzz");
        assert!(m.current().is_none());
    }

    #[test]
    fn place_et_lignes() {
        let m = menu();
        assert_eq!(m.y, 126, "sous la ligne du curseur");
        let r = m.rows()[1].1;
        assert_eq!(m.row_at(r.0 + 5, r.1 + 5), Some(1));
        let haut = CommandMenu::open("b", 0, parse_items("a|A;b|B"), (100, 760, 780), (0, 0, 1000, 800));
        assert!(haut.y + haut.rect().3 as i32 <= 760, "au-dessus en bas de fenetre");
    }
}
