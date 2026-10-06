// Les graphes d'azure-libraire (`chart-*`, `<graphe>`) : la page
// d'exemple dessinee (target/tmp/graphes-<theme>.ppm), et ce que chaque
// type met a l'ecran.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::components::with_default_styles;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse;
use azure_foundation::compiler::rsh::services::lexer::tokenize;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::theme;
use azure_foundation::ui::models::control::ControlKind;
use azure_foundation::ui::models::graphe::{Genre, Graphe};
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;

fn dessiner(nodes: &[UiNode], w: u32, h: u32) -> Canvas {
    let mut canvas = Canvas::new(w, h);
    draw_ui(nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
    canvas
}

fn construire(rsh: &str, rsc: &str) -> Vec<UiNode> {
    let sheet = parse_rsc(tokenize_rsc(&with_default_styles(rsc, None))).unwrap();
    build_ui_with_context(&parse(tokenize(rsh)).unwrap(), &StyleSource::Rsc(&sheet), &Context::new())
}

fn graphe(nodes: &[UiNode]) -> Option<&Graphe> {
    nodes.iter().find_map(|n| match n {
        UiNode::Control(c) if c.kind == ControlKind::Graphe => c.graphe.as_deref(),
        UiNode::Container(c) => graphe(&c.children),
        _ => None,
    })
}

/// Nombre de pixels de la couleur (r, v, b).
fn compte(canvas: &Canvas, (r, v, b): (u8, u8, u8)) -> usize {
    canvas.buffer.chunks(4).filter(|p| (p[2], p[1], p[0]) == (r, v, b)).count()
}

/// Le theme en cours est commun au processus : un test a la fois.
static THEME: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn les_attributs_decrivent_le_graphe() {
    let _un_seul = THEME.lock().unwrap_or_else(|e| e.into_inner());
    let nodes = construire("<chart-line titre=\"CPU\" unite=\" %\" max=\"100\" legende=\"true\" series=\"A: 1 2 3; B: 4 5\" etiquettes=\"x, y, z\"/>", "");
    let g = graphe(&nodes).unwrap();
    assert_eq!((g.genre, g.series.len(), g.points(), g.max, g.unite.as_str(), g.legende, g.grille), (Genre::Ligne, 2, 3, Some(100.0), " %", true, true));
    assert_eq!(g.etiquettes, ["x", "y", "z"]);
    // Couleurs : l'accent du theme, puis sa palette ; ou celles demandees.
    let t = theme::actif();
    let c = |nom: &str| t.couleur(nom).unwrap();
    assert_eq!(((g.couleur(0).r, g.couleur(0).g, g.couleur(0).b), (g.couleur(1).r, g.couleur(1).g, g.couleur(1).b)), (c("accent"), c("graphe-2")));
    let nodes = construire("<graphe type=\"anneau\" valeurs=\"3, 1\" couleurs=\"#ff0000, #00ff00\" grille=\"false\"/>", "");
    let g = graphe(&nodes).unwrap();
    assert_eq!((g.genre, g.premiere(), g.couleurs.len(), g.grille), (Genre::Anneau, &[3.0, 1.0][..], 2, false));
    // `accent-color` de la page : la premiere serie.
    let nodes = construire("<graphe.x valeurs=\"1, 2\"/>", ".x { accent-color: #102030; }");
    assert_eq!(graphe(&nodes).unwrap().couleur(0).r, 0x10);
}

#[test]
fn chaque_type_se_dessine_dans_sa_boite() {
    let _un_seul = THEME.lock().unwrap_or_else(|e| e.into_inner());
    for genre in ["ligne", "aire", "barres", "barres-h", "empile", "points", "anneau", "secteurs", "jauge", "radar", "spark"] {
        for donnees in ["series=\"A: 5 9 2 7 4; B: 1 3 8 2 6\" etiquettes=\"a, b, c, d, e\" legende=\"true\"", "valeurs=\"\"", "valeurs=\"0, 0\"", "valeurs=\"4\"", "valeurs=\"-3, 5, -1\""] {
            let nodes = construire(&format!("<container.fond><container.cadre><graphe.g type=\"{genre}\" {donnees} couleurs=\"#ff00ff, #00ffff\"/><!container><!container>"), ".fond { padding: 40px; } .cadre { width: 320px; } .g { width: 320px; height: 180px; }");
            let canvas = dessiner(&nodes, 400, 260);
            // Rien hors de la boite (40..360 x 40..220) : le fond reste intact.
            for (i, p) in canvas.buffer.chunks(4).enumerate() {
                let (x, y) = (i as u32 % 400, i as u32 / 400);
                if !(40..360).contains(&x) || !(40..220).contains(&y) {
                    assert_eq!(&p[..3], &[0, 0, 0], "{genre} {donnees} : pixel ({x}, {y}) hors du graphe");
                }
            }
            // Des donnees : la couleur de la premiere serie est a l'ecran.
            if donnees.starts_with("series") {
                assert!(compte(&canvas, (255, 0, 255)) > 20, "{genre} : rien de dessine");
            }
        }
    }
}

#[test]
fn la_page_d_exemple_suit_le_theme() {
    let _un_seul = THEME.lock().unwrap_or_else(|e| e.into_inner());
    let d = format!("{}/../azure-libraire/exemples", env!("CARGO_MANIFEST_DIR"));
    let routes = RouteTable::new().view("/", &format!("{d}/graphes.rsh"), &format!("{d}/graphes.rsc"));
    let (w, h) = (1280, 1500);
    for nom in ["sable", "ivoire"] {
        let t = theme::Theme::integre(nom).unwrap();
        theme::definir(t.clone());
        let canvas = dessiner(&routes.resolve(&Route::new("/", "")).unwrap(), w, h);
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        for px in canvas.buffer.chunks(4) {
            ppm.extend_from_slice(&[px[2], px[1], px[0]]);
        }
        std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("graphes-{nom}.ppm")), ppm).unwrap();
        // L'accent et la deuxieme couleur de la palette sont bien la.
        assert!(compte(&canvas, t.couleur("accent").unwrap()) > 500, "{nom} : accent");
        assert!(compte(&canvas, t.couleur("graphe-2").unwrap()) > 200, "{nom} : graphe-2");
    }
    theme::choisir(theme::DEFAUT).unwrap();
}
