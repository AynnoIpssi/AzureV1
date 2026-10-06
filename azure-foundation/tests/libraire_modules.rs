// Les modules d'interface d'azure-libraire, charges par la fondation :
// chacun se construit, et leurs styles sont une feuille rsC sans reproche.
use azure_foundation::compiler::components::with_default_styles;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse;
use azure_foundation::compiler::rsh::services::lexer::tokenize;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::libraire::interface::modules;
use azure_foundation::ui::models::ui_node::UiNode;

fn texts(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(l) => out.push(l.text.clone()),
            UiNode::Button(b) => out.push(format!("[{}] {}", b.id, b.text)),
            UiNode::Control(c) => out.push(format!("<{:?}#{}> {}", c.kind, c.id, c.label)),
            UiNode::TextArea(t) => out.push(format!("<input#{}> {}", t.id, t.placeholder)),
            UiNode::Container(c) => texts(&c.children, out),
            _ => {}
        }
    }
}

fn build(rsh: &str) -> Vec<String> {
    let sheet = parse_rsc(tokenize_rsc(&with_default_styles("", None))).unwrap();
    let nodes = build_ui_with_context(&parse(tokenize(rsh)).unwrap(), &StyleSource::Rsc(&sheet), &Context::new());
    let mut out = Vec::new();
    texts(&nodes, &mut out);
    out
}

#[test]
fn chaque_module_se_construit() {
    for m in modules() {
        let page = format!("<{0} id=\"x\" titre=\"T\" ouvert=\"true\" items=\"A, B\" pages=\"2\" touches=\"Ctrl\" nom=\"Ada L\">contenu<!{0}>", m.nom);
        let out = build(&page);
        assert!(!out.iter().any(|t| t.contains("inconnu") || t.contains("invalide")), "<{}> : {out:?}", m.nom);
    }
}

#[test]
fn la_feuille_des_modules_est_valide() {
    let sheet = parse_rsc(tokenize_rsc(&azure_foundation::compiler::components::default_styles())).unwrap();
    assert_eq!(azure_foundation::compiler::rsc::warnings(&sheet), Vec::<String>::new());
}

#[test]
fn les_nouveaux_modules_affichent_ce_qu_on_leur_passe() {
    assert_eq!(build("<sidebar titre=\"Menu\"><sidebar-item id=\"a\" actif=\"true\">Accueil<!sidebar-item><!sidebar>"), ["Menu", "[a] Accueil"]);
    assert_eq!(build("<header titre=\"Projets\" description=\"Tous\"><btn id=\"n\">Nouveau<!btn><!header>"), ["Projets", "Tous", "[n] Nouveau"]);
    assert_eq!(build("<setting titre=\"Son\" description=\"Actif\"><switch#son/><!setting>"), ["Son", "Actif", "<Switch#son> "]);
    // Le champ prend l'id du module, le bouton `#id-ok`.
    assert_eq!(build("<search-bar id=\"q\" placeholder=\"Chercher\" bouton=\"OK\"/>"), ["<input#q> Chercher", "[q-ok] OK"]);
    assert_eq!(build("<search-bar id=\"q\"/>"), ["<input#q> "]);
    // Arbre : dossier ouvert, ferme, feuille.
    assert_eq!(build("<tree-item id=\"d\" ouvert=\"true\">src<!tree-item><tree-item id=\"e\" ouvert=\"false\">ui<!tree-item><tree-item id=\"f\" niveau=\"1\">main.rs<!tree-item>"), ["[d] - src", "[e] + ui", "[f] main.rs"]);
    assert_eq!(build("<shortcut label=\"Chercher\" touches=\"Ctrl, K\"/>"), ["Chercher", "Ctrl", "K"]);
    assert_eq!(build("<media nom=\"Ada Lovelace\" titre=\"Ada\" description=\"Analyste\"/>"), ["AL", "Ada", "Analyste"]);
    // Question : fermee, rien ; ouverte, boutons par defaut ou choisis.
    assert_eq!(build("<confirm id=\"c\" titre=\"Supprimer ?\">Definitif.<!confirm>"), Vec::<String>::new());
    assert_eq!(build("<confirm id=\"c\" titre=\"Supprimer ?\" ouvert=\"true\">Definitif.<!confirm>"), ["Supprimer ?", "Definitif.", "[c-fermer] Annuler", "[c-oui] Confirmer"]);
    assert_eq!(build("<confirm id=\"c\" titre=\"Quitter ?\" ouvert=\"true\" oui=\"Quitter\" non=\"Rester\"/>"), ["Quitter ?", "[c-fermer] Rester", "[c-oui] Quitter"]);
}
