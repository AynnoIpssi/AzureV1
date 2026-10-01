// `fenetre` : la boucle de la fenetre sur chaque ecran - le tic (lecture
// des champs + `on_tick` de Testeur), la souris, la molette, un clic dans
// l'editeur de code, la frappe.
use crate::commun::*;
use azure_core::rules::window_event::WindowEvent as W;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::models::keys::BTN_LEFT;
use azure_foundation::event::services::dispatch::{handle_event, handle_tick};
use azure_foundation::ui::services::interact::{self, KeyboardLayout};
use azure_testeur::clics::{cliquer, ranger};
use azure_testeur::ecran::Testeur;
use std::sync::Arc;

const Z: &str = "fenetre";
const CONTENU: (u32, u32, u32, u32) = (0, 0, VUE.0, VUE.1);
const KEY_A: u32 = 30;

fn ev(st: &mut EventState, e: W) -> bool {
    handle_event(st, e, KeyboardLayout::Qwerty, CONTENU)
}

fn boucle(nom: &str, t: &Arc<Testeur>, editeur: Option<&str>) {
    let nodes = construire(t);
    // Le tic de la fenetre quand rien ne change : `form_values` (fait par
    // `with_context` avant d'appeler `on_tick`) + la lecture de la version.
    mesure(Z, &format!("[{nom}] tic au repos (form_values + on_tick)"), true, || {
        let v = Valeurs::de(&nodes);
        let _ = t.lanceur.lire().version;
        drop(v);
    });
    // Le tic quand l'execution a avance : ranger + redessin complet.
    mesure(Z, &format!("[{nom}] tic avec resultats (ranger + reconstruire + dessiner)"), true, || {
        ranger(t, &Valeurs::de(&nodes));
        let mut neuf = construire(t);
        interact::carry_scroll(&nodes, &mut neuf);
        dessiner(&neuf);
    });
    let mut st = EventState::new(construire(t));
    mesure(Z, &format!("[{nom}] handle_tick (curseur, defilement)"), true, || {
        let _ = handle_tick(&mut st, KeyboardLayout::Qwerty, CONTENU);
    });
    mesure(Z, &format!("[{nom}] souris : 10 mouvements"), true, || {
        for k in 0..10 {
            ev(&mut st, W::WindowMouseMove(300 + k * 60, 200 + k * 40));
        }
    });
    // Le detail d'un mouvement : ce que `handle_mouse_move` calcule.
    mesure(Z, &format!("[{nom}] souris > hover_kind_at"), true, || { let _ = interact::hover_kind_at(&st.ui_nodes, 640, 400, CONTENU); });
    mesure(Z, &format!("[{nom}] souris > hover_group_at"), true, || { let _ = interact::hover_group_at(&st.ui_nodes, 640, 400, CONTENU); });
    mesure(Z, &format!("[{nom}] mise en page complete (walk)"), true, || interact::walk(&st.ui_nodes, CONTENU, &mut |_, _| {}));
    mesure(Z, &format!("[{nom}] molette"), true, || {
        ev(&mut st, W::WindowMouseMove(640, 500));
        ev(&mut st, W::WindowScroll(15.0));
    });
    let Some(id) = editeur else { return };
    let Some((x, y)) = position(&nodes, id) else {
        eprintln!("  (#{id} pas visible : frappe non mesuree)");
        return;
    };
    ev(&mut st, W::WindowMouseMove(x, y));
    mesure(Z, &format!("[{nom}] clic dans #{id}"), true, || {
        ev(&mut st, W::WindowMouseButton(BTN_LEFT, true));
        ev(&mut st, W::WindowMouseButton(BTN_LEFT, false));
    });
    mesure(Z, &format!("[{nom}] frappe d'une touche dans #{id}"), true, || {
        ev(&mut st, W::WindowKeyPress(KEY_A, true));
        ev(&mut st, W::WindowKeyPress(KEY_A, false));
    });
    // Apres la frappe, le tic relit ce champ (texte riche encode).
    mesure(Z, &format!("[{nom}] tic apres frappe (form_values)"), true, || drop(Valeurs::de(&st.ui_nodes)));
}

pub fn mesurer() {
    eprintln!("\n=== fenetre ===");
    let Some(env) = environnement() else { return };
    let t = testeur(Some(env));
    let rien = Valeurs(Default::default());
    boucle("tests", &t, Some("filtre"));
    cliquer(&t, "ouvrir-tout", &rien);
    boucle("tests deplies", &t, None);
    cliquer(&t, "voir-0", &rien);
    boucle("detail", &t, None);
    cliquer(&t, "onglet-atelier", &rien);
    boucle("atelier", &t, None);
    cliquer(&t, "ouvrir-0", &rien);
    boucle("atelier + test ouvert", &t, Some("code-0"));
    // Le plus long test du fichier ouvert.
    let plus_long = t.etat().lu.as_ref().and_then(|l| l.tests.iter().enumerate().max_by_key(|(_, s)| s.code.len()).map(|(i, _)| i)).unwrap_or(0);
    t.etat().ouvert = None;
    cliquer(&t, &format!("ouvrir-{plus_long}"), &rien);
    boucle("atelier + plus long test", &t, Some(&format!("code-{plus_long}")));
    t.etat().ouvert = None;
    cliquer(&t, "ouvrir-generique", &rien);
    boucle("atelier + generique", &t, Some("generique"));
}

#[test]
#[ignore]
fn fenetre() {
    mesurer();
    fin_de_zone(Z);
}
