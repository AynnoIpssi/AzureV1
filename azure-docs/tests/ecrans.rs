// Rend chaque vue d'Azure Docs hors fenetre (meme `draw_ui` que la vraie
// fenetre) dans target/tmp/azure-docs/<vue>.ppm, et verifie ce qui s'y
// affiche.
use azure_docs::ecrans::{self, action, Action};
use azure_docs::Docs;
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::window::models::header_bar::{ButtonLayout, HeaderBar, HeaderButton, HEADER_HEIGHT};
use azure_foundation::window::services::draw_header::draw_header;
use std::path::Path;

const W: u32 = 1280;
const H: u32 = 824;

fn dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn vue(chemin: &str, payload: &str) -> Vec<UiNode> {
    let table = ecrans::routes(&dir().join("ui"), &dir().join("contenu"));
    table.resolve(&Route::new(chemin, payload)).unwrap_or_else(|| panic!("aucune vue pour {chemin}"))
}

fn textes(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(l) => out.push(l.text.clone()),
            UiNode::Button(b) => out.push(format!("[{}]", b.text)),
            UiNode::Container(c) => textes(&c.children, out),
            _ => {}
        }
    }
}

fn tout(nodes: &[UiNode]) -> String {
    let mut out = Vec::new();
    textes(nodes, &mut out);
    out.join("\n")
}

fn capture(nodes: &[UiNode], nom: &str) {
    capture_avec(nodes, nom, None);
}

/// Comme la vraie fenetre : la barre de titre d'Azure, puis la page dessous.
fn capture_avec(nodes: &[UiNode], nom: &str, survol: Option<HeaderButton>) {
    let hauteur = H + HEADER_HEIGHT;
    let mut canvas = Canvas::new(W, hauteur);
    let barre = HeaderBar::new("Azure Docs".into(), None, ButtonLayout::mac(), true);
    draw_header(&mut canvas, W, &barre, survol, false);
    draw_ui(nodes, (0, HEADER_HEIGHT, W, H), &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{W} {hauteur}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-docs");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join(format!("{nom}.ppm")), ppm).unwrap();
}

#[test]
fn chaque_vue_s_affiche() {
    let docs = Docs::charger(&dir().join("contenu")).unwrap();
    let accueil = vue("/", "");
    assert!(tout(&accueil).contains("Tout Azure, point par point."));
    capture(&accueil, "accueil");
    capture_avec(&accueil, "accueil-survol-boutons", Some(HeaderButton::Close));
    for s in &docs.sections {
        let v = vue(&format!("/section/{}", s.id), "");
        assert!(tout(&v).contains(&s.titre));
        capture(&v, &format!("section-{}", s.id));
        for p in &s.pages {
            let v = vue(&p.chemin(), "");
            let t = tout(&v);
            assert!(t.contains(&p.titre), "{}", p.chemin());
            // Chaque exemple montre son identifiant et son code (colore :
            // un texte par morceau, recolles ici).
            let colle = t.replace('\n', "");
            for e in p.exemples() {
                assert!(t.contains(&e.id), "{} : identifiant {} absent", p.chemin(), e.id);
                assert!(colle.contains(e.code.lines().next().unwrap_or("").replace('\t', "    ").as_str()), "{} : code de {} absent", p.chemin(), e.id);
            }
            capture(&v, &format!("page-{}-{}", s.id, p.id));
        }
    }
    let r = vue("/recherche", "balise");
    assert!(tout(&r).contains("résultat(s)"));
    capture(&r, "recherche");
}

#[test]
fn les_clics_menent_aux_bonnes_pages() {
    assert_eq!(action("accueil"), Some(Action::Aller("/".into())));
    assert_eq!(action("chercher"), Some(Action::Chercher));
    assert_eq!(action("q"), Some(Action::Chercher), "Entree dans le champ de recherche");
    assert_eq!(action("s-rsc"), Some(Action::Aller("/section/rsc".into())));
    assert_eq!(action("p-rsc__flex"), Some(Action::Aller("/doc/rsc/flex".into())));
    assert_eq!(action("p-demarrer__premiere-app"), Some(Action::Aller("/doc/demarrer/premiere-app".into())));
    assert_eq!(action("copier-rsc_flex_1"), Some(Action::Copier("rsc_flex_1".into())));
    assert_eq!(action("demo-routeur"), Some(Action::Demo("routeur".into())));
    assert_eq!(action("autre"), None);
}

#[test]
fn chaque_exemple_a_son_bouton_copier_et_chaque_demo_son_bouton_ouvrir() {
    use azure_foundation::ui::services::interact::set_button_text;
    let mut page = vue("/doc/app/fenetre", "");
    let texte = tout(&page);
    assert_eq!(texte.matches("[Copier]").count(), 3, "{texte}");
    assert_eq!(texte.matches("[Ouvrir]").count(), 2, "{texte}");
    // Ce que fait ctx.flash sur le bouton.
    assert_eq!(set_button_text(&mut page, &format!("copier-{}", ecrans::cle("app.fenetre.1")), "Copié").as_deref(), Some("Copier"));
    assert_eq!(set_button_text(&mut page, "introuvable", "x"), None);
    capture(&page, "page-demo");
}

/// Boite du premier texte qui commence par `texte`.
fn boite(nodes: &[UiNode], texte: &str) -> (i32, i32, u32, u32) {
    let mut found = None;
    azure_foundation::ui::services::interact::walk(nodes, (0, 0, W, H), &mut |n, b| {
        if let UiNode::Label(l) = n
            && l.text.starts_with(texte)
            && found.is_none()
        {
            found = Some(b);
        }
    });
    found.unwrap_or_else(|| panic!("« {texte} » introuvable"))
}

#[test]
fn les_cartes_sont_cliquables_partout() {
    use azure_foundation::ui::services::interact::button_id_at;
    // Le bouton transparent pose sur la carte recoit le clic, meme sur le
    // texte de la carte.
    let accueil = vue("/", "");
    let (x, y, w, h) = boite(&accueil, "Comprendre Azure");
    assert_eq!(button_id_at(&accueil, x + w as i32 / 2, y + h as i32 / 2, (0, 0, W, H)).as_deref(), Some("p-demarrer__azure"));
    let section = vue("/section/demarrer", "");
    let docs = Docs::charger(&dir().join("contenu")).unwrap();
    let p = &docs.section("demarrer").unwrap().pages[0];
    let (x, y, w, h) = boite(&section, &p.titre);
    assert_eq!(button_id_at(&section, x + w as i32 / 2, y + h as i32 / 2, (0, 0, W, H)), Some(format!("p-demarrer__{}", p.id)));
}

#[test]
fn une_ligne_de_code_trop_longue_defile_a_l_horizontale() {
    use azure_foundation::ui::services::interact::{animate_scroll, scroll_x_at};
    let mut page = vue("/doc/rss/introduction", "");
    // Le premier bloc de code de la page a des lignes plus larges que lui.
    let (x, y, _, _) = boite(&page, "azure_foundation");
    assert!(scroll_x_at(&mut page, x + 40, y + 20, 200.0, (0, 0, W, H)), "le bloc de code doit defiler");
    while animate_scroll(&mut page) {}
    capture(&page, "code-defile");
}

#[test]
fn copier_un_exemple_redonne_le_code_exact() {
    use azure_core::rules::window_event::WindowEvent;
    use azure_foundation::event::models::app_state::EventState;
    use azure_foundation::event::models::keys::{BTN_LEFT, KEY_LEFTCTRL};
    use azure_foundation::event::services::dispatch::handle_event;
    use azure_foundation::ui::services::interact::KeyboardLayout;
    let docs = Docs::charger(&dir().join("contenu")).unwrap();
    let (_, exemple) = docs.exemple("demarrer.introduction.1").unwrap();
    let mut s = EventState::new(vue("/doc/demarrer/introduction", ""));
    let content = (0, 0, W, H);
    // Les morceaux colores du bloc : les textes qui suivent son identifiant.
    let mut textes = Vec::new();
    azure_foundation::ui::services::interact::walk(&s.ui_nodes, content, &mut |n, b| {
        if let UiNode::Label(l) = n {
            textes.push((l.text.clone(), b));
        }
    });
    let debut = textes.iter().position(|(t, _)| t == &exemple.id).unwrap() + 1;
    let nb_lignes = exemple.code.lines().count();
    let mut lignes = Vec::new();
    for (_, b) in &textes[debut..] {
        if !lignes.contains(&b.1) {
            if lignes.len() == nb_lignes {
                break;
            }
            lignes.push(b.1);
        }
    }
    let premier = textes[debut].1;
    let dernier = textes[debut..].iter().rfind(|(_, b)| b.1 == *lignes.last().unwrap()).unwrap().1;
    let ev = |s: &mut EventState, e| handle_event(s, e, KeyboardLayout::Qwerty, content);
    ev(&mut s, WindowEvent::WindowMouseMove(premier.0 + 1, premier.1 + 4));
    ev(&mut s, WindowEvent::WindowMouseButton(BTN_LEFT, true));
    ev(&mut s, WindowEvent::WindowMouseMove(dernier.0 + dernier.2 as i32 + 3, dernier.1 + 4));
    ev(&mut s, WindowEvent::WindowMouseButton(BTN_LEFT, false));
    ev(&mut s, WindowEvent::WindowKeyPress(KEY_LEFTCTRL, true));
    ev(&mut s, WindowEvent::WindowKeyPress(46, true));
    assert_eq!(s.clipboard, exemple.code);
    capture(&s.ui_nodes, "selection-code");
}
