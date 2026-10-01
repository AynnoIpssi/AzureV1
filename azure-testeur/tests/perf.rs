// Mesures de la boucle de la fenetre (en release : cargo test --release
// -p azure-testeur --test perf -- --nocapture --ignored).
use azure_core::rules::window_event::WindowEvent as W;
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::interact::KeyboardLayout;
use azure_testeur::clics::{cliquer, Lecture};
use azure_testeur::ecran::{routes, Testeur};
use azure_testeur::projet::{self, EnMemoire, Projets};
use std::time::Instant;

struct Rien;
impl Lecture for Rien {
    fn valeur(&self, _: &str) -> Option<String> { None }
}

/// Le plus court de 5 essais (la machine peut etre occupee ailleurs).
fn min5(mut f: impl FnMut() -> std::time::Duration) -> std::time::Duration {
    (0..5).map(|_| f()).min().unwrap()
}

fn mesurer(nom: &str, t: &std::sync::Arc<Testeur>) {
    let ui = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
    let r = routes(&ui, t);
    let vue = (0, 0, 1280, 820);
    let donnees = min5(|| { let d = Instant::now(); let _ = azure_testeur::ecran::contexte(t); d.elapsed() });
    let resolve = min5(|| { let d = Instant::now(); let _ = r.resolve(&Route::new("/", "")); d.elapsed() });
    let nodes = r.resolve(&Route::new("/", "")).unwrap();
    let dessin = min5(|| { let d = Instant::now(); let mut c = Canvas::new(1280, 820); draw_ui(&nodes, vue, &mut c, -1, -1, false); d.elapsed() });
    let mut st = EventState::new(nodes);
    let souris = min5(|| {
        let d = Instant::now();
        for k in 0..10 { handle_event(&mut st, W::WindowMouseMove(500 + k * 20, 300 + k * 10), KeyboardLayout::Qwerty, vue); }
        d.elapsed() / 10
    });
    let molette = min5(|| { let d = Instant::now(); handle_event(&mut st, W::WindowScroll(15.0), KeyboardLayout::Qwerty, vue); d.elapsed() });
    eprintln!("{nom:<26} donnees {donnees:>8.1?}  construire {resolve:>8.1?}  dessiner {dessin:>8.1?}  souris {souris:>8.1?}  molette {molette:>8.1?}");
}

#[test]
#[ignore]
fn boucle_de_la_fenetre() {
    let Some(env) = projet::environnement() else { return };
    let t = Testeur::new(Projets::new(Some(env), Box::new(EnMemoire::default())));
    mesurer("tests (replies)", &t);
    cliquer(&t, "ouvrir-tout", &Rien);
    mesurer("tests (tout deplie)", &t);
    cliquer(&t, "fermer-tout", &Rien);
    {
        let mut e = t.lanceur.lire();
        for i in 0..4000 { e.lignes.push(format!("test azure::module::test_{i} ... ok")); }
    }
    cliquer(&t, "console", &Rien);
    mesurer("tests + sortie (4000 l.)", &t);
    cliquer(&t, "console", &Rien);
    cliquer(&t, "voir-0", &Rien);
    mesurer("tests + detail", &t);
    cliquer(&t, "onglet-atelier", &Rien);
    mesurer("atelier", &t);
    cliquer(&t, "ouvrir-0", &Rien);
    mesurer("atelier + un test ouvert", &t);
}
