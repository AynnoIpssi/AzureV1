// L'inspecteur (F12) : il s'ouvre, trouve l'element sous le pointeur, le
// choisit au clic, connait ses classes et les regles rsC qui le visent (la
// declaration ecrasee par une regle plus forte est marquee). Le rendu est
// ecrit dans <target>/tmp/inspecteur.ppm pour le regarder.
use azure_core::rules::window_event::WindowEvent;
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::components::with_default_styles;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::inspector::{self, Inspector};
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::interact::node_at_path;

const FENETRE: (u32, u32, u32, u32) = (0, 0, 1200, 640);

const RSH: &str = "<container.app>
    <title2.h>Mes apps<!title2>
    <container.carte>
        <text.nom>Azure Note<!text>
        <button.btn#ouvrir>Ouvrir<!button>
    <!container>
<!container>";

const RSC: &str = ".app { display: flex; flex-direction: column; gap: 12px; padding: 24px; background-color: #121212; height: 100%; }
.h { color: #f0eeea; font-size: 22px; }
.carte { display: flex; gap: 16px; align-items: center; padding: 16px 20px; margin: 8px; border: 1px solid #333; border-radius: 10px; background-color: #1e1e1e; }
.nom { color: #e7e5e1; font-size: 15px; }
button { padding: 6px 10px; }
.btn { padding: 8px 16px; background-color: #26241f; color: #e7e5e1; border-radius: 6px; }
.btn:hover { background-color: #302c26; }";

fn page() -> Vec<UiNode> {
    let sheet = parse_rsc(tokenize_rsc(&with_default_styles(RSC, None))).unwrap();
    build_ui(&parse_rsh(tokenize_rsh(RSH)).unwrap(), &StyleSource::Rsc(&sheet))
}

fn centre(nodes: &[UiNode], inspector: &Inspector, path: &[usize]) -> (i32, i32) {
    let mut found = None;
    azure_foundation::ui::services::interact::walk(nodes, inspector.page_box(FENETRE), &mut |node, b| {
        if std::ptr::eq(node, node_at_path(nodes, path).unwrap()) {
            found = Some((b.0 + b.2 as i32 / 2, b.1 + b.3 as i32 / 2));
        }
    });
    found.unwrap()
}

#[test]
fn ouvrir_survoler_choisir() {
    let nodes = page();
    let mut inspector = Inspector::default();

    // F12 ouvre en mode « choisir ».
    assert_eq!(inspector.handle_event(&WindowEvent::WindowKeyPress(88, true), &nodes, FENETRE, (0, 0), false), Some(true));
    assert!(inspector.open && inspector.picking);
    // La page laisse la place au panneau.
    assert!(inspector.page_box(FENETRE).2 < FENETRE.2);

    // Le bouton : .app > .carte > button.
    let bouton = vec![0, 1, 1];
    let (x, y) = centre(&nodes, &inspector, &bouton);
    inspector.handle_event(&WindowEvent::WindowMouseMove(x, y), &nodes, FENETRE, (x, y), false);
    assert_eq!(inspector.hovered.as_ref(), Some(&bouton));
    // Le clic choisit, sans aller a l'app.
    assert!(inspector.handle_event(&WindowEvent::WindowMouseButton(272, true), &nodes, FENETRE, (x, y), false).is_some());
    assert_eq!(inspector.selected.as_ref(), Some(&bouton));

    let info = node_at_path(&nodes, &bouton).unwrap().decoration().inspect.clone().unwrap();
    assert_eq!(info.selector(), "button.btn#ouvrir");
    let selecteurs: Vec<(&str, &str)> = info.rules.iter().map(|r| (r.selector.as_str(), r.state)).collect();
    // La plus forte d'abord, puis celle du survol.
    assert_eq!(&selecteurs[..2], &[(".btn", ""), ("button", "")], "{selecteurs:?}");
    assert!(selecteurs.contains(&(".btn:hover", ":hover")), "{selecteurs:?}");
    // `padding` de `button` est ecrase par celui de `.btn`.
    let button = info.rules.iter().find(|r| r.selector == "button").unwrap();
    assert_eq!(button.declarations[0], ("padding".to_string(), "6px 10px".to_string(), true));
    let btn = info.rules.iter().find(|r| r.selector == ".btn").unwrap();
    assert!(btn.declarations.iter().any(|(n, v, ecrasee)| n == "padding" && v == "8px 16px" && !ecrasee));

    // Fleche gauche : replie le bouton (pas d'enfants) -> remonte au parent.
    inspector.handle_event(&WindowEvent::WindowKeyPress(105, true), &nodes, FENETRE, (x, y), false);
    assert_eq!(inspector.selected.as_ref(), Some(&vec![0, 1]));

    // Images : la page reduite, l'element survole entoure, le panneau ;
    // puis les regles du bouton (details defiles).
    inspector.selected = Some(vec![0, 1]);
    image(&mut inspector, &nodes, "inspecteur");
    inspector.selected = Some(bouton.clone());
    inspector.detail_scroll = 1000.0;
    image(&mut inspector, &nodes, "inspecteur-styles");

    // F12 referme.
    inspector.handle_event(&WindowEvent::WindowKeyPress(88, true), &nodes, FENETRE, (0, 0), false);
    assert!(!inspector.open);
    assert_eq!(inspector.page_box(FENETRE), FENETRE);
}

fn image(inspector: &mut Inspector, nodes: &[UiNode], nom: &str) {
    let mut canvas = Canvas::new(FENETRE.2, FENETRE.3);
    draw_ui(nodes, inspector.page_box(FENETRE), &mut canvas, -1, -1, false);
    inspector::draw(inspector, nodes, FENETRE, &mut canvas);
    let mut ppm = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(format!("{}/{nom}.ppm", env!("CARGO_TARGET_TMPDIR")), ppm).unwrap();
}

#[test]
fn bouton_inspecter_dans_la_barre_de_titre() {
    use azure_foundation::window::models::header_bar::{self, ButtonLayout, HeaderBar};
    let largeur = 900;
    let (bx, by, bw, bh) = header_bar::inspect_button(largeur);
    assert!(header_bar::on_inspect_button(largeur, (bx + bw / 2) as i32, (by + bh / 2) as i32));
    assert!(!header_bar::on_inspect_button(largeur, 10, 10), "pas sur les feux a gauche");
    // Ni sur un bouton de fenetre (fermer, reduire, agrandir).
    let header = HeaderBar::new("Azure Note".into(), None, ButtonLayout::mac(), true);
    assert_eq!(header_bar::button_at(largeur, &header.layout, (bx + 4) as i32, (by + 4) as i32), None);

    let mut canvas = Canvas::new(largeur, header_bar::HEADER_HEIGHT);
    azure_foundation::window::services::draw_header::draw_header(&mut canvas, largeur, &header, None, false);
    inspector::draw_header_button(header_bar::inspect_button(largeur), false, false, false, &mut canvas);
    let mut ppm = format!("P6\n{} {}\n255\n", canvas.width, canvas.height).into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(format!("{}/barre-inspecter.ppm", env!("CARGO_TARGET_TMPDIR")), ppm).unwrap();
}

#[test]
fn enregistrer_un_scenario() {
    use azure_foundation::ui::models::ui_node::UiNode as N;
    let sheet = parse_rsc(tokenize_rsc(&with_default_styles(".app { display: flex; flex-direction: column; gap: 10px; padding: 20px; height: 100%; } .liste { display: flex; flex-direction: column; height: 200px; overflow-y: auto; } .l { height: 40px; }", None))).unwrap();
    let rsh = "<container.app>
        <input#nom placeholder=\"Nom\"/>
        <button#valider>Valider<!button>
        <container.liste#liste><text.l>un<!text><text.l>deux<!text><text.l>trois<!text><text.l>quatre<!text><text.l>cinq<!text><text.l>six<!text><!container>
        <text#bilan>3 tests trouvés<!text>
        <container.sans-id><text>pas d'id<!text><!container>
    <!container>";
    let mut nodes = build_ui(&parse_rsh(tokenize_rsh(rsh)).unwrap(), &StyleSource::Rsc(&sheet));
    let mut ins = Inspector::default();
    ins.basculer_enregistrement(&nodes);
    assert!(ins.enregistrement.is_some() && !ins.picking, "on enregistre ce que fait l'app");
    let page = FENETRE;
    let centre = |nodes: &[N], id: &str| {
        let mut c = None;
        azure_foundation::ui::services::interact::walk(nodes, page, &mut |n, b| {
            if c.is_none() && n.decoration().anchor == id {
                c = Some((b.0 + b.2 as i32 / 2, b.1 + b.3 as i32 / 2));
            }
        });
        c.unwrap_or_else(|| panic!("#{id}"))
    };
    let noter = |ins: &mut Inspector, nodes: &[N], e: WindowEvent, souris: (i32, i32)| ins.enregistrement.as_mut().unwrap().noter(&e, nodes, page, souris);
    // Clic dans le champ, frappe (le champ change), Entree.
    let c = centre(&nodes, "nom");
    noter(&mut ins, &nodes, WindowEvent::WindowMouseButton(272, true), c);
    fn ecrire(nodes: &mut [N], t: &str) {
        for n in nodes {
            match n {
                N::TextArea(a) if a.id == "nom" => a.text = t.into(),
                N::Container(c) => ecrire(&mut c.children, t),
                _ => {}
            }
        }
    }
    ecrire(&mut nodes, "Mon");
    ecrire(&mut nodes, "Mon projet");
    noter(&mut ins, &nodes, WindowEvent::WindowKeyPress(28, true), c);
    // Le bouton, trois crans de molette sur la liste, un clic sans id.
    noter(&mut ins, &nodes, WindowEvent::WindowMouseButton(272, true), centre(&nodes, "valider"));
    let l = centre(&nodes, "liste");
    for _ in 0..3 {
        noter(&mut ins, &nodes, WindowEvent::WindowScroll(40.0), l);
    }
    // « Verifier ce texte » : l'element choisi dans l'inspecteur, puis le
    // bouton du panneau.
    ins.open = true;
    ins.selected = Some(vec![0, 3]);
    image(&mut ins, &nodes, "inspecteur-enregistrement");
    let (bx, by, _, _) = ins.zone_du_bouton(azure_foundation::inspector::Action::Verifier).expect("le bouton Verifier ce texte");
    ins.handle_event(&WindowEvent::WindowMouseButton(272, true), &nodes, FENETRE, (bx as i32 + 5, by as i32 + 5), false);
    ins.basculer_enregistrement(&nodes);
    let code = ins.code.clone().expect("le test est genere a l'arret");
    assert_eq!(ins.a_copier.as_deref(), Some(code.as_str()), "et copie");
    let attendu = "    app.clic(\"nom\");\n    app.remplir(\"nom\", \"Mon projet\");\n    app.touche(\"entree\");\n    app.clic(\"valider\");\n    app.molette(\"liste\", 120.0);\n    app.attendre_texte(\"3 tests trouvés\");\n";
    assert!(code.contains(attendu), "{code}");
    assert!(code.contains("Environnement::demarrer") && code.contains("CARGO_BIN_EXE_"), "{code}");
    // Le panneau montre le test genere.
    image(&mut ins, &nodes, "inspecteur-scenario");
}
