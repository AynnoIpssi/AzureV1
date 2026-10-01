// Texte riche (`<richtext>`) : styles tenus a jour a chaque frappe,
// annulation, barre d'outils, valeur lue par l'app, dessin et clic.
use azure_core::rules::window_event::WindowEvent;
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::components::with_default_styles;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::BTN_LEFT;
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::layout::models::layout_props::LayoutProps;
use azure_foundation::ui::models::rich::{self, Mark, RichStyle};
use azure_foundation::ui::models::textarea::TextArea;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::form::form_values;
use azure_foundation::ui::services::interact::{self, KeyInput, KeyboardLayout};
use azure_engine::rendering::models::color::Color;

fn zone(value: &str) -> TextArea {
    TextArea::rich(LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0), Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), value)
}

fn bold(area: &TextArea) -> Vec<bool> {
    (0..area.char_count()).map(|i| area.style_at(i).bold).collect()
}

#[test]
fn marques_frappe_et_annulation() {
    let mut a = zone("abcd");
    a.cursor = 1;
    a.selection_anchor = Some(3);
    assert!(a.toggle_mark(&Mark::Bold));
    assert_eq!(bold(&a), vec![false, true, true, false]);
    // Une 2e fois : tout est deja gras, on l'enleve.
    a.toggle_mark(&Mark::Bold);
    assert_eq!(bold(&a), vec![false; 4]);
    a.undo();
    assert_eq!(bold(&a), vec![false, true, true, false]);

    // Taper apres un caractere gras continue en gras.
    a.selection_anchor = None;
    a.cursor = 3;
    a.insert_char('X');
    assert_eq!(a.text, "abcXd");
    assert_eq!(bold(&a), vec![false, true, true, true, false]);
    a.backspace();
    a.move_left(false);
    a.delete_forward();
    assert_eq!(a.text, "abd");
    assert_eq!(bold(&a), vec![false, true, false]);

    // Sans selection : le style vaut pour la frappe suivante seulement.
    a.move_end(false);
    a.toggle_mark(&Mark::Italic);
    a.insert_str("yz");
    assert!(a.style_at(3).italic && a.style_at(4).italic && !a.style_at(2).italic);
    a.toggle_mark(&Mark::Code);
    a.move_left(false);
    a.insert_char('!');
    assert!(!a.style_at(4).code, "le curseur a bouge : style oublie");
}

#[test]
fn valeur_au_format_d_echange() {
    let mut a = zone("Un mot");
    a.cursor = 3;
    a.selection_anchor = Some(6);
    a.toggle_mark(&Mark::parse("couleur-e06c75").unwrap());
    a.toggle_mark(&Mark::Underline);
    let value = a.rich_value().unwrap();
    let spans = rich::parse(&value);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[1].text, "mot");
    assert_eq!(spans[1].style, RichStyle { underline: true, color: rich::parse_color("#e06c75"), ..Default::default() });
    // Relu tel quel.
    assert_eq!(zone(&value).rich_value().unwrap(), value);
    // Sans style : le texte seul.
    assert_eq!(zone("simple").rich_value().unwrap(), "simple");
}

const ZONE: (u32, u32, u32, u32) = (0, 0, 600, 300);

fn page(value: &str) -> EventState {
    let rsc = with_default_styles("richtext { width: 560px; height: 200px; }", None);
    let sheet = parse_rsc(tokenize_rsc(&rsc)).unwrap();
    let rsh = format!("<richbar pour=\"corps\"><!richbar><richtext#corps valeur=\"{value}\"><!richtext>");
    EventState::new(build_ui(&parse_rsh(tokenize_rsh(&rsh)).unwrap(), &StyleSource::Rsc(&sheet)))
}

fn area(s: &EventState) -> &TextArea {
    fn find(nodes: &[UiNode]) -> Option<&TextArea> {
        nodes.iter().find_map(|n| match n {
            UiNode::TextArea(a) => Some(a),
            UiNode::Container(c) => find(&c.children),
            _ => None,
        })
    }
    find(&s.ui_nodes).expect("richtext")
}

fn boxes(s: &EventState) -> Vec<(String, (i32, i32, u32, u32))> {
    let mut out = Vec::new();
    interact::walk(&s.ui_nodes, ZONE, &mut |n, b| match n {
        UiNode::Button(btn) => out.push((btn.id.clone(), b)),
        UiNode::TextArea(a) => out.push((format!("zone:{}", a.id), b)),
        _ => {}
    });
    out
}

fn click(s: &mut EventState, x: i32, y: i32) {
    handle_event(s, WindowEvent::WindowMouseMove(x, y), KeyboardLayout::Qwerty, ZONE);
    handle_event(s, WindowEvent::WindowMouseButton(BTN_LEFT, true), KeyboardLayout::Qwerty, ZONE);
    handle_event(s, WindowEvent::WindowMouseButton(BTN_LEFT, false), KeyboardLayout::Qwerty, ZONE);
}

#[test]
fn barre_d_outils_valeur_dessin_et_clic() {
    let mut s = page("Bonjour tout le monde");
    let b = boxes(&s);
    let at = |id: &str| b.iter().find(|(i, _)| i == id).unwrap_or_else(|| panic!("{id} absent : {b:?}")).1;
    let texte = at("zone:corps");
    // Clic dans le texte : focus ; puis tout selectionner au clavier.
    click(&mut s, texte.0 + 20, texte.1 + 12);
    assert!(area(&s).focused);
    let mut clip = String::new();
    interact::type_into_focused(&mut s.ui_nodes, KeyInput::SelectAll, &mut clip);
    // Le bouton Gras de la barre : la zone garde le focus et sa selection.
    let g = at("rt-corps-gras");
    click(&mut s, g.0 + g.2 as i32 / 2, g.1 + g.3 as i32 / 2);
    assert!(area(&s).focused);
    assert!(bold(area(&s)).iter().all(|b| *b));
    // Ctrl+I au clavier (meme chemin que la frappe).
    interact::type_into_focused(&mut s.ui_nodes, KeyInput::Format('i'), &mut clip);
    assert!(area(&s).style_at(0).italic);
    // L'app lit la valeur au format d'echange.
    let value = form_values(&s.ui_nodes).get("corps").unwrap().as_text();
    assert_eq!(rich::parse(&value)[0].style, RichStyle { bold: true, italic: true, ..Default::default() });

    // Le texte gras se dessine plus large : un clic en fin de ligne tombe
    // en fin de texte (positions du texte riche, pas du texte simple).
    let mut canvas = Canvas::new(600, 300);
    draw_ui(&s.ui_nodes, ZONE, &mut canvas, -1, -1, true);
    let positions = azure_foundation::ui::services::rich_layout::positions(&area(&s).text, &area(&s).rich.as_ref().unwrap().styles, 16.0, 400.0);
    let end_x = texte.0 + 6 + *positions.last().unwrap() as i32;
    click(&mut s, end_x - 1, texte.1 + 12);
    assert_eq!(area(&s).cursor, area(&s).char_count());
}

// Apercu a regarder : target/tmp/texte_riche.ppm.
#[test]
fn apercu() {
    let f = |m: &str, c: &str, t: &str| format!("{m}\u{1f}{c}\u{1f}\u{1f}{t}");
    let value = [
        f("", "", "Texte normal, "),
        f("g", "", "gras"),
        f("", "", ", "),
        f("i", "", "italique"),
        f("", "", ", "),
        f("s", "", "souligné"),
        f("", "", ", "),
        f("b", "", "barré"),
        f("", "", ", "),
        f("c", "", "du_code()"),
        f("", "", " et des "),
        f("g", "#e06c75", "couleurs"),
        f("", "", " "),
        f("", "#98c379", "variées"),
        f("", "", ". Une ligne assez longue pour revenir à la ligne toute seule dans la zone.\nEt un vrai retour."),
    ]
    .join("\u{1e}");
    let mut s = page(&value.replace('"', ""));
    let b = boxes(&s);
    let texte = b.iter().find(|(i, _)| i == "zone:corps").unwrap().1;
    click(&mut s, texte.0 + 30, texte.1 + 12);
    let mut canvas = Canvas::new(600, 300);
    azure_engine::rendering::managers::renderer::draw_rect(0, 0, 600, 300, &Color::new(24, 22, 20, 255), &mut canvas);
    draw_ui(&s.ui_nodes, ZONE, &mut canvas, -1, -1, true);
    let mut ppm = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("texte_riche.ppm"), ppm).unwrap();
}

#[test]
fn fleches_haut_bas() {
    let mut s = page("ligne une\nligne deux\nfin");
    let texte = boxes(&s).into_iter().find(|(i, _)| i == "zone:corps").unwrap().1;
    click(&mut s, texte.0 + 8, texte.1 + 10);
    assert_eq!(area(&s).cursor, 0);
    interact::move_vertical(&mut s.ui_nodes, true, false, ZONE);
    assert_eq!(area(&s).cursor, 10);
    interact::move_vertical(&mut s.ui_nodes, true, true, ZONE);
    assert_eq!((area(&s).cursor, area(&s).selection_anchor), (21, Some(10)));
    interact::move_vertical(&mut s.ui_nodes, false, false, ZONE);
    assert_eq!(area(&s).cursor, 10);
}

#[test]
fn hauteur_suit_le_texte() {
    let rsc = with_default_styles("richtext { width: 300px; min-height: 0px; padding: 0px; }", None);
    let sheet = parse_rsc(tokenize_rsc(&rsc)).unwrap();
    let mut s = EventState::new(build_ui(&parse_rsh(tokenize_rsh("<richtext#r valeur=\"une ligne\"><!richtext>")).unwrap(), &StyleSource::Rsc(&sheet)));
    let h = |s: &EventState| boxes(s).into_iter().find(|(i, _)| i == "zone:r").unwrap().1 .3;
    let une = h(&s);
    click(&mut s, 10, 10);
    for _ in 0..3 {
        handle_event(&mut s, WindowEvent::WindowKeyPress(28, true), KeyboardLayout::Qwerty, ZONE);
        handle_event(&mut s, WindowEvent::WindowKeyPress(28, false), KeyboardLayout::Qwerty, ZONE);
    }
    assert_eq!(area(&s).text.matches('\n').count(), 3);
    assert_eq!(h(&s), une * 4 - 12 * 3, "4 lignes (padding 6+6 compte une fois)");
}

#[test]
fn barre_sans_pour_agit_sur_la_zone_en_cours() {
    let rsc = with_default_styles("richtext { width: 500px; height: 60px; }", None);
    let sheet = parse_rsc(tokenize_rsc(&rsc)).unwrap();
    let rsh = "<richbar><!richbar><richtext#a valeur=\"premier\"><!richtext><richtext#b valeur=\"second\"><!richtext>";
    let mut s = EventState::new(build_ui(&parse_rsh(tokenize_rsh(rsh)).unwrap(), &StyleSource::Rsc(&sheet)));
    let b = boxes(&s);
    let at = |id: &str| b.iter().find(|(i, _)| i == id).unwrap_or_else(|| panic!("{id} : {b:?}")).1;
    let zone_b = at("zone:b");
    click(&mut s, zone_b.0 + 10, zone_b.1 + 10);
    let mut clip = String::new();
    interact::type_into_focused(&mut s.ui_nodes, KeyInput::SelectAll, &mut clip);
    let g = at("rt--gras");
    click(&mut s, g.0 + 5, g.1 + 5);
    let value = form_values(&s.ui_nodes);
    assert_eq!(value.get("a").unwrap().as_text(), "premier");
    assert!(rich::parse(&value.get("b").unwrap().as_text())[0].style.bold);
}

#[test]
fn slash_entree_et_focus() {
    let rsc = with_default_styles("richtext { width: 400px; }", None);
    let sheet = parse_rsc(tokenize_rsc(&rsc)).unwrap();
    let rsh = "<richtext#x valeur=\"abc\" commandes=\"true\" entree=\"true\" focus=\"true\"><!richtext>";
    let mut s = EventState::new(build_ui(&parse_rsh(tokenize_rsh(rsh)).unwrap(), &StyleSource::Rsc(&sheet)));
    assert!(area(&s).focused, "focus a la construction");
    let key = |s: &mut EventState, code: u32, down: bool| {
        handle_event(s, WindowEvent::WindowKeyPress(code, down), KeyboardLayout::Qwerty, ZONE);
    };
    // « a/ » : pas en debut de mot, rien.
    key(&mut s, 53, true);
    key(&mut s, 53, false);
    assert!(!s.take_activation());
    // Espace puis « / » : l'app est prevenue.
    for code in [57, 53] {
        key(&mut s, code, true);
        key(&mut s, code, false);
    }
    assert!(s.take_activation());
    assert_eq!(s.clicked_id.as_deref(), Some("slash-x"));
    assert_eq!(area(&s).text, "abc/ /");
    // Entree : pour l'app, avec le curseur.
    key(&mut s, 28, true);
    key(&mut s, 28, false);
    assert!(s.take_activation());
    assert_eq!(s.clicked_id.as_deref(), Some("entree-x@6"));
    assert_eq!(area(&s).text, "abc/ /");
    // Maj+Entree : retour a la ligne.
    key(&mut s, 42, true);
    key(&mut s, 28, true);
    key(&mut s, 28, false);
    key(&mut s, 42, false);
    assert_eq!(area(&s).text, "abc/ /\n");
}

#[test]
fn clic_droit_panneau_retour_et_focus_position() {
    let rsc = with_default_styles("richtext { width: 400px; }", None);
    let sheet = parse_rsc(tokenize_rsc(&rsc)).unwrap();
    let rsh = "<richtext#x valeur=\"bonjour\" entree=\"true\" focus=\"3\"><!richtext>";
    let mut s = EventState::new(build_ui(&parse_rsh(tokenize_rsh(rsh)).unwrap(), &StyleSource::Rsc(&sheet)));
    assert!(area(&s).focused);
    assert_eq!(area(&s).cursor, 3);
    let bouton = |s: &mut EventState, b: u32, x: i32, y: i32| {
        handle_event(s, WindowEvent::WindowMouseMove(x, y), KeyboardLayout::Qwerty, ZONE);
        handle_event(s, WindowEvent::WindowMouseButton(b, true), KeyboardLayout::Qwerty, ZONE);
        handle_event(s, WindowEvent::WindowMouseButton(b, false), KeyboardLayout::Qwerty, ZONE);
    };
    // Sans selection : pas de panneau.
    bouton(&mut s, 273, 40, 10);
    assert!(s.format_menu.is_none());
    let mut clip = String::new();
    interact::type_into_focused(&mut s.ui_nodes, KeyInput::SelectAll, &mut clip);
    bouton(&mut s, 273, 40, 10);
    let menu = s.format_menu.clone().expect("panneau ouvert");
    // Dessine sans paniquer.
    let mut canvas = Canvas::new(600, 300);
    azure_foundation::ui::services::draw_ui::draw_format_menu(&menu, &mut canvas, -1, -1);
    // Clic sur « G » puis sur le rouge (le panneau se rouvre au clic droit).
    let items = menu.items();
    bouton(&mut s, BTN_LEFT, items[0].rect.0 + 3, items[0].rect.1 + 3);
    assert!(s.format_menu.is_none());
    assert!(bold(area(&s)).iter().all(|b| *b), "gras sur la selection");
    assert!(area(&s).focused && area(&s).selection_range() == Some((0, 7)), "selection gardee");
    bouton(&mut s, 273, 40, 10);
    let rouge = s.format_menu.as_ref().unwrap().items()[6].rect;
    bouton(&mut s, BTN_LEFT, rouge.0 + 3, rouge.1 + 3);
    assert_eq!(area(&s).style_at(2).color, rich::parse_color("#e06c75"));
    // Un clic a cote ferme sans rien faire.
    bouton(&mut s, 273, 40, 10);
    bouton(&mut s, BTN_LEFT, 590, 290);
    assert!(s.format_menu.is_none());
    // Retour arriere en tete de zone : pour l'app.
    interact::type_into_focused(&mut s.ui_nodes, KeyInput::Home(false), &mut clip);
    handle_event(&mut s, WindowEvent::WindowKeyPress(14, true), KeyboardLayout::Qwerty, ZONE);
    assert!(s.take_activation());
    assert_eq!(s.clicked_id.as_deref(), Some("retour-x"));
    assert_eq!(area(&s).text, "bonjour");
}

#[test]
fn menu_slash_au_clavier() {
    let rsc = with_default_styles("richtext { width: 400px; }", None);
    let sheet = parse_rsc(tokenize_rsc(&rsc)).unwrap();
    let rsh = "<richtext#x valeur=\"ab \" commandes=\"texte|Texte|Un paragraphe;titre1|Titre 1|Grand titre|h1;tache|Tâche|Case|todo\" entree=\"true\" focus=\"true\"><!richtext>";
    let mut s = EventState::new(build_ui(&parse_rsh(tokenize_rsh(rsh)).unwrap(), &StyleSource::Rsc(&sheet)));
    let taper = |s: &mut EventState, codes: &[u32]| {
        for &c in codes {
            handle_event(s, WindowEvent::WindowKeyPress(c, true), KeyboardLayout::Qwerty, ZONE);
            handle_event(s, WindowEvent::WindowKeyPress(c, false), KeyboardLayout::Qwerty, ZONE);
        }
    };
    // « / » : le menu s'ouvre, l'app n'est pas prevenue.
    taper(&mut s, &[53]);
    assert!(!s.take_activation());
    let menu = s.command_menu.clone().expect("menu ouvert");
    assert_eq!((menu.start, menu.shown.len()), (3, 3));
    // « /tex » : filtre, completion « te ».
    taper(&mut s, &[20, 18, 45]);
    let menu = s.command_menu.clone().unwrap();
    assert_eq!(menu.query, "tex");
    assert_eq!(menu.current().unwrap().code, "texte");
    assert_eq!(menu.completion(), "te");
    let mut canvas = Canvas::new(600, 400);
    azure_foundation::ui::services::draw_ui::draw_command_menu(&menu, &mut canvas, -1, -1);
    // Retour arriere x3 puis « h1 » : un mot-cle ; ↓ tourne ; Entree choisit.
    taper(&mut s, &[14, 14, 14, 35, 2]);
    assert_eq!(s.command_menu.as_ref().unwrap().current().unwrap().code, "titre1");
    taper(&mut s, &[28]);
    assert!(s.take_activation());
    assert_eq!(s.clicked_id.as_deref(), Some("commande-x@titre1@3"));
    assert_eq!(area(&s).text, "ab ", "le /h1 tape est parti");
    assert!(s.command_menu.is_none());
    // Echap ferme et garde le texte ; un espace apres une recherche vaine aussi.
    taper(&mut s, &[53, 1]);
    assert!(s.command_menu.is_none());
    assert_eq!(area(&s).text, "ab /");
    taper(&mut s, &[57, 53, 16, 16, 57]);
    assert!(s.command_menu.is_none(), "« /qq » puis espace : ferme");
    // Tab valide aussi.
    taper(&mut s, &[57, 53, 20, 15]);
    assert_eq!(s.clicked_id.as_deref(), Some("commande-x@texte@10"));
}
