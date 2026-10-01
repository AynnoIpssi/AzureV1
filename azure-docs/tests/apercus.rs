// Les apercus rsC : chacun se compile avec son exemple et donne des widgets,
// et la page les affiche a leur place. Chaque rendu est ecrit dans
// target/tmp/azure-docs/apercu-<exemple>.ppm (pour les regarder).
use azure_docs::apercu;
use azure_docs::contenu::Bloc;
use azure_docs::ecrans;
use azure_docs::Docs;
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use std::path::Path;

fn dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn capture(nodes: &[UiNode], nom: &str) {
    let (w, h) = (760, 240);
    let mut canvas = Canvas::new(w, h);
    // Le fond du cadre de la page (#0d0d0d), avec sa marge.
    for px in canvas.buffer.chunks_mut(4) {
        px.copy_from_slice(&[13, 13, 13, 255]);
    }
    draw_ui(nodes, (18, 18, w - 36, h - 36), &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-docs");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join(format!("apercu-{nom}.ppm")), ppm).unwrap();
}

#[test]
fn chaque_apercu_se_construit() {
    let docs = Docs::charger(&dir().join("contenu")).unwrap();
    let base = std::fs::read_to_string(dir().join("ui/apercu.rsc")).unwrap();
    let mut total = 0;
    for page in docs.pages() {
        let mut exemple = String::new();
        for bloc in &page.blocs {
            match bloc {
                Bloc::Code(e) => exemple = e.id.clone(),
                Bloc::Apercu(a) => {
                    let rendu = apercu::rendre(a, &base).unwrap_or_else(|e| panic!("{} (après {exemple}) : {e}", page.chemin()));
                    assert!(!rendu.is_empty(), "{} : aperçu vide après {exemple}", page.chemin());
                    capture(&rendu, &exemple);
                    total += 1;
                }
                _ => {}
            }
        }
    }
    assert!(total >= 30, "{total} aperçus");
}

#[test]
fn la_page_met_chaque_rendu_dans_son_cadre() {
    let table = ecrans::routes(&dir().join("ui"), &dir().join("contenu"));
    let nodes = table.resolve(&Route::new("/doc/rsc/flex", "")).unwrap();
    let mut cadres = Vec::new();
    fn chercher(nodes: &[UiNode], out: &mut Vec<(String, usize)>) {
        for n in nodes {
            if let UiNode::Container(c) = n {
                if c.decoration.anchor.starts_with("apercu-") {
                    out.push((c.decoration.anchor.clone(), c.children.len()));
                }
                chercher(&c.children, out);
            }
        }
    }
    chercher(&nodes, &mut cadres);
    assert_eq!(cadres.len(), 4, "{cadres:?}");
    assert!(cadres.iter().all(|(_, n)| *n > 0), "un cadre est resté vide : {cadres:?}");
    // Toute la page d'un coup (fenetre tres haute) : target/tmp/azure-docs/page-rsc-flex.ppm
    let (w, h) = (1280, 2400);
    let mut canvas = Canvas::new(w, h);
    draw_ui(&nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-docs/page-rsc-flex.ppm"), ppm).unwrap();
}
