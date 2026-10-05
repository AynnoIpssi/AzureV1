// Les composants rsH : toute balise qui n'est pas un element de base.
//
// - Les **champs** : `<input>`, `<password>`, `<number>`, `<checkbox>`,
//   `<radio>`, `<switch>`, `<slider>`, `<progress>`, `<select>`,
//   `<segmented>`, `<rating>` (voir `ui::models::control`).
// - `<tooltip texte="...">` : infobulle sur ce qu'elle entoure.
// - `<draggable id="...">` / `<dropzone id="...">` : glisser-deposer.
// - `<richtext#id valeur="...">` : zone de texte riche (WYSIWYG), avec sa
//   barre d'outils `<richbar pour="id">` (composant rsH).
// - `<include src="parts/menu.rsh"/>` : insere un autre fichier rsH.
// - Les **composants ecrits en rsH** : ceux d'Azure (`builtin/`, carte,
//   badge, alerte, onglets, tableau...) et ceux de l'app (dossier
//   `components/` a cote de la page : `components/fiche.rsh` = `<fiche>`).
//   Un composant recoit ses attributs comme variables (`{{titre}}`, et
//   `{{titre_list}}` decoupe aux virgules), `{{class}}`, `{{id}}`,
//   `{{contenu}}` (le texte passe entre ses balises), et affiche ce qu'on
//   lui passe a la place de `<slot/>`.
use crate::compiler::rsc::services::link::ElementInfo;
use crate::compiler::rsh::mangers::parser::{parse, AstNode};
use crate::compiler::rsh::services::lexer::tokenize;
use crate::compiler::services::codegen::{base_textarea_style, decoration_for, resolve, resolve_pseudo_background, StyleSource};
use crate::style::models::style::Style;
use crate::compiler::services::condition::{interpolate, ConditionValue, Context};
use crate::compiler::services::interpreter::{build_flat, build_label_text, color_of, extract_text, layout_for};
use crate::ui::models::control::{Control, ControlKind, DEFAULT_ACCENT, DEFAULT_TEXT, DEFAULT_TRACK};
use crate::ui::models::textarea::TextArea;
use crate::ui::models::ui_node::UiNode;
use azure_engine::rendering::models::color::Color;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Profondeur maximale de composants imbriques.
const MAX_DEPTH: u32 = 24;

/// Les composants d'Azure : (nom, source rsH).
pub const BUILTIN: &[(&str, &str)] = &[
    ("row", include_str!("builtin/row.rsh")),
    ("column", include_str!("builtin/column.rsh")),
    ("stack", include_str!("builtin/stack.rsh")),
    ("grid", include_str!("builtin/grid.rsh")),
    ("center", include_str!("builtin/center.rsh")),
    ("spacer", include_str!("builtin/spacer.rsh")),
    ("divider", include_str!("builtin/divider.rsh")),
    ("section", include_str!("builtin/section.rsh")),
    ("page", include_str!("builtin/page.rsh")),
    ("navbar", include_str!("builtin/navbar.rsh")),
    ("footer", include_str!("builtin/footer.rsh")),
    ("card", include_str!("builtin/card.rsh")),
    ("badge", include_str!("builtin/badge.rsh")),
    ("tag", include_str!("builtin/tag.rsh")),
    ("chip", include_str!("builtin/chip.rsh")),
    ("avatar", include_str!("builtin/avatar.rsh")),
    ("alert", include_str!("builtin/alert.rsh")),
    ("kbd", include_str!("builtin/kbd.rsh")),
    ("code", include_str!("builtin/code.rsh")),
    ("quote", include_str!("builtin/quote.rsh")),
    ("stat", include_str!("builtin/stat.rsh")),
    ("empty", include_str!("builtin/empty.rsh")),
    ("skeleton", include_str!("builtin/skeleton.rsh")),
    ("btn", include_str!("builtin/btn.rsh")),
    ("link", include_str!("builtin/link.rsh")),
    ("ancre", include_str!("builtin/ancre.rsh")),
    ("list", include_str!("builtin/list.rsh")),
    ("list-item", include_str!("builtin/list-item.rsh")),
    ("menu", include_str!("builtin/menu.rsh")),
    ("menu-item", include_str!("builtin/menu-item.rsh")),
    ("tabs", include_str!("builtin/tabs.rsh")),
    ("breadcrumb", include_str!("builtin/breadcrumb.rsh")),
    ("steps", include_str!("builtin/steps.rsh")),
    ("pagination", include_str!("builtin/pagination.rsh")),
    ("timeline", include_str!("builtin/timeline.rsh")),
    ("timeline-item", include_str!("builtin/timeline-item.rsh")),
    ("accordion", include_str!("builtin/accordion.rsh")),
    ("table", include_str!("builtin/table.rsh")),
    ("tr", include_str!("builtin/tr.rsh")),
    ("th", include_str!("builtin/th.rsh")),
    ("td", include_str!("builtin/td.rsh")),
    ("form", include_str!("builtin/form.rsh")),
    ("field", include_str!("builtin/field.rsh")),
    ("info", include_str!("builtin/info.rsh")),
    ("meter", include_str!("builtin/meter.rsh")),
    ("toast", include_str!("builtin/toast.rsh")),
    ("hero", include_str!("builtin/hero.rsh")),
    ("feature", include_str!("builtin/feature.rsh")),
    ("price", include_str!("builtin/price.rsh")),
    ("modal", include_str!("builtin/modal.rsh")),
    ("drawer", include_str!("builtin/drawer.rsh")),
    ("toasts", include_str!("builtin/toasts.rsh")),
    ("banner", include_str!("builtin/banner.rsh")),
    ("richbar", include_str!("builtin/richbar.rsh")),
];

/// Les regles rsC des composants d'Azure, placees AVANT celles de l'app :
/// une regle de l'app sur le meme selecteur l'emporte toujours.
pub const DEFAULT_STYLES: &str = include_str!("builtin/components.rsc");

/// Ajoute les styles des composants (Azure, puis ceux de l'app) devant la
/// feuille rsC `rsc` de la page.
pub fn with_default_styles(rsc: &str, library: Option<&Library>) -> String {
    let user = library.map(Library::styles).unwrap_or_default();
    format!("{DEFAULT_STYLES}\n{user}\n{rsc}")
}

/// Le contenu passe a un composant (voir `<slot/>`), avec le contexte de la
/// page qui l'a ecrit.
#[derive(Debug)]
pub struct Slot {
    pub nodes: Vec<AstNode>,
    pub ctx: Context,
}

/// Ou trouver les composants et les fichiers inclus.
#[derive(Debug, Default)]
pub struct Library {
    /// Dossiers de composants de l'app, par priorite.
    dirs: Vec<PathBuf>,
    /// Base des `<include src="...">` relatifs.
    base: Option<PathBuf>,
    /// Fichiers deja lus, avec leur date de modification (relus s'ils
    /// changent : une page gardee en cache suit quand meme ses `<include>`).
    cache: Mutex<HashMap<PathBuf, (Option<std::time::SystemTime>, Arc<Vec<AstNode>>)>>,
}

fn builtin_templates() -> &'static HashMap<&'static str, Arc<Vec<AstNode>>> {
    static TEMPLATES: OnceLock<HashMap<&'static str, Arc<Vec<AstNode>>>> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        BUILTIN
            .iter()
            .map(|(name, source)| (*name, Arc::new(parse(tokenize(source)).unwrap_or_else(|e| panic!("composant integre <{name}> invalide : {e}")))))
            .collect()
    })
}

fn default_library() -> Arc<Library> {
    static DEFAULT: OnceLock<Arc<Library>> = OnceLock::new();
    DEFAULT.get_or_init(|| Arc::new(Library::default())).clone()
}

impl Library {
    pub fn new() -> Library {
        Library::default()
    }

    /// Les composants d'une page `page.rsh` : dossier `components/` a cote
    /// d'elle ; ses `<include>` sont relatifs a son dossier.
    pub fn for_page(page: &Path) -> Library {
        let dir = page.parent().map(Path::to_path_buf).unwrap_or_default();
        Library::new().with_dir(dir.join("components")).with_base(dir)
    }

    pub fn with_dir(mut self, dir: impl Into<PathBuf>) -> Library {
        self.dirs.push(dir.into());
        self
    }

    pub fn with_base(mut self, dir: impl Into<PathBuf>) -> Library {
        self.base = Some(dir.into());
        self
    }

    fn load(&self, path: &Path) -> Result<Arc<Vec<AstNode>>, String> {
        let modifie = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        if let Some((quand, nodes)) = self.cache.lock().ok().and_then(|c| c.get(path).cloned())
            && quand == modifie
        {
            return Ok(nodes);
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{} : {e}", path.display()))?;
        let nodes = Arc::new(parse(tokenize(&text)).map_err(|e| format!("{} : {e}", path.display()))?);
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(path.to_path_buf(), (modifie, nodes.clone()));
        }
        Ok(nodes)
    }

    /// Le modele du composant `name` : celui de l'app d'abord, sinon celui
    /// d'Azure.
    pub fn template(&self, name: &str) -> Option<Result<Arc<Vec<AstNode>>, String>> {
        for dir in &self.dirs {
            let path = dir.join(format!("{name}.rsh"));
            if path.is_file() {
                return Some(self.load(&path));
            }
        }
        builtin_templates().get(name).cloned().map(Ok)
    }

    pub fn include(&self, src: &str) -> Result<Arc<Vec<AstNode>>, String> {
        let path = match &self.base {
            Some(base) if Path::new(src).is_relative() => base.join(src),
            _ => PathBuf::from(src),
        };
        self.load(&path)
    }

    /// Les `.rsc` des dossiers de composants de l'app (un par composant).
    pub fn styles(&self) -> String {
        let mut out = String::new();
        for dir in &self.dirs {
            let mut files: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "rsc")).collect();
            files.sort();
            for file in files {
                if let Ok(text) = std::fs::read_to_string(&file) {
                    out.push_str(&text);
                    out.push('\n');
                }
            }
        }
        out
    }

    /// Les noms de tous les composants disponibles (Azure + app).
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = builtin_templates().keys().map(|n| n.to_string()).collect();
        for dir in &self.dirs {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|x| x == "rsh") {
                    names.extend(path.file_stem().map(|s| s.to_string_lossy().into_owned()));
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }
}

fn attr(attrs: &[(String, String)], name: &str, ctx: &Context) -> Option<String> {
    attrs.iter().find(|(k, _)| k == name).map(|(_, v)| interpolate(v, ctx))
}

fn truthy(value: Option<String>) -> bool {
    value.is_some_and(|v| matches!(v.trim(), "" | "true" | "oui" | "1" | "checked" | "on" | "yes"))
}

fn number(value: Option<String>) -> Option<f64> {
    value.and_then(|v| v.trim().replace(',', ".").parse().ok())
}

/// Couleur `accent-color` rsC de l'element. Heritee comme en CSS : sans
/// valeur propre, celle de l'ancetre le plus proche qui en a une
/// (`.formulaire { accent-color: ... }` colore tous ses champs).
fn accent(tag: &str, class: &str, id: &str, source: &StyleSource, ancestors: &[ElementInfo], preceding: &[ElementInfo]) -> Option<Color> {
    use crate::compiler::rsc::services::link::resolve_element;
    let StyleSource::Rsc(sheet) = source else { return None };
    let color = |ancestors: &[ElementInfo], preceding: &[ElementInfo], element: &ElementInfo| {
        resolve_element(sheet, ancestors, preceding, element, Default::default()).accent_color.as_ref().and_then(crate::compiler::rsc::models::value::Value::as_color)
    };
    color(ancestors, preceding, &ElementInfo::new(tag, class, id)).or_else(|| (0..ancestors.len()).rev().find_map(|i| color(&ancestors[..i], &[], &ancestors[i])))
}

/// Construit l'element `<tag ...>` (appele par l'interpreteur pour toute
/// balise qui n'est pas un element de base).
#[allow(clippy::too_many_arguments)]
pub fn build_element(tag: &str, class: &str, id: &str, attrs: &[(String, String)], children: &[AstNode], source: &StyleSource, ancestors: &[ElementInfo], ctx: &Context, preceding: &mut Vec<ElementInfo>, out: &mut Vec<UiNode>) {
    let library = ctx.library.clone().unwrap_or_else(default_library);
    match tag {
        // Le contenu passe au composant en cours.
        "slot" => {
            if let Some(slot) = &ctx.slot {
                build_flat(&slot.nodes, source, ancestors, &slot.ctx, preceding, out);
            }
        }
        "include" => match attr(attrs, "src", ctx).ok_or_else(|| "<include> : src manquant".to_string()).and_then(|src| library.include(&src)) {
            Ok(nodes) => build_flat(&nodes, source, ancestors, ctx, preceding, out),
            Err(e) => error_label(&e, source, ancestors, preceding, out),
        },
        // Infobulle sur ce qu'elle entoure : `<tooltip texte="...">...<!tooltip>`.
        "tooltip" => {
            let text = attr(attrs, "texte", ctx).or_else(|| attr(attrs, "text", ctx)).unwrap_or_default();
            let mut inner = Vec::new();
            build_flat(children, source, ancestors, ctx, preceding, &mut inner);
            for mut node in inner {
                node.decoration_mut().tooltip = text.clone();
                out.push(node);
            }
        }
        // Glisser-deposer : `<draggable id="carte-3">...<!draggable>` se
        // saisit a la souris, `<dropzone id="colonne-fait">...<!dropzone>`
        // recoit (voir `ui::services::interact::drag`, `AzureWindow::on_drop`).
        "draggable" | "dropzone" => {
            let key = attr(attrs, "id", ctx).unwrap_or_else(|| interpolate(id, ctx));
            let mut inner = Vec::new();
            build_flat(children, source, ancestors, ctx, preceding, &mut inner);
            for mut node in inner {
                let decoration = node.decoration_mut();
                if tag == "draggable" { decoration.drag = key.clone() } else { decoration.drop = key.clone() }
                out.push(node);
            }
        }
        "richtext" => {
            let style = resolve("richtext", class, id, base_textarea_style(), source, ancestors, preceding);
            let value = attr(attrs, "valeur", ctx).or_else(|| attr(attrs, "value", ctx)).unwrap_or_else(|| interpolate(extract_text(children).trim(), ctx));
            let mut area = TextArea::rich(layout_for(&style, source), color_of(style.background), color_of(style.color), &value);
            area.id = interpolate(id, ctx);
            area.placeholder = attr(attrs, "placeholder", ctx).unwrap_or_default();
            area.font_size = style.font_size.unwrap_or(area.font_size);
            area.font_weight = style.font_weight.unwrap_or(area.font_weight);
            // `commandes="true"` : l'app fait son menu ; `commandes="code|Nom|..."` :
            // le menu au clavier de la fondation (voir `interact::CommandMenu`).
            let commandes = attr(attrs, "commandes", ctx);
            area.command_list = commandes.clone().filter(|c| c.contains('|')).unwrap_or_default();
            area.commands = !area.command_list.is_empty() || truthy(commandes);
            area.enter_submits = truthy(attr(attrs, "entree", ctx));
            // `focus="true"` : pret a taper des la construction (curseur a la
            // fin) ; `focus="12"` : curseur au caractere 12.
            let focus = attrs.iter().any(|(k, _)| k == "focus").then(|| attr(attrs, "focus", ctx)).flatten();
            if let Some(n) = focus.as_deref().and_then(|f| f.trim().parse::<usize>().ok()) {
                area.focused = true;
                area.cursor = n.min(area.char_count());
            } else {
                area.focused = focus.is_some() && truthy(focus);
            }
            area.focus_background = resolve_pseudo_background("richtext", class, id, source, ancestors, preceding, crate::compiler::rsc::models::selector::PseudoState { hover: false, focus: true, active: false }).filter(|c| Some(*c) != style.background);
            area.decoration = decoration_for("richtext", class, id, &style, source, ancestors, preceding);
            out.push(UiNode::TextArea(area));
            preceding.push(ElementInfo::new("richtext", class, id));
        }
        "input" | "password" | "number" | "email" | "search" => build_input(tag, class, id, attrs, children, source, ancestors, ctx, preceding, out),
        // Une option hors d'une liste : rien.
        "option" => {}
        _ => {
            if let Some(kind) = ControlKind::from_tag(tag) {
                return build_control(kind, tag, class, id, attrs, children, source, ancestors, ctx, preceding, out);
            }
            match library.template(tag) {
                Some(Ok(template)) => expand(&template, class, id, attrs, children, source, ancestors, ctx, preceding, out),
                Some(Err(e)) => error_label(&e, source, ancestors, preceding, out),
                None => error_label(&format!("<{tag}> : composant inconnu"), source, ancestors, preceding, out),
            }
        }
    }
}

/// Une erreur visible dans la page (plutot qu'un element qui disparait).
fn error_label(message: &str, source: &StyleSource, ancestors: &[ElementInfo], preceding: &mut Vec<ElementInfo>, out: &mut Vec<UiNode>) {
    eprintln!("rsH : {message}");
    let node = build_label_text("text", "az-error", "", message, 13.0, 500.0, source, ancestors, preceding);
    out.push(node);
}

#[allow(clippy::too_many_arguments)]
fn expand(template: &[AstNode], class: &str, id: &str, attrs: &[(String, String)], children: &[AstNode], source: &StyleSource, ancestors: &[ElementInfo], ctx: &Context, preceding: &mut Vec<ElementInfo>, out: &mut Vec<UiNode>) {
    if ctx.depth >= MAX_DEPTH {
        return error_label("composants imbriques trop profondement (un composant s'utilise lui-meme ?)", source, ancestors, preceding, out);
    }
    let mut inner = ctx.clone();
    inner.depth += 1;
    for (name, _) in attrs {
        let value = attr(attrs, name, ctx).unwrap_or_default();
        let list: Vec<ConditionValue> = value.split(',').map(str::trim).filter(|s| !s.is_empty()).map(ConditionValue::from).collect();
        inner = inner.with_value(&format!("{name}_list"), ConditionValue::List(list)).with_text(name, &value);
        // `pages="5"` : `{{pages_range}}` = 1, 2, ... 5 (pour `<for>`).
        if let Ok(n) = value.trim().parse::<usize>()
            && (1..=500).contains(&n) {
                inner = inner.with_value(&format!("{name}_range"), ConditionValue::List((1..=n).map(|i| ConditionValue::Number(i as f64)).collect()));
            }
    }
    if let Some(nom) = attr(attrs, "nom", ctx) {
        let initials: String = nom.split_whitespace().filter_map(|w| w.chars().next()).take(2).flat_map(char::to_uppercase).collect();
        inner = inner.with_text("initiales", &initials);
    }
    let text = interpolate(extract_text(children).trim(), ctx);
    // `<chip id="x">` vaut `<chip#x>`.
    let id = if id.is_empty() { attr(attrs, "id", ctx).unwrap_or_default() } else { id.to_string() };
    inner = inner.with_text("class", class).with_text("id", &id).with_text("contenu", &text).with_value("a_contenu", children.iter().any(|c| !matches!(c, AstNode::RawText(t) if t.trim().is_empty())));
    inner.slot = Some(Arc::new(Slot { nodes: children.to_vec(), ctx: ctx.clone() }));
    build_flat(template, source, ancestors, &inner, preceding, out);
}

fn base_control_style() -> Style {
    Style { background: Some(DEFAULT_TRACK), color: Some(DEFAULT_TEXT), ..crate::compiler::services::codegen::base_layout() }
}

#[allow(clippy::too_many_arguments)]
fn build_control(kind: ControlKind, tag: &str, class: &str, id: &str, attrs: &[(String, String)], children: &[AstNode], source: &StyleSource, ancestors: &[ElementInfo], ctx: &Context, preceding: &mut Vec<ElementInfo>, out: &mut Vec<UiNode>) {
    let style = resolve(tag, class, id, base_control_style(), source, ancestors, preceding);
    let mut control = Control::new(kind, layout_for(&style, source));
    control.decoration = decoration_for(tag, class, id, &style, source, ancestors, preceding);
    // `background-color` d'un champ = sa piste / sa case, pas le fond de sa
    // boite.
    control.decoration.fill = None;
    control.decoration.hover_fill = None;
    control.id = id.to_string();
    control.track = style.background.unwrap_or(DEFAULT_TRACK);
    control.text_color = style.color.unwrap_or(DEFAULT_TEXT);
    control.font_size = style.font_size.unwrap_or(14.0);
    control.accent = accent(tag, class, id, source, ancestors, preceding).unwrap_or(DEFAULT_ACCENT);
    control.disabled = truthy(attr(attrs, "disabled", ctx));

    // Options : enfants `<option value="fr">France<!option>`, ou
    // `options="France, Belgique"`.
    let mut options: Vec<(String, String)> = children
        .iter()
        .filter_map(|c| match c {
            AstNode::Element { tag, attrs: oattrs, children: ochildren, .. } if tag == "option" => {
                let label = interpolate(extract_text(ochildren).trim(), ctx);
                let value = attr(oattrs, "value", ctx).unwrap_or_else(|| label.clone());
                Some((value, label))
            }
            _ => None,
        })
        .collect();
    if options.is_empty()
        && let Some(list) = attr(attrs, "options", ctx) {
            options = list.split(',').map(str::trim).filter(|s| !s.is_empty()).map(|s| (s.to_string(), s.to_string())).collect();
        }
    let label = attr(attrs, "label", ctx).unwrap_or_else(|| {
        let text: String = children.iter().filter(|c| matches!(c, AstNode::RawText(_))).map(|c| extract_text(std::slice::from_ref(c))).collect();
        interpolate(text.trim(), ctx)
    });
    control.label = label.clone();
    control.checked = attrs.iter().any(|(k, _)| k == "checked") && truthy(attr(attrs, "checked", ctx));

    match kind {
        ControlKind::Slider | ControlKind::Progress | ControlKind::Rating => {
            control.min = number(attr(attrs, "min", ctx)).unwrap_or(0.0);
            control.max = number(attr(attrs, "max", ctx)).unwrap_or(if kind == ControlKind::Rating { 5.0 } else { 100.0 });
            control.step = number(attr(attrs, "step", ctx)).unwrap_or(1.0);
            let value = number(attr(attrs, "value", ctx)).unwrap_or(control.min);
            control.value = value.clamp(control.min.min(control.max), control.max.max(control.min));
        }
        ControlKind::Radio => {
            control.name = attr(attrs, "name", ctx).unwrap_or_else(|| "radio".to_string());
            control.radio_value = attr(attrs, "value", ctx).unwrap_or(label);
        }
        ControlKind::Select | ControlKind::Segmented => {
            let wanted = attr(attrs, "value", ctx);
            control.selected = wanted
                .and_then(|w| options.iter().position(|(v, l)| *v == w || *l == w))
                .or_else(|| number(attr(attrs, "selected", ctx)).map(|n| n as usize))
                .unwrap_or(0)
                .min(options.len().saturating_sub(1));
            control.options = options;
        }
        // `valeur` : le dessin (`toile::Dessin::encoder`) ; `choisi` : la
        // boite mise en avant ; `mode="relier"`.
        ControlKind::Toile => {
            let mut t = crate::ui::models::toile::Toile::default();
            t.dessin = crate::ui::models::toile::Dessin::decoder(&attr(attrs, "valeur", ctx).unwrap_or_default());
            t.choisi = attr(attrs, "choisi", ctx).filter(|c| !c.is_empty());
            t.relier = attr(attrs, "mode", ctx).as_deref() == Some("relier");
            t.vue = attr(attrs, "vue", ctx).unwrap_or_default();
            t.focus = attr(attrs, "focus", ctx).is_some_and(|f| f == "survol" || f == "oui");
            t.a_cadrer = !t.vue.is_empty();
            control.toile = Some(Box::new(t));
        }
        _ => {}
    }
    out.push(UiNode::Control(control));
    preceding.push(ElementInfo::new(tag, class, id));
}

#[allow(clippy::too_many_arguments)]
fn build_input(tag: &str, class: &str, id: &str, attrs: &[(String, String)], children: &[AstNode], source: &StyleSource, ancestors: &[ElementInfo], ctx: &Context, preceding: &mut Vec<ElementInfo>, out: &mut Vec<UiNode>) {
    let kind = attr(attrs, "type", ctx).unwrap_or_else(|| tag.to_string());
    let style = resolve("input", class, id, base_textarea_style(), source, ancestors, preceding);
    let initial = attr(attrs, "value", ctx).unwrap_or_else(|| interpolate(extract_text(children).trim(), ctx));
    let mut area = TextArea::new(layout_for(&style, source), color_of(style.background), color_of(style.color), initial);
    area.single_line = true;
    area.id = id.to_string();
    area.placeholder = attr(attrs, "placeholder", ctx).unwrap_or_default();
    area.password = kind == "password";
    area.numeric = kind == "number";
    area.font_size = style.font_size.unwrap_or(area.font_size);
    area.font_weight = style.font_weight.unwrap_or(area.font_weight);
    area.hover_background = resolve_pseudo_background("input", class, id, source, ancestors, preceding, crate::compiler::rsc::models::selector::PseudoState { hover: true, focus: false, active: false }).filter(|c| Some(*c) != style.background);
    area.focus_background = resolve_pseudo_background("input", class, id, source, ancestors, preceding, crate::compiler::rsc::models::selector::PseudoState { hover: false, focus: true, active: false }).filter(|c| Some(*c) != style.background);
    area.decoration = decoration_for("input", class, id, &style, source, ancestors, preceding);
    // `focus="true"` : pret a taper (curseur a la fin) ; `focus="tout"` : le
    // texte selectionne, ce qu'on tape le remplace.
    let focus = attrs.iter().any(|(k, _)| k == "focus").then(|| attr(attrs, "focus", ctx)).flatten();
    if focus.as_deref() == Some("tout") || truthy(focus.clone()) {
        area.focused = true;
        area.cursor = area.char_count();
        if focus.as_deref() == Some("tout") && area.cursor > 0 {
            area.selection_anchor = Some(0);
        }
    }
    out.push(UiNode::TextArea(area));
    preceding.push(ElementInfo::new("input", class, id));
}
