// Inspecteur (bouton « Inspecter » de la barre de titre, F12, Ctrl+Maj+I ou
// Ctrl+Maj+C dans n'importe quelle fenetre Azure), comme
// celui de Chrome : un panneau a droite avec l'arbre des elements de la page
// (balise, classes, id), et pour l'element choisi sa boite (marge, bordure,
// padding, contenu) et les regles rsC qui le visent. Survoler un element de
// la page ou une ligne de l'arbre l'entoure sur la page ; un clic le choisit.
//
// Ce que l'inspecteur sait d'un element vient de sa construction depuis le
// .rsh (voir `codegen::decoration_for`, qui remplit `Decoration::inspect`) ;
// sa boite vient du meme calcul que le dessin (`interact::tree`).
mod draw;
pub mod enregistreur;

pub use draw::{draw, draw_header_button};

use crate::compiler::rsc::services::link::MatchedRule;
use crate::layout::managers::layout_manager::Rect;
use crate::ui::models::ui_node::UiNode;
use azure_core::rules::window_event::WindowEvent;
use std::collections::HashSet;
use std::sync::Arc;

/// Un rectangle de la fenetre : `(x, y, largeur, hauteur)`.
pub type Zone = (u32, u32, u32, u32);

/// Ce qu'un element etait dans le .rsh, garde pour l'inspecteur.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeInfo {
    pub tag: String,
    /// Classes separees par des espaces, telles qu'interpolees.
    pub class: String,
    pub id: String,
    /// Regles rsC qui le visent, la plus forte d'abord.
    pub rules: Arc<Vec<MatchedRule>>,
}

impl NodeInfo {
    /// `button.btn.large#ouvrir`.
    pub fn selector(&self) -> String {
        let mut out = self.tag.clone();
        for class in self.class.split_whitespace() {
            out.push('.');
            out.push_str(class);
        }
        if !self.id.is_empty() {
            out.push('#');
            out.push_str(&self.id);
        }
        out
    }
}

// Touches (codes evdev).
const KEY_F12: u32 = 88;
const KEY_I: u32 = 23;
const KEY_C: u32 = 46;
const KEY_ESC: u32 = 1;
const KEY_UP: u32 = 103;
const KEY_DOWN: u32 = 108;
const KEY_LEFT: u32 = 105;
const KEY_RIGHT: u32 = 106;
const BTN_LEFT: u32 = 272;

pub(crate) const PANEL_WIDTH: u32 = 440;
pub(crate) const HEADER_H: u32 = 34;
pub(crate) const ROW_H: u32 = 20;

/// L'etat de l'inspecteur d'une fenetre.
#[derive(Default)]
pub struct Inspector {
    pub open: bool,
    /// Un clic dans la page choisit l'element au lieu d'agir sur l'app.
    pub picking: bool,
    pub selected: Option<Vec<usize>>,
    /// Element survole (dans la page ou dans l'arbre), entoure sur la page.
    pub hovered: Option<Vec<usize>>,
    /// Elements replies dans l'arbre.
    pub collapsed: HashSet<Vec<usize>>,
    pub tree_scroll: f32,
    pub detail_scroll: f32,
    /// Faire defiler l'arbre jusqu'a l'element choisi au prochain dessin.
    reveal: bool,
    /// Un scenario s'enregistre (voir `enregistreur`).
    pub enregistrement: Option<enregistreur::Enregistrement>,
    /// Le test genere au dernier arret, montre dans le panneau.
    pub code: Option<String>,
    /// Texte a mettre dans le presse-papiers (pris par la fenetre).
    pub a_copier: Option<String>,
    /// Les boutons des details au dernier dessin.
    pub(crate) boutons: Vec<(Zone, Action)>,
}

/// Ce que font les boutons du panneau (hors en-tete).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Verifier,
    FermerCode,
}

/// Une ligne de l'arbre : un element, sa boite, sa partie visible.
pub(crate) struct Row {
    pub path: Vec<usize>,
    pub own_box: Rect,
    pub visible: Rect,
    pub has_children: bool,
}

/// Le panneau et la page quand l'inspecteur est ouvert, dans la boite de
/// contenu de la fenetre `content`.
pub(crate) fn split(content: Zone) -> (Zone, Zone) {
    let (x, y, w, h) = content;
    let panel_w = PANEL_WIDTH.min(w / 2);
    ((x, y, w - panel_w, h), (x + w - panel_w, y, panel_w, h))
}

impl Inspector {
    /// Commence ou arrete l'enregistrement d'un scenario. A l'arret, le test
    /// est genere, montre et copie.
    pub fn basculer_enregistrement(&mut self, nodes: &[UiNode]) {
        match self.enregistrement.take() {
            None => {
                self.enregistrement = Some(enregistreur::Enregistrement::commencer(nodes));
                // On enregistre ce que fait l'app : les clics vont a elle.
                self.picking = false;
                self.code = None;
            }
            Some(mut r) => {
                r.noter_champs(nodes);
                let exe = std::env::current_exe().ok().and_then(|e| e.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_else(|| "mon_app".into());
                let code = r.code(&exe);
                self.a_copier = Some(code.clone());
                self.code = Some(code);
                self.open = true;
                self.detail_scroll = 0.0;
            }
        }
    }

    /// Ajoute au scenario : verifier le texte de l'element choisi.
    fn verifier_choisi(&mut self, nodes: &[UiNode]) {
        let texte = self.selected.as_ref().and_then(|p| crate::ui::services::interact::node_at_path(nodes, p)).and_then(crate::window::models::pilote::texte_de);
        if let (Some(r), Some(t)) = (self.enregistrement.as_mut(), texte) {
            r.noter_champs(nodes);
            r.etapes.push(enregistreur::Etape::Verifier(t.lines().next().unwrap_or("").trim().to_string()));
        }
    }

    /// Ou est le bouton `action` du panneau (au dernier dessin).
    pub fn zone_du_bouton(&self, action: Action) -> Option<Zone> {
        self.boutons.iter().find(|(_, a)| *a == action).map(|(z, _)| *z)
    }

    /// Ouvre (en mode « choisir ») ou ferme le panneau.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.picking = self.open;
        self.hovered = None;
    }

    /// La boite de la page : toute la fenetre, ou ce que le panneau laisse.
    pub fn page_box(&self, content: Zone) -> Zone {
        if self.open { split(content).0 } else { content }
    }

    pub(crate) fn rows(&self, nodes: &[UiNode], page: Zone) -> Vec<Row> {
        let mut rows = Vec::new();
        crate::ui::services::interact::walk_with_paths(nodes, page, &mut |node, own_box, visible, path| {
            let has_children = matches!(node, UiNode::Container(c) if !c.children.is_empty());
            rows.push(Row { path: path.to_vec(), own_box, visible, has_children });
        });
        rows
    }

    /// Les lignes affichees (sans les enfants des elements replies).
    pub(crate) fn shown_rows(&self, nodes: &[UiNode], page: Zone) -> Vec<Row> {
        let mut rows = self.rows(nodes, page);
        rows.retain(|r| !(1..r.path.len()).any(|n| self.collapsed.contains(&r.path[..n])));
        rows
    }

    /// L'element visible le plus au-dessus sous `(x, y)` dans la page.
    fn element_at(&self, nodes: &[UiNode], page: Zone, x: i32, y: i32) -> Option<Vec<usize>> {
        let inside = |r: Rect| x >= r.0 && y >= r.1 && x < r.0 + r.2 as i32 && y < r.1 + r.3 as i32;
        self.rows(nodes, page)
            .into_iter()
            .filter(|r| inside(r.visible) && inside(r.own_box))
            .filter(|r| crate::ui::services::interact::node_at_path(nodes, &r.path).is_some_and(|n| n.decoration().visible))
            .last()
            .map(|r| r.path)
    }

    fn select(&mut self, path: Option<Vec<usize>>) {
        if self.selected != path {
            self.detail_scroll = 0.0;
        }
        // Les ancetres de l'element choisi se deplient.
        if let Some(path) = &path {
            for n in 1..path.len() {
                self.collapsed.remove(&path[..n]);
            }
        }
        self.selected = path;
        self.reveal = true;
    }

    /// Un ecran a ete remplace : oublie ce qui n'existe plus.
    pub fn validate(&mut self, nodes: &[UiNode]) {
        let exists = |p: &Vec<usize>| crate::ui::services::interact::node_at_path(nodes, p).is_some();
        if self.selected.as_ref().is_some_and(|p| !exists(p)) {
            self.selected = None;
        }
        if self.hovered.as_ref().is_some_and(|p| !exists(p)) {
            self.hovered = None;
        }
    }

    /// Traite `event` s'il concerne l'inspecteur. `Some(redessiner)` : pris
    /// par l'inspecteur (l'app ne le voit pas) ; `None` : pour l'app.
    /// `mouse` : la position du pointeur (pour les clics et la molette).
    pub fn handle_event(&mut self, event: &WindowEvent, nodes: &[UiNode], content: Zone, mouse: (i32, i32), ctrl_shift: bool) -> Option<bool> {
        if let WindowEvent::WindowKeyPress(key, true) = event
            && (*key == KEY_F12 || ((*key == KEY_I || *key == KEY_C) && ctrl_shift))
        {
            self.toggle();
            return Some(true);
        }
        if !self.open {
            return None;
        }
        let (page, panel) = split(content);
        let in_rect = |r: Zone, x: i32, y: i32| x >= r.0 as i32 && y >= r.1 as i32 && x < (r.0 + r.2) as i32 && y < (r.1 + r.3) as i32;
        let (tree, details) = draw::panel_parts(panel);
        match *event {
            WindowEvent::WindowMouseMove(x, y) if in_rect(panel, x, y) => {
                let hovered = if in_rect(tree, x, y) { self.row_at(nodes, page, tree, y).map(|r| r.path) } else { None };
                Some(std::mem::replace(&mut self.hovered, hovered.clone()) != hovered)
            }
            WindowEvent::WindowMouseMove(x, y) if self.picking && in_rect(page, x, y) => {
                let hovered = self.element_at(nodes, page, x, y);
                Some(std::mem::replace(&mut self.hovered, hovered.clone()) != hovered)
            }
            // Hors du panneau sans choisir : l'app voit le mouvement (le
            // survol efface est redessine avec ce qu'elle fait de lui).
            WindowEvent::WindowMouseMove(..) => {
                let changed = self.hovered.take().is_some();
                if self.picking { Some(changed) } else { None }
            }
            WindowEvent::WindowMouseButton(button, pressed) if in_rect(panel, mouse.0, mouse.1) => {
                if button == BTN_LEFT && pressed {
                    self.click_panel(nodes, page, panel, tree, mouse);
                }
                Some(true)
            }
            WindowEvent::WindowMouseButton(button, pressed) if self.picking && in_rect(page, mouse.0, mouse.1) => {
                if button == BTN_LEFT && pressed {
                    let path = self.element_at(nodes, page, mouse.0, mouse.1);
                    self.select(path);
                }
                Some(true)
            }
            WindowEvent::WindowScroll(dy) if in_rect(panel, mouse.0, mouse.1) => {
                let scroll = if in_rect(details, mouse.0, mouse.1) { &mut self.detail_scroll } else { &mut self.tree_scroll };
                *scroll = (*scroll + dy as f32 * 1.5).max(0.0);
                Some(true)
            }
            WindowEvent::WindowKeyPress(key, pressed) if self.picking && matches!(key, KEY_ESC | KEY_UP | KEY_DOWN | KEY_LEFT | KEY_RIGHT) => {
                if pressed {
                    self.key(key, nodes, page);
                }
                Some(true)
            }
            _ => None,
        }
    }

    fn row_at(&self, nodes: &[UiNode], page: Zone, tree: Zone, y: i32) -> Option<Row> {
        let index = (y - tree.1 as i32 + self.tree_scroll as i32).div_euclid(ROW_H as i32);
        let rows = self.shown_rows(nodes, page);
        usize::try_from(index).ok().and_then(|i| rows.into_iter().nth(i))
    }

    fn click_panel(&mut self, nodes: &[UiNode], page: Zone, panel: Zone, tree: Zone, mouse: (i32, i32)) {
        // En-tete : « Enregistrer », « Choisir » et « Fermer ».
        if (mouse.1 as u32) < panel.1 + HEADER_H {
            let (rec, pick, close) = draw::header_buttons(panel);
            let hit = |b: Zone| mouse.0 >= b.0 as i32 && mouse.0 < (b.0 + b.2) as i32;
            if hit(close) {
                self.open = false;
                self.hovered = None;
            } else if hit(pick) {
                self.picking = !self.picking;
            } else if hit(rec) {
                self.basculer_enregistrement(nodes);
            }
            return;
        }
        let dans = |b: &Zone| mouse.0 >= b.0 as i32 && mouse.1 >= b.1 as i32 && mouse.0 < (b.0 + b.2) as i32 && mouse.1 < (b.1 + b.3) as i32;
        if let Some((_, action)) = self.boutons.iter().find(|(b, _)| dans(b)).copied() {
            match action {
                Action::Verifier => self.verifier_choisi(nodes),
                Action::FermerCode => self.code = None,
            }
            return;
        }
        if mouse.1 >= tree.1 as i32 && mouse.1 < (tree.1 + tree.3) as i32
            && let Some(row) = self.row_at(nodes, page, tree, mouse.1)
        {
            let arrow_x = tree.0 as i32 + draw::indent(row.path.len()) as i32;
            if row.has_children && mouse.0 < arrow_x + 14 {
                if !self.collapsed.remove(&row.path) {
                    self.collapsed.insert(row.path);
                }
            } else {
                self.select(Some(row.path));
                self.reveal = false;
            }
        }
    }

    // Fleches : element precedent/suivant dans l'arbre, parent, deplier.
    fn key(&mut self, key: u32, nodes: &[UiNode], page: Zone) {
        if key == KEY_ESC {
            self.picking = false;
            return;
        }
        let rows = self.shown_rows(nodes, page);
        let current = self.selected.as_ref().and_then(|s| rows.iter().position(|r| &r.path == s));
        let next = match (key, current) {
            (KEY_DOWN, Some(i)) => rows.get(i + 1).map(|r| r.path.clone()),
            (KEY_UP, Some(i)) => i.checked_sub(1).and_then(|i| rows.get(i)).map(|r| r.path.clone()),
            (KEY_LEFT, Some(i)) => {
                let path = &rows[i].path;
                if rows[i].has_children && !self.collapsed.contains(path) {
                    self.collapsed.insert(path.clone());
                    return;
                }
                (path.len() > 1).then(|| path[..path.len() - 1].to_vec())
            }
            (KEY_RIGHT, Some(i)) => {
                self.collapsed.remove(&rows[i].path);
                return;
            }
            (_, None) => rows.first().map(|r| r.path.clone()),
            _ => None,
        };
        if next.is_some() {
            self.select(next);
        }
    }
}
