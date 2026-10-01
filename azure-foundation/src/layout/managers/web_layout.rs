// Mise en page "web" des arbres construits depuis rsC : le modele de boite
// CSS (marge, bordure, padding, contenu), le flux normal des blocs, flexbox
// et grid, avec des tailles en px / % / auto et des hauteurs qui suivent le
// contenu (texte qui revient a la ligne compris) - comme un navigateur.
//
// Deux operations :
// - `measure` : la taille (bordure comprise) d'un nœud pour une largeur de
//   bloc conteneur donnee. Resultat mis en cache dans sa `CssBox` : il ne
//   depend que de cette largeur, pas de la position ni du defilement.
// - `place_children` : la position de chaque enfant dans la boite de contenu
//   d'un conteneur, calculee a partir des mesures.
//
// Ecarts connus avec CSS : pas de fusion des marges verticales, pas de flux
// "inline" (chaque texte est un bloc), `position: absolute` se place dans
// son conteneur direct (pas le plus proche ancetre positionne), et un
// pourcentage de padding se rapporte a la largeur du bloc conteneur connue
// lors de la derniere mesure.
use crate::layout::managers::layout_manager::{resolve, ContainerLayout, Rect};
use crate::layout::models::css_box::{AlignContent, CssBox, CssDisplay, CssPosition, CssTrack, Length};
use crate::layout::models::layout_props::{AlignItems, FlexDirection, JustifyContent};
use crate::ui::models::ui_node::UiNode;
use crate::ui::services::text_block;
use azure_engine::rendering::managers::renderer::{load_image, measure_text_width};

/// Largeur "infinie" : mesure d'un contenu sans contrainte de largeur.
const UNBOUNDED: f32 = 1.0e7;

// Ce qui est mesure (dernier element de la cle de cache).
const FILL: u8 = 0;
const SHRINK: u8 = 1;
const MAX_CONTENT: u8 = 2;
const MIN_CONTENT: u8 = 3;

fn css_of(node: &UiNode) -> Option<&CssBox> {
    node.layout().css.as_deref()
}

/// Hors du flux : `display: none`, `position: fixed` (place a part, voir
/// `fixed_box`) ou `absolute` (voir `layout_web_container`).
fn out_of_flow(css: &CssBox) -> bool {
    css.display == CssDisplay::None || css.fixed || css.position == CssPosition::Absolute
}

/// `position: absolute` ?
pub fn is_absolute(node: &UiNode) -> bool {
    css_of(node).is_some_and(|c| c.position == CssPosition::Absolute && !c.fixed && c.display != CssDisplay::None)
}

/// `position: fixed` ?
pub fn is_fixed(node: &UiNode) -> bool {
    css_of(node).is_some_and(|c| c.fixed && c.display != CssDisplay::None)
}

pub fn z_index(node: &UiNode) -> i32 {
    css_of(node).map(|c| c.z_index).unwrap_or(0)
}

/// Boite d'un element `position: fixed` dans la fenetre `viewport` :
/// `left`/`right`/`top`/`bottom` et `width`/`height` comme en CSS (les deux
/// cotes opposes donnes = etire entre eux ; sinon la taille du contenu).
pub fn fixed_box(node: &UiNode, viewport: (u32, u32, u32, u32)) -> Rect {
    inset_box(node, (viewport.0 as i32, viewport.1 as i32, viewport.2, viewport.3))
}

/// Boite d'un element place par `top`/`right`/`bottom`/`left` dans la boite
/// de reference `viewport` (la fenetre pour `fixed`, la boite de son
/// conteneur pour `absolute`).
fn inset_box(node: &UiNode, viewport: Rect) -> Rect {
    let Some(css) = css_of(node) else { return (viewport.0, viewport.1, 0, 0) };
    let (vw, vh) = (viewport.2 as f32, viewport.3 as f32);
    let inset = css.inset;
    let (l, r, t, b) = (inset.left.resolve(Some(vw)), inset.right.resolve(Some(vw)), inset.top.resolve(Some(vh)), inset.bottom.resolve(Some(vh)));
    let f = frame(css, vw);
    let (frame_w, frame_h) = (f[1] + f[3], f[0] + f[2]);
    let width = match (to_border_box(css, css.width, Some(vw), frame_w), l, r) {
        (Some(w), _, _) => w,
        (None, Some(l), Some(r)) => (vw - l - r).max(0.0),
        _ => measure(node, vw, None, false).0,
    };
    let width = clamp_size(css, width, css.min_width, css.max_width, Some(vw), frame_w);
    let height = match (to_border_box(css, css.height, Some(vh), frame_h), t, b) {
        (Some(h), _, _) => h,
        (None, Some(t), Some(b)) => (vh - t - b).max(0.0),
        _ => content_height(node, width - frame_w, None) + frame_h,
    };
    let height = clamp_size(css, height, css.min_height, css.max_height, Some(vh), frame_h);
    let x = match (l, r) {
        (Some(l), _) => l,
        (None, Some(r)) => vw - r - width,
        _ => 0.0,
    };
    let y = match (t, b) {
        (Some(t), _) => t,
        (None, Some(b)) => vh - b - height,
        _ => 0.0,
    };
    to_pixels((viewport.0, viewport.1), (x, y, width, height))
}

/// `true` si ce nœud suit le modele web (construit depuis rsC).
pub fn is_web(node: &UiNode) -> bool {
    css_of(node).is_some()
}

/// Oublie les tailles memorisees de tout l'arbre : a appeler quand un
/// contenu change de taille sans reconstruire la page (texte riche tape).
pub fn forget_sizes(nodes: &[UiNode]) {
    for node in nodes {
        if let Some(css) = css_of(node) {
            css.cache.borrow_mut().clear();
        }
        if let UiNode::Container(c) = node {
            forget_sizes(&c.children);
        }
    }
}

fn cached(css: &CssBox, key: (i32, i32, u8), compute: impl FnOnce() -> (f32, f32)) -> (f32, f32) {
    if let Some((_, v)) = css.cache.borrow().iter().find(|(k, _)| *k == key) {
        return *v;
    }
    let value = compute();
    let mut cache = css.cache.borrow_mut();
    if cache.len() >= 8 {
        cache.remove(0);
    }
    cache.push((key, value));
    value
}

fn key(width: f32, height: Option<f32>, what: u8) -> (i32, i32, u8) {
    ((width.min(UNBOUNDED) * 10.0).round() as i32, height.map(|h| (h * 10.0).round() as i32).unwrap_or(-1), what)
}

// Padding + bordure (haut, droite, bas, gauche) pour un bloc conteneur de
// largeur `cbw` - memorise pour `layout_web_container`.
fn frame(css: &CssBox, cbw: f32) -> [f32; 4] {
    let r = if cbw >= UNBOUNDED { 0.0 } else { cbw };
    let p = css.padding;
    let b = css.border;
    let f = [p.top.or_zero(r) + b.top, p.right.or_zero(r) + b.right, p.bottom.or_zero(r) + b.bottom, p.left.or_zero(r) + b.left];
    css.frame.set(f);
    f
}

// Marges en px (auto = 0) pour un bloc conteneur de largeur `cbw`.
fn margins(css: &CssBox, cbw: f32) -> [f32; 4] {
    let r = if cbw >= UNBOUNDED { 0.0 } else { cbw };
    let m = css.margin;
    [m.top.or_zero(r), m.right.or_zero(r), m.bottom.or_zero(r), m.left.or_zero(r)]
}

// Une taille specifiee (`width`, `min-height`...) convertie en taille de
// boite bordure comprise.
fn to_border_box(css: &CssBox, length: Length, reference: Option<f32>, frame_size: f32) -> Option<f32> {
    length.resolve(reference).map(|v| if css.border_box { v.max(frame_size) } else { v + frame_size })
}

fn clamp_size(css: &CssBox, value: f32, min: Length, max: Length, reference: Option<f32>, frame_size: f32) -> f32 {
    let mut v = value;
    if let Some(max) = to_border_box(css, max, reference, frame_size) {
        v = v.min(max);
    }
    if let Some(min) = to_border_box(css, min, reference, frame_size) {
        v = v.max(min);
    }
    v.max(frame_size)
}

// Un element "bloc" occupe toute la largeur disponible quand sa largeur est
// `auto` ; les autres (bouton, image, video) prennent la largeur de leur
// contenu, comme les elements `inline-block` d'un navigateur.
fn fills_width(node: &UiNode) -> bool {
    match node {
        UiNode::Container(_) | UiNode::Label(_) => true,
        UiNode::TextArea(area) => !area.single_line,
        UiNode::Control(c) => c.kind == crate::ui::models::control::ControlKind::Progress,
        _ => false,
    }
}

/// Taille (bordure comprise) de `node` dans un bloc conteneur de largeur
/// `cbw` et de hauteur `cbh` (si elle est connue). `fill` : une largeur
/// `auto` occupe toute la place (bloc), sinon elle suit le contenu.
pub fn measure(node: &UiNode, cbw: f32, cbh: Option<f32>, fill: bool) -> (f32, f32) {
    let Some(css) = css_of(node) else {
        let (_, _, w, h) = resolve(node.layout(), (0, 0, cbw.min(UNBOUNDED) as u32, cbh.unwrap_or(0.0) as u32));
        return (w as f32, h as f32);
    };
    if css.display == CssDisplay::None {
        return (0.0, 0.0);
    }
    let fill = fill && cbw < UNBOUNDED;
    cached(css, key(cbw, cbh, if fill { FILL } else { SHRINK }), || {
        let f = frame(css, cbw);
        let m = margins(css, cbw);
        let (frame_w, frame_h) = (f[1] + f[3], f[0] + f[2]);
        let available = (cbw - m[1] - m[3]).max(0.0);
        let reference_w = (cbw < UNBOUNDED).then_some(cbw);

        let width = match to_border_box(css, css.width, reference_w, frame_w) {
            Some(w) => w,
            None if fill => available,
            None => {
                let max = max_content_width(node) + frame_w;
                let min = min_content_width(node) + frame_w;
                max.min(available.max(min))
            }
        };
        let width = clamp_size(css, width, css.min_width, css.max_width, reference_w, frame_w);

        let specified_h = to_border_box(css, css.height, cbh, frame_h);
        let height = match specified_h {
            Some(h) => h,
            None => content_height(node, width - frame_w, None) + frame_h,
        };
        (width, clamp_size(css, height, css.min_height, css.max_height, cbh, frame_h))
    })
}

// Hauteur du contenu de `node` pour une largeur de contenu `cw`.
fn content_height(node: &UiNode, cw: f32, ch: Option<f32>) -> f32 {
    match node {
        UiNode::Label(label) => match &label.text_style {
            Some(style) => {
                let wrap = (cw < UNBOUNDED).then_some(cw.max(1.0));
                text_block::lines(&label.text, style, label.font_size, label.weight, wrap).len() as f32 * style.line_height
            }
            None => label.font_size * 1.25,
        },
        UiNode::Button(button) => button.text_style.as_ref().map(|s| s.line_height).unwrap_or(button.font_size * 1.25),
        UiNode::Image(image) => match load_image(&image.src) {
            Ok(img) if img.width > 0 => img.height as f32 * cw / img.width as f32,
            _ => 150.0,
        },
        UiNode::Video(_) => cw * 9.0 / 16.0,
        UiNode::TextArea(area) if area.single_line => 36.0,
        // Texte riche : grandit avec ses lignes (voir `rich_layout`).
        UiNode::TextArea(area) if area.rich.is_some() => {
            let pad = crate::ui::services::draw_ui::TEXTAREA_TEXT_PADDING as f32;
            let styles = area.rich.as_ref().map(|r| r.styles.as_slice()).unwrap_or(&[]);
            let (_, lines) = crate::ui::services::rich_layout::layout(&area.text, styles, area.font_size, area.font_weight, (cw - 2.0 * pad).max(1.0));
            crate::ui::services::rich_layout::line_heights(&area.text, styles, area.font_size, &lines).iter().sum::<u32>() as f32 + 2.0 * pad
        }
        UiNode::TextArea(_) => 96.0,
        UiNode::Control(control) => crate::ui::services::draw_control::natural_height(control),
        UiNode::Container(container) => {
            let Some(css) = container.layout.css.as_deref() else { return 0.0 };
            place_children(css, &container.children, cw, ch).height
        }
    }
}

/// Largeur de contenu (sans padding ni bordure) sans aucune contrainte : le
/// texte sur une seule ligne, les enfants cote a cote en flex ligne...
pub fn max_content_width(node: &UiNode) -> f32 {
    intrinsic_width(node, MAX_CONTENT)
}

/// La plus petite largeur de contenu sans debordement (le plus long mot).
pub fn min_content_width(node: &UiNode) -> f32 {
    intrinsic_width(node, MIN_CONTENT)
}

fn intrinsic_width(node: &UiNode, what: u8) -> f32 {
    let compute = || -> f32 {
        match node {
            UiNode::Label(label) => match &label.text_style {
                Some(style) if what == MIN_CONTENT => text_block::min_content_width(&label.text, style, label.font_size, label.weight),
                Some(style) => text_block::lines(&label.text, style, label.font_size, label.weight, None).iter().map(|l| l.2).fold(0.0, f32::max),
                None => measure_text_width(&label.text, crate::ui::services::draw_ui::FONT_PATH, label.font_size, label.weight).unwrap_or(0.0),
            },
            UiNode::Button(button) => {
                let font = button.text_style.as_ref().map(|s| s.font).unwrap_or(crate::ui::services::draw_ui::FONT_PATH);
                measure_text_width(&button.text, font, button.font_size, button.font_weight).unwrap_or(0.0).ceil()
            }
            UiNode::Image(image) => load_image(&image.src).map(|img| img.width as f32).unwrap_or(300.0),
            UiNode::Video(_) => 300.0,
            // Un champ peut retrecir (son texte defile) : petite largeur minimale.
            UiNode::TextArea(_) if what == MIN_CONTENT => 40.0,
            UiNode::TextArea(_) => 240.0,
            UiNode::Control(control) => crate::ui::services::draw_control::natural_width(control),
            UiNode::Container(container) => {
                let Some(css) = container.layout.css.as_deref() else { return 0.0 };
                let outer: Vec<f32> = container
                    .children
                    .iter()
                    .filter(|c| css_of_display(c) != Some(CssDisplay::None) && !is_fixed(c))
                    .map(|c| outer_intrinsic_width(c, what))
                    .collect();
                let gap = css.column_gap.resolve(None).unwrap_or(0.0);
                let gaps = gap * outer.len().saturating_sub(1) as f32;
                match css.display {
                    CssDisplay::Flex if css.flex_direction == FlexDirection::Row => {
                        if what == MIN_CONTENT && css.flex_wrap {
                            outer.iter().copied().fold(0.0, f32::max)
                        } else {
                            outer.iter().sum::<f32>() + gaps
                        }
                    }
                    CssDisplay::Grid => {
                        let columns = css.grid_template_columns.len().max(1);
                        let widest = outer.iter().copied().fold(0.0, f32::max);
                        let fixed: f32 = css.grid_template_columns.iter().map(|t| if let CssTrack::Px(px) = t { *px } else { widest }).sum();
                        fixed.max(widest) + gap * (columns - 1) as f32
                    }
                    _ => outer.iter().copied().fold(0.0, f32::max),
                }
            }
        }
    };
    match css_of(node) {
        Some(css) => cached(css, (0, 0, what), || (compute(), 0.0)).0,
        None => compute(),
    }
}

fn css_of_display(node: &UiNode) -> Option<CssDisplay> {
    css_of(node).map(|c| c.display)
}

// Largeur intrinseque d'un enfant marges, padding et bordure compris ; une
// largeur fixee en px l'emporte sur son contenu.
fn outer_intrinsic_width(node: &UiNode, what: u8) -> f32 {
    let Some(css) = css_of(node) else { return 0.0 };
    let f = frame(css, 0.0);
    let m = margins(css, 0.0);
    let frame_w = f[1] + f[3];
    let inner = match css.width {
        Length::Px(px) => if css.border_box { px } else { px + frame_w },
        _ => intrinsic_width(node, what) + frame_w,
    };
    // `min-width` / `max-width` comptent aussi (sinon un bouton elargi par
    // son min-width deborde du conteneur qui s'est mesure sans).
    let inner = clamp_size(css, inner, css.min_width, css.max_width, None, frame_w);
    inner + m[1] + m[3]
}

/// Position des enfants dans une boite de contenu, relative a son coin haut
/// gauche, et hauteur occupee.
pub struct Placement {
    pub boxes: Vec<(f32, f32, f32, f32)>,
    pub height: f32,
}

/// Place `children` dans une boite de contenu de largeur `cw` et de hauteur
/// `ch` (connue si le conteneur a une hauteur definie).
pub fn place_children(css: &CssBox, children: &[UiNode], cw: f32, ch: Option<f32>) -> Placement {
    let mut placement = match css.display {
        CssDisplay::Flex => place_flex(css, children, cw, ch),
        CssDisplay::Grid => place_grid(css, children, cw, ch),
        _ => place_block(children, cw, ch),
    };
    // `position: relative` : decale apres coup, sans rien deplacer autour.
    for (child, b) in children.iter().zip(placement.boxes.iter_mut()) {
        let Some(c) = css_of(child) else { continue };
        if c.position != CssPosition::Relative {
            continue;
        }
        let dx = c.inset.left.resolve(Some(cw)).or_else(|| c.inset.right.resolve(Some(cw)).map(|r| -r)).unwrap_or(0.0);
        let dy = c.inset.top.resolve(ch).or_else(|| c.inset.bottom.resolve(ch).map(|v| -v)).unwrap_or(0.0);
        b.0 += dx;
        b.1 += dy;
    }
    placement
}

// Flux normal : les blocs s'empilent verticalement, chacun sur toute la
// largeur (sauf largeur fixee) ; `margin: 0 auto` centre un bloc.
fn place_block(children: &[UiNode], cw: f32, ch: Option<f32>) -> Placement {
    let mut y = 0.0f32;
    let mut boxes = Vec::with_capacity(children.len());
    for child in children {
        let Some(c) = css_of(child) else {
            let (x, cy, w, h) = resolve(child.layout(), (0, 0, cw as u32, ch.unwrap_or(0.0) as u32));
            boxes.push((x as f32, cy as f32, w as f32, h as f32));
            continue;
        };
        if out_of_flow(c) {
            boxes.push((0.0, y, 0.0, 0.0));
            continue;
        }
        let (w, h) = measure(child, cw, ch, fills_width(child));
        let m = margins(c, cw);
        let free = (cw - w - m[1] - m[3]).max(0.0);
        let x = match (c.margin.left.is_auto(), c.margin.right.is_auto()) {
            (true, true) => m[3] + free / 2.0,
            (true, false) => m[3] + free,
            _ => m[3],
        };
        y += m[0];
        boxes.push((x, y, w, h));
        y += h + m[2];
    }
    Placement { boxes, height: y }
}

struct FlexItem {
    index: usize,
    margin: [f32; 4],
    // Marges `auto` sur l'axe principal (debut, fin).
    auto_margin: (bool, bool),
    base: f32,
    min_main: f32,
    max_main: f32,
    grow: f32,
    shrink: f32,
    main: f32,
    cross: f32,
    align: AlignItems,
}

fn place_flex(css: &CssBox, children: &[UiNode], cw: f32, ch: Option<f32>) -> Placement {
    let row = css.flex_direction == FlexDirection::Row;
    let main_size = if row { Some(cw) } else { ch };
    let main_gap = if row { css.column_gap.or_zero(cw) } else { css.row_gap.resolve(ch).unwrap_or(0.0) };
    let cross_gap = if row { css.row_gap.resolve(ch).unwrap_or(0.0) } else { css.column_gap.or_zero(cw) };

    // 1. Taille de base de chaque element sur l'axe principal.
    let mut items: Vec<FlexItem> = Vec::new();
    for (index, child) in children.iter().enumerate() {
        let Some(c) = css_of(child) else { continue };
        if out_of_flow(c) {
            continue;
        }
        let m = margins(c, cw);
        let f = frame(c, cw);
        let (frame_w, frame_h) = (f[1] + f[3], f[0] + f[2]);
        let align = c.align_self.unwrap_or(css.align_items);
        let (base, min_main, max_main, auto_margin) = if row {
            let basis = to_border_box(c, c.flex_basis, Some(cw), frame_w).or_else(|| to_border_box(c, c.width, Some(cw), frame_w));
            let base = basis.unwrap_or_else(|| measure(child, UNBOUNDED, ch, false).0);
            let min_auto = if c.min_width.is_auto() {
                if child_scrolls(child) { frame_w } else { min_content_width(child) + frame_w }
            } else {
                to_border_box(c, c.min_width, Some(cw), frame_w).unwrap_or(0.0)
            };
            let max = to_border_box(c, c.max_width, Some(cw), frame_w).unwrap_or(f32::MAX);
            (base, min_auto.min(max), max, (c.margin.left.is_auto(), c.margin.right.is_auto()))
        } else {
            // Colonne : la largeur (axe secondaire) d'abord, pour savoir
            // sur quelle largeur mesurer la hauteur du contenu.
            let width = column_item_width(child, c, cw, &m, align);
            let basis = to_border_box(c, c.flex_basis, ch, frame_h).or_else(|| to_border_box(c, c.height, ch, frame_h));
            let base = basis.unwrap_or_else(|| content_height(child, width - frame_w, None) + frame_h);
            let min = if c.min_height.is_auto() {
                if child_scrolls(child) { frame_h } else { content_height(child, width - frame_w, None) + frame_h }
            } else {
                to_border_box(c, c.min_height, ch, frame_h).unwrap_or(0.0)
            };
            let max = to_border_box(c, c.max_height, ch, frame_h).unwrap_or(f32::MAX);
            (base, min.min(max).min(base.max(min.min(max))), max, (c.margin.top.is_auto(), c.margin.bottom.is_auto()))
        };
        items.push(FlexItem {
            index,
            margin: m,
            auto_margin,
            base: base.clamp(min_main.min(max_main), max_main),
            min_main,
            max_main,
            grow: c.flex_grow,
            shrink: c.flex_shrink,
            main: 0.0,
            cross: 0.0,
            align,
        });
    }

    let outer_main = |it: &FlexItem, size: f32| size + if row { it.margin[1] + it.margin[3] } else { it.margin[0] + it.margin[2] };

    // 2. Decoupage en lignes (`flex-wrap`).
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut used = 0.0f32;
    for (k, it) in items.iter().enumerate() {
        let add = outer_main(it, it.base) + if current.is_empty() { 0.0 } else { main_gap };
        if css.flex_wrap && !current.is_empty() && main_size.is_some_and(|m| used + add > m) {
            lines.push(std::mem::take(&mut current));
            used = outer_main(it, it.base);
        } else {
            used += add;
        }
        current.push(k);
    }
    if !current.is_empty() {
        lines.push(current);
    }

    let mut boxes = vec![(0.0, 0.0, 0.0, 0.0); children.len()];
    let mut cross_cursor = 0.0f32;
    let single_line = lines.len() <= 1;
    let mut total_main = 0.0f32;

    // `align-content` (plusieurs lignes, taille secondaire connue) : il faut
    // d'abord la hauteur de chaque ligne, pour repartir l'espace restant.
    let container_cross = if row { ch } else { Some(cw) };
    let mut extra_per_line = 0.0f32;
    let mut between = 0.0f32;
    if !single_line
        && let Some(total) = container_cross {
            let mut sum = 0.0f32;
            for line in &lines {
                let gaps = main_gap * line.len().saturating_sub(1) as f32;
                let used: f32 = line.iter().map(|&k| outer_main(&items[k], items[k].base)).sum::<f32>() + gaps;
                for &k in line {
                    items[k].main = items[k].base;
                }
                if let Some(main) = main_size {
                    resolve_flexible_lengths(&mut items, line, main - used);
                }
                sum += line
                    .iter()
                    .map(|&k| {
                        let child = &children[items[k].index];
                        let c = css_of(child).unwrap();
                        let f = c.frame.get();
                        let cross = if row {
                            to_border_box(c, c.height, ch, f[0] + f[2]).unwrap_or_else(|| content_height(child, items[k].main - f[1] - f[3], None) + f[0] + f[2])
                        } else {
                            column_item_width(child, c, cw, &items[k].margin, items[k].align)
                        };
                        cross + cross_margins(&items[k], row)
                    })
                    .fold(0.0, f32::max);
            }
            let n = lines.len() as f32;
            let free = (total - sum - cross_gap * (n - 1.0)).max(0.0);
            let (start, gap_extra, stretch) = match css.align_content {
                AlignContent::Start => (0.0, 0.0, 0.0),
                AlignContent::End => (free, 0.0, 0.0),
                AlignContent::Center => (free / 2.0, 0.0, 0.0),
                AlignContent::SpaceBetween => (0.0, free / (n - 1.0), 0.0),
                AlignContent::SpaceAround => (free / n / 2.0, free / n, 0.0),
                AlignContent::SpaceEvenly => (free / (n + 1.0), free / (n + 1.0), 0.0),
                AlignContent::Stretch => (0.0, 0.0, free / n),
            };
            cross_cursor = start;
            between = gap_extra;
            extra_per_line = stretch;
        }

    for line in &lines {
        // 3. Grandir / retrecir pour remplir l'axe principal.
        let gaps = main_gap * line.len().saturating_sub(1) as f32;
        let sum: f32 = line.iter().map(|&k| outer_main(&items[k], items[k].base)).sum::<f32>() + gaps;
        for &k in line {
            items[k].main = items[k].base;
        }
        if let Some(main) = main_size {
            resolve_flexible_lengths(&mut items, line, main - sum);
        }

        // 4. Taille sur l'axe secondaire.
        for &k in line {
            let child = &children[items[k].index];
            let c = css_of(child).unwrap();
            let f = c.frame.get();
            items[k].cross = if row {
                let h = to_border_box(c, c.height, ch, f[0] + f[2]);
                h.unwrap_or_else(|| content_height(child, items[k].main - f[1] - f[3], None) + f[0] + f[2])
            } else {
                column_item_width(child, c, cw, &items[k].margin, items[k].align)
            };
        }
        let line_cross = if single_line {
            let container_cross = if row { ch } else { Some(cw) };
            container_cross.unwrap_or_else(|| line.iter().map(|&k| items[k].cross + cross_margins(&items[k], row)).fold(0.0, f32::max))
        } else {
            line.iter().map(|&k| items[k].cross + cross_margins(&items[k], row)).fold(0.0, f32::max) + extra_per_line
        };

        // 5. Positions sur l'axe principal : marges auto, puis justify-content.
        let used: f32 = line.iter().map(|&k| outer_main(&items[k], items[k].main)).sum::<f32>() + gaps;
        let free = main_size.map(|m| (m - used).max(0.0)).unwrap_or(0.0);
        let auto_count = line.iter().map(|&k| items[k].auto_margin.0 as u32 + items[k].auto_margin.1 as u32).sum::<u32>();
        let per_auto = if auto_count > 0 { free / auto_count as f32 } else { 0.0 };
        let count = line.len() as f32;
        let (mut cursor, spacing) = if auto_count > 0 {
            (0.0, 0.0)
        } else {
            match css.justify_content {
                JustifyContent::Start => (0.0, 0.0),
                JustifyContent::End => (free, 0.0),
                JustifyContent::Center => (free / 2.0, 0.0),
                JustifyContent::SpaceBetween if count > 1.0 => (0.0, free / (count - 1.0)),
                JustifyContent::SpaceBetween => (0.0, 0.0),
                JustifyContent::SpaceAround => (free / count / 2.0, free / count),
                JustifyContent::SpaceEvenly => (free / (count + 1.0), free / (count + 1.0)),
            }
        };

        for &k in line {
            let it = &items[k];
            let (m_start, m_end) = if row { (it.margin[3], it.margin[1]) } else { (it.margin[0], it.margin[2]) };
            cursor += m_start + if it.auto_margin.0 { per_auto } else { 0.0 };
            let main_pos = cursor;
            cursor += it.main + m_end + if it.auto_margin.1 { per_auto } else { 0.0 } + main_gap + spacing;

            let (cm_start, cm_end) = if row { (it.margin[0], it.margin[2]) } else { (it.margin[3], it.margin[1]) };
            let child = &children[it.index];
            let c = css_of(child).unwrap();
            let cross_specified = if row { !c.height.is_auto() } else { !c.width.is_auto() };
            let room = (line_cross - cm_start - cm_end).max(0.0);
            let cross = if it.align == AlignItems::Stretch && !cross_specified { room } else { it.cross.min(room.max(it.cross)) };
            let cross_pos = cross_cursor
                + cm_start
                + match it.align {
                    AlignItems::Center => (room - cross) / 2.0,
                    AlignItems::End => room - cross,
                    _ => 0.0,
                };
            boxes[it.index] = if row { (main_pos, cross_pos, it.main, cross) } else { (cross_pos, main_pos, cross, it.main) };
        }
        total_main = total_main.max(cursor - main_gap - spacing);
        cross_cursor += line_cross + cross_gap + between;
    }
    let cross_total = (cross_cursor - cross_gap - between).max(0.0);

    // En colonne, des enfants qui ne retrecissent pas (`flex-shrink: 0`)
    // peuvent depasser une hauteur fixee : l'etendue reelle est rendue pour
    // qu'un conteneur `overflow-y: auto` puisse defiler jusqu'en bas.
    let height = if row { cross_total } else { ch.map_or(total_main, |c| c.max(total_main)).max(0.0) };
    Placement { boxes, height }
}

fn cross_margins(it: &FlexItem, row: bool) -> f32 {
    if row { it.margin[0] + it.margin[2] } else { it.margin[1] + it.margin[3] }
}

// Un conteneur qui defile n'impose pas la taille de son contenu comme
// minimum (comme `overflow: auto` en CSS).
fn child_scrolls(node: &UiNode) -> bool {
    node.layout().scrollable()
}

// Largeur d'un element d'une colonne flex : toute la largeur si `stretch`
// et `width: auto`, sinon celle de son contenu.
fn column_item_width(child: &UiNode, c: &CssBox, cw: f32, m: &[f32; 4], align: AlignItems) -> f32 {
    let available = (cw - m[1] - m[3]).max(0.0);
    if c.width.is_auto() && align == AlignItems::Stretch {
        let f = c.frame.get();
        clamp_size(c, available, c.min_width, c.max_width, Some(cw), f[1] + f[3])
    } else {
        measure(child, cw, None, false).0
    }
}

// Repartit `free` (positif : espace libre, negatif : depassement) entre les
// elements d'une ligne selon `flex-grow` / `flex-shrink`, en respectant
// leurs tailles min/max (les elements bornes sont figes, et on recommence).
fn resolve_flexible_lengths(items: &mut [FlexItem], line: &[usize], free: f32) {
    let mut frozen = vec![false; items.len()];
    let mut remaining = free;
    for _ in 0..line.len() + 1 {
        if remaining.abs() < 0.01 {
            break;
        }
        let active: Vec<usize> = line.iter().copied().filter(|&k| !frozen[k]).collect();
        let weights: Vec<f32> = active
            .iter()
            .map(|&k| if remaining > 0.0 { items[k].grow } else { items[k].shrink * items[k].base })
            .collect();
        let total: f32 = weights.iter().sum();
        if total <= 0.0 {
            break;
        }
        let mut consumed = 0.0;
        for (&k, w) in active.iter().zip(&weights) {
            let target = items[k].main + remaining * w / total;
            let clamped = target.clamp(items[k].min_main, items[k].max_main.max(items[k].min_main));
            if (clamped - target).abs() > 0.01 {
                frozen[k] = true;
            }
            consumed += clamped - items[k].main;
            items[k].main = clamped;
        }
        remaining -= consumed;
        if !active.iter().any(|&k| frozen[k]) {
            break;
        }
    }
}

// Grid : pistes en px / % / fr / auto, placement automatique ligne par
// ligne (ou a la colonne demandee), hauteur des lignes selon leur contenu.
fn place_grid(css: &CssBox, children: &[UiNode], cw: f32, ch: Option<f32>) -> Placement {
    let col_gap = css.column_gap.or_zero(cw);
    let row_gap = css.row_gap.resolve(ch).unwrap_or(0.0);
    let template = if css.grid_template_columns.is_empty() { vec![CssTrack::Fr(1.0)] } else { css.grid_template_columns.clone() };
    let n_cols = template.len();

    // Placement de chaque element (colonne, ligne, etendue en colonnes).
    let mut cells: Vec<(usize, usize, usize, usize)> = Vec::new(); // (index, col, row, span)
    let (mut col, mut row) = (0usize, 0usize);
    for (index, child) in children.iter().enumerate() {
        let Some(c) = css_of(child) else { continue };
        if out_of_flow(c) {
            continue;
        }
        let span = c.grid_column_span.clamp(1, n_cols);
        if let Some(start) = c.grid_column {
            let start = (start.max(1) - 1).min(n_cols - 1);
            if start < col {
                row += 1;
            }
            col = start;
        }
        if col + span > n_cols {
            col = 0;
            row += 1;
        }
        if let Some(r) = c.grid_row {
            row = r.max(1) - 1;
        }
        cells.push((index, col, row, span));
        col += span;
        if col >= n_cols {
            col = 0;
            row += 1;
        }
    }

    // Largeur des colonnes.
    let fixed = |t: &CssTrack| match t {
        CssTrack::Px(px) => Some(*px),
        CssTrack::Percent(p) => Some(cw * p / 100.0),
        _ => None,
    };
    let mut widths: Vec<f32> = template.iter().map(|t| fixed(t).unwrap_or(0.0)).collect();
    for (k, t) in template.iter().enumerate() {
        if *t == CssTrack::Auto {
            widths[k] = cells
                .iter()
                .filter(|(_, c, _, span)| *c == k && *span == 1)
                .map(|(i, ..)| outer_intrinsic_width(&children[*i], MAX_CONTENT))
                .fold(0.0, f32::max);
        }
    }
    let used: f32 = widths.iter().sum::<f32>() + col_gap * (n_cols - 1) as f32;
    let total_fr: f32 = template.iter().map(|t| if let CssTrack::Fr(f) = t { *f } else { 0.0 }).sum();
    if total_fr > 0.0 {
        let per_fr = (cw - used).max(0.0) / total_fr;
        for (k, t) in template.iter().enumerate() {
            if let CssTrack::Fr(f) = t {
                widths[k] = per_fr * f;
            }
        }
    }
    let col_x: Vec<f32> = widths.iter().scan(0.0, |x, w| { let start = *x; *x += w + col_gap; Some(start) }).collect();
    let span_width = |c: usize, span: usize| widths[c..c + span].iter().sum::<f32>() + col_gap * (span - 1) as f32;

    // Hauteur des lignes : gabarit, sinon le plus haut de leurs elements.
    let n_rows = cells.iter().map(|(_, _, r, _)| r + 1).max().unwrap_or(0).max(css.grid_template_rows.len());
    let mut heights = vec![0.0f32; n_rows];
    for (r, h) in heights.iter_mut().enumerate() {
        match css.grid_template_rows.get(r) {
            Some(CssTrack::Px(px)) => *h = *px,
            Some(CssTrack::Percent(p)) if ch.is_some() => *h = ch.unwrap() * p / 100.0,
            _ => {
                *h = cells
                    .iter()
                    .filter(|(_, _, row, _)| *row == r)
                    .map(|&(i, c, _, span)| {
                        let m = css_of(&children[i]).map(|cc| margins(cc, cw)).unwrap_or([0.0; 4]);
                        measure(&children[i], span_width(c, span), None, true).1 + m[0] + m[2]
                    })
                    .fold(0.0, f32::max);
            }
        }
    }
    let rows_total: f32 = heights.iter().sum::<f32>() + row_gap * n_rows.saturating_sub(1) as f32;
    let row_fr: f32 = css.grid_template_rows.iter().map(|t| if let CssTrack::Fr(f) = t { *f } else { 0.0 }).sum();
    if row_fr > 0.0
        && let Some(ch) = ch {
            let per_fr = (ch - rows_total).max(0.0) / row_fr;
            for (r, t) in css.grid_template_rows.iter().enumerate() {
                if let CssTrack::Fr(f) = t {
                    heights[r] += per_fr * f;
                }
            }
        }
    let row_y: Vec<f32> = heights.iter().scan(0.0, |y, h| { let start = *y; *y += h + row_gap; Some(start) }).collect();

    let mut boxes = vec![(0.0, 0.0, 0.0, 0.0); children.len()];
    for &(i, c, r, span) in &cells {
        let m = css_of(&children[i]).map(|cc| margins(cc, cw)).unwrap_or([0.0; 4]);
        let w = (span_width(c, span) - m[1] - m[3]).max(0.0);
        let h = (heights[r] - m[0] - m[2]).max(0.0);
        // Remplit la cellule (etirement par defaut de grid) ; frame
        // memorise pour la largeur reelle de la cellule.
        let _ = measure(&children[i], span_width(c, span), None, true);
        boxes[i] = (col_x[c] + m[3], row_y[r] + m[0], w, h);
    }
    let height = heights.iter().sum::<f32>() + row_gap * n_rows.saturating_sub(1) as f32;
    Placement { boxes, height }
}

// Arrondit une boite flottante (relative a `origin`) en pixels entiers, sans
// trou entre deux boites voisines : chaque bord est arrondi separement.
fn to_pixels(origin: (i32, i32), b: (f32, f32, f32, f32)) -> Rect {
    let x0 = (origin.0 as f32 + b.0).round() as i32;
    let y0 = (origin.1 as f32 + b.1).round() as i32;
    let x1 = (origin.0 as f32 + b.0 + b.2).round() as i32;
    let y1 = (origin.1 as f32 + b.1 + b.3).round() as i32;
    (x0, y0, (x1 - x0).max(0) as u32, (y1 - y0).max(0) as u32)
}

/// Mise en page des enfants d'un conteneur web deja place en `own` (sa
/// boite bordure comprise) - voir `layout_manager::layout_container`.
pub fn layout_web_container(css: &CssBox, children: &[UiNode], scrollable: (bool, bool), scroll: (u32, u32), own: Rect) -> ContainerLayout {
    let f = css.frame.get();
    let content = (
        own.0 + f[3].round() as i32,
        own.1 + f[0].round() as i32,
        (own.2 as f32 - f[1] - f[3]).max(0.0),
        (own.3 as f32 - f[0] - f[2]).max(0.0),
    );
    // La hauteur du contenu est connue si elle est fixee, ou pour un
    // conteneur flex (etire ou dimensionne par son parent) : en colonne ses
    // enfants peuvent grandir pour la remplir ; en ligne, elle est la
    // hauteur de sa ligne unique, a laquelle ses enfants s'etirent (sinon un
    // enfant `overflow-y: auto` prendrait la hauteur de son contenu et ne
    // defilerait jamais).
    let definite = !css.height.is_auto() || css.display == CssDisplay::Flex || css.fixed;
    let ch = definite.then_some(content.3);
    let placement = place_children(css, children, content.2, ch);

    let visible_w = content.2.round() as u32;
    let visible_h = content.3.round() as u32;
    let content_height = placement.height.ceil().max(0.0) as u32;
    // Largeur reelle du contenu : ce qui depasse a droite (enfants trop
    // larges, ligne flex qui ne revient pas a la ligne).
    let content_width = children
        .iter()
        .zip(&placement.boxes)
        .filter(|(c, _)| css_of(c).is_some_and(|c| !out_of_flow(c)))
        .map(|(_, b)| b.0 + b.2)
        .fold(0.0f32, f32::max)
        .ceil()
        .max(0.0) as u32;
    let max_scroll = if scrollable.1 { content_height.saturating_sub(visible_h) } else { 0 };
    let max_scroll_x = if scrollable.0 { content_width.saturating_sub(visible_w) } else { 0 };
    let (sx, sy) = (scroll.0.min(max_scroll_x) as i32, scroll.1.min(max_scroll) as i32);
    // Boite de reference d'un enfant `absolute` : le conteneur sans sa
    // bordure (padding compris), comme en CSS.
    let b = &css.border;
    let padding_box = (
        own.0 + b.left.round() as i32,
        own.1 + b.top.round() as i32,
        (own.2 as f32 - b.left - b.right).max(0.0).round() as u32,
        (own.3 as f32 - b.top - b.bottom).max(0.0).round() as u32,
    );
    ContainerLayout {
        visible: (content.0, content.1, visible_w, visible_h),
        clip: own,
        children: placement
            .boxes
            .into_iter()
            .zip(children)
            .map(|(b, child)| {
                if is_absolute(child) {
                    let r = inset_box(child, padding_box);
                    (r.0 - sx, r.1 - sy, r.2, r.3)
                } else {
                    to_pixels((content.0 - sx, content.1 - sy), b)
                }
            })
            .collect(),
        content_height: content_height.max(visible_h),
        max_scroll,
        content_width: content_width.max(visible_w),
        max_scroll_x,
    }
}

/// Boites des nœuds racine dans la fenetre `viewport` : les nœuds web
/// s'empilent en flux normal (un `height: 100%` se rapporte a la fenetre),
/// les autres gardent leur placement en pourcentages.
pub fn layout_roots(nodes: &[UiNode], viewport: (u32, u32, u32, u32)) -> Vec<Rect> {
    if !nodes.iter().any(is_web) {
        return nodes.iter().map(|n| crate::layout::managers::layout_manager::to_rect(resolve(n.layout(), viewport))).collect();
    }
    let placement = place_block(nodes, viewport.2 as f32, Some(viewport.3 as f32));
    placement
        .boxes
        .into_iter()
        .zip(nodes)
        .map(|(b, node)| {
            if is_fixed(node) {
                fixed_box(node, viewport)
            } else if is_web(node) {
                to_pixels((viewport.0 as i32, viewport.1 as i32), b)
            } else {
                crate::layout::managers::layout_manager::to_rect(resolve(node.layout(), viewport))
            }
        })
        .collect()
}
