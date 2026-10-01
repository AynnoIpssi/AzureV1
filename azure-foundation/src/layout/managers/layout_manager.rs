// Calcule les boites reelles (en pixels) d'un arbre de `UiNode` a partir
// de leurs `LayoutProps` - x/y/largeur/hauteur/marge sont des pourcentages
// de la boite de CONTENU du PARENT (pas toujours de la fenetre entiere :
// un conteneur imbrique dans un autre positionne ses enfants relativement
// a SA PROPRE boite, comme en CSS), et le padding retrecit reellement
// l'espace transmis aux enfants au lieu d'etre ignore.
use crate::layout::models::layout_props::{AlignItems, DisplayMode, FlexDirection, JustifyContent, LayoutProps, Track};
// Couplage volontaire vers `ui` (dans l'autre sens que d'habitude : `ui`
// depend normalement de `layout`, pas l'inverse) : le flex/grid a besoin de
// voir les `LayoutProps` de TOUS les enfants d'un coup (repartition de
// l'espace, placement de grille), pas un a la fois comme `resolve` - `UiNode`
// est le seul endroit qui les regroupe deja. Sans consequence puisque les
// deux modules vivent dans le meme crate (pas de cycle entre crates).
use crate::ui::models::ui_node::UiNode;

/// Resout la boite `(x, y, largeur, hauteur)` en pixels d'un element a
/// partir de ses `LayoutProps` et de la boite de CONTENU de son parent
/// (voir `content_box`) - passer `(0, 0, window_width, window_height)`
/// pour un element racine, qui n'a pas de vrai parent.
///
/// `x`/`y`/`width`/`height` sont des pourcentages de la largeur/hauteur du
/// parent (`props.x`/`props.width` de `parent.2`, `props.y`/`props.height`
/// de `parent.3`). `margin` deplace l'element entier, uniformement sur les
/// deux axes, sans changer sa taille - un pourcentage de la
/// largeur/hauteur du parent lui aussi, comme x/y.
pub fn resolve(props: &LayoutProps, parent: (u32, u32, u32, u32)) -> (u32, u32, u32, u32) {
    let (parent_x, parent_y, parent_width, parent_height) = parent;

    let margin_x = (parent_width as f32 * props.margin / 100.0) as u32;
    let margin_y = (parent_height as f32 * props.margin / 100.0) as u32;

    let x = parent_x + margin_x + (parent_width as f32 * props.x / 100.0) as u32;
    let y = parent_y + margin_y + (parent_height as f32 * props.y / 100.0) as u32;
    let width = (parent_width as f32 * props.width / 100.0) as u32;
    let height = (parent_height as f32 * props.height / 100.0) as u32;

    (x, y, width, height)
}

/// La boite de CONTENU d'un element deja resolu (voir `resolve`) : sa
/// propre boite retrecie par son `padding` sur les quatre cotes - un
/// pourcentage de sa PROPRE largeur/hauteur (pas celle de son parent).
/// C'est ce rectangle, pas la boite de l'element lui-meme, qu'il faut
/// transmettre a `resolve` pour chacun de ses enfants (voir
/// `ui::services::draw_ui`, `ui::services::interact`) - sinon le padding
/// resterait purement decoratif, sans jamais reellement faire de place.
pub fn content_box(props: &LayoutProps, resolved: (u32, u32, u32, u32)) -> (u32, u32, u32, u32) {
    let (x, y, width, height) = resolved;
    let padding_x = (width as f32 * props.padding / 100.0) as u32;
    let padding_y = (height as f32 * props.padding / 100.0) as u32;

    (
        x + padding_x,
        y + padding_y,
        // `saturating_sub` : un padding trop grand pour la boite (ex: >50%
        // sur chaque cote) donne une boite de contenu vide plutot que de
        // deborder en dessous de zero (u32 ne peut pas etre negatif).
        width.saturating_sub(padding_x * 2),
        height.saturating_sub(padding_y * 2),
    )
}

/// Resout la boite de CHACUN des enfants de `container` d'un coup, dans sa
/// boite de CONTENU `content` (voir `content_box`/`scrollable_content_box`) -
/// le point d'entree unique que `ui::services::draw_ui` et
/// `ui::services::interact` appellent tous les deux pour un `Container`, ce
/// qui garantit qu'ils obtiennent TOUJOURS exactement les memes boites
/// (rendu et hit-test ne peuvent pas diverger, contrairement a si chacun
/// reimplementait sa propre logique de repartition).
///
/// `Block` (par defaut) : comportement historique inchange, chaque enfant se
/// `resolve()` independamment des autres contre `content` - equivalent bit a
/// bit a l'ancien `draw_ui`/`interact` qui appelaient `resolve` eux-memes
/// nœud par nœud. `Flex`/`Grid` : voir `flex_layout`/`grid_layout`.
///
/// `scroll_offset` (voir `ui::models::container::Container::scroll_offset`)
/// decale ENSUITE chaque boite obtenue de `-scroll_offset` verticalement,
/// borne a zero (`saturating_sub`) - `0` = aucun effet, comportement
/// historique inchange. C'est deliberement applique en dernier, apres le
/// calcul Block/Flex/Grid, pour que le defilement reste une simple
/// translation visuelle qui n'interfere avec aucun de ces trois algorithmes
/// de repartition.
pub fn resolve_children(container: &LayoutProps, children: &[UiNode], content: (u32, u32, u32, u32), scroll_offset: u32) -> Vec<(u32, u32, u32, u32)> {
    let boxes = match container.display {
        DisplayMode::Block => children.iter().map(|c| resolve(c.layout(), content)).collect(),
        DisplayMode::Flex => flex_layout(container, children, content),
        DisplayMode::Grid => grid_layout(container, children, content),
    };
    if scroll_offset == 0 {
        return boxes;
    }
    boxes.into_iter().map(|(x, y, w, h)| (x, y.saturating_sub(scroll_offset), w, h)).collect()
}

/// Boite `(x, y, largeur, hauteur)` a position SIGNEE : le contenu d'un
/// conteneur defile peut se trouver au-dessus du haut de la fenetre (ou a
/// gauche), ce qu'un `u32` ne sait pas representer (l'ancien
/// `saturating_sub` recollait alors ces enfants en haut, a y = 0).
pub type Rect = (i32, i32, u32, u32);

pub fn to_rect(b: (u32, u32, u32, u32)) -> Rect {
    (b.0 as i32, b.1 as i32, b.2, b.3)
}

/// Intersection de deux `Rect` - hauteur/largeur 0 si elles ne se
/// touchent pas.
pub fn intersect(a: Rect, b: Rect) -> Rect {
    let x = a.0.max(b.0);
    let y = a.1.max(b.1);
    let right = (a.0 + a.2 as i32).min(b.0 + b.2 as i32);
    let bottom = (a.1 + a.3 as i32).min(b.1 + b.3 as i32);
    (x, y, (right - x).max(0) as u32, (bottom - y).max(0) as u32)
}

/// `true` si `(px, py)` tombe dans `r`.
pub fn contains(r: Rect, px: i32, py: i32) -> bool {
    px >= r.0 && py >= r.1 && px < r.0 + r.2 as i32 && py < r.1 + r.3 as i32
}

/// Resultat de `layout_container` : ou sont les enfants d'un `Container`, et
/// quelle partie de lui les montre.
pub struct ContainerLayout {
    /// Boite de contenu REELLE (le conteneur moins son padding) : sert a
    /// mesurer ce qui depasse (defilement) et a placer la barre de defilement.
    pub visible: Rect,
    /// Zone ou les enfants sont dessines et cliquables : toute la boite du
    /// conteneur, padding compris - comme en CSS, ou le contenu (et son
    /// ombre) peut deborder dans le padding mais jamais au-dela.
    pub clip: Rect,
    /// Boite de chaque enfant, defilement deja applique.
    pub children: Vec<Rect>,
    /// Hauteur totale du contenu (>= `visible.3`) : ce qui depasse de
    /// `visible` est accessible en defilant, jusqu'a `max_scroll`.
    pub content_height: u32,
    pub max_scroll: u32,
    /// Largeur totale du contenu (>= `visible.2`) et defilement horizontal
    /// maximal (`overflow-x`).
    pub content_width: u32,
    pub max_scroll_x: u32,
}

/// Mise en page d'un `Container` avec ses deux defilements.
pub fn container_layout(container: &crate::ui::models::container::Container, own: Rect) -> ContainerLayout {
    layout_container_xy(&container.layout, &container.children, container.scroll_x, container.scroll_offset, own)
}

/// LE calcul de mise en page d'un `Container`, partage par le dessin
/// (`ui::services::draw_ui`) et toutes les interactions
/// (`ui::services::interact`) : ils ne peuvent donc jamais diverger.
///
/// Les enfants sont mis en page une fois a l'origine (0, 0), puis translates
/// a la position (signee) du conteneur moins son defilement - le calcul
/// Block/Flex/Grid ne fait que des additions de positions entieres, donc le
/// resultat est identique a une mise en page faite directement sur place,
/// sans jamais passer par une coordonnee negative en `u32`.
///
/// Un conteneur defilable (`LayoutProps::scrollable`) mesure son contenu :
/// le bas du plus bas de ses enfants (padding bas compris). Avec l'ancien
/// `content_height`, les enfants se placent en plus contre une boite plus
/// haute que la zone visible (voir `scrollable_content_box`).
pub fn layout_container(props: &LayoutProps, children: &[UiNode], scroll_offset: u32, own: Rect) -> ContainerLayout {
    layout_container_xy(props, children, 0, scroll_offset, own)
}

/// Comme `layout_container`, avec aussi le defilement horizontal.
pub fn layout_container_xy(props: &LayoutProps, children: &[UiNode], scroll_x: u32, scroll_offset: u32, own: Rect) -> ContainerLayout {
    // Arbre construit depuis rsC : mise en page CSS (voir `web_layout`).
    if let Some(css) = props.css.as_deref() {
        return crate::layout::managers::web_layout::layout_web_container(css, children, (props.scrollable_x(), props.scrollable()), (scroll_x, scroll_offset), own);
    }
    let (w, h) = (own.2, own.3);
    let content = content_box(props, (0, 0, w, h));
    let layout_box = scrollable_content_box(props, (0, 0, w, h));
    let boxes = resolve_children(props, children, layout_box, 0);

    let (content_height, max_scroll) = if props.scrollable() {
        let padding_bottom = content.1;
        let bottom = boxes.iter().map(|b| b.1 + b.3).max().unwrap_or(0) + padding_bottom;
        let extent = bottom.max(layout_box.1 + layout_box.3).saturating_sub(content.1);
        (extent.max(content.3), extent.saturating_sub(content.3))
    } else {
        (content.3, 0)
    };
    let scroll = scroll_offset.min(max_scroll) as i32;

    ContainerLayout {
        visible: (own.0 + content.0 as i32, own.1 + content.1 as i32, content.2, content.3),
        clip: own,
        children: boxes.into_iter().map(|(x, y, bw, bh)| (own.0 + x as i32, own.1 + y as i32 - scroll, bw, bh)).collect(),
        content_height,
        max_scroll,
        content_width: content.2,
        max_scroll_x: 0,
    }
}

/// La boite de CONTENU "defilable" d'un `Container` : identique a
/// `content_box` si `props.content_height` est `None` (comportement
/// historique inchange), sinon une boite de MEME position/largeur mais dont
/// la hauteur est le pourcentage `content_height` de la hauteur REELLE de
/// `resolved` (le nœud lui-meme, avant retrecissement par le padding) -
/// c'est cette boite, plus haute que ce qui est reellement visible, qu'il
/// faut transmettre a `resolve_children` pour que les enfants d'un
/// `Container` defilable se positionnent au-dela de sa zone visible plutot
/// que d'y etre ecrases. La boite REELLE (`content_box`) reste seule
/// utilisee pour le decoupage a l'affichage (voir
/// `ui::services::draw_ui::draw_container`) et pour le hit-test de survol
/// (`ui::services::interact::hit`) : c'est cette difference entre boite de
/// LAYOUT (virtuelle, ici) et boite de CLIP/HIT (reelle, `content_box`) qui
/// rend le contenu debordant invisible et non cliquable hors de la zone
/// visible, sans jamais fausser sa position calculee.
pub fn scrollable_content_box(props: &LayoutProps, resolved: (u32, u32, u32, u32)) -> (u32, u32, u32, u32) {
    let (x, y, width, height) = content_box(props, resolved);
    match props.content_height {
        None => (x, y, width, height),
        Some(pct) => {
            let (_, _, _, own_height) = resolved;
            let virtual_height = (own_height as f32 * pct / 100.0) as u32;
            (x, y, width, virtual_height)
        }
    }
}

// Retrecit `box_` de `margin_pct` sur les quatre cotes - contrairement a
// `resolve` (ou `margin` deplace l'element SANS changer sa taille, relatif
// aux dimensions du PARENT), un enfant flex/grid occupe une cellule qui lui
// est exclusivement allouee : une simple translation deborderait sur la
// cellule voisine plutot que de creer une vraie gouttiere. `margin` y est
// donc un pourcentage de la taille de la cellule ELLE-MEME (meme convention
// que `padding` dans `content_box`), retire des deux cotes de chaque axe.
fn inset_by_margin(box_: (u32, u32, u32, u32), margin_pct: f32) -> (u32, u32, u32, u32) {
    let (x, y, width, height) = box_;
    let mx = (width as f32 * margin_pct / 100.0) as u32;
    let my = (height as f32 * margin_pct / 100.0) as u32;
    (x + mx, y + my, width.saturating_sub(mx * 2), height.saturating_sub(my * 2))
}

struct FlexItem {
    basis: f32,
    grow: f32,
    shrink: f32,
    cross_natural: f32,
    margin: f32,
}

/// Vrai moteur flexbox (mono-passe, sans notion de min/max-content) : chaque
/// "ligne" (voir `flex_wrap`) repartit son espace libre par
/// `flex-grow`/`flex-shrink`, positionne ses elements sur l'axe principal
/// selon `justify-content`, puis les lignes s'empilent le long de l'axe
/// secondaire dans l'ordre - PAS de repartition `align-content` de l'espace
/// secondaire restant entre plusieurs lignes (elles restent simplement
/// empaquetees depuis le debut) : limitation assumee, le cas a une seule
/// ligne (le plus courant) n'est pas concerne.
fn flex_layout(container: &LayoutProps, children: &[UiNode], content: (u32, u32, u32, u32)) -> Vec<(u32, u32, u32, u32)> {
    let (cx, cy, cw, ch) = content;
    let row = container.flex_direction == FlexDirection::Row;
    let main_size = if row { cw as f32 } else { ch as f32 };
    let cross_size = if row { ch as f32 } else { cw as f32 };
    let gap = main_size * container.gap / 100.0;

    let items: Vec<FlexItem> = children
        .iter()
        .map(|c| {
            let l = c.layout();
            let main_pct = l.flex_basis.unwrap_or(if row { l.width } else { l.height });
            let cross_pct = if row { l.height } else { l.width };
            FlexItem {
                basis: main_size * main_pct / 100.0,
                grow: l.flex_grow,
                shrink: l.flex_shrink,
                cross_natural: cross_size * cross_pct / 100.0,
                margin: l.margin,
            }
        })
        .collect();

    // Decoupage en lignes : `!flex_wrap` (le defaut) garde tout sur une
    // seule ligne, quitte a deborder/se retrecir - exactement `nowrap` en CSS.
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut current_main = 0.0f32;
    for (i, item) in items.iter().enumerate() {
        let added = item.basis + if current.is_empty() { 0.0 } else { gap };
        if container.flex_wrap && !current.is_empty() && current_main + added > main_size {
            lines.push(std::mem::take(&mut current));
            current_main = 0.0;
            current.push(i);
            current_main += item.basis;
        } else {
            current.push(i);
            current_main += added;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    let single_line = lines.len() <= 1;

    let mut result = vec![(0u32, 0u32, 0u32, 0u32); children.len()];
    let mut cross_cursor = 0.0f32;

    for line in &lines {
        let sum_basis: f32 = line.iter().map(|&i| items[i].basis).sum();
        let total_gap = gap * (line.len().saturating_sub(1)) as f32;
        let free = main_size - total_gap - sum_basis;

        let mut sizes = vec![0.0f32; line.len()];
        if free >= 0.0 {
            let sum_grow: f32 = line.iter().map(|&i| items[i].grow).sum();
            for (k, &i) in line.iter().enumerate() {
                sizes[k] = items[i].basis + if sum_grow > 0.0 { free * items[i].grow / sum_grow } else { 0.0 };
            }
        } else {
            let sum_weight: f32 = line.iter().map(|&i| items[i].shrink * items[i].basis).sum();
            for (k, &i) in line.iter().enumerate() {
                let weight = items[i].shrink * items[i].basis;
                let reduction = if sum_weight > 0.0 { (-free) * weight / sum_weight } else { 0.0 };
                sizes[k] = (items[i].basis - reduction).max(0.0);
            }
        }

        let used = sizes.iter().sum::<f32>() + total_gap;
        let remaining = (main_size - used).max(0.0);
        let count = line.len();
        let (mut cursor, extra_gap) = match container.justify_content {
            JustifyContent::Start => (0.0, 0.0),
            JustifyContent::End => (remaining, 0.0),
            JustifyContent::Center => (remaining / 2.0, 0.0),
            JustifyContent::SpaceBetween if count > 1 => (0.0, remaining / (count - 1) as f32),
            JustifyContent::SpaceBetween => (0.0, 0.0),
            JustifyContent::SpaceAround => {
                let slot = if count > 0 { remaining / count as f32 } else { 0.0 };
                (slot / 2.0, slot)
            }
            JustifyContent::SpaceEvenly => {
                let slot = remaining / (count + 1) as f32;
                (slot, slot)
            }
        };

        // Etendue transversale de CETTE ligne : plein axe secondaire du
        // conteneur s'il n'y a qu'une seule ligne (le cas `stretch` usuel,
        // sans wrap), sinon la plus grande taille naturelle parmi ses
        // elements (evite que plusieurs lignes "stretch" se chevauchent).
        let line_cross_extent = if single_line {
            cross_size
        } else {
            line.iter().map(|&i| items[i].cross_natural).fold(0.0, f32::max)
        };

        for (k, &i) in line.iter().enumerate() {
            let cross_size_i = if container.align_items == AlignItems::Stretch {
                line_cross_extent
            } else {
                items[i].cross_natural
            };
            let cross_pos = match container.align_items {
                AlignItems::Stretch | AlignItems::Start => 0.0,
                AlignItems::End => line_cross_extent - cross_size_i,
                AlignItems::Center => (line_cross_extent - cross_size_i) / 2.0,
            };

            let (main_pos, main_len) = (cursor, sizes[k]);
            let box_ = if row {
                (cx + main_pos as u32, cy + (cross_cursor + cross_pos) as u32, main_len as u32, cross_size_i as u32)
            } else {
                (cx + (cross_cursor + cross_pos) as u32, cy + main_pos as u32, cross_size_i as u32, main_len as u32)
            };
            result[i] = inset_by_margin(box_, items[i].margin);

            cursor += main_len + gap + extra_gap;
        }

        cross_cursor += line_cross_extent + gap;
    }

    result
}

// Etend `tracks` a `n` pistes : repete la derniere piste declaree tant qu'il
// en manque (lignes implicites du flux automatique d'une grille - voir
// `grid_layout`), ou partage l'axe en `n` pistes egales si aucune piste
// n'est declaree du tout (une grille sans template ne doit pas paniquer,
// juste se comporter comme `n` colonnes/lignes de meme taille).
fn extend_tracks(tracks: &[Track], n: usize) -> Vec<Track> {
    let n = n.max(1);
    if tracks.is_empty() {
        return vec![Track::Percent(100.0 / n as f32); n];
    }
    let mut out = tracks.to_vec();
    while out.len() < n {
        out.push(*tracks.last().unwrap());
    }
    out
}

// Position/taille en pixels de chacune des `n` pistes d'un axe (colonnes OU
// lignes) : les pistes `Percent` sont une part fixe de `total_px`, les
// pistes `Fr` se partagent ce qu'il en reste une fois les pistes `Percent`
// ET les `gap` entre pistes retires - meme semantique que l'unite CSS `fr`.
fn track_offsets(tracks: &[Track], n: usize, total_px: f32, gap_pct: f32) -> Vec<(f32, f32)> {
    let gap_px = total_px * gap_pct / 100.0;
    let total_gap = gap_px * n.saturating_sub(1) as f32;
    let available = (total_px - total_gap).max(0.0);

    let sizes = extend_tracks(tracks, n);
    let sum_percent: f32 = sizes.iter().map(|t| match t { Track::Percent(p) => *p, Track::Fr(_) => 0.0 }).sum();
    let sum_fr: f32 = sizes.iter().map(|t| match t { Track::Fr(f) => *f, Track::Percent(_) => 0.0 }).sum();
    // Garde-fou si la somme des pourcentages depasse 100% : ne consomme
    // jamais plus que `available`, plutot que de donner un `remaining`
    // negatif aux pistes `Fr`.
    let percent_px = available * (sum_percent / 100.0).min(1.0);
    let remaining = (available - percent_px).max(0.0);
    let fr_unit = if sum_fr > 0.0 { remaining / sum_fr } else { 0.0 };

    let mut offsets = Vec::with_capacity(n);
    let mut cursor = 0.0f32;
    for t in sizes {
        let size = match t {
            Track::Percent(p) => available * p / 100.0,
            Track::Fr(f) => f * fr_unit,
        };
        offsets.push((cursor, size));
        cursor += size + gap_px;
    }
    offsets
}

/// Grille 2D avec pistes de taille fixe (`Track::Percent`) ou flexible
/// (`Track::Fr`) : chaque enfant se place soit explicitement
/// (`grid_column`/`grid_row`, 1-based, + leur `_span`), soit en flux
/// automatique ligne par ligne (comme des balises `<div>` dans une grille
/// CSS sans `grid-column` explicite). Simplification assumee : un enfant qui
/// precise UN SEUL des deux axes retombe sur la piste 1 de l'autre plutot
/// que de continuer le flux automatique sur cet axe - pas la reelle
/// interaction (complexe) entre placement explicite et implicite de CSS Grid.
///
/// Un placement explicite qui deborde du gabarit declare (`grid_column`/
/// `grid_row` au-dela de `grid_template_columns`/`_rows`) ETEND la grille
/// plutot que d'etre ecrase dans la derniere piste existante (voir
/// `max_col_end`/`max_row_end` ci-dessous, et `extend_tracks` qui repete la
/// derniere piste declaree pour ces colonnes/lignes implicites) - les deux
/// axes suivent desormais exactement la meme regle, la ou seul l'axe des
/// lignes le faisait avant ce correctif (un `grid_column` trop grand se
/// serait alors superpose silencieusement au contenu de la derniere colonne
/// declaree).
fn grid_layout(container: &LayoutProps, children: &[UiNode], content: (u32, u32, u32, u32)) -> Vec<(u32, u32, u32, u32)> {
    let (cx, cy, cw, ch) = content;
    // Largeur de ligne (en nombre de pistes) utilisee par le flux
    // AUTOMATIQUE pour decider quand passer a la ligne suivante - toujours
    // celle du gabarit declare, jamais etendue par un placement explicite
    // (voir `n_cols` plus bas, qui lui peut l'etre) : un flux automatique ne
    // doit pas se mettre a deborder juste parce qu'un AUTRE enfant, ailleurs,
    // a demande une colonne hors gabarit.
    let template_cols = container.grid_template_columns.len().max(1);

    // Passe 1 : determine le placement (colonne, ligne, etendues) de chaque
    // enfant, et le nombre de colonnes/lignes reellement necessaires (le
    // gabarit peut etre plus court que ce qu'un placement explicite ou le
    // flux automatique remplissent).
    let mut placements = Vec::with_capacity(children.len());
    let mut cursor_col = 1usize;
    let mut cursor_row = 1usize;
    let mut max_row_end = 1usize;
    let mut max_col_end = template_cols;
    for child in children {
        let l = child.layout();
        let col_span = l.grid_column_span.max(1);
        let row_span = l.grid_row_span.max(1);
        let (col, row) = match (l.grid_column, l.grid_row) {
            (None, None) => {
                if cursor_col != 1 && cursor_col + col_span - 1 > template_cols {
                    cursor_col = 1;
                    cursor_row += 1;
                }
                let placed = (cursor_col, cursor_row);
                cursor_col += col_span;
                placed
            }
            (col, row) => (col.unwrap_or(1).max(1), row.unwrap_or(1).max(1)),
        };
        max_row_end = max_row_end.max(row + row_span - 1);
        max_col_end = max_col_end.max(col + col_span - 1);
        placements.push((col, row, col_span, row_span));
    }
    let n_cols = max_col_end;
    let n_rows = container.grid_template_rows.len().max(1).max(max_row_end);

    let col_tracks = track_offsets(&container.grid_template_columns, n_cols, cw as f32, container.gap);
    let row_tracks = track_offsets(&container.grid_template_rows, n_rows, ch as f32, container.gap);

    children
        .iter()
        .zip(placements)
        .map(|(child, (col, row, col_span, row_span))| {
            let col_i = (col - 1).min(n_cols - 1);
            let col_end_i = (col + col_span - 2).min(n_cols - 1);
            let row_i = (row - 1).min(n_rows - 1);
            let row_end_i = (row + row_span - 2).min(n_rows - 1);

            let x = cx as f32 + col_tracks[col_i].0;
            let y = cy as f32 + row_tracks[row_i].0;
            let width = (col_tracks[col_end_i].0 + col_tracks[col_end_i].1 - col_tracks[col_i].0).max(0.0);
            let height = (row_tracks[row_end_i].0 + row_tracks[row_end_i].1 - row_tracks[row_i].0).max(0.0);

            inset_by_margin((x as u32, y as u32, width as u32, height as u32), child.layout().margin)
        })
        .collect()
}
