// Les maquettes des projets (maquettes/<nom>.rsh + commun.rsc + <nom>.rsc)
// dessinees dans target/tmp/maquette-<nom>.ppm. A la main : voir
// maquettes/README.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::services::draw_ui::draw_ui;
use std::path::Path;

pub const MAQUETTES: [&str; 4] = ["tests-maui", "scratch", "boutique", "echecs"];

#[test]
#[ignore]
fn dessiner_les_maquettes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("maquettes");
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let commun = std::fs::read_to_string(dir.join("commun.rsc")).unwrap();
    for nom in MAQUETTES {
        let rsc = tmp.join(format!("maquette-{nom}.rsc"));
        std::fs::write(&rsc, commun.clone() + &std::fs::read_to_string(dir.join(format!("{nom}.rsc"))).unwrap()).unwrap();
        let table = RouteTable::new().view("/", dir.join(format!("{nom}.rsh")).to_str().unwrap(), rsc.to_str().unwrap());
        let nodes = table.resolve(&Route::new("/", "")).unwrap();
        assert!(!nodes.is_empty(), "{nom} : rsH ou rsC invalide");
        let (w, h) = (1200, 760);
        let mut canvas = Canvas::new(w, h);
        draw_ui(&nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        for px in canvas.buffer.chunks(4) {
            ppm.extend_from_slice(&[px[2], px[1], px[0]]);
        }
        std::fs::write(tmp.join(format!("maquette-{nom}.ppm")), ppm).unwrap();
    }
}
