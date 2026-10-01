// Test d'integration "visuel" : ouvre une vraie fenetre Wayland avec
// l'arbre .rsh de window_demo.rsh, entierement style par window_demo.rsc via
// le systeme de lien rsC (services::link + services::interpreter::build_ui).
//
// Ignore par defaut (comme azure_foundation::tests::test_window) car il
// bloque jusqu'a la fermeture manuelle de la fenetre : ni `cargo test` en
// CI ni une session sans compositeur Wayland ne peuvent le faire aboutir.
// A lancer explicitement, depuis azure-foundation/, avec une session
// graphique disponible :
//
//   cargo test --test window_integration_test -- --ignored --nocapture
//
// La fenetre doit montrer : un fond `.hero`, trois niveaux de titre et un
// texte positionnes independamment (selecteurs de type), une `.card`
// contenant deux boutons `.primary-button` positionnes chacun par leur
// propre id (`#btn-users`/`#btn-logs`), et un placeholder image/video.
// Fermer la fenetre (bouton de fermeture) termine le test normalement.
use std::fs;

use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::window::models::window::AzureWindow;

#[test]
#[ignore = "ouvre une vraie fenetre et bloque jusqu'a sa fermeture manuelle"]
fn opens_a_window_with_every_rsh_element_styled_by_rsc() {
    let rsh_src = fs::read_to_string("examples/window_demo.rsh").expect("lecture window_demo.rsh");
    let rsc_src = fs::read_to_string("examples/window_demo.rsc").expect("lecture window_demo.rsc");

    let rsc_sheet = parse_rsc(tokenize_rsc(&rsc_src)).expect("window_demo.rsc devrait parser");
    let ast = parse_rsh(tokenize_rsh(&rsh_src)).expect("window_demo.rsh devrait parser");
    let nodes = build_ui(&ast, &StyleSource::Rsc(&rsc_sheet));

    AzureWindow::new("rsH + rsC - test d'integration")
        .size(1000, 700)
        .ui(nodes)
        .run();
}

// Meme contenu que ci-dessus, mais avec la barre d'en-tete auto-dessinee
// en plus (voir `window::models::header_bar`/`window::services::draw_header`) :
// icone d'application (le PNG a la racine du workspace, converti depuis
// le JPEG d'origine - voir la conversation), titre, et les 3 boutons
// (reduire/plein ecran/fermer). Partie volontairement PARTIELLE de la
// gestion de fenetre - a completer une fois le rooter integre (voir la
// discussion) : pas encore de menu, d'onglets, ni de vraie gestion
// multi-fenetre.
//
//   cargo test --test window_integration_test -- --ignored --nocapture opens_a_window_with_a_custom_header
//
// A verifier a l'oeil : icone + titre en haut a gauche, 3 boutons en haut
// a droite avec un fond au survol - "-" minimise reellement la fenetre,
// le carre passe en plein ecran (et se change en icone "restaurer" une
// fois qu'on y est), la croix ferme la fenetre. Le contenu (.hero, boutons,
// textarea...) doit commencer SOUS la barre, jamais derriere.
#[test]
#[ignore = "ouvre une vraie fenetre et bloque jusqu'a sa fermeture manuelle"]
fn opens_a_window_with_a_custom_header_icon_title_and_the_three_buttons() {
    let rsh_src = fs::read_to_string("examples/window_demo.rsh").expect("lecture window_demo.rsh");
    let rsc_src = fs::read_to_string("examples/window_demo.rsc").expect("lecture window_demo.rsc");

    let rsc_sheet = parse_rsc(tokenize_rsc(&rsc_src)).expect("window_demo.rsc devrait parser");
    let ast = parse_rsh(tokenize_rsh(&rsh_src)).expect("window_demo.rsh devrait parser");
    let nodes = build_ui(&ast, &StyleSource::Rsc(&rsc_sheet));

    AzureWindow::new("Azure - en-tete + icone + rsH/rsC")
        .size(1000, 700)
        .icon("../luffy_picture_test.png")
        .ui(nodes)
        .run();
}

