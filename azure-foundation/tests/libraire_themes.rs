// Les themes d'azure-libraire : la meme page, dessinee dans chacun
// (target/tmp/vitrine-<theme>.ppm). Un seul test : le theme en cours est
// commun a tout le processus.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::theme::{self, Theme};
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;

fn dessiner(nodes: &[UiNode], w: u32, h: u32) -> Canvas {
    let mut canvas = Canvas::new(w, h);
    draw_ui(nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
    canvas
}

fn enregistrer(canvas: &Canvas, w: u32, h: u32, name: &str) {
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.ppm")), ppm).unwrap();
}

/// Le pixel (x, y) en (r, v, b).
fn pixel(canvas: &Canvas, w: u32, x: u32, y: u32) -> (u8, u8, u8) {
    let i = ((y * w + x) * 4) as usize;
    (canvas.buffer[i + 2], canvas.buffer[i + 1], canvas.buffer[i])
}

#[test]
fn la_meme_page_suit_le_theme_en_cours() {
    let d = format!("{}/../azure-libraire/exemples", env!("CARGO_MANIFEST_DIR"));
    let question = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let q = question.clone();
    let routes = RouteTable::new().view_with("/", &format!("{d}/vitrine.rsh"), &format!("{d}/vitrine.rsc"), move |_| {
        Context::new().with_text("qui", "Ada").with_text("question", if q.load(std::sync::atomic::Ordering::Relaxed) { "true" } else { "false" })
    });
    let (w, h) = (1280, 900);
    let page = || routes.resolve(&Route::new("/", "")).unwrap();
    // Le coin bas gauche : la barre laterale, de la couleur `fond-barre`.
    let mut barres = Vec::new();
    for nom in theme::integres() {
        theme::choisir(nom).unwrap();
        let canvas = dessiner(&page(), w, h);
        enregistrer(&canvas, w, h, &format!("vitrine-{nom}"));
        barres.push((nom, pixel(&canvas, w, 5, h - 5)));
    }
    assert_eq!(barres, [("sable", (0x0f, 0x0e, 0x0d)), ("ivoire", (0xef, 0xea, 0xe2)), ("ardoise", (0x0d, 0x0f, 0x10))]);

    // Un theme d'app : un theme fourni, avec ses propres jetons.
    theme::definir(Theme::lire("nom = maison\nbase = sable\nfond-barre = #102030\n").unwrap());
    assert_eq!(pixel(&dessiner(&page(), w, h), w, 5, h - 5), (0x10, 0x20, 0x30));

    // La question par-dessus la page.
    theme::choisir("ivoire").unwrap();
    question.store(true, std::sync::atomic::Ordering::Relaxed);
    enregistrer(&dessiner(&page(), w, h), w, h, "vitrine-ivoire-question");
    theme::choisir(theme::DEFAUT).unwrap();
}
