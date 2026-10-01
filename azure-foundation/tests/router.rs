// Routes facon Laravel (`navigation::models::router::Router`), utilisees par
// `RouteTable` (ecrans) et `WindowTable` (fenetres).
use azure_foundation::navigation::models::route::Route;
use azure_foundation::navigation::models::route_table::RouteTable;
use azure_foundation::navigation::models::router::Router;
use azure_foundation::ui::models::ui_node::UiNode;

fn texts(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(l) => out.push(l.text.clone()),
            UiNode::Container(c) => texts(&c.children, out),
            _ => {}
        }
    }
}

#[test]
fn one_line_routes_and_params() {
    let routes = Router::new()
        .route("/accueil", |_| "accueil".to_string())
        .route("/user/{id}", |r| format!("user {}", r.param("id").unwrap()));
    assert_eq!(routes.dispatch("/accueil", ""), Some("accueil".into()));
    assert_eq!(routes.dispatch("accueil/", ""), Some("accueil".into()));
    assert_eq!(routes.dispatch("/user/7", ""), Some("user 7".into()));
    assert_eq!(routes.dispatch("/user", ""), None);
    assert_eq!(routes.dispatch("/user/7/plus", ""), None);
}

#[test]
fn optional_param_and_payload() {
    let routes = Router::new().route("/logs/{jour?}", |r| format!("{:?} {}", r.param("jour"), r.payload));
    assert_eq!(routes.dispatch("/logs", "p"), Some("None p".into()));
    assert_eq!(routes.dispatch("/logs/lundi", "p"), Some("Some(\"lundi\") p".into()));
}

#[test]
fn where_constraints_let_the_next_route_match() {
    let routes = Router::new()
        .route("/user/{id}", |_| "par id").where_number("id")
        .route("/user/{nom}", |_| "par nom").where_alpha("nom")
        .route("/onglet/{t}", |_| "onglet").where_in("t", &["rsh", "rsc"]);
    assert_eq!(routes.dispatch("/user/42", ""), Some("par id"));
    assert_eq!(routes.dispatch("/user/alice", ""), Some("par nom"));
    assert_eq!(routes.dispatch("/user/a1", ""), None);
    assert_eq!(routes.dispatch("/onglet/rsc", ""), Some("onglet"));
    assert_eq!(routes.dispatch("/onglet/js", ""), None);
}

#[test]
fn groups_nest_and_prefix_redirects() {
    let routes = Router::new().group("/admin", |g| g
        .route("/stats", |_| "stats")
        .redirect("/", "/stats")
        .group("/users", |u| u.route("/{id}", |_| "user").name("admin.user")));
    assert_eq!(routes.dispatch("/admin/stats", ""), Some("stats"));
    assert_eq!(routes.dispatch("/admin", ""), Some("stats"));
    assert_eq!(routes.dispatch("/admin/users/3", ""), Some("user"));
    assert_eq!(routes.dispatch("/stats", ""), None);
    assert_eq!(routes.url("admin.user", &[("id", "3")]), Some("/admin/users/3".into()));
}

#[test]
fn named_urls() {
    let routes = Router::new()
        .route("/user/{id}/{onglet?}", |_| ()).name("user.show")
        .route("/accueil", |_| ()).name("home");
    assert_eq!(routes.url("home", &[]), Some("/accueil".into()));
    assert_eq!(routes.url("user.show", &[("id", "4")]), Some("/user/4".into()));
    assert_eq!(routes.url("user.show", &[("id", "4"), ("onglet", "infos")]), Some("/user/4/infos".into()));
    assert_eq!(routes.url("user.show", &[]), None);
    assert_eq!(routes.url("inconnue", &[]), None);
}

#[test]
fn fallback_redirect_loop_and_overwrite() {
    let routes = Router::new()
        .route("/a", |_| "premier")
        .route("/a", |_| "second")
        .redirect("/x", "/y")
        .redirect("/y", "/x")
        .fallback(|r| if r.path == "/x" { "jamais" } else { "404" });
    assert_eq!(routes.dispatch("/a", ""), Some("second"));
    assert_eq!(routes.dispatch("/nulle-part", ""), Some("404"));
    assert_eq!(routes.dispatch("/x", ""), None, "une boucle de redirections ne doit pas tourner a l'infini");
}

#[test]
fn route_table_view_loads_rsh_with_params() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let routes = RouteTable::new().view("/user/{id}", &format!("{dir}/tests/router/profil.rsh"), &format!("{dir}/tests/router/page.rsc"));

    let mut out = Vec::new();
    texts(&routes.resolve(&Route::new("/user/42", "depuis-menu")).unwrap(), &mut out);
    assert_eq!(out, ["Profil", "Utilisateur 42", "Ouvert depuis le menu"]);

    let mut out = Vec::new();
    texts(&routes.resolve(&Route::new("/user/9", "")).unwrap(), &mut out);
    assert_eq!(out, ["Profil", "Autre utilisateur"]);
}

#[test]
fn route_table_old_on_syntax_still_works() {
    let routes = RouteTable::new().on("/vide", |payload| {
        assert_eq!(payload, "p");
        Vec::new()
    });
    assert_eq!(routes.resolve(&Route::new("/vide", "p")).map(|n| n.len()), Some(0));
    assert!(routes.resolve(&Route::new("/autre", "")).is_none());
}

#[test]
fn named_paths_are_translated_by_the_receiver() {
    use azure_foundation::navigation::models::router::named_path;
    let routes = Router::new()
        .route("/user/{id}/{onglet?}", |r| format!("{} {:?}", r.path, r.param("onglet"))).name("user.show")
        .fallback(|r| format!("404 {}", r.path));
    let path = named_path("user.show", &[("id", "4"), ("onglet", "a=b")]).unwrap();
    assert_eq!(routes.dispatch(&path, ""), Some("/user/4/a=b Some(\"a=b\")".into()));
    assert_eq!(routes.dispatch(&named_path("inconnue", &[]).unwrap(), ""), Some("404 inconnue".into()));
    assert_eq!(routes.dispatch(&named_path("user.show", &[]).unwrap(), ""), Some("404 user.show".into()), "parametre obligatoire manquant");
    assert!(named_path("", &[]).is_err());
    assert!(named_path("a", &[("k", "x\u{1F}y")]).is_err());
}
