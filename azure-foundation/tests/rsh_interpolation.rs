use azure_foundation::compiler::services::condition::{evaluate, interpolate, ConditionValue, Context};
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::ui::models::ui_node::UiNode;

fn collect(nodes: &[UiNode], texts: &mut Vec<String>, buttons: &mut Vec<(String, String)>) {
    for node in nodes {
        match node {
            UiNode::Label(l) => texts.push(l.text.clone()),
            UiNode::Button(b) => buttons.push((b.id.clone(), b.text.clone())),
            UiNode::Container(c) => collect(&c.children, texts, buttons),
            _ => {}
        }
    }
}

fn app(nom: &str, actif: bool, pid: u32) -> ConditionValue {
    ConditionValue::map([("nom", nom.into()), ("actif", actif.into()), ("pid", pid.into())])
}

#[test]
fn a_page_shows_its_data_with_loops_objects_and_ids() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let routes = RouteTable::new().view_with("/apps/{titre}", &format!("{dir}/tests/rsh/apps.rsh"), &format!("{dir}/tests/rsh/apps.rsc"), |_| {
        Context::new().with_value("apps", ConditionValue::List(vec![app("notes", true, 42), app("caisse", false, 0)])).with_value("total", 2i64)
    });
    let (mut texts, mut buttons) = (Vec::new(), Vec::new());
    collect(&routes.resolve(&Route::new("/apps/Mes apps", "")).unwrap(), &mut texts, &mut buttons);
    assert_eq!(texts, ["Mes apps (2 apps)", "0. notes", "actif (pid 42)", "1. caisse", "arrete", "Inconnu : [] - fin"]);
    assert_eq!(buttons, [("ouvrir-notes".to_string(), "Ouvrir notes".to_string()), ("ouvrir-caisse".to_string(), "Ouvrir caisse".to_string())]);
}

#[test]
fn interpolate_and_dotted_conditions() {
    let ctx = Context::new()
        .with_value("app", ConditionValue::map([("nom", "notes".into()), ("version", 1.5.into()), ("tags", ConditionValue::List(vec!["a".into(), "b".into()]))]))
        .with_value("n", 3i64);
    assert_eq!(interpolate("{{app.nom}} v{{app.version}} [{{app.tags}}] x{{n}} {{app.tags.1}}", &ctx), "notes v1.5 [a, b] x3 b");
    assert_eq!(interpolate("pas {{ferme", &ctx), "pas {{ferme");
    assert_eq!(interpolate("sans rien", &ctx), "sans rien");
    assert_eq!(evaluate("app.nom == \"notes\" && n > 2", &ctx), Some(true));
    assert_eq!(evaluate("app.absent == 1", &ctx), None);
    // Un nombre a virgule reste un nombre.
    assert_eq!(evaluate("app.version == 1.5", &ctx), Some(true));
}
