// Les composants rsH : galerie dessinee hors fenetre (target/tmp/
// galerie.ppm), champs interactifs, valeurs lues par l'app, composants de
// l'app, erreurs.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::control::ControlKind;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::form::{form_values, FieldValue};

fn dir() -> String {
    format!("{}/tests/components", env!("CARGO_MANIFEST_DIR"))
}

fn galerie() -> Vec<UiNode> {
    let d = dir();
    RouteTable::new().view("/", &format!("{d}/galerie.rsh"), &format!("{d}/galerie.rsc")).resolve(&Route::new("/", "")).unwrap()
}

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

pub fn save_ppm(nodes: &[UiNode], w: u32, h: u32, name: &str) {
    let mut canvas = Canvas::new(w, h);
    draw_ui(nodes, (0, 0, w, h), &mut canvas, -1, -1, false);
    let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
    for px in canvas.buffer.chunks(4) {
        ppm.extend_from_slice(&[px[2], px[1], px[0]]);
    }
    std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.ppm")), ppm).unwrap();
}

#[test]
fn the_gallery_uses_every_component() {
    let nodes = galerie();
    save_ppm(&nodes, 1200, 3600, "galerie");
    let mut all = Vec::new();
    texts(&nodes, &mut all);
    let has = |s: &str| all.iter().any(|t| t.contains(s));
    for expected in [
        "Azure UI", "[nav-accueil] Accueil", "AL", "Galerie", "Composants", "<input#email> vous@exemple.fr", "8 caracteres minimum", "Jamais partagee.",
        "<Select#pays>", "<Checkbox#cgu> J'accepte les conditions", "<Radio#> M", "<Switch#sombre> Theme sombre", "<Slider#volume>", "<Segmented#vue>",
        "<Rating#note> 4 / 5", "[envoyer] Envoyer", "[aide] Besoin d'aide ?", "12 480", "+8 % ce mois", "72 / 100 Go", "<Progress#>", "Nouveau", "En ligne", "rsH",
        "[retirer-rsh] x", "Ctrl", "Impossible de joindre le serveur.", "Le lien est dans le presse-papiers.", "[onglets-1] Securite", "Confirmation", "[page-6] 6",
        "Carte simple", "Le contenu passe entre les balises.", "Mise a jour il y a 2 min", "Grace Hopper", "Amirale - Arlington", "GH", "Aucun message", "[ecrire] Ecrire",
        "Facture 2024-118", "Payee", "Nom", "Alan", "[faq-1] Comment ca marche ?", "Chaque composant est un fichier rsH", "[faq-2] Et les styles ?", "Expediee",
        "Grace Hopper", "let app = azure_app!()?;", "Version", "1.4.2", "Ce texte vient de parts/pied.rsh (include).", "Construisez vite", "[commencer] Commencer",
        "Connecte", "Essentiel", "+ Apps illimitees", "[pro] Choisir", "<inconnu> : composant inconnu", "Azure UI - galerie",
    ] {
        assert!(has(expected), "manque : {expected}\n{all:#?}");
    }
    // Accordeon ferme : son contenu n'est pas construit. Chaque texte "Accueil" : breadcrumb separe.
    assert!(!has("Invisible tant que ferme."));
    // Les etapes : 1 et 2 faites, 3 en cours.
    assert!(has("3") && has("Paiement"));
}

#[test]
fn field_values_are_read_by_name() {
    let values = form_values(&galerie());
    assert_eq!(values.get("email"), Some(&FieldValue::Text(String::new())));
    assert_eq!(values.get("mdp"), Some(&FieldValue::Text("secret".into())));
    assert_eq!(values.get("quantite"), Some(&FieldValue::Text("3".into())));
    assert_eq!(values.get("pays"), Some(&FieldValue::Text("be".into())));
    assert_eq!(values.get("cgu"), Some(&FieldValue::Bool(true)));
    assert_eq!(values.get("news"), Some(&FieldValue::Bool(false)));
    assert_eq!(values.get("taille"), Some(&FieldValue::Text("M".into())), "le radio coche du groupe");
    assert_eq!(values.get("volume"), Some(&FieldValue::Number(60.0)));
    assert_eq!(values.get("vue"), Some(&FieldValue::Text("Semaine".into())));
    assert_eq!(values.get("note"), Some(&FieldValue::Number(4.0)));
}

fn find(nodes: &[UiNode], kind: ControlKind) -> Option<&azure_foundation::ui::models::control::Control> {
    nodes.iter().find_map(|n| match n {
        UiNode::Control(c) if c.kind == kind => Some(c),
        UiNode::Container(c) => find(&c.children, kind),
        _ => None,
    })
}

#[test]
fn builtin_components_are_listed_and_styles_warn_about_mistakes() {
    let library = azure_foundation::compiler::components::Library::new().with_dir(format!("{}/components", dir()));
    let names = library.names();
    assert!(names.len() >= 53, "{} composants", names.len());
    for name in ["card", "tabs", "table", "price", "modal", "drawer", "toasts", "fiche"] {
        assert!(names.contains(&name.to_string()), "{name}");
    }
    // Les styles par defaut sont une feuille rsC valide, sans propriete inutile.
    use azure_foundation::compiler::rsc::mangers::parser::parse;
    use azure_foundation::compiler::rsc::services::lexer::tokenize;
    let sheet = parse(tokenize(azure_foundation::compiler::components::DEFAULT_STYLES)).unwrap();
    assert_eq!(azure_foundation::compiler::rsc::warnings(&sheet), Vec::<String>::new());
    let sheet = parse(tokenize(".x { colour: red; cursor: pointer; color: #fff; }")).unwrap();
    assert_eq!(azure_foundation::compiler::rsc::warnings(&sheet), ["propriete 'colour' inconnue, ignoree"]);
    let _ = find(&[], ControlKind::Slider);
}

fn build(rsh: &str, library: Option<azure_foundation::compiler::components::Library>) -> Vec<String> {
    use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
    use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
    use azure_foundation::compiler::rsh::mangers::parser::parse;
    use azure_foundation::compiler::rsh::services::lexer::tokenize;
    use azure_foundation::compiler::services::codegen::StyleSource;
    use azure_foundation::compiler::services::condition::Context;
    use azure_foundation::compiler::services::interpreter::build_ui_with_context;
    let sheet = parse_rsc(tokenize_rsc(&azure_foundation::compiler::components::with_default_styles("", library.as_ref()))).unwrap();
    let mut ctx = Context::new().with_text("statut", "danger").with_text("qui", "Ada");
    if let Some(library) = library {
        ctx = ctx.with_library(std::sync::Arc::new(library));
    }
    let nodes = build_ui_with_context(&parse(tokenize(rsh)).unwrap(), &StyleSource::Rsc(&sheet), &ctx);
    let mut out = Vec::new();
    texts(&nodes, &mut out);
    out
}

#[test]
fn components_get_attributes_content_and_slots() {
    // Attributs, contenu, slot avec le contexte de la page, id en attribut.
    assert_eq!(build("<card titre=\"Bonjour {{qui}}\"><text>pour {{qui}}<!text><!card>", None), ["Bonjour Ada", "pour Ada"]);
    assert_eq!(build("<chip id=\"x\">rsH<!chip>", None), ["rsH", "[x] x"]);
    // Liste et intervalle.
    assert_eq!(build("<tabs id=\"t\" items=\"A, B\" actif=\"B\"/>", None), ["[t-0] A", "[t-1] B"]);
    assert_eq!(build("<pagination id=\"p\" pages=\"3\" page=\"3\"/>", None), ["[p-1] 1", "[p-2] 2", "[p-3] 3"]);
    // Un composant de l'app remplace celui d'Azure du meme nom ; un composant
    // qui s'utilise lui-meme est arrete proprement.
    let mine = || azure_foundation::compiler::components::Library::new().with_dir(format!("{}/tests/components/moi/components", env!("CARGO_MANIFEST_DIR")));
    assert_eq!(build("<card titre=\"X\"/>", Some(mine())), ["Ma carte : X"]);
    let recursion = build("<boucle/>", Some(mine()));
    assert!(recursion.iter().any(|t| t.contains("imbriques trop profondement")), "{recursion:?}");
    // include introuvable : erreur visible.
    let missing = build("<include src=\"/nulle/part.rsh\"/>", None);
    assert!(missing[0].contains("/nulle/part.rsh"), "{missing:?}");
}

#[test]
fn computed_classes_pick_the_right_style() {
    use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
    use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
    use azure_foundation::compiler::rsh::mangers::parser::parse;
    use azure_foundation::compiler::rsh::services::lexer::tokenize;
    use azure_foundation::compiler::services::codegen::StyleSource;
    use azure_foundation::compiler::services::condition::Context;
    use azure_foundation::compiler::services::interpreter::build_ui_with_context;
    let sheet = parse_rsc(tokenize_rsc(".b.danger { color: #ff0000; } .b.ok { color: #00ff00; }")).unwrap();
    let ctx = Context::new().with_text("statut", "danger");
    let nodes = build_ui_with_context(&parse(tokenize("<text.b.{{statut}}>x<!text>")).unwrap(), &StyleSource::Rsc(&sheet), &ctx);
    let UiNode::Label(label) = &nodes[0] else { panic!() };
    assert_eq!((label.color.r, label.color.g), (255, 0));
}

#[test]
fn codegen_builds_components_at_runtime() {
    use azure_foundation::compiler::rsh::mangers::parser::parse;
    use azure_foundation::compiler::rsh::services::lexer::tokenize;
    let code = azure_foundation::compiler::services::codegen::generate_with_rsc(&parse(tokenize("<card titre=\"x\"/>")).unwrap(), &Default::default());
    // Le composant est construit par l'interpreteur, au lancement (voir
    // tests/codegen_dynamic.rs pour l'egalite du rendu).
    assert!(code.contains("build_nodes") && code.contains("tag: \"card\".to_string()"), "{code}");
}
