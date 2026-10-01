// Portfolio : les pages (voir app.azure) et le menu, testables sans fenetre.
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::navigation::models::route_table::RouteTable;
use std::path::Path;

/// Les pages : (chemin, fichier dans ui/, nom pour le menu).
pub const PAGES: [(&str, &str, &str); 5] = [
    ("/", "presentation.rsh", "presentation"),
    ("/etudes", "etudes.rsh", "etudes"),
    ("/projets", "projets.rsh", "projets"),
    ("/azure", "azure.rsh", "azure"),
    ("/veille", "veille.rsh", "veille"),
];

/// Le chemin d'un bouton du menu (#nav-<page>).
pub fn target(clicked: &str) -> Option<&'static str> {
    let page = clicked.strip_prefix("nav-")?;
    PAGES.iter().find(|(_, _, name)| *name == page).map(|(path, _, _)| *path)
}

/// Ce qu'ouvre un bouton de la page Azure : (app, chemin).
/// `#doc-<page>` : une page de la section Fonctionnement d'Azure Docs ;
/// `#ouvrir-<app>` : l'accueil de l'app.
pub fn link(clicked: &str) -> Option<(&str, String)> {
    if let Some(page) = clicked.strip_prefix("doc-") {
        return Some(("docs", format!("/doc/fonctionnement/{page}")));
    }
    let app = clicked.strip_prefix("ouvrir-")?;
    OPENS.contains(&app).then(|| (app, "/".to_string()))
}

/// Les apps declarees `[open ...]` dans app.azure.
pub const OPENS: [&str; 3] = ["docs", "note", "dashboard"];

/// Ajoute a `table` les pages de `ui` ; chacune recoit `page` : le menu
/// sait laquelle est ouverte.
pub fn routes(mut table: RouteTable, ui: &Path) -> RouteTable {
    let style = ui.join("app.rsc").to_string_lossy().into_owned();
    for (path, file, name) in PAGES {
        let view = ui.join(file).to_string_lossy().into_owned();
        table = table.view_with(path, &view, &style, move |_| Context::new().with_text("page", name));
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu() {
        assert_eq!(target("nav-presentation"), Some("/"));
        assert_eq!(target("nav-veille"), Some("/veille"));
        assert_eq!(target("nav-inconnue"), None);
        assert_eq!(target("autre"), None);
    }

    #[test]
    fn links() {
        assert_eq!(link("doc-engine"), Some(("docs", "/doc/fonctionnement/engine".to_string())));
        assert_eq!(link("ouvrir-note"), Some(("note", "/".to_string())));
        assert_eq!(link("ouvrir-inconnue"), None);
        assert_eq!(link("nav-azure"), None);
    }
}
