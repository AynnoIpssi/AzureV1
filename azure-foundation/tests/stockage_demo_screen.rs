// Capture hors fenetre de la demo `examples/stockage_demo.rs` avec des
// valeurs venues d'un vrai daemon : target/tmp/stockage-demo.ppm.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::storage::Stockage;
use azure_foundation::ui::services::draw_ui::draw_ui;
use std::time::{Duration, Instant};

#[test]
fn demo_page_renders_with_stored_values() {
    let socket = format!(concat!(env!("CARGO_TARGET_TMPDIR"), "/azure-stockage-demo-screen-{}.sock"), std::process::id());
    let root = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stockage-demo-screen");
    let _ = std::fs::remove_dir_all(&root);
    let s = socket.clone();
    std::thread::spawn(move || azure_stockage::managers::daemon::start_daemon_at(&s, &root).unwrap());
    let deadline = Instant::now() + Duration::from_secs(2);
    let store = loop {
        match Stockage::connect_at(&socket, 1) {
            Ok(store) => break store,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("{e}"),
        }
    };
    for _ in 0..7 {
        store.update("clics", 0, |n: i64| n + 1).unwrap();
    }
    store.set("ouvertures", 3).unwrap();
    store.set("dernier-clic", "14:32:05 UTC").unwrap();

    let rsh = include_str!("../examples/stockage/compteur.rsh")
        .replace("{{app}}", "1")
        .replace("{{clics}}", &store.get_or("clics", 0i64).to_string())
        .replace("{{ouvertures}}", &store.get_or("ouvertures", 0i64).to_string())
        .replace("{{dernier}}", &store.get_or("dernier-clic", String::new()));
    let sheet = parse_rsc(tokenize_rsc(include_str!("../examples/stockage/compteur.rsc"))).unwrap();
    let nodes = build_ui(&parse_rsh(tokenize_rsh(&rsh)).unwrap(), &StyleSource::Rsc(&sheet));

    let (w, h) = (560, 524);
    let mut canvas = Canvas::new(w, h);
    draw_ui(&nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("stockage-demo.ppm");
    std::fs::write(&path, ppm).unwrap();
    println!("{}", path.display());
}
