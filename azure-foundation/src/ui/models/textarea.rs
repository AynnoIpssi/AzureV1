use crate::ui::models::decoration::Decoration;
use crate::layout::models::layout_props::LayoutProps;
use crate::ui::models::rich::{self, Mark, RichStyle, RichText};
use azure_engine::rendering::models::color::Color;

// Zone de texte interactive : contrairement aux autres UiNode, mutee a
// l'execution (voir ui::services::interact) plutot que fixee a la
// construction - `text`/`focused`/`cursor`/`selection_anchor` changent en
// reponse aux clics/frappes clavier de l'utilisateur.
//
// `cursor` et `selection_anchor` sont des index en CARACTERES (pas en
// octets) : le texte peut contenir des caracteres multi-octets en UTF-8
// (meme si le clavier ne sait taper que de l'ASCII pour l'instant, voir
// `ui::services::interact::key_to_input`), donc toute conversion vers un
// index dans `text` passe par `byte_index`.
pub struct TextArea {
    pub layout: LayoutProps,
    pub background: Color,
    pub text_color: Color,
    pub text: String,
    pub focused: bool,
    pub cursor: usize,
    pub selection_anchor: Option<usize>,
    // Historique lineaire pour Ctrl+Z (voir `snapshot`/`undo`) : une entree
    // par operation qui modifie le texte (taper, coller, couper,
    // effacer...), pas de regroupement "par mot" comme le ferait un
    // editeur plus complet, et pas de "refaire" (Ctrl+Y) pour l'instant.
    // Le 3e element : les styles du texte riche (vide sinon).
    undo_stack: Vec<(String, usize, Vec<RichStyle>)>,
    // Index (dans les lignes affichees apres retour a la ligne
    // automatique, voir `ui::services::text_layout::wrap_lines`) de la
    // premiere ligne visible quand le texte deborde de la hauteur de la
    // boite - molette et auto-scroll (garder le curseur visible en
    // tapant) l'ajustent, voir `ui::services::interact::scroll_at`/
    // `ensure_cursor_visible`. Pas une position en pixels : recalcule a
    // chaque redessin contre les lignes du moment, donc reste coherent
    // meme apres une modification du texte qui change le nombre de lignes.
    pub scroll_offset: usize,
    /// Couleur de fond a utiliser a la place de `background` quand la
    /// souris survole la textarea - resolue depuis une regle rsC
    /// `textarea:hover { background-color: ...; }` (voir
    /// `compiler::services::codegen::resolve_pseudo_background`). `None`
    /// par defaut : a fixer explicitement si besoin, comme `focused`.
    pub hover_background: Option<Color>,
    /// Meme mecanisme que `hover_background`, mais pour `textarea:focus` :
    /// prioritaire sur `hover_background` quand la textarea est a la fois
    /// focalisee ET survolee (voir `ui::services::draw_ui`), comme une
    /// regle `:focus` plus specifique l'emporterait en CSS.
    pub focus_background: Option<Color>,
    pub decoration: Decoration,
    /// `#id` rsH : cle de la valeur dans `WindowContext::value`.
    pub id: String,
    /// Champ d'une ligne (`<input>`) : Entree n'ajoute pas de ligne, le
    /// texte defile horizontalement.
    pub single_line: bool,
    /// Texte grise affiche tant que le champ est vide.
    pub placeholder: String,
    /// Mot de passe : chaque caractere affiche comme `*`.
    pub password: bool,
    /// Nombre : seuls chiffres, `.`, `,` et `-` sont acceptes.
    pub numeric: bool,
    /// Texte riche (`<richtext>`) : un style par caractere (voir
    /// `ui::models::rich`), tenu a jour par chaque modification du texte.
    pub rich: Option<RichText>,
    /// Taille et graisse du texte riche (`font-size`, `font-weight` en rsC ;
    /// un champ simple garde la taille fixe de `draw_ui`).
    pub font_size: f32,
    pub font_weight: f32,
    /// `commandes` (texte riche) : taper `/` en debut de mot previent l'app
    /// (clic `slash-<id>`), pour un menu de commandes.
    pub commands: bool,
    /// Les commandes du menu `/` (voir `interact::command_menu::parse_items`) ;
    /// vide : l'app recoit `slash-<id>` et fait son propre menu.
    pub command_list: String,
    /// `entree` (texte riche) : Entree (sans Maj) previent l'app (clic
    /// `entree-<id>@<curseur>`) au lieu d'aller a la ligne.
    pub enter_submits: bool,
}

// Nombre d'annulations conservees - largement suffisant pour une session
// de frappe normale, sans laisser l'historique grossir indefiniment.
const MAX_UNDO_STEPS: usize = 200;

impl TextArea {
    pub fn new(layout: LayoutProps, background: Color, text_color: Color, text: String) -> TextArea {
        let cursor = text.chars().count();
        TextArea {
            layout,
            background,
            text_color,
            text,
            focused: false,
            cursor,
            selection_anchor: None,
            undo_stack: Vec::new(),
            scroll_offset: 0,
            hover_background: None,
            focus_background: None,
            decoration: Decoration::default(),
            id: String::new(),
            single_line: false,
            placeholder: String::new(),
            password: false,
            numeric: false,
            rich: None,
            font_size: crate::ui::services::draw_ui::TEXTAREA_FONT_SIZE,
            font_weight: crate::ui::services::draw_ui::TEXTAREA_FONT_WEIGHT,
            commands: false,
            command_list: String::new(),
            enter_submits: false,
        }
    }

    /// Une zone de texte riche, remplie depuis le format d'echange (voir
    /// `ui::models::rich::parse`).
    pub fn rich(layout: LayoutProps, background: Color, text_color: Color, value: &str) -> TextArea {
        let (text, styles) = rich::flatten(&rich::parse(value));
        let mut area = TextArea::new(layout, background, text_color, text);
        area.rich = Some(RichText { styles, pending: None });
        area
    }

    /// Le contenu au format d'echange (texte riche seulement).
    pub fn rich_value(&self) -> Option<String> {
        self.rich.as_ref().map(|r| rich::serialize(&rich::spans_of(&self.text, &r.styles)))
    }

    /// Le style du caractere `i` (defaut hors texte ou sans texte riche).
    pub fn style_at(&self, i: usize) -> RichStyle {
        self.rich.as_ref().and_then(|r| r.styles.get(i).cloned()).unwrap_or_default()
    }

    // Style d'un caractere tape au curseur : celui demande (Ctrl+B sans
    // selection), sinon celui du caractere d'avant (du premier selectionne
    // si on remplace une selection). Un lien ne se prolonge pas.
    fn typing_style(&self) -> RichStyle {
        let Some(r) = &self.rich else { return RichStyle::default() };
        if let Some(p) = &r.pending {
            return p.clone();
        }
        let at = match self.selection_range() {
            Some((start, _)) => start,
            None => self.cursor.saturating_sub(1),
        };
        let mut style = r.styles.get(at).cloned().unwrap_or_default();
        style.link.clear();
        style
    }

    fn rich_insert(&mut self, at: usize, count: usize, style: RichStyle) {
        if let Some(r) = &mut self.rich {
            let at = at.min(r.styles.len());
            r.styles.splice(at..at, std::iter::repeat_n(style, count));
            r.pending = None;
        }
    }

    fn rich_remove(&mut self, start: usize, end: usize) {
        if let Some(r) = &mut self.rich {
            let end = end.min(r.styles.len());
            if start < end {
                r.styles.drain(start..end);
            }
        }
    }

    // Le curseur bouge : le style demande pour la frappe suivante est oublie.
    fn clear_pending(&mut self) {
        if let Some(r) = &mut self.rich {
            r.pending = None;
        }
    }

    /// Met ou enleve `mark` sur la selection : gras si une partie ne l'est
    /// pas, sinon retire le gras (une couleur, un lien se posent toujours).
    /// Sans selection, s'applique a ce qui sera tape ensuite. `true` si
    /// quelque chose change.
    pub fn toggle_mark(&mut self, mark: &Mark) -> bool {
        if self.rich.is_none() {
            return false;
        }
        let Some((start, end)) = self.selection_range().filter(|(s, e)| s < e) else {
            let mut style = self.typing_style();
            let on = !mark.toggles() || !mark.on(&style);
            mark.apply(&mut style, on);
            if let Some(r) = &mut self.rich {
                r.pending = Some(style);
            }
            return true;
        };
        let all_on = (start..end).all(|i| mark.on(&self.style_at(i)));
        let on = !mark.toggles() || !all_on;
        self.snapshot();
        if let Some(r) = &mut self.rich {
            for style in r.styles.iter_mut().take(end).skip(start) {
                mark.apply(style, on);
            }
        }
        true
    }

    /// Le texte tel qu'affiche (masque pour un mot de passe, meme nombre de
    /// caracteres : curseur et selection restent alignes).
    pub fn display_text(&self) -> std::borrow::Cow<'_, str> {
        if self.password {
            std::borrow::Cow::Owned("*".repeat(self.text.chars().count()))
        } else {
            std::borrow::Cow::Borrowed(&self.text)
        }
    }

    /// Ce caractere peut-il etre tape dans ce champ ?
    pub fn accepts(&self, c: char) -> bool {
        if self.single_line && (c == '\n' || c == '\r') {
            return false;
        }
        !self.numeric || c.is_ascii_digit() || matches!(c, '.' | ',' | '-')
    }

    // Sauvegarde l'etat courant (texte + curseur) avant une modification -
    // a appeler en tout premier dans toute methode publique qui change
    // `self.text` (pas les simples deplacements/selections, qui ne
    // modifient pas le texte). Volontairement PAS dans `delete_selection`
    // elle-meme : `insert_char`/`insert_str` l'appellent en interne pour
    // remplacer une selection, et une seule frappe qui remplace une
    // selection doit rester une seule etape d'annulation, pas deux.
    fn snapshot(&mut self) {
        let styles = self.rich.as_ref().map(|r| r.styles.clone()).unwrap_or_default();
        self.undo_stack.push((self.text.clone(), self.cursor, styles));
        if self.undo_stack.len() > MAX_UNDO_STEPS {
            self.undo_stack.remove(0);
        }
    }

    /// Annule la derniere modification de texte (Ctrl+Z). Retourne `true`
    /// si quelque chose a effectivement ete restaure.
    pub fn undo(&mut self) -> bool {
        match self.undo_stack.pop() {
            Some((text, cursor, styles)) => {
                self.text = text;
                self.cursor = cursor;
                self.selection_anchor = None;
                if let Some(r) = &mut self.rich {
                    r.styles = styles;
                    r.pending = None;
                }
                true
            }
            None => false,
        }
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.text.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(self.text.len())
    }

    /// Le texte entre le debut et la position `char_idx` (en caracteres) -
    /// utilise pour mesurer ou placer le curseur/la selection a l'ecran
    /// (voir `ui::services::draw_ui::draw_textarea`).
    pub fn text_up_to(&self, char_idx: usize) -> &str {
        &self.text[..self.byte_index(char_idx)]
    }

    /// `(debut, fin)` en caracteres, toujours dans l'ordre croissant peu
    /// importe le sens dans lequel la selection a ete etendue (ancre avant
    /// ou apres le curseur) - `None` si rien n'est selectionne.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| if anchor <= self.cursor { (anchor, self.cursor) } else { (self.cursor, anchor) })
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection_range().map(|(start, end)| {
            let (bs, be) = (self.byte_index(start), self.byte_index(end));
            self.text[bs..be].to_string()
        })
    }

    /// Supprime la selection courante (s'il y en a une) et place le
    /// curseur a son ancien bord gauche. Retourne `true` si quelque chose
    /// a effectivement ete supprime. Public pour `Cut` (voir
    /// `ui::services::interact`), qui doit supprimer la selection sans
    /// passer par `backspace`/`delete_forward`.
    pub fn delete_selection(&mut self) -> bool {
        match self.selection_range() {
            Some((start, end)) => {
                let (bs, be) = (self.byte_index(start), self.byte_index(end));
                self.text.drain(bs..be);
                self.rich_remove(start, end);
                self.cursor = start;
                self.selection_anchor = None;
                true
            }
            None => false,
        }
    }

    /// Insere `c` a la position du curseur (en remplacant la selection
    /// courante s'il y en a une), puis avance le curseur juste apres.
    pub fn insert_char(&mut self, c: char) {
        self.snapshot();
        let style = self.typing_style();
        self.delete_selection();
        let b = self.byte_index(self.cursor);
        self.text.insert(b, c);
        self.rich_insert(self.cursor, 1, style);
        self.cursor += 1;
    }

    /// Meme chose que `insert_char`, mais pour une chaine entiere d'un
    /// coup - utilise par `Paste` (voir `ui::services::interact`).
    pub fn insert_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.snapshot();
        let style = self.typing_style();
        self.delete_selection();
        let b = self.byte_index(self.cursor);
        self.text.insert_str(b, s);
        let count = s.chars().count();
        self.rich_insert(self.cursor, count, style);
        self.cursor += count;
    }

    /// Retire le caractere juste avant le curseur (ou la selection s'il y
    /// en a une). Retourne `true` si quelque chose a effectivement change.
    pub fn backspace(&mut self) -> bool {
        self.snapshot();
        if self.delete_selection() {
            return true;
        }
        if self.cursor == 0 {
            self.undo_stack.pop(); // rien n'a change, pas la peine de polluer l'historique
            return false;
        }
        let start = self.byte_index(self.cursor - 1);
        let end = self.byte_index(self.cursor);
        self.text.drain(start..end);
        self.rich_remove(self.cursor - 1, self.cursor);
        self.cursor -= 1;
        true
    }

    /// Retire le caractere juste apres le curseur (ou la selection s'il y
    /// en a une), sans deplacer le curseur - le "Suppr" classique, a
    /// distinguer de `backspace`. Retourne `true` si quelque chose a
    /// effectivement change.
    pub fn delete_forward(&mut self) -> bool {
        self.snapshot();
        if self.delete_selection() {
            return true;
        }
        if self.cursor >= self.char_count() {
            self.undo_stack.pop();
            return false;
        }
        let start = self.byte_index(self.cursor);
        let end = self.byte_index(self.cursor + 1);
        self.text.drain(start..end);
        self.rich_remove(self.cursor, self.cursor + 1);
        true
    }

    /// Retire la selection courante et la retourne - pour "Couper" (voir
    /// `ui::services::interact::KeyInput::Cut`), sans passer par
    /// `backspace`/`delete_forward`. Fait aussi partie de l'historique
    /// d'annulation, comme les autres operations qui modifient le texte.
    pub fn cut_selection(&mut self) -> Option<String> {
        let text = self.selected_text()?;
        self.snapshot();
        self.delete_selection();
        Some(text)
    }

    // Les quatre deplacements suivants partagent la meme regle : avec
    // `extend` (Shift maintenu), on demarre/agrandit une selection depuis
    // l'ancre courante ; sans `extend`, un deplacement alors qu'une
    // selection existe la referme (colle le curseur au bord touche) au
    // lieu de deplacer d'un cran de plus - le comportement standard d'un
    // champ de texte.
    pub fn move_left(&mut self, extend: bool) {
        self.clear_pending();
        if extend {
            self.selection_anchor.get_or_insert(self.cursor);
            self.cursor = self.cursor.saturating_sub(1);
        } else if let Some((start, _)) = self.selection_range() {
            self.cursor = start;
            self.selection_anchor = None;
        } else {
            self.cursor = self.cursor.saturating_sub(1);
        }
    }

    pub fn move_right(&mut self, extend: bool) {
        self.clear_pending();
        let len = self.char_count();
        if extend {
            self.selection_anchor.get_or_insert(self.cursor);
            self.cursor = (self.cursor + 1).min(len);
        } else if let Some((_, end)) = self.selection_range() {
            self.cursor = end;
            self.selection_anchor = None;
        } else {
            self.cursor = (self.cursor + 1).min(len);
        }
    }

    pub fn move_home(&mut self, extend: bool) {
        self.clear_pending();
        if extend {
            self.selection_anchor.get_or_insert(self.cursor);
        } else {
            self.selection_anchor = None;
        }
        self.cursor = 0;
    }

    pub fn move_end(&mut self, extend: bool) {
        self.clear_pending();
        if extend {
            self.selection_anchor.get_or_insert(self.cursor);
        } else {
            self.selection_anchor = None;
        }
        self.cursor = self.char_count();
    }

    /// Place le curseur (clic, glisser) : comme un deplacement.
    pub fn set_cursor(&mut self, cursor: usize) {
        self.clear_pending();
        self.cursor = cursor;
    }

    pub fn select_all(&mut self) {
        self.clear_pending();
        self.selection_anchor = Some(0);
        self.cursor = self.char_count();
    }
}
