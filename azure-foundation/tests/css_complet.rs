// Les proprietes rsC qui etaient reconnues sans effet (PROBLEMES #13, #20,
// #26) : border-style, font-style, letter-spacing, text-decoration,
// visibility, cursor, transition, overflow-x, position relative/absolute,
// align-content, ombre interieure, coins arrondis qui decoupent le contenu,
// barre de defilement saisie a la souris, couleur de la barre
// (scrollbar-color). Chaque test passe par rsH + rsC,
// comme une app.
use azure_core::rules::window_event::WindowEvent;
use azure_engine::rendering::models::canvas::Canvas;
use azure_engine::rendering::models::paint::BorderStyle;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::cursor::models::cursor_kind::CursorKind;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::BTN_LEFT;
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::layout::managers::layout_manager::{container_layout, Rect};
use azure_foundation::layout::managers::web_layout::layout_roots;
use azure_foundation::ui::models::transition::take_pending;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::interact::{self, animate_scroll, button_id_at, hover_kind_at, scroll_x_at, HoverKind, KeyboardLayout};
use std::time::Duration;

const VIEW: (u32, u32, u32, u32) = (0, 0, 800, 600);

fn build(rsh: &str, rsc: &str) -> Vec<UiNode> {
    let ast = parse_rsh(tokenize_rsh(rsh)).unwrap();
    let sheet = parse_rsc(tokenize_rsc(rsc)).unwrap();
    build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &Context::new())
}

fn box_of(nodes: &[UiNode], id: &str) -> Rect {
    let mut found = None;
    interact::walk(nodes, VIEW, &mut |n, b| {
        let nid = match n {
            UiNode::Button(x) => &x.id,
            UiNode::TextArea(x) => &x.id,
            _ => return,
        };
        if nid == id {
            found = Some(b);
        }
    });
    found.unwrap_or_else(|| panic!("{id} introuvable"))
}

fn root(nodes: &mut [UiNode]) -> &mut azure_foundation::ui::models::container::Container {
    match &mut nodes[0] {
        UiNode::Container(c) => c,
        _ => panic!("un conteneur attendu"),
    }
}

fn px(c: &Canvas, x: i32, y: i32) -> (u8, u8, u8) {
    let i = ((y as u32 * c.width + x as u32) * 4) as usize;
    (c.buffer[i + 2], c.buffer[i + 1], c.buffer[i])
}

#[test]
fn every_property_reaches_the_widgets() {
    let mut nodes = build(
        "<container.page><button.b#b>Payer<!button><text.t>Bonjour<!text><!container>",
        ".page { cursor: move; overflow-x: auto; }
         .b { border: 2px dashed #ff0000; transition: background-color 200ms ease-in 50ms; box-shadow: inset 0 2px 4px #000000; visibility: hidden; }
         .t { font-style: italic; letter-spacing: 2px; text-decoration: underline line-through; }",
    );
    let page = root(&mut nodes);
    assert!(page.layout.scrollable_x(), "overflow-x");
    assert_eq!(page.decoration.cursor, Some(CursorKind::Move));
    let UiNode::Button(b) = &page.children[0] else { panic!() };
    assert_eq!(b.decoration.border_style, BorderStyle::Dashed);
    let t = b.decoration.transition.expect("transition");
    assert!((t.duration - 0.2).abs() < 1e-4 && (t.delay - 0.05).abs() < 1e-4);
    assert!(b.decoration.shadow.expect("ombre").inset, "box-shadow inset");
    assert!(!b.decoration.visible, "visibility: hidden");
    assert_eq!(b.decoration.cursor, Some(CursorKind::Move), "cursor se transmet aux enfants");
    let UiNode::Label(label) = &page.children[1] else { panic!() };
    let style = label.text_style.as_ref().unwrap();
    assert!(style.options.italic);
    assert_eq!(style.options.letter_spacing, 2.0);
    assert!(style.decoration.underline && style.decoration.line_through && !style.decoration.overline);
}

#[test]
fn hidden_elements_keep_their_place_but_are_neither_drawn_nor_clickable() {
    let rsc = ".page { background: #000000; } .b { background: #ff0000; width: 120px; height: 40px; } .cache { visibility: hidden; }";
    let visible = build("<container.page><button.b#un>A<!button><button.b#deux>B<!button><!container>", rsc);
    let hidden = build("<container.page><button.b.cache#un>A<!button><button.b#deux>B<!button><!container>", rsc);
    let (un, deux) = (box_of(&hidden, "un"), box_of(&hidden, "deux"));
    assert_eq!(deux, box_of(&visible, "deux"), "la place reste prise");
    assert_eq!(button_id_at(&hidden, un.0 + 10, un.1 + 10, VIEW), None, "pas cliquable");
    assert_eq!(button_id_at(&hidden, deux.0 + 10, deux.1 + 10, VIEW).as_deref(), Some("deux"));
    let mut canvas = Canvas::new(800, 600);
    draw_ui(&hidden, VIEW, &mut canvas, -1, -1, false);
    assert_eq!(px(&canvas, un.0 + 3, un.1 + 3), (0, 0, 0), "pas dessine");
    assert_eq!(px(&canvas, deux.0 + 3, deux.1 + 3), (255, 0, 0));
}

#[test]
fn absolute_and_relative_positioning() {
    let nodes = build(
        "<container.page><button.shift#s>Decale<!button><button.badge#badge>3<!button><!container>",
        ".page { width: 400px; height: 300px; padding: 10px; }
         .shift { position: relative; left: 30px; top: 10px; width: 200px; height: 40px; }
         .badge { position: absolute; top: 25px; left: 60px; width: 50px; height: 20px; }",
    );
    let page = layout_roots(&nodes, VIEW)[0];
    let shift = box_of(&nodes, "s");
    assert_eq!((shift.0, shift.1), (page.0 + 10 + 30, page.1 + 10 + 10), "relative : decale depuis sa place");
    let badge = box_of(&nodes, "badge");
    assert_eq!((badge.0, badge.1, badge.2, badge.3), (page.0 + 60, page.1 + 25, 50, 20), "absolute : dans la boite du conteneur");
    // Les deux se chevauchent : l'element positionne est au-dessus.
    assert_eq!(button_id_at(&nodes, badge.0 + 5, badge.1 + 5, VIEW).as_deref(), Some("badge"));
    assert_eq!(button_id_at(&nodes, shift.0 + 5, shift.1 + 5, VIEW).as_deref(), Some("s"));
}

#[test]
fn align_content_places_wrapped_lines() {
    let place = |value: &str| {
        let nodes = build(
            "<container.row><button.i#a>A<!button><button.i#b>B<!button><!container>",
            &format!(".row {{ display: flex; flex-wrap: wrap; width: 200px; height: 300px; align-content: {value}; }} .i {{ width: 120px; height: 50px; }}"),
        );
        let top = layout_roots(&nodes, VIEW)[0].1;
        (box_of(&nodes, "a").1 - top, box_of(&nodes, "b").1 - top)
    };
    assert_eq!(place("flex-start"), (0, 50));
    assert_eq!(place("center"), (100, 150));
    assert_eq!(place("flex-end"), (200, 250));
    assert_eq!(place("space-between"), (0, 250));
}

#[test]
fn overflow_x_scrolls_sideways() {
    let mut nodes = build(
        "<container.strip><button.card#c1>1<!button><button.card#c2>2<!button><button.card#c3>3<!button><!container>",
        ".strip { display: flex; width: 300px; height: 80px; overflow-x: auto; } .card { width: 200px; height: 60px; flex-shrink: 0; }",
    );
    let own = layout_roots(&nodes, VIEW)[0];
    let layout = container_layout(root(&mut nodes), own);
    assert_eq!((layout.content_width, layout.max_scroll_x), (600, 300));
    let before = box_of(&nodes, "c1").0;
    assert!(scroll_x_at(&mut nodes, own.0 + 50, own.1 + 20, 120.0, VIEW));
    while animate_scroll(&mut nodes) {}
    assert_eq!(root(&mut nodes).scroll_x, 120);
    assert_eq!(box_of(&nodes, "c1").0, before - 120);
    // Maj + molette, et molette horizontale : meme effet, par la fenetre.
    let mut state = EventState::new(nodes);
    handle_event(&mut state, WindowEvent::WindowMouseMove(own.0 + 50, own.1 + 20), KeyboardLayout::Qwerty, VIEW);
    handle_event(&mut state, WindowEvent::WindowScrollH(1000.0), KeyboardLayout::Qwerty, VIEW);
    while animate_scroll(&mut state.ui_nodes) {}
    assert_eq!(root(&mut state.ui_nodes).scroll_x, 300, "borne a l'etendue");
    handle_event(&mut state, WindowEvent::WindowKeyPress(42, true), KeyboardLayout::Qwerty, VIEW);
    handle_event(&mut state, WindowEvent::WindowScroll(-100.0), KeyboardLayout::Qwerty, VIEW);
    while animate_scroll(&mut state.ui_nodes) {}
    assert_eq!(root(&mut state.ui_nodes).scroll_x, 200, "Maj + molette");
}

#[test]
fn the_scrollbar_can_be_dragged() {
    let items: String = (0..20).map(|i| format!("<button.item#i{i}>{i}<!button>")).collect();
    let nodes = build(&format!("<container.list>{items}<!container>"), ".list { width: 300px; height: 200px; overflow-y: auto; } .item { height: 50px; width: 100px; }");
    let own = layout_roots(&nodes, VIEW)[0];
    let mut state = EventState::new(nodes);
    let layout = container_layout(root(&mut state.ui_nodes), own);
    assert_eq!(layout.max_scroll, 1000 - 200);
    // Poignee en haut a droite, au repos.
    let (x, y) = (own.0 + own.2 as i32 - 5, own.1 + 10);
    let ev = |s: &mut EventState, e| handle_event(s, e, KeyboardLayout::Qwerty, VIEW);
    ev(&mut state, WindowEvent::WindowMouseMove(x, y));
    assert!(ev(&mut state, WindowEvent::WindowMouseButton(BTN_LEFT, true)), "poignee saisie");
    assert!(state.scroll_drag.is_some());
    // Poignee de 38px sur une piste de 194px : 156px de course pour 800px.
    ev(&mut state, WindowEvent::WindowMouseMove(x, y + 78));
    let offset = root(&mut state.ui_nodes).scroll_offset;
    assert_eq!(offset, 400, "a mi-course");
    ev(&mut state, WindowEvent::WindowMouseMove(x, y + 500));
    assert_eq!(root(&mut state.ui_nodes).scroll_offset, 800, "en butee");
    ev(&mut state, WindowEvent::WindowMouseButton(BTN_LEFT, false));
    assert!(state.scroll_drag.is_none());
}

#[test]
fn scrollbar_color_paints_the_thumb() {
    let items: String = (0..20).map(|i| format!("<button.item#i{i}>{i}<!button>")).collect();
    let draw = |rsc: &str| {
        let nodes = build(&format!("<container.list>{items}<!container>"), &format!(".list {{ width: 300px; height: 200px; overflow-y: auto; background-color: #000000; }} .item {{ height: 50px; width: 100px; }} {rsc}"));
        let mut canvas = Canvas::new(800, 600);
        draw_ui(&nodes, VIEW, &mut canvas, -1, -1, false);
        // Milieu de la poignee (6px de large, 3px du bord droit).
        px(&canvas, 300 - 6, 20)
    };
    assert_eq!(draw(""), (78, 74, 68), "couleur par defaut");
    assert_eq!(draw(".list { scrollbar-color: #505050 #111111; }"), (80, 80, 80));
    // Heritee, comme en CSS.
    let nodes = build("<container.app><container.list><!container><!container>", ".app { scrollbar-color: #505050; } .list { overflow-y: auto; }");
    let UiNode::Container(app) = &nodes[0] else { panic!() };
    assert_eq!(app.children[0].decoration().scrollbar_color, Some(azure_engine::rendering::models::color::Color::new(80, 80, 80, 255)));
}

#[test]
fn rounded_corners_clip_the_content() {
    let nodes = build(
        "<container.card><container.fill><!container><!container>",
        ".card { width: 200px; height: 120px; border-radius: 30px; background: #0000ff; } .fill { height: 120px; background: #ff0000; }",
    );
    let own = layout_roots(&nodes, VIEW)[0];
    let mut canvas = Canvas::new(800, 600);
    draw_ui(&nodes, VIEW, &mut canvas, -1, -1, false);
    assert_eq!(px(&canvas, own.0 + 100, own.1 + 60), (255, 0, 0), "le contenu au centre");
    assert_eq!(px(&canvas, own.0 + 1, own.1 + 1), (0, 0, 0), "rien dans l'arrondi");
    assert_eq!(px(&canvas, own.0 + 198, own.1 + 118), (0, 0, 0), "coin bas droit aussi");
    assert_eq!(px(&canvas, own.0 + 30, own.1 + 1), (255, 0, 0), "le bord droit du haut reste plein");
}

#[test]
fn transitions_progress_over_frames() {
    let nodes = build(
        "<container.page><button.b#b>Ok<!button><!container>",
        ".b { width: 160px; height: 60px; background: #202020; transition: background-color 200ms linear; } .b:hover { background: #e0e0e0; }",
    );
    let b = box_of(&nodes, "b");
    let (mx, my) = (b.0 + 80, b.1 + 30);
    let sample = |nodes: &[UiNode]| {
        let mut canvas = Canvas::new(800, 600);
        draw_ui(nodes, VIEW, &mut canvas, mx, my, false);
        px(&canvas, b.0 + 4, b.1 + 4).0
    };
    let _ = take_pending();
    assert_eq!(sample(&nodes), 0x20, "premiere image : pas encore bouge");
    std::thread::sleep(Duration::from_millis(90));
    let mid = sample(&nodes);
    assert!(mid > 0x40 && mid < 0xc0, "a mi-chemin : {mid:#x}");
    assert!(take_pending(), "la fenetre doit redessiner");
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(sample(&nodes), 0xe0, "arrivee");
    let _ = take_pending();
    assert_eq!(sample(&nodes), 0xe0);
    assert!(!take_pending(), "fini : plus de redessin demande");
}

#[test]
fn css_cursor_overrides_the_default() {
    let nodes = build(
        "<container.zone><button.b#b>Interdit<!button><button#c>Normal<!button><!container>",
        ".zone { cursor: move; height: 300px; } .b { cursor: not-allowed; }",
    );
    let zone = layout_roots(&nodes, VIEW)[0];
    let (b, c) = (box_of(&nodes, "b"), box_of(&nodes, "c"));
    assert_eq!(hover_kind_at(&nodes, b.0 + 3, b.1 + 3, VIEW), HoverKind::Styled(CursorKind::NotAllowed));
    // Le bouton sans regle herite `move` de son conteneur.
    assert_eq!(hover_kind_at(&nodes, c.0 + 3, c.1 + 3, VIEW), HoverKind::Styled(CursorKind::Move));
    assert_eq!(hover_kind_at(&nodes, zone.0 + 5, zone.1 + 280, VIEW), HoverKind::Styled(CursorKind::Move));
    assert_eq!(CursorKind::from_hover(HoverKind::Styled(CursorKind::Move)), CursorKind::Move);
}

#[test]
fn anchor_links_scroll_to_their_section() {
    let sections: String = (1..=6).map(|i| format!("<container.section#s{i}><text>Section {i}<!text><!container>")).collect();
    let nodes = build(
        &format!("<container.page><container.toc><ancre vers=\"s5\">Aller a 5<!ancre><!container><container.article>{sections}<!container><!container>"),
        ".page { display: flex; height: 400px; } .toc { width: 150px; } .article { flex-grow: 1; overflow-y: auto; } .section { height: 300px; }",
    );
    let mut state = EventState::new(nodes);
    let link = box_of(&state.ui_nodes, "ancre-s5");
    let ev = |s: &mut EventState, e| handle_event(s, e, KeyboardLayout::Qwerty, VIEW);
    ev(&mut state, WindowEvent::WindowMouseMove(link.0 + 5, link.1 + 5));
    assert!(ev(&mut state, WindowEvent::WindowMouseButton(BTN_LEFT, true)));
    ev(&mut state, WindowEvent::WindowMouseButton(BTN_LEFT, false));
    while animate_scroll(&mut state.ui_nodes) {}
    // La section 5 commence a 4 x 300 = 1200px : en haut de l'article, 8px d'air.
    let UiNode::Container(page) = &state.ui_nodes[0] else { panic!() };
    let UiNode::Container(article) = &page.children[1] else { panic!() };
    assert_eq!(article.scroll_offset, 1200 - 8);

    // Au clavier aussi : Tab jusqu'au lien, puis Entree (retour en haut via s1).
    let nodes = build(
        &format!("<container.page><container.toc><ancre vers=\"s1\">Haut<!ancre><!container><container.article>{sections}<!container><!container>"),
        ".page { display: flex; height: 400px; } .toc { width: 150px; } .article { flex-grow: 1; overflow-y: auto; } .section { height: 300px; }",
    );
    let mut state = EventState::new(nodes);
    {
        let UiNode::Container(page) = &mut state.ui_nodes[0] else { panic!() };
        let UiNode::Container(article) = &mut page.children[1] else { panic!() };
        article.scroll_offset = 900;
        article.scroll_target = 900;
    }
    ev(&mut state, WindowEvent::WindowKeyPress(15, true));
    ev(&mut state, WindowEvent::WindowKeyPress(15, false));
    ev(&mut state, WindowEvent::WindowKeyPress(28, true));
    ev(&mut state, WindowEvent::WindowKeyPress(28, false));
    while animate_scroll(&mut state.ui_nodes) {}
    let UiNode::Container(page) = &state.ui_nodes[0] else { panic!() };
    let UiNode::Container(article) = &page.children[1] else { panic!() };
    assert_eq!(article.scroll_offset, 0);
    assert!(!interact::scroll_to_anchor(&mut state.ui_nodes, "inconnue", VIEW));
}

#[test]
fn background_images_cover_contain_and_position() {
    let png = concat!(env!("CARGO_MANIFEST_DIR"), "/../azure-engine/src/codec/fixtures/rgb_4x4.png");
    let img = azure_engine::rendering::managers::renderer::load_image(png).unwrap();
    let src = |x: u32, y: u32| {
        let i = ((y * img.width + x) * 4) as usize;
        (img.pixels[i], img.pixels[i + 1], img.pixels[i + 2])
    };
    let render = |rule: &str| {
        let nodes = build("<container.fond><!container>", &format!(".fond {{ width: 100px; height: 60px; background: #00ff00; background-image: url({png}); {rule} }}"));
        let own = layout_roots(&nodes, VIEW)[0];
        let mut canvas = Canvas::new(800, 600);
        draw_ui(&nodes, VIEW, &mut canvas, -1, -1, false);
        (own, canvas)
    };
    // cover + center : 100x100, decale de 20px vers le haut.
    let (own, c) = render("background-size: cover; background-position: center;");
    assert_eq!(px(&c, own.0 + 50, own.1 + 30), src(2, 2));
    assert_eq!(px(&c, own.0 + 99, own.1 + 59), src(3, 3));
    assert_eq!(px(&c, own.0 + 150, own.1 + 30), (0, 0, 0), "rien hors de la boite");
    // contain + center : 60x60 au milieu, bandes vertes a gauche et a droite.
    let (own, c) = render("background-size: contain; background-position: center;");
    assert_eq!(px(&c, own.0 + 5, own.1 + 30), (0, 255, 0), "bande de la couleur de fond");
    assert_eq!(px(&c, own.0 + 21, own.1 + 1), src(0, 0));
    // Taille naturelle en haut a gauche (defauts CSS).
    let (own, c) = render("");
    assert_eq!(px(&c, own.0 + 1, own.1 + 1), src(1, 1));
    assert_eq!(px(&c, own.0 + 10, own.1 + 10), (0, 255, 0));
}

#[test]
fn an_unreadable_declaration_does_not_break_the_sheet() {
    let sheet = parse_rsc(tokenize_rsc(".a { color: #ff0000; width: calc(100% - (; } .b { height: 30px; background: url(img/fond.png) center / cover; }")).unwrap();
    assert_eq!(sheet.rules.len(), 2, "les deux regles sont gardees");
    assert!(sheet.rules[0].declarations.iter().any(|d| d.name == "color"));
    assert!(sheet.rules[1].declarations.iter().any(|d| d.name == "height"));
    let warnings = azure_foundation::compiler::rsc::warnings(&sheet);
    assert!(warnings.iter().any(|w| w.contains("declaration ignoree")), "{warnings:?}");
}
