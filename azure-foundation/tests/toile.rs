// La toile (`<toile>`) pour de vrai : construite depuis un rsH, pilotee par
// les memes evenements que la fenetre (appui, glisser, relacher, molette).
use azure_core::rules::window_event::WindowEvent as W;
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::compiler::components::with_default_styles;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::event::services::dispatch::handle_event;
use azure_foundation::ui::models::toile::{Dessin, GenreBoite, LigneBoite, StyleLigne};
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::interact::{self, KeyboardLayout};

const Z: (u32, u32, u32, u32) = (0, 0, 900, 560);

fn dessin() -> Dessin {
    let mut d = Dessin::default();
    let c = d.boite("CLIENT", 40.0, 60.0, GenreBoite::Entite, "CLIENT");
    c.lignes = vec![LigneBoite { texte: "id_client".into(), style: StyleLigne::Souligne }, LigneBoite { texte: "nom_client".into(), style: StyleLigne::Normal }];
    let k = d.boite("COMMANDE", 460.0, 60.0, GenreBoite::Entite, "COMMANDE");
    k.lignes = vec![LigneBoite { texte: "num_commande".into(), style: StyleLigne::Souligne }, LigneBoite { texte: "date_commande".into(), style: StyleLigne::Normal }];
    d.boite("passer", 260.0, 80.0, GenreBoite::Association, "passer");
    d.lien("passer", "CLIENT", "0,n", false);
    d.lien("passer", "COMMANDE", "1,1", false);
    let t = d.boite("t_commande", 460.0, 260.0, GenreBoite::Table, "commande");
    t.lignes = vec![LigneBoite { texte: "num_commande".into(), style: StyleLigne::Souligne }, LigneBoite { texte: "#id_client".into(), style: StyleLigne::Accent }];
    d.lien("t_commande", "CLIENT", "", true);
    d
}

fn page(d: &Dessin, mode: &str) -> Vec<UiNode> {
    let sheet = parse_rsc(tokenize_rsc(&with_default_styles(".fond { width: 100%; height: 100%; } .t { width: 100%; height: 100%; }", None))).unwrap();
    let rsh = format!("<container.fond><toile.t#mcd valeur=\"{{{{d}}}}\" choisi=\"passer\" mode=\"{mode}\"/><!container>");
    build_ui_with_context(&parse_rsh(tokenize_rsh(&rsh)).unwrap(), &StyleSource::Rsc(&sheet), &Context::new().with_text("d", &d.encoder()))
}

fn ev(st: &mut EventState, e: W) {
    handle_event(st, e, KeyboardLayout::Qwerty, Z);
}

fn toile(nodes: &[UiNode]) -> &azure_foundation::ui::models::toile::Toile {
    let mut out = None;
    fn go<'a>(n: &'a [UiNode], out: &mut Option<&'a azure_foundation::ui::models::toile::Toile>) {
        for x in n {
            match x {
                UiNode::Control(c) if c.toile.is_some() => *out = c.toile.as_deref(),
                UiNode::Container(c) => go(&c.children, out),
                _ => {}
            }
        }
    }
    go(nodes, &mut out);
    out.expect("une toile")
}

#[test]
fn deplacer_une_boite_puis_zoomer() {
    let d = dessin();
    let mut st = EventState::new(page(&d, ""));
    // CLIENT a l'ecran : toile en (0,0), decalage (24,24) -> (64, 84).
    ev(&mut st, W::WindowMouseMove(80, 100));
    ev(&mut st, W::WindowMouseButton(272, true));
    assert_eq!(st.clicked_id, None, "un appui seul ne dit rien a l'app");
    ev(&mut st, W::WindowMouseMove(140, 150));
    ev(&mut st, W::WindowMouseButton(272, false));
    assert!(st.take_activation(), "le relachement appelle on_click");
    assert_eq!(st.clicked_id.as_deref(), Some("mcd@deplacer@CLIENT@100@110"));

    // Ctrl + molette : zoom ; la page, elle, ne defile pas.
    ev(&mut st, W::WindowKeyPress(29, true));
    ev(&mut st, W::WindowScroll(-1.0));
    ev(&mut st, W::WindowKeyPress(29, false));
    assert_eq!(toile(&st.ui_nodes).zoom, 1.1);

    // L'app reconstruit la page : la vue est gardee.
    let mut neuf = page(&d, "");
    interact::carry_toiles(&st.ui_nodes, &mut neuf);
    assert_eq!(toile(&neuf).zoom, 1.1);

    let mut canvas = Canvas::new(Z.2, Z.3);
    draw_ui(&page(&d, ""), Z, &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{} {}\n255\n", Z.2, Z.3).into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(format!("{}/toile.ppm", env!("CARGO_TARGET_TMPDIR")), ppm).unwrap();
}

#[test]
fn relier_deux_boites() {
    let mut st = EventState::new(page(&dessin(), "relier"));
    ev(&mut st, W::WindowMouseMove(80, 100));
    ev(&mut st, W::WindowMouseButton(272, true));
    // Vers COMMANDE : (460+24, 60+24).
    ev(&mut st, W::WindowMouseMove(520, 110));
    ev(&mut st, W::WindowMouseButton(272, false));
    assert!(st.take_activation());
    assert_eq!(st.clicked_id.as_deref(), Some("mcd@relier@CLIENT@COMMANDE"));
}

/// Une boite loin a droite (hors de la vue) : la barre horizontale apparait ;
/// la tirer, ou le pave tactile vers la droite, amene la vue jusqu'a elle.
#[test]
fn barres_de_defilement_et_pave_tactile() {
    use azure_foundation::ui::models::toile::{mesure_par_defaut, Toile};
    let mut d = dessin();
    d.boite("LOIN", 1900.0, 60.0, GenreBoite::Entite, "LOIN");
    let mut st = EventState::new(page(&d, ""));
    let own = (0, 0, Z.2, Z.3);
    let (h, v) = toile(&st.ui_nodes).barres(own, &mesure_par_defaut);
    let h = h.expect("le dessin depasse a droite : barre horizontale");
    assert!(v.is_none(), "rien ne depasse en hauteur");
    let visible = |t: &Toile| {
        let b = t.dessin.boites.iter().find(|b| b.id == "LOIN").unwrap();
        let (x, _) = t.vers_ecran(own, b.x, b.y);
        x >= 0.0 && x < Z.2 as f32
    };
    assert!(!visible(toile(&st.ui_nodes)));
    // Tirer la poignee tout a droite.
    let (px, py) = ((h.poignee.0 + h.poignee.2 / 2.0) as i32, (h.poignee.1 + h.poignee.3 / 2.0) as i32);
    ev(&mut st, W::WindowMouseMove(px, py));
    ev(&mut st, W::WindowMouseButton(272, true));
    ev(&mut st, W::WindowMouseMove(px + 800, py));
    ev(&mut st, W::WindowMouseButton(272, false));
    assert!(visible(toile(&st.ui_nodes)), "decalage {:?}", toile(&st.ui_nodes).decalage);
    // Le pave tactile vers la gauche ramene la vue.
    let avant = toile(&st.ui_nodes).decalage.0;
    ev(&mut st, W::WindowMouseMove(400, 300));
    ev(&mut st, W::WindowScrollH(-300.0));
    assert_eq!(toile(&st.ui_nodes).decalage.0, avant + 300.0);

    let mut canvas = Canvas::new(Z.2, Z.3);
    draw_ui(&st.ui_nodes, Z, &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{} {}\n255\n", Z.2, Z.3).into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(format!("{}/toile-barres.ppm", env!("CARGO_TARGET_TMPDIR")), ppm).unwrap();
}

/// Un double-clic sur une boite arrive a l'app (`ouvrir`).
#[test]
fn double_clic_ouvre_la_boite() {
    let mut st = EventState::new(page(&dessin(), ""));
    ev(&mut st, W::WindowMouseMove(80, 100));
    for _ in 0..2 {
        ev(&mut st, W::WindowMouseButton(272, true));
        ev(&mut st, W::WindowMouseButton(272, false));
    }
    // Le second appui : `ouvrir`, tout de suite.
    assert_eq!(st.clicked_id.as_deref(), Some("mcd@ouvrir@CLIENT"));
}
