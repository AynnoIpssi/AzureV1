// La coloration ne perd ni n'ajoute aucun caractere, et reconnait les
// elements principaux de chaque langage.
use azure_docs::coloration::colorer;
use azure_docs::Docs;
use std::path::Path;

fn genres(langage: &str, code: &str) -> Vec<(&'static str, String)> {
    colorer(langage, code).into_iter().flatten().map(|(g, t)| (g, t.trim().to_string())).collect()
}

fn genre_de(langage: &str, code: &str, morceau: &str) -> &'static str {
    genres(langage, code).into_iter().find(|(_, t)| t == morceau).unwrap_or_else(|| panic!("« {morceau} » introuvable dans {:?}", genres(langage, code))).0
}

#[test]
fn chaque_exemple_se_recompose_a_l_identique() {
    let docs = Docs::charger(&Path::new(env!("CARGO_MANIFEST_DIR")).join("contenu")).unwrap();
    for e in docs.pages().flat_map(|p| p.exemples()) {
        let lignes = colorer(&e.langage, &e.code);
        let rendu: Vec<String> = lignes.iter().map(|l| l.iter().map(|(_, t)| t.as_str()).collect::<String>()).collect();
        let attendu: Vec<String> = e.code.lines().map(|l| if l.is_empty() { " ".to_string() } else { l.replace('\t', "    ") }).collect();
        assert_eq!(rendu, attendu, "{}", e.id);
    }
}

#[test]
fn rsh() {
    let code = r#"<container.page#haut>
    <if.vue == "accueil"><text>Bonjour {{nom}}<!text><!if>
    <search.champ placeholder="Chercher"/>
<!container>"#;
    assert_eq!(genre_de("rsh", code, "container"), "balise");
    assert_eq!(genre_de("rsh", code, "if"), "mot");
    assert_eq!(genre_de("rsh", code, "\"accueil\""), "chaine");
    assert_eq!(genre_de("rsh", code, "{{nom}}"), "interp");
    assert_eq!(genre_de("rsh", code, "placeholder"), "attr");
    assert!(genres("rsh", code).iter().any(|(g, t)| *g == "classe" && t == ".page#haut"));
}

#[test]
fn rsc() {
    let code = ".carte:hover { background-color: #1e1e1e; padding: 8px 12px; } /* note */";
    assert_eq!(genre_de("rsc", code, ".carte"), "classe");
    assert_eq!(genre_de("rsc", code, ":hover"), "mot");
    assert_eq!(genre_de("rsc", code, "background-color"), "attr");
    assert_eq!(genre_de("rsc", code, "#1e1e1e"), "nombre");
    assert_eq!(genre_de("rsc", code, "/* note */"), "commentaire");
}

#[test]
fn rss() {
    let code = "SELECT count(*) FROM @3.notes WHERE titre = 'a' AND id > ? -- fin";
    assert_eq!(genre_de("rss", code, "SELECT"), "mot");
    assert_eq!(genre_de("rss", code, "count"), "fonction");
    assert_eq!(genre_de("rss", code, "'a'"), "chaine");
    assert_eq!(genre_de("rss", code, "@3"), "interp");
    assert_eq!(genre_de("rss", code, "-- fin"), "commentaire");
    assert_eq!(genre_de("rss", "CREATE TABLE t (id INT)", "INT"), "classe");
}

#[test]
fn rust() {
    let code = "fn main() { let db = Db::open(\"x\")?; println!(\"{}\", 42); } // fin";
    assert_eq!(genre_de("rust", code, "fn"), "mot");
    assert_eq!(genre_de("rust", code, "Db"), "classe");
    assert_eq!(genre_de("rust", code, "println!"), "fonction");
    assert_eq!(genre_de("rust", code, "42"), "nombre");
    assert_eq!(genre_de("rust", code, "// fin"), "commentaire");
}

#[test]
fn un_commentaire_peut_couvrir_plusieurs_lignes() {
    let lignes = colorer("rsc", "/* debut\nsuite */\n.a { color: #fff; }");
    assert_eq!(lignes[1][0].0, "commentaire");
    assert_eq!(lignes[2][0].0, "classe");
}
