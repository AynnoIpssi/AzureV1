// Verifie que CHAQUE apercu live d'Azure Docs passe reellement par le
// pipeline rsC/rsH d'azure-foundation sans erreur de parse, et produit un
// arbre non vide - sans ouvrir de fenetre. azure-docs etant un binaire, les
// modules sont inclus par chemin plutot qu'importes depuis une lib.
#[path = "../src/content/mod.rs"]
#[allow(dead_code)]
mod content;
#[path = "../src/ui/preview.rs"]
#[allow(dead_code)]
mod preview;

use azure_foundation::ui::models::ui_node::UiNode;
use content::Recipe;

fn render(recipe: &Recipe, rsh: &str) -> Vec<UiNode> {
    match (recipe.rsc, recipe.ctx) {
        (Some(rsc), _) => preview::render_rsh_with_rsc(rsh, rsc),
        (None, Some(ctx)) => preview::render_rsh_ctx(rsh, &ctx()),
        (None, None) => preview::render_rsh(rsh),
    }
}

fn texts(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Label(label) => out.push(label.text.clone()),
            UiNode::Container(container) => texts(&container.children, out),
            _ => {}
        }
    }
}

fn check_category(name: &str, recipes: Vec<Recipe>) {
    for recipe in &recipes {
        let Some(rsh) = recipe.rsh else { continue };
        let nodes = render(recipe, rsh);
        let mut found = Vec::new();
        texts(&nodes, &mut found);
        println!("[{name}] {} -> {} noeud(s), textes: {:?}", recipe.title, nodes.len(), found);
        assert!(!nodes.is_empty(), "[{name}] {}: apercu vide", recipe.title);
        assert!(
            !found.iter().any(|t| t.starts_with("Erreur rsC") || t.starts_with("Erreur rsH")),
            "[{name}] {}: {:?}",
            recipe.title,
            found
        );
    }
}

#[test]
fn rsh_previews_compile() {
    check_category("rsH", content::rsh::recipes());
}

#[test]
fn rsc_previews_compile() {
    check_category("rsC", content::rsc::recipes());
}

#[test]
fn router_recipes_are_static() {
    assert!(content::router::recipes().iter().all(|r| r.rsh.is_none() && r.snippet.is_some()));
}

// Chaque apercu de flux de controle doit montrer EXACTEMENT la branche
// attendue - regression : l'indentation entre `<!if>` et `<elseif>`/`<else>`
// cassait la chaine, et le `else` s'affichait en plus de la branche vraie.
#[test]
fn control_flow_previews_show_expected_branch() {
    let expected: &[(&str, &[&str])] = &[
        ("if / elseif / else", &["Bonjour Admin"]),
        ("match / arm", &["Acces limite"]),
        ("for / foreach", &["Nouvelle notification"; 3]),
    ];
    let recipes = content::rsh::recipes();
    for (title, want) in expected {
        let recipe = recipes.iter().find(|r| r.title == *title).expect(title);
        let mut found = Vec::new();
        texts(&render(recipe, recipe.rsh.unwrap()), &mut found);
        assert_eq!(&found, want, "{title}");
    }
}
