use crate::cursor::managers::cursor_manager::CursorSet;
use crate::cursor::models::cursor_kind::CursorKind;
use crate::event::models::app_state::EventState;
use crate::event::models::keys::BTN_LEFT;
use crate::event::services::dispatch;
use crate::navigation::managers::{intra_navigation_manager, navigation_manager};
use crate::navigation::models::intra_client::IntraClient;
use crate::navigation::models::navigation_client::NavigationClient;
use crate::navigation::models::incoming::Incoming;
use crate::navigation::models::route_table::RouteTable;
use crate::ui::models::ui_node::UiNode;
use crate::ui::services::draw_ui::draw_ui;
use crate::ui::services::interact::{self, KeyboardLayout};
use crate::window::models::header_bar::{self, HeaderBar, HeaderButton};
use crate::window::models::hote::{Hote, WaylandHote};
use crate::window::models::resize_edge::resize_edge_at;
use crate::window::services::system_theme;
use crate::window::models::window_context::{Effects, WindowContext};
use crate::window::models::window_table::WindowTable;
use crate::storage::models::stockage::Stockage;
use crate::window::services::draw_header::draw_header;
use azure_core::models::window_model::WindowSpec;
use azure_core::rules::window_event::WindowEvent;
use azure_engine::platform::wayland::managers::window_manager::window_create;
use azure_engine::rendering::managers::renderer::{draw_rect, load_image};
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::color::Color;
use azure_engine::platform::wayland::managers::surface_manager::run_event_loop_interactive;
use azure_engine::platform::wayland::managers::shared_memory_manager::unmap_memory;
use azure_engine::platform::wayland::managers::xdg_manager::{set_title, set_app_id};
use azure_engine::platform::wayland::managers::output_manager::get_screen_resolution;
use azure_engine::platform::wayland::models::screen_output::ScreenOutput;
use std::cell::RefCell;
use std::sync::mpsc::Receiver;
use azure_service::flux::{FluxEvent, Listener, Value as FluxValue};
use std::time::{Duration, Instant};

// Cadence a laquelle la boucle se reveille meme sans evenement Wayland
// (voir `run_event_loop_interactive`) : assez court pour que le
// clignotement et la repetition de touche paraissent fluides, sans pour
// autant reveiller la boucle inutilement souvent quand rien ne se passe.
/// Rappel d'un clic (voir `AzureWindow::on_click`).
type OnClick = Box<dyn FnMut(&mut WindowContext) + Send>;
/// Rappel d'un glisser-deposer (voir `AzureWindow::on_drop`).
type OnDrop = Box<dyn FnMut(&mut WindowContext, &interact::Dropped) + Send>;

const TICK_MS: i32 = 16;
// Plafond du rythme reel d'envoi au compositeur. Sans lui, taper vite
// (plusieurs touches en moins de 16ms) declenchait un `commit()` par
// frappe : le compositeur ne peut pas afficher plus vite que son propre
// taux de rafraichissement, donc certains de ces commits etaient
// silencieusement remplaces par le suivant avant meme d'etre affiches -
// percu comme le curseur "sautant" par-dessus des caracteres plutot que
// d'avancer d'un cran a chaque frappe. Regrouper les redessins rapproches
// dans une seule presentation par tic (voir `flush_if_dirty`) les rend
// fluides, sans jamais retarder l'affichage de plus d'un tic.
const MIN_PRESENT_INTERVAL: Duration = Duration::from_millis(TICK_MS as u64);
const BACKGROUND_COLOR: Color = Color::new(20, 19, 18, 255);

// Tout l'etat mutable partage entre les deux callbacks de la boucle
// d'evenements (`on_event`/`on_tick`, voir `run`). Regroupe dans une seule
// struct plutot que des variables locales captures separement : deux
// closures `FnMut` distinctes ne peuvent pas chacune capturer une
// reference mutable a la meme variable exterieure (erreur de borrow), mais
// peuvent toutes les deux capturer une reference partagee vers un seul
// `RefCell` et emprunter son contenu en mutable au moment de l'appel.
//
// L'interpretation des evenements du CONTENU (clic, frappe, glisser,
// defilement, clignotement, repetition...) ne vit plus ici : elle est
// entierement deleguee a `event::models::app_state::EventState` +
// `event::services::dispatch`, reutilisables tels quels par n'importe
// quelle autre application. Ce qui reste ici est propre a CETTE fenetre
// Wayland : le `Canvas` a presenter, le regroupement des redessins
// (`dirty`/`last_present`), et la barre d'en-tete auto-dessinee (voir
// `window::models::header_bar`) - un chrome de fenetre, pas un `UiNode`
// de l'application, donc distinct de `EventState`.
pub(crate) struct LoopState {
    pub(crate) event: EventState,
    // Boutons dont le texte est change pour un temps (voir
    // `WindowContext::flash`), rendus a leur texte par `on_tick`.
    flashes: Vec<Flash>,
    // Le prochain ecran garde le defilement (voir `WindowContext::refresh`).
    keep_scroll: bool,
    // Jeton avec lequel passer au premier plan (voir
    // `WindowContext::activate`), pris par la boucle au tic suivant.
    activate: Option<String>,
    // Ce que coute la boucle (voir `Perf`).
    pub(crate) perf: Perf,
    pub(crate) canvas: Canvas,
    pub(crate) dirty: bool,
    last_present: Instant,
    header: HeaderBar,
    header_hover: Option<HeaderButton>,
    fullscreen: bool,
    // `true` quand le compositeur dessine lui-meme la decoration (voir
    // `WaylandWindow::is_server_side_decorated`) : la barre d'en-tete
    // maison (`header`) n'est alors ni dessinee ni cliquable, et
    // `content_box` couvre toute la fenetre plutot que de laisser de la
    // place sous une barre qui n'existe pas.
    decorated: bool,
    // La `RouteTable` de cette fenetre (voir `AzureWindow::routes`) - le
    // meme arbre "chemin -> ecran final" sert a resoudre AUSSI BIEN les
    // routes recues via `incoming_routes` (inter-app) que via `intra`
    // (intra-app) : seul le transport qui les fait arriver differe, pas la
    // facon de les resoudre en `UiNode`.
    routes: Option<RouteTable>,
    // Cote reception du routeur INTER-app par socket (voir `navigation` et
    // `AzureWindow::navigation`) : `None` quand la fenetre n'y a pas ete
    // branchee, auquel cas `on_tick` ne verifie jamais ce champ.
    incoming_routes: Option<Receiver<Incoming>>,
    // Le client inter-app lui-meme (voir `navigation`), conserve ici pour
    // que `WindowContext::navigate_to` (voir `on_click` ci-dessous) puisse
    // s'en servir - `incoming_routes` ne garde qu'une COPIE de sa connexion
    // (voir `navigation_manager::listen`), pas de quoi envoyer.
    nav: Option<NavigationClient>,
    // Cote routeur INTRA-app, 100% en memoire (voir
    // `navigation::managers::intra_navigation_manager` et
    // `AzureWindow::intra`) : `None` quand la fenetre n'y a pas ete
    // branchee, auquel cas ni `on_click` (`WindowContext::goto`) ni
    // `on_tick` ne le sondent jamais.
    intra: Option<IntraClient>,
    // La `WindowTable` de cette fenetre (voir `AzureWindow::windows`) :
    // troisieme "expression routeur" apres `routes` (inter/intra-app) -
    // celle-ci resout un `path` en une `AzureWindow` ENTIERE plutot qu'un
    // arbre de `UiNode`, ouverte via `WindowContext::open_window`. `None`
    // tant qu'elle n'a pas ete branchee, auquel cas `open_window` echoue
    // toujours (voir `window_context`).
    windows: Option<WindowTable>,
    // L'app qui a cree cette fenetre (voir `AzureWindow::spec`), transmise a
    // `WindowContext` pour que `open_window` verifie que les fenetres
    // ouvertes depuis celle-ci appartiennent bien a la meme app.
    owner_app_id: Option<u32>,
    // Le stockage de l'app (voir `AzureWindow::stockage`), prete a
    // `WindowContext` a chaque clic.
    stockage: Option<Stockage>,
    // Rappel applicatif declenche par `AzureWindow::on_click` (voir
    // celle-ci) - `None` tant qu'elle n'a pas ete appelee, auquel cas un
    // clic ne fait jamais plus que son effet visuel habituel (toggle de
    // bouton, focus de textarea). Recoit un `WindowContext` : c'est LUI qui
    // distingue navigation intra-app (`goto`, locale), inter-app
    // (`navigate_to`, via azure-rooter) et ouverture de fenetre
    // (`open_window`) - voir `window_context`. `+ Send` : une fenetre
    // ouverte depuis une autre (voir `WindowContext::open_window`) est
    // deplacee vers un thread a part avec TOUTE sa `AzureWindow`, `on_click`
    // inclus.
    on_click: Option<OnClick>,
    // Voir `AzureWindow::on_close`.
    on_close: Option<OnClick>,
    // Voir `AzureWindow::on_drop`.
    on_drop: Option<OnDrop>,
    // Flux ecoutes par cette fenetre (voir `AzureWindow::flux`), sondes a
    // chaque tic.
    fluxes: Vec<(Listener, FluxHandler)>,
    // Voir `AzureWindow::on_tick`.
    on_tick: Option<OnClick>,
    // L'inspecteur (F12, voir `crate::inspector`).
    pub(crate) inspector: crate::inspector::Inspector,
    // La boite « Ouvrir » ouverte par-dessus la page (voir `Actif`).
    selecteur: Option<Actif>,
}

// La boite « Ouvrir » (voir `crate::selecteur`) pendant qu'elle est a
// l'ecran. Elle prend la place de la page dans `LoopState::event` : souris,
// clavier, defilement, essais et inspecteur s'adressent a elle sans rien
// savoir d'elle. La page attend ici, dessinee dessous, et revient a la
// fermeture.
struct Actif {
    ouvert: crate::selecteur::Ouvert,
    page: EventState,
}

// L'arbre de la PAGE de l'app, que la boite « Ouvrir » soit ouverte ou non.
fn noeuds_page<'a>(event: &'a EventState, selecteur: &'a Option<Actif>) -> &'a [UiNode] {
    match selecteur {
        Some(actif) => &actif.page.ui_nodes,
        None => &event.ui_nodes,
    }
}

fn page_mut<'a>(event: &'a mut EventState, selecteur: &'a mut Option<Actif>) -> &'a mut EventState {
    match selecteur {
        Some(actif) => &mut actif.page,
        None => event,
    }
}

// Ouvre la boite « Ouvrir » (voir `WindowContext::choisir`) ; deja ouverte,
// la nouvelle demande la remplace.
fn ouvrir_selecteur(s: &mut LoopState, demande: crate::selecteur::Selecteur) {
    let ouvert = crate::selecteur::Ouvert::new(demande);
    let mut boite = EventState::new(ouvert.noeuds());
    boite.mouse_x = s.event.mouse_x;
    boite.mouse_y = s.event.mouse_y;
    boite.held_modifiers = s.event.held_modifiers.clone();
    boite.clipboard = s.event.clipboard.clone();
    match s.selecteur.as_mut() {
        Some(actif) => {
            actif.ouvert = ouvert;
            s.event = boite;
        }
        None => {
            let mut page = std::mem::replace(&mut s.event, boite);
            // Le relachement du clic (ou de la touche) qui ouvre la boite
            // ira a la boite : la page ne doit pas le croire encore tenu.
            interact::release_all(&mut page.ui_nodes);
            page.dragging = false;
            page.scroll_drag = None;
            page.drag = None;
            page.held_key = None;
            page.next_repeat_at = None;
            page.tooltip = None;
            s.selecteur = Some(Actif { ouvert, page });
        }
    }
    s.dirty = true;
}

// Ferme la boite « Ouvrir » : la page reprend sa place.
fn fermer_selecteur(s: &mut LoopState) -> Option<crate::selecteur::Ouvert> {
    let Actif { ouvert, page } = s.selecteur.take()?;
    let boite = std::mem::replace(&mut s.event, page);
    s.event.mouse_x = boite.mouse_x;
    s.event.mouse_y = boite.mouse_y;
    s.event.held_modifiers = boite.held_modifiers;
    s.event.clipboard = boite.clipboard;
    s.event.tooltip_sought = false;
    s.dirty = true;
    Some(ouvert)
}

// Un clic (ou une activation au clavier) dans la boite « Ouvrir ».
fn clic_selecteur(s: &mut LoopState, h: &mut dyn Hote) {
    use crate::selecteur::Suite;
    let Some(id) = s.event.clicked_id.clone() else { return };
    let double = s.event.last_click.is_some_and(|(_, _, _, rang)| rang >= 2);
    let Some(actif) = s.selecteur.as_mut() else { return };
    match actif.ouvert.cliquer(&id, double) {
        Suite::Rien => {}
        suite @ (Suite::Liste | Suite::Garder) => {
            let mut noeuds = actif.ouvert.noeuds();
            if suite == Suite::Garder {
                let content = content_box(s);
                interact::carry_scroll_anchored(&s.event.ui_nodes, &mut noeuds, content);
            }
            s.event.ui_nodes = noeuds;
            s.event.tooltip = None;
            s.event.tooltip_sought = false;
            s.dirty = true;
        }
        Suite::Annuler => {
            fermer_selecteur(s);
        }
        Suite::Choisi(chemin) => {
            let Some(ouvert) = fermer_selecteur(s) else { return };
            let chemin = chemin.to_string_lossy().into_owned();
            if let Some(champ) = ouvert.demande().champ() {
                interact::set_field_text(&mut s.event.ui_nodes, champ, &chemin);
            }
            // L'app apprend le choix comme un clic sur `puis`.
            s.event.clicked_id = ouvert.demande().suite().map(str::to_string);
            appeler_on_click(s, h, Some(&chemin));
        }
    }
}

// Rappel d'un flux ecoute (voir `AzureWindow::flux`).
type FluxHandler = Box<dyn FnMut(&mut WindowContext, &FluxEvent, &FluxValue) + Send>;

// La forme du curseur pour la position actuelle de la souris : main sur les
// boutons de la barre d'en-tete maison, fleche sur ses bords de
// redimensionnement et sa zone de deplacement, sinon ce que le contenu
// survole (voir `CursorKind::from_hover` - barre de texte uniquement sur
// une zone editable). `s.event.hover`/`s.header_hover` sont deja a jour :
// appelee apres le traitement de l'evenement.
fn cursor_kind(s: &LoopState) -> CursorKind {
    if !s.decorated {
        if s.header_hover.is_some() {
            return CursorKind::Pointer;
        }
        let in_header = s.event.mouse_y >= 0 && (s.event.mouse_y as u32) < header_bar::HEADER_HEIGHT;
        if in_header || resize_edge_at(s.canvas.width, s.canvas.height, s.event.mouse_x, s.event.mouse_y).is_some() {
            return CursorKind::Default;
        }
    }
    CursorKind::from_hover(s.event.hover)
}

// La boite de contenu a utiliser pour CE tour de boucle - `(0, 0, w, h)`
// quand le compositeur decore lui-meme la fenetre (voir `LoopState::decorated`),
// sinon celle sous la barre d'en-tete maison (voir
// `header_bar::content_box`). Un point d'entree unique plutot que d'appeler
// `header_bar::content_box` directement partout : sinon, sous decoration
// native, le contenu applicatif se dessinerait avec une marge de
// `HEADER_HEIGHT` vide en haut, pour une barre qui n'est plus dessinee.
pub(crate) fn content_box(state: &LoopState) -> (u32, u32, u32, u32) {
    // L'inspecteur ouvert prend la droite de la fenetre : la page se
    // recalcule dans ce qui reste, comme dans Chrome.
    state.inspector.page_box(window_content(state))
}

// Toute la place sous la barre d'en-tete (page + inspecteur).
fn window_content(state: &LoopState) -> (u32, u32, u32, u32) {
    if state.decorated {
        (0, 0, state.canvas.width, state.canvas.height)
    } else {
        header_bar::content_box(state.canvas.width, state.canvas.height)
    }
}

// Redessine le fond + la barre d'en-tete + tout l'arbre de widgets dans
// `state.canvas` (sans rien envoyer au compositeur : voir `present`). Les
// dimensions viennent de `state.canvas` lui-meme, pas d'un parametre a
// part : apres un redimensionnement de fenetre (voir `AzureWindow::run`,
// le bras `WindowEvent::WindowResize`), `state.canvas` est deja recree a
// la bonne taille, donc tout ici suit automatiquement.
pub(crate) fn redraw(state: &mut LoopState) {
    let (width, height) = (state.canvas.width, state.canvas.height);
    // `draw_rect` remplit une ligne entiere d'un coup (voir sa
    // documentation) : bien plus rapide qu'ecrire chaque pixel du fond
    // individuellement, ce qui comptait pour une bonne partie du cout
    // d'un redessin complet.
    draw_rect(0, 0, width, height, &BACKGROUND_COLOR, &mut state.canvas);
    if !state.decorated {
        draw_header(&mut state.canvas, width, &state.header, state.header_hover, state.fullscreen);
        let hover = header_bar::on_inspect_button(width, state.event.mouse_x, state.event.mouse_y);
        crate::inspector::draw_header_button(header_bar::inspect_button(width), state.inspector.open, hover, state.inspector.enregistrement.is_some(), &mut state.canvas);
    }

    // Le contenu de l'application se dessine SOUS la barre d'en-tete
    // maison quand elle existe, jamais par-dessus ni derriere (voir
    // `content_box` ci-dessus) - c'est ce rectangle-ci, pas
    // `(0, 0, width, height)` directement, qui sert de boite racine au
    // layout en pourcentage de `ui::services::draw_ui`.
    let content = content_box(state);
    // La boite « Ouvrir » est ouverte : la page reste visible dessous,
    // sans survol ni curseur.
    if let Some(actif) = &state.selecteur {
        draw_ui(&actif.page.ui_nodes, content, &mut state.canvas, -1, -1, false);
    }
    draw_ui(
        &state.event.ui_nodes,
        content,
        &mut state.canvas,
        state.event.mouse_x,
        state.event.mouse_y,
        state.event.caret_visible,
    );
    if let Some(menu) = &state.event.format_menu {
        crate::ui::services::draw_ui::draw_format_menu(menu, &mut state.canvas, state.event.mouse_x, state.event.mouse_y);
    }
    if let Some(menu) = &state.event.command_menu {
        crate::ui::services::draw_ui::draw_command_menu(menu, &mut state.canvas, state.event.mouse_x, state.event.mouse_y);
    }
    if let Some(drag) = &state.event.drag {
        crate::ui::services::draw_ui::draw_drag(&state.event.ui_nodes, drag, state.event.mouse_x, state.event.mouse_y, &mut state.canvas);
    }
    if let Some((text, x, y)) = &state.event.tooltip {
        crate::ui::services::draw_ui::draw_tooltip(text, *x, *y, content, &mut state.canvas);
    }
    let full = window_content(state);
    crate::inspector::draw(&mut state.inspector, &state.event.ui_nodes, full, &mut state.canvas);
}

// Un bouton affiche `shown` a la place de `original` jusqu'a `until`.
struct Flash {
    id: String,
    shown: String,
    original: String,
    until: Instant,
}

// Applique ce qu'un rappel a demande (voir `window_context::Effects`).
fn apply_effects(s: &mut LoopState, effects: Effects) {
    s.keep_scroll |= effects.keep_scroll;
    if let Some(demande) = effects.selecteur {
        ouvrir_selecteur(s, demande);
    }
    if effects.activate.is_some() {
        s.activate = effects.activate;
    }
    if let Some(text) = effects.copy {
        s.event.clipboard = text;
        s.event.clipboard_changed = true;
    }
    for (id, text, duration) in effects.flashes {
        let Some(previous) = interact::set_button_text(&mut s.event.ui_nodes, &id, &text) else { continue };
        let until = Instant::now() + duration;
        // Deja en cours : on garde le vrai texte d'origine.
        match s.flashes.iter_mut().find(|f| f.id == id) {
            Some(flash) => {
                flash.shown = text;
                flash.until = until;
            }
            None => s.flashes.push(Flash { id, shown: text, original: previous, until }),
        }
        s.dirty = true;
    }
}

// Rend leur texte aux boutons dont le flash est fini ; `true` si l'un a change.
fn end_flashes(s: &mut LoopState) -> bool {
    let now = Instant::now();
    let mut changed = false;
    let (done, kept): (Vec<Flash>, Vec<Flash>) = std::mem::take(&mut s.flashes).into_iter().partition(|f| f.until <= now);
    s.flashes = kept;
    for flash in done {
        // Une page reconstruite entre-temps n'a plus ce texte : on n'y touche pas.
        if let Some(current) = interact::set_button_text(&mut s.event.ui_nodes, &flash.id, &flash.original)
            && current != flash.shown
        {
            interact::set_button_text(&mut s.event.ui_nodes, &flash.id, &current);
        } else {
            changed = true;
        }
    }
    changed
}

// `AzureWindow::on_click` avec le contexte du moment (bouton touche,
// valeurs des champs) - apres un clic souris ou une activation au clavier.
fn call_on_click(s: &mut LoopState, h: &mut dyn Hote) {
    // La boite « Ouvrir » est ouverte : le clic est pour elle, pas pour l'app.
    if s.selecteur.is_some() {
        clic_selecteur(s, h);
        return;
    }
    appeler_on_click(s, h, None);
}

// `choix` : le chemin qui vient d'etre choisi dans la boite « Ouvrir »
// (voir `WindowContext::choix`).
fn appeler_on_click(s: &mut LoopState, h: &mut dyn Hote, choix: Option<&str>) {
    let Some(mut on_click) = s.on_click.take() else { return };
    let clicked = s.event.clicked_id.clone();
    // Jeton demande seulement si le rappel en a besoin (voir
    // `WindowContext::activation_token`).
    let mut token = || h.jeton_activation();
    let nom = crate::perf::generaliser(clicked.as_deref().unwrap_or("?"));
    let (scroll, effects) = crate::perf::mesurer("Clic", &nom, || with_context_activation(s, clicked.as_deref(), Some(&mut token), choix, |ctx| on_click(ctx)));
    s.on_click = Some(on_click);
    let plein_ecran = effects.plein_ecran;
    apply_effects(s, effects);
    if let Some(actif) = plein_ecran
        && actif != s.fullscreen
    {
        s.fullscreen = actif;
        h.plein_ecran(actif);
        s.dirty = true;
    }
    if let Some(target) = scroll {
        let content = content_box(s);
        if crate::ui::services::interact::scroll_to_anchor(&mut s.event.ui_nodes, &target, content) {
            s.dirty = true;
        }
    }
}

// `AzureWindow::on_drop` pour l'element qui vient d'etre lache.
fn call_on_drop(s: &mut LoopState, dropped: &interact::Dropped) {
    let Some(mut on_drop) = s.on_drop.take() else { return };
    let (scroll, effects) = with_context(s, None, |ctx| on_drop(ctx, dropped));
    s.on_drop = Some(on_drop);
    s.dirty = true;
    apply_effects(s, effects);
    if let Some(target) = scroll {
        let content = content_box(s);
        crate::ui::services::interact::scroll_to_anchor(&mut s.event.ui_nodes, &target, content);
    }
}

// Appelle `f` avec le contexte du moment (valeurs des champs, stockage,
// routeurs) ; rend ce qu'il a demande (defilement, effets).
fn with_context(s: &mut LoopState, clicked: Option<&str>, f: impl FnOnce(&mut WindowContext)) -> (Option<String>, Effects) {
    with_context_activation(s, clicked, None, None, f)
}

fn with_context_activation(s: &mut LoopState, clicked: Option<&str>, activation: Option<&mut dyn FnMut() -> Option<String>>, choix: Option<&str>, f: impl FnOnce(&mut WindowContext)) -> (Option<String>, Effects) {
    let values = crate::ui::services::form::form_values(noeuds_page(&s.event, &s.selecteur));
    let mut ctx = WindowContext {
        intra: s.intra.as_ref(),
        nav: s.nav.as_mut(),
        windows: s.windows.as_ref(),
        routes: s.routes.as_ref(),
        stockage: s.stockage.as_ref(),
        app_id: s.owner_app_id,
        clicked,
        values: Some(&values),
        scroll_request: None,
        effects: Effects::default(),
        // Ramenee a la duree de vie du contexte.
        activation: activation.map(|a| a as &mut dyn FnMut() -> Option<String>),
        choix,
    };
    f(&mut ctx);
    (ctx.scroll_request.take(), std::mem::take(&mut ctx.effects))
}

fn redraw_and_present(h: &mut dyn Hote, state: &mut LoopState) {
    redraw(state);
    h.presenter(&state.canvas);
}

// Presente `state` seulement si (a) quelque chose a effectivement change
// depuis le dernier present (`state.dirty`) ET (b) au moins
// `MIN_PRESENT_INTERVAL` s'est ecoule depuis ce dernier present. Sinon,
// laisse `dirty` a `true` : le tic suivant (au plus tard dans `TICK_MS`)
// rappellera cette fonction et flushera alors. Appele en fin de chaque
// callback (`on_event`/`on_tick`) plutot que de presenter directement a
// chaque changement d'etat - voir `MIN_PRESENT_INTERVAL`.
/// Ce que coute la boucle de la fenetre. Toujours : une etape de plus de
/// `Perf::LENT` est ecrite sur la sortie d'erreur (le journal de l'app).
/// Avec `AZURE_PERF=1` : un bilan par seconde (tics, evenements, redessins
/// et leurs causes, temps passe dans chaque etape) pour trouver ce qui
/// charge ou fige une app.
pub(crate) struct Perf {
    bilan: bool,
    depuis: Instant,
    tics: u32,
    evenements: u32,
    pub(crate) redessins: u32,
    causes: std::collections::BTreeMap<&'static str, u32>,
    temps: std::collections::BTreeMap<&'static str, Duration>,
}

impl Perf {
    const LENT: Duration = Duration::from_millis(100);

    fn new() -> Perf {
        Perf { bilan: std::env::var_os("AZURE_PERF").is_some_and(|v| v != "0"), depuis: Instant::now(), tics: 0, evenements: 0, redessins: 0, causes: Default::default(), temps: Default::default() }
    }

    /// `etape` a pris `duree`.
    fn temps(&mut self, etape: &'static str, duree: Duration) {
        if duree >= Self::LENT {
            eprintln!("AzureWindow : lent : {etape} {:.0} ms", duree.as_secs_f64() * 1000.0);
        }
        if self.bilan {
            *self.temps.entry(etape).or_default() += duree;
        }
    }

    /// Un redessin a ete demande pour `cause`.
    fn cause(&mut self, cause: &'static str) {
        if self.bilan {
            *self.causes.entry(cause).or_default() += 1;
        }
    }

    fn fin_de_tic(&mut self) {
        self.tics += 1;
        if !self.bilan || self.depuis.elapsed() < Duration::from_secs(1) {
            return;
        }
        let ms = |d: &Duration| d.as_secs_f64() * 1000.0;
        let causes: Vec<String> = self.causes.iter().map(|(c, n)| format!("{c} {n}")).collect();
        let temps: Vec<String> = self.temps.iter().map(|(e, d)| format!("{e} {:.0} ms", ms(d))).collect();
        eprintln!("AzureWindow : 1 s : {} tics, {} evenements, {} redessins [{}] ; temps : {}", self.tics, self.evenements, self.redessins, causes.join(", "), temps.join(", "));
        *self = Perf { bilan: true, ..Perf::new() };
    }
}

pub(crate) fn flush_if_dirty(h: &mut dyn Hote, state: &mut LoopState) {
    if !state.dirty || state.last_present.elapsed() < MIN_PRESENT_INTERVAL {
        return;
    }
    let debut = Instant::now();
    crate::perf::mesurer("Fenêtre", "dessin", || redraw_and_present(h, state));
    state.perf.redessins += 1;
    state.perf.temps("dessin", debut.elapsed());
    state.dirty = false;
    state.last_present = Instant::now();
}

// Un evenement de la fenetre (souris, clavier, taille...) : le meme
// traitement pour la vraie fenetre et pour le pilote d'essai (voir
// `hote`). `false` : fermer.
pub(crate) fn sur_evenement(s: &mut LoopState, h: &mut dyn Hote, event: WindowEvent, layout: &std::cell::Cell<KeyboardLayout>) -> bool {
    let debut_evenement = Instant::now();
    let deja_sale = s.dirty;
    s.perf.evenements += 1;
    h.clavier(&mut s.event, layout);
    let layout = layout.get();

    // L'inspecteur (F12) prend ses evenements avant l'app : son
    // panneau, et la page tant qu'il sert a choisir un element.
    if let WindowEvent::WindowMouseMove(x, y) = event {
        s.event.mouse_x = x;
        s.event.mouse_y = y;
    }
    // Bouton « Inspecter » de la barre de titre.
    if !s.decorated {
        if let WindowEvent::WindowMouseButton(button, true) = event
            && button == BTN_LEFT
            && header_bar::on_inspect_button(s.canvas.width, s.event.mouse_x, s.event.mouse_y)
        {
            s.inspector.toggle();
            s.dirty = true;
            flush_if_dirty(h, s);
            return true;
        }
        if let WindowEvent::WindowMouseMove(x, y) = event {
            let (bx, by, bw, bh) = header_bar::inspect_button(s.canvas.width);
            let near = x >= bx as i32 - 40 && y >= 0 && x < (bx + bw) as i32 + 40 && y < (by + bh) as i32 + 10;
            if near {
                s.dirty = true;
            }
        }
    }
    let full = window_content(s);
    let ctrl_shift = s.event.ctrl_held() && s.event.shift_held();
    if let Some(redraw) = s.inspector.handle_event(&event, &s.event.ui_nodes, full, (s.event.mouse_x, s.event.mouse_y), ctrl_shift) {
        if redraw {
            s.dirty = true;
        }
        // Le test genere a l'arret d'un enregistrement : copie.
        if let Some(code) = s.inspector.a_copier.take() {
            s.event.clipboard = code;
            if let Err(err) = h.ecrire_presse_papiers(&s.event.clipboard) {
                eprintln!("AzureWindow: presse-papiers du systeme indisponible: {err}");
            }
        }
        flush_if_dirty(h, s);
        return true;
    }
    // Un scenario s'enregistre (voir `inspector::enregistreur`) : ce que
    // l'app va recevoir, tant que l'ecran est celui que la personne voit.
    let page = content_box(s);
    let dans_la_page = s.event.mouse_x >= page.0 as i32 && s.event.mouse_y >= page.1 as i32;
    if let Some(r) = s.inspector.enregistrement.as_mut()
        && (dans_la_page || matches!(event, WindowEvent::WindowKeyPress(..)))
    {
        r.noter(&event, &s.event.ui_nodes, page, (s.event.mouse_x, s.event.mouse_y));
        s.dirty = true;
    }

    // Coller : le texte vient du presse-papiers du systeme (copie
    // par n'importe quelle app, Azure ou non), pas seulement de
    // cette fenetre.
    if let WindowEvent::WindowKeyPress(key, true) = event
        && interact::key_to_input_with(key, layout, s.event.modifiers(layout)) == Some(interact::KeyInput::Paste)
        && let Some(text) = h.presse_papiers()
    {
        s.event.clipboard = text;
    }

    // Un clic gauche qui tombe sur un des 3 boutons de la
    // barre d'en-tete (voir `header_bar::button_at`) est gere
    // ICI, jamais transmis a `dispatch::handle_event` : ce
    // n'est pas une interaction sur l'arbre `ui_nodes` de
    // l'application, mais sur le chrome de CETTE fenetre.
    // Desactive entierement quand le compositeur decore lui-meme
    // la fenetre (`s.decorated`) : il n'y a alors ni barre ni
    // boutons dessines par nous, et le haut de la surface EST du
    // contenu applicatif (la decoration native se dessine hors
    // de notre buffer) - y detecter des boutons y avalerait des
    // clics destines a l'application.
    let header_click = if s.decorated {
        None
    } else {
        match event {
            WindowEvent::WindowMouseButton(button, true) if button == BTN_LEFT => {
                header_bar::button_at(s.canvas.width, &s.header.layout, s.event.mouse_x, s.event.mouse_y)
            }
            _ => None,
        }
    };

    if let Some(button) = header_click {
        match button {
            HeaderButton::Minimize => {
                h.minimiser();
            }
            HeaderButton::Fullscreen => {
                s.fullscreen = !s.fullscreen;
                if s.fullscreen {
                    h.plein_ecran(true);
                } else {
                    h.plein_ecran(false);
                }
                s.dirty = true;
            }
            // Ferme la fenetre nous-memes (voir
            // `run_event_loop_interactive`, qui arrete sa
            // boucle quand ce callback retourne `false`) -
            // exactement comme un vrai `xdg_toplevel::close`
            // du compositeur, juste demande par notre propre
            // bouton plutot que par lui.
            HeaderButton::Close => return false,
        }
    } else if !s.decorated
        && matches!(event, WindowEvent::WindowMouseButton(button, true) if button == BTN_LEFT)
        && resize_edge_at(s.canvas.width, s.canvas.height, s.event.mouse_x, s.event.mouse_y).is_some()
    {
        // Clic gauche pres d'un bord/coin de la fenetre (voir
        // `resize_edge::resize_edge_at` - n'importe quel bord,
        // header inclus, contrairement a la poignee de
        // deplacement ci-dessous qui n'existe QUE dans la barre
        // d'en-tete) : demande au compositeur un
        // redimensionnement interactif le long de ce bord,
        // meme mecanisme que `move_toplevel` (le compositeur
        // prend la main sur le pointeur jusqu'au relachement du
        // bouton). Prioritaire sur la poignee de deplacement
        // pour que les quelques pixels du bord superieur de la
        // barre d'en-tete restent redimensionnables plutot que
        // toujours interpretes comme un deplacement.
        let edge = resize_edge_at(s.canvas.width, s.canvas.height, s.event.mouse_x, s.event.mouse_y)
            .expect("checked by the guard above");
        h.redimensionner_au_bord(edge);
    } else if !s.decorated
        && matches!(event, WindowEvent::WindowMouseButton(button, true) if button == BTN_LEFT)
        && s.event.mouse_y >= 0
        && (s.event.mouse_y as u32) < header_bar::HEADER_HEIGHT
    {
        // Clic gauche dans la barre d'en-tete maison, hors des 3
        // boutons (deja geres ci-dessus par `header_click`) :
        // c'est la poignee de deplacement de la fenetre. Demande
        // au compositeur de prendre la main sur le pointeur
        // jusqu'au relachement du bouton (voir
        // `xdg_manager::move_toplevel`) - exactement ce qu'une
        // decoration native ferait pour un glisser-deposer de
        // fenetre. Le serial DOIT etre celui de CET appui (voir
        // `Window::last_pointer_serial`, mis a jour juste avant
        // que ce callback soit appele).
        h.deplacer();
    } else {
        match event {
            // Redimensionnement (bordure tiree a la souris,
            // passage en plein ecran via le bouton
            // ci-dessus, "snap" du compositeur, ...) : propre
            // a CETTE fenetre Wayland (recreer le
            // buffer/canvas), pas une interaction sur l'arbre
            // de widgets. `(0, 0)` (le compositeur laisse le
            // client choisir) est ignore : on garde la taille
            // courante plutot que de tenter un buffer vide.
            WindowEvent::WindowResize(new_width, new_height) if new_width > 0 && new_height > 0 => {
                h.nouvelle_taille(new_width, new_height);
                // Le layout (pourcentages de la boite de
                // contenu, voir `layout::managers::layout_manager`)
                // se recalcule tout seul au prochain redessin
                // a partir des nouvelles dimensions de
                // `state.canvas` - rien d'autre a recalculer ici.
                s.canvas = Canvas::new(new_width as u32, new_height as u32);
                s.dirty = true;
            }
            WindowEvent::WindowResize(_, _) => {}
            // Distingue du `other` ci-dessous UNIQUEMENT pour
            // pouvoir appeler `on_click` (voir
            // `AzureWindow::on_click`) apres le dispatch normal
            // - le relachement du bouton, les autres boutons de
            // souris et tout le reste continuent de tomber dans
            // `other`, inchanges.
            WindowEvent::WindowMouseButton(button, true) if button == BTN_LEFT => {
                let content = content_box(s);
                if crate::perf::mesurer("Fenêtre", "souris et clavier", || dispatch::handle_event(&mut s.event, event, layout, content)) {
                    s.dirty = true;
                    call_on_click(s, h);
                }
            }
            WindowEvent::WindowMouseMove(x, y) => {
                if !s.decorated {
                    let new_header_hover = header_bar::button_at(s.canvas.width, &s.header.layout, x, y);
                    if new_header_hover != s.header_hover {
                        s.header_hover = new_header_hover;
                        s.dirty = true;
                    }
                }
                let content = content_box(s);
                if crate::perf::mesurer("Fenêtre", "souris et clavier", || dispatch::handle_event(&mut s.event, event, layout, content)) {
                    s.dirty = true;
                }
            }
            // Tout le reste (clic dans le contenu, glisser,
            // frappe, defilement, raccourcis...) vit dans
            // `event::services::dispatch`, reutilisable telle
            // quelle hors de cette fenetre - voir sa
            // documentation.
            other => {
                let content = content_box(s);
                if crate::perf::mesurer("Fenêtre", "souris et clavier", || dispatch::handle_event(&mut s.event, other, layout, content)) {
                    s.dirty = true;
                }
                // Bouton ou champ active au clavier (Entree,
                // Espace, fleches, Echap) : comme un clic.
                if s.event.take_activation() {
                    call_on_click(s, h);
                }
                // Element lache dans une zone.
                if let Some(dropped) = s.event.take_dropped() {
                    call_on_drop(s, &dropped);
                }
            }
        }
    }

    h.curseur(cursor_kind(s));

    // Copier : la selection du systeme devient la notre, les
    // autres apps peuvent coller.
    if s.event.take_clipboard_change()
        && let Err(err) = h.ecrire_presse_papiers(&s.event.clipboard)
    {
        eprintln!("AzureWindow: presse-papiers du systeme indisponible: {err}");
    }

    if s.dirty && !deja_sale {
        s.perf.cause("evenement");
    }
    s.perf.temps("evenements", debut_evenement.elapsed());
    flush_if_dirty(h, s);
    true
}

// Un tic (~60 Hz) : routes recues, flux, `on_tick`, animations, redessin.
// `false` : fermer.
pub(crate) fn sur_tic(s: &mut LoopState, h: &mut dyn Hote, layout: &std::cell::Cell<KeyboardLayout>) -> bool {
    // `kill` (voir azure_core::security::termination) : fermer
    // comme par le bouton, `on_close` compris.
    if azure_core::security::termination::requested() {
        return false;
    }
    let content = content_box(s);
    // Transition CSS en cours (voir `ui::models::transition`) :
    // l'image suivante au prochain tic.
    if crate::ui::models::transition::take_pending() {
        s.dirty = true;
        s.perf.cause("transition");
    }

    // Nouvel ecran recu via l'un ou l'autre transport de routes
    // (voir `LoopState::routes`) : remplace `ui_nodes` tel quel,
    // comme un `.ui(nodes)` rappele en cours de session. Draine
    // tout ce qui est en attente sur CHACUN (pas juste le
    // premier) mais ne garde que le tout dernier ecran resolu -
    // inutile de redessiner pour des routes deja perimees par
    // une plus recente, qu'elle vienne du meme transport ou de
    // l'autre.
    let mut pending_screen = None;

    // INTER-app (voir `navigation` et `AzureWindow::navigation`) :
    // un thread dedie a deja pousse chaque route decodee dans ce
    // canal (voir `navigation_manager::listen`).
    if let Some(incoming) = &s.incoming_routes {
        while let Ok(message) = incoming.try_recv() {
            match message {
                Incoming::Route(route) => {
                    // Jeton de l'app qui a demande la page :
                    // la fenetre passe au premier plan.
                    if !route.activation.is_empty()
                        && let Err(err) = h.activer(&route.activation)
                    {
                        eprintln!("AzureWindow: premier plan impossible : {err}");
                    }
                    if let Some(table) = &s.routes
                        && let Some(nodes) = table.resolve(&route) {
                            pending_screen = Some(nodes);
                        }
                }
                // Fenetre envoyee par une autre app (voir
                // `WindowContext::send_window`) : ouverte dans
                // son propre thread, comme `open_window`.
                Incoming::Window(shared) => match shared.to_window() {
                    Ok(window) => {
                        std::thread::spawn(move || window.run());
                    }
                    Err(err) => eprintln!("AzureWindow: fenetre recue invalide : {err}"),
                },
            }
        }
    }

    // Flux ecoutes (voir `AzureWindow::flux`) : AVANT le
    // sondage intra-app, pour qu'un `ctx.goto` lance par un
    // rappel prenne effet des ce tic.
    let mut scrolls = Vec::new();
    let mut all_effects = Vec::new();
    for (listener, handler) in s.fluxes.iter_mut() {
        for event in listener.poll() {
            let values = crate::ui::services::form::form_values(noeuds_page(&s.event, &s.selecteur));
            let mut ctx = WindowContext {
                intra: s.intra.as_ref(),
                nav: s.nav.as_mut(),
                windows: s.windows.as_ref(),
                routes: s.routes.as_ref(),
                stockage: s.stockage.as_ref(),
                app_id: s.owner_app_id,
                clicked: None,
                values: Some(&values),
                scroll_request: None,
                effects: Effects::default(),
                activation: None,
                choix: None,
            };
            crate::perf::mesurer("Flux", "écoute", || handler(&mut ctx, &event, listener.state()));
            scrolls.extend(ctx.scroll_request.take());
            all_effects.push(std::mem::take(&mut ctx.effects));
        }
    }
    if let Some(mut tick) = s.on_tick.take() {
        let debut = Instant::now();
        let (scroll, effects) = crate::perf::mesurer("Tâche de fond", "on_tick", || with_context(s, None, |ctx| tick(ctx)));
        s.perf.temps("on_tick", debut.elapsed());
        s.on_tick = Some(tick);
        scrolls.extend(scroll);
        all_effects.push(effects);
    }
    for effects in all_effects {
        apply_effects(s, effects);
    }
    if let Some(token) = s.activate.take()
        && let Err(err) = h.activer(&token)
    {
        eprintln!("AzureWindow: premier plan impossible : {err}");
    }
    for target in scrolls {
        if crate::ui::services::interact::scroll_to_anchor(&mut s.event.ui_nodes, &target, content) {
            s.dirty = true;
            s.perf.cause("ancre");
        }
    }

    // INTRA-app (voir `AzureWindow::intra`) : sondage non
    // bloquant de l'`IntraRouter` local, aucun thread ni canal -
    // voir `intra_navigation_manager::drain`.
    let debut_ecran = Instant::now();
    if let Some(intra) = &s.intra
        && let Some(table) = &s.routes
            && let Some(nodes) = intra_navigation_manager::drain(intra, table) {
                pending_screen = Some(nodes);
            }
    if pending_screen.is_some() {
        s.perf.temps("nouvel ecran", debut_ecran.elapsed());
    }

    if let Some(mut nodes) = pending_screen {
        // La page de l'app, meme sous la boite « Ouvrir ».
        let page = page_mut(&mut s.event, &mut s.selecteur);
        if std::mem::take(&mut s.keep_scroll) {
            interact::carry_scroll_anchored(&page.ui_nodes, &mut nodes, content);
        }
        // Les toiles gardent leur vue (zoom, decalage).
        interact::carry_toiles(&page.ui_nodes, &mut nodes);
        page.ui_nodes = nodes;
        page.tooltip_sought = false;
        page.command_menu = None;
        s.flashes.clear();
        s.dirty = true;
        s.perf.cause("nouvel ecran");
    }
    if !s.flashes.is_empty() && end_flashes(s) {
        s.dirty = true;
        s.perf.cause("flash");
    }

    // Clignotement du curseur + repetition d'une touche
    // maintenue : ne dependent que du temps ecoule, pas d'un
    // evenement recu - voir `dispatch::handle_tick`.
    let debut = Instant::now();
    if crate::perf::mesurer("Fenêtre", "curseur et défilement", || dispatch::handle_tick(&mut s.event, layout.get(), content)) {
        s.dirty = true;
        s.perf.cause("defilement/curseur/touche");
    }
    s.perf.temps("tic", debut.elapsed());

    // C'est aussi ici (appele a chaque tic, ~60Hz) que les
    // redessins mis en attente par `on_event` (voir
    // `MIN_PRESENT_INTERVAL`) sont effectivement flushes.
    flush_if_dirty(h, s);
    s.perf.fin_de_tic();
    crate::perf::publier_si_temps();
    true
}

// La fenetre se ferme : `on_close` (derniere chance d'enregistrer), puis
// l'app quitte le routeur.
pub(crate) fn terminer(s: &mut LoopState) {
    // Derniere chance d'enregistrer ce qui a ete tape (voir `on_close`).
    if let Some(mut on_close) = s.on_close.take() {
        with_context(s, None, |ctx| on_close(ctx));
    }
    if let Some(nav) = s.nav.as_mut() {
        let _ = navigation_manager::disconnect(nav);
    }
}

pub struct AzureWindow{
    title: String,
    width: Option<u32>,
    height: Option<u32>,
    ui_nodes: Vec<UiNode>,
    icon_path: Option<String>,
    app_id: Option<String>,
    nav: Option<NavigationClient>,
    intra: Option<IntraClient>,
    routes: Option<RouteTable>,
    windows: Option<WindowTable>,
    spec: Option<WindowSpec>,
    stockage: Option<Stockage>,
    on_click: Option<OnClick>,
    on_close: Option<OnClick>,
    on_drop: Option<OnDrop>,
    fluxes: Vec<(Listener, FluxHandler)>,
    on_tick: Option<OnClick>,
}

// Identifiant par defaut derive du titre quand `AzureWindow::app_id` n'est
// pas appele : minuscules, tout sauf [a-z0-9.-] remplace par '-', tirets
// consecutifs/en bordure retires. Ne correspondra a aucun fichier .desktop
// reel tant qu'aucun n'est installe sous ce nom exact - c'est ce fichier
// (son `Icon=`), pas cette fonction, qui determine l'icone affichee dans
// la barre des taches/le dock (voir `AzureWindow::app_id`).
fn slugify_app_id(title: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = true; // evite un tiret de tete
    for ch in title.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() { "azure-app".to_string() } else { slug }
}

impl AzureWindow {
    pub fn new(title: &str) -> AzureWindow {
        AzureWindow {
            title: title.to_string(),
            width: None,
            height: None,
            ui_nodes: Vec::new(),
            icon_path: None,
            app_id: None,
            nav: None,
            intra: None,
            routes: None,
            windows: None,
            spec: None,
            stockage: None,
            on_click: None,
            on_close: None,
            on_drop: None,
            fluxes: Vec::new(),
            on_tick: None,
        }
    }

    /// La taille demandee par l'app (`size` ou `spec`), s'il y en a une.
    pub(crate) fn taille_demandee(&self) -> Option<(u32, u32)> {
        if let Some(spec) = &self.spec {
            return Some((spec.size().width(), spec.size().height()));
        }
        Some((self.width?, self.height?))
    }

    pub fn size(mut self, width: u32, height: u32) -> AzureWindow {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Les donnees minimales de la fenetre (app creatrice, taille, etat,
    /// scope, type - voir `azure_core::models::window_model::WindowSpec`).
    /// Obligatoire pour une fenetre ouverte depuis une app via
    /// `WindowContext::open_window`. Remplace la taille donnee par `size` /
    /// `fullscreen` : c'est celle de la spec qui compte.
    pub fn spec(mut self, spec: WindowSpec) -> AzureWindow {
        self.width = Some(spec.size().width());
        self.height = Some(spec.size().height());
        self.spec = Some(spec);
        self
    }

    pub fn window_spec(&self) -> Option<&WindowSpec> {
        self.spec.as_ref()
    }

    pub fn fullscreen(mut self) -> AzureWindow {
        self.width = None;
        self.height = None;
        self
    }

    /// Chemin (relatif au repertoire de travail, comme un chemin de police -
    /// voir `ui::services::draw_ui::FONT_PATH`) d'un PNG a afficher comme
    /// icone d'application dans la barre d'en-tete auto-dessinee (voir
    /// `window::services::draw_header`). Silencieusement ignoree si le
    /// fichier est absent ou invalide (voir `azure_engine::codec::png`
    /// pour le perimetre supporte, un message est tout de meme ecrit sur
    /// stderr) - la barre garde alors juste son titre, sans icone.
    pub fn icon(mut self, path: &str) -> AzureWindow {
        self.icon_path = Some(path.to_string());
        self
    }

    /// Identifiant transmis au compositeur via `xdg_toplevel::set_app_id`
    /// (voir `azure_engine::platform::wayland::managers::xdg_manager::set_app_id`).
    /// C'est CET identifiant, pas `icon()` (qui ne dessine que dans la
    /// barre d'en-tete maison), que le bureau utilise pour choisir l'icone
    /// affichee dans la barre des taches/le dock, via un fichier .desktop
    /// dont `Icon=`/le nom correspond. Sans appel a cette methode, un
    /// identifiant derive du titre est utilise par defaut (voir
    /// `slugify_app_id`) - a definir explicitement pour qu'il corresponde
    /// a un vrai fichier .desktop installe si une icone personnalisee dans
    /// le dock est voulue.
    pub fn app_id(mut self, id: &str) -> AzureWindow {
        self.app_id = Some(id.to_string());
        self
    }

    /// L'arbre de widgets a dessiner a l'ouverture de la fenetre - typiquement
    /// celui construit par `compiler::services::interpreter::build_ui` a
    /// partir d'un .rsh style par rsC ou par l'ancien systeme `.style`.
    /// Interactif : un clic sur un `Button` bascule sa couleur, un clic sur
    /// une `TextArea` la focalise pour y taper, avec curseur clignotant et
    /// repetition d'une touche maintenue (voir `ui::services::interact`).
    /// Se dessine SOUS la barre d'en-tete (voir `window::models::header_bar::content_box`),
    /// jamais par-dessus.
    pub fn ui(mut self, nodes: Vec<UiNode>) -> AzureWindow {
        self.ui_nodes = nodes;
        self
    }

    /// Branche cette fenetre au routeur inter-app (voir `navigation`) :
    /// `client` doit venir de `navigation::managers::navigation_manager::connect`,
    /// deja enregistre sous l'`app_id` que d'autres apps utiliseront pour
    /// lui envoyer des `navigate(...)`. Sans appel a `routes` egalement, les
    /// messages recus sont ecoutes (voir `run`) mais n'ont aucun effet -
    /// aucun ecran a leur associer.
    pub fn navigation(mut self, client: NavigationClient) -> AzureWindow {
        self.nav = Some(client);
        self
    }

    /// Branche cette fenetre au routeur INTRA-app (voir
    /// `navigation::managers::intra_navigation_manager`, 100% en memoire,
    /// aucune socket) : `client` doit venir de
    /// `intra_navigation_manager::connect`. Permet a `WindowContext::goto`
    /// (voir `on_click`) de naviguer LOCALEMENT - entre les ecrans de cette
    /// meme fenetre, ou vers une autre vue enregistree aupres du meme
    /// `IntraRouter` partage dans ce process. Sans appel a `routes`
    /// egalement, les messages recus sont sondes (voir `run`) mais n'ont
    /// aucun effet - aucun ecran a leur associer.
    pub fn intra(mut self, client: IntraClient) -> AzureWindow {
        self.intra = Some(client);
        self
    }

    /// Les ecrans que cette fenetre accepte de servir - voir `RouteTable::on`.
    /// Partagee par les DEUX transports de routes (voir `navigation` et
    /// `intra`) : une route recue, qu'elle vienne d'une autre app via le
    /// routeur inter-app OU d'un `goto` intra-app, est resolue de la meme
    /// facon et remplace immediatement `ui_nodes` (voir `run`, le bras
    /// `on_tick`) - exactement comme si `.ui(nodes)` avait ete rappele.
    pub fn routes(mut self, table: RouteTable) -> AzureWindow {
        self.routes = Some(table);
        self
    }

    /// Les fenetres que cette fenetre accepte d'OUVRIR a la demande - voir
    /// `window_table::WindowTable::on`. Troisieme "expression routeur"
    /// (avec `routes`, resolue par `WindowContext::goto`/`navigate_to`) :
    /// celle-ci resout un `path` en une `AzureWindow` ENTIERE plutot qu'un
    /// arbre de `UiNode`, ouverte via `WindowContext::open_window` (voir
    /// `on_click`) dans son PROPRE thread - une fenetre-enfant n'a donc pas
    /// besoin d'etre branchee au meme `NavigationClient`/`IntraRouter` que
    /// celle qui l'ouvre, elle peut avoir les siens (ou aucun).
    pub fn windows(mut self, table: WindowTable) -> AzureWindow {
        self.windows = Some(table);
        self
    }

    /// Le stockage de l'app (voir `storage::models::stockage::Stockage`),
    /// disponible dans chaque `on_click` via `ctx.stockage()`. `Stockage`
    /// se clone : la meme connexion peut aussi servir aux routes.
    pub fn stockage(mut self, store: Stockage) -> AzureWindow {
        self.stockage = Some(store);
        self
    }

    /// Appele apres CHAQUE clic gauche qui bascule effectivement un
    /// `Button` de `ui_nodes` (voir `run`, le bras
    /// `WindowMouseButton(BTN_LEFT, true)`), avec un `WindowContext` en
    /// parametre - voir `window_context::WindowContext` pour la distinction
    /// entre `ctx.goto(path, payload)` (navigation INTRA-app, via
    /// `IntraRouter` - voir `intra`), `ctx.navigate_to(app_id, path, payload)`
    /// (navigation INTER-app, via azure-rooter - voir `navigation`) et
    /// `ctx.open_window(path, payload)` (ouverture d'une nouvelle fenetre -
    /// voir `windows`). Ne distingue pas QUEL bouton a ete clique si
    /// `ui_nodes` en contient plusieurs - suffisant tant qu'une fenetre n'en
    /// a qu'un seul avec un effet de navigation. `+ Send` : voir la
    /// documentation du champ `on_click` sur `AzureWindow`.
    pub fn on_click(mut self, handler: impl FnMut(&mut WindowContext) + Send + 'static) -> AzureWindow {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Appele une fois quand la fenetre se ferme (bouton, compositeur), avec
    /// les dernieres valeurs des champs (`ctx.value`) : de quoi enregistrer
    /// ce qui a ete tape depuis le dernier clic. `ctx.clicked` est `None`
    /// et les effets (`copy`, `flash`, `goto`...) n'ont plus lieu.
    pub fn on_close(mut self, handler: impl FnMut(&mut WindowContext) + Send + 'static) -> AzureWindow {
        self.on_close = Some(Box::new(handler));
        self
    }

    /// Appele quand un element `<draggable id="...">` est lache sur une
    /// `<dropzone id="...">` : `dropped.source`, `dropped.target` et
    /// `dropped.position` (le rang parmi les elements saisissables de la
    /// zone, l'element lache non compte). Meme contexte que `on_click`
    /// (`ctx.goto` pour redessiner la page...).
    pub fn on_drop(mut self, handler: impl FnMut(&mut WindowContext, &interact::Dropped) + Send + 'static) -> AzureWindow {
        self.on_drop = Some(Box::new(handler));
        self
    }

    /// Ecoute un flux d'une autre app (voir `crate::flux`) : a chaque tic,
    /// ce qui est arrive est applique a la copie locale de l'ecoute, puis
    /// `handler` est appele pour chaque evenement avec cette copie et un
    /// `WindowContext` - de quoi changer d'ecran (`ctx.goto_route`), ecrire
    /// dans le stockage, etc. Peut etre appele plusieurs fois (un flux par
    /// appel).
    pub fn flux(mut self, listener: Listener, handler: impl FnMut(&mut WindowContext, &FluxEvent, &FluxValue) + Send + 'static) -> AzureWindow {
        self.fluxes.push((listener, Box::new(handler)));
        self
    }

    /// Appele a chaque tic de la boucle (~60 par seconde), sans clic : de
    /// quoi montrer ce qu'un thread a part a fini (`ctx.goto` pour
    /// redessiner la page). Doit rester tres court.
    pub fn on_tick(mut self, handler: impl FnMut(&mut WindowContext) + Send + 'static) -> AzureWindow {
        self.on_tick = Some(Box::new(handler));
        self
    }

    /// L'etat de la boucle pour cette fenetre (vraie ou pilotee).
    pub(crate) fn en_etat(self, width: u32, height: u32, header: HeaderBar, decorated: bool) -> LoopState {
        // Demarre l'ecoute AVANT de construire `state` (voir
        // `navigation_manager::listen`) : elle ne bloque pas (un thread a
        // part), donc rien n'empeche de le faire ici plutot que dans la
        // boucle elle-meme.
        let incoming_routes = self.nav.as_ref().map(|nav| {
            navigation_manager::listen(nav).expect("Failed to start navigation listener")
        });
        let nav = self.nav;
        let intra = self.intra;
        let routes = self.routes;
        let windows = self.windows;
        let owner_app_id = self.spec.map(|spec| spec.owner_app_id());
        let on_click = self.on_click;
        let on_close = self.on_close;
        let on_drop = self.on_drop;
        let stockage = self.stockage;
        let fluxes = self.fluxes;
        let on_tick = self.on_tick;

        LoopState {
            event: EventState::new(self.ui_nodes),
            flashes: Vec::new(),
            keep_scroll: false,
            activate: None,
            perf: Perf::new(),
            canvas: Canvas::new(width, height),
            dirty: false,
            last_present: Instant::now(),
            header,
            header_hover: None,
            fullscreen: false,
            decorated,
            routes,
            incoming_routes,
            nav,
            intra,
            windows,
            owner_app_id,
            stockage,
            on_click,
            on_close,
            on_drop,
            fluxes,
            on_tick,
            inspector: Default::default(),
            selecteur: None,
        }
    }

    pub fn run(self) {
        // Pilotee par un essai (voir `crate::essai`) : sans fenetre.
        if let Some(socket) = std::env::var_os("AZURE_PILOTE") {
            crate::window::models::pilote::piloter(self, std::path::PathBuf::from(socket));
            return;
        }
        let (width, height) = if let Some(spec) = &self.spec {
            (spec.size().width(), spec.size().height())
        } else if self.width.is_none() && self.height.is_none() {
            let screen = get_screen_resolution().unwrap_or(ScreenOutput::new(1920, 1080));
            (screen.width as u32, screen.height as u32)
        } else {
            (self.width.unwrap_or(1920), self.height.unwrap_or(1080))
        };
        let mut window = window_create(width as i32, height as i32).expect("Failed to create window");
        let xdg_surface_id = window.xdg_surface_id();
        let xdg_toplevel_id = window.xdg_toplevel_id();
        let surface_id = window.surface_id();
        let xdg_wm_id = window.xdg_wm_id();

        // Detectee une seule fois au demarrage (interroge l'OS via
        // `localectl`, pas quelque chose a refaire a chaque frappe) - voir
        // `interact::detect_keyboard_layout`.
        // Remplacee par la vraie carte du clavier des que le compositeur
        // l'envoie (voir `refresh_keyboard`).
        let layout = std::cell::Cell::new(interact::detect_keyboard_layout());

        // Decodee une seule fois au demarrage (voir `services::image::load_png`
        // cote azure_engine pour son propre cache par chemin, redondant
        // mais inoffensif ici puisqu'on ne la recharge jamais nous-memes).
        let icon = self.icon_path.as_deref().and_then(|path| match load_image(path) {
            Ok(decoded) => Some(decoded),
            Err(err) => {
                eprintln!("AzureWindow: impossible de charger l'icone '{path}': {err}");
                None
            }
        });
        // Theme sombre/clair + disposition des boutons du bureau (voir
        // `system_theme::detect`) - interroge, comme `layout` ci-dessus,
        // une seule fois au demarrage, pas suivi en cours de session.
        let theme = system_theme::detect();
        // Boutons toujours a la mac (fermer, reduire, agrandir a gauche) :
        // l'identite d'Azure, quel que soit le reglage du bureau.
        let header = HeaderBar::new(self.title.clone(), icon, header_bar::ButtonLayout::mac(), theme.dark);
        let decorated = window.is_server_side_decorated();

        let title = self.title.clone();
        let app_id = self.app_id.clone().unwrap_or_else(|| slugify_app_id(&self.title));
        let state = RefCell::new(self.en_etat(width, height, header, decorated));

        // Icones de curseur generees puis envoyees une seule fois au
        // compositeur (voir `cursor`) - une erreur ici n'empeche pas la
        // fenetre de fonctionner, elle garde juste le curseur du systeme.
        let cursors = CursorSet::new(&mut window)
            .map_err(|err| eprintln!("AzureWindow: curseurs indisponibles: {err}"))
            .ok();

        {
            let mut hote = WaylandHote { win: &mut window, toplevel: xdg_toplevel_id, surface: surface_id, curseurs: cursors.as_ref() };
            redraw_and_present(&mut hote, &mut state.borrow_mut());
        }
        state.borrow_mut().last_present = Instant::now();

        let toplevel_id = window.xdg_toplevel_id();
        set_title(window.connection_mut(), toplevel_id, &title).expect("Failed to set title");
        set_app_id(window.connection_mut(), toplevel_id, &app_id).expect("Failed to set app id");

        run_event_loop_interactive(
            &mut window,
            xdg_surface_id,
            xdg_toplevel_id,
            surface_id,
            xdg_wm_id,
            TICK_MS,
            |win, event| {
                let mut hote = WaylandHote { win, toplevel: xdg_toplevel_id, surface: surface_id, curseurs: cursors.as_ref() };
                sur_evenement(&mut state.borrow_mut(), &mut hote, event, &layout)
            },
            |win| {
                let mut hote = WaylandHote { win, toplevel: xdg_toplevel_id, surface: surface_id, curseurs: cursors.as_ref() };
                sur_tic(&mut state.borrow_mut(), &mut hote, &layout)
            },
        )
        .expect("Event loop failed");

        // Libere proprement l'`app_id` aupres du routeur (voir
        // `navigation_manager::disconnect`) plutot que de laisser le
        // routeur decouvrir la fermeture au prochain echec d'ecriture d'une
        // AUTRE app vers cet `app_id` - un cas qui, cote azure-rooter,
        // tue silencieusement le thread de l'EXPEDITEUR plutot que de
        // nettoyer le destinataire mort. Sans effet si cette fenetre n'a
        // jamais ete branchee au routeur (voir `navigation`) - l'erreur
        // d'un `disconnect` qui echoue (connexion deja coupee) est
        // volontairement ignoree, il n'y a rien de plus a faire a ce stade.
        let mut s = state.into_inner();
        terminer(&mut s);

        // `window.mapped_size()`, pas `width * height * 4` : un
        // redimensionnement en cours de session peut avoir agrandi la
        // memoire partagee sous-jacente au-dela de la taille de creation
        // (voir `Window::resize`, qui ne la retrecit jamais) - demapper
        // moins que ce qui est reellement mappe serait une fuite.
        unmap_memory(window.ptr(), window.mapped_size())
            .expect("Failed to unmap memory");
    }
}
