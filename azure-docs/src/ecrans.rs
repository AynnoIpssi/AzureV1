// Les ecrans d'Azure Docs : un seul gabarit (ui/docs.rsh + ui/docs.rsc)
// affiche quatre vues selon `vue` :
//
//   /                          accueil : toutes les sections en cartes
//   /section/{section}         une section : ses pages (variable `rubrique`)
//   /doc/{section}/{page}      une page (variable `doc` : `page` et
//                              `section` sont les parametres du chemin)
//   /recherche  (payload = q)  resultats de recherche
//
// Ce module ne construit aucun widget : il fournit les donnees (listes,
// objets) que le gabarit parcourt, et traduit les clics en chemins.
use crate::apercu;
use crate::coloration::colorer;
use crate::contenu::{Bloc, Docs, Page};
use crate::index::{chercher, Genre};
use azure_foundation::compiler::services::condition::{ConditionValue, Context};
use azure_foundation::navigation::models::route_table::{load_view, RouteTable};
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::navigation::models::router::Request;
use std::path::Path;
use std::sync::Arc;

fn texte(s: &str) -> ConditionValue {
    ConditionValue::Text(s.to_string())
}

fn nombre(n: usize) -> ConditionValue {
    ConditionValue::Number(n as f64)
}

fn liste(v: impl IntoIterator<Item = ConditionValue>) -> ConditionValue {
    ConditionValue::List(v.into_iter().collect())
}

fn page_courte(page: &Page, active: bool) -> ConditionValue {
    ConditionValue::map([
        ("id", texte(&page.id)),
        ("section", texte(&page.section)),
        ("titre", texte(&page.titre)),
        ("resume", texte(&page.resume)),
        ("exemples", nombre(page.exemples().count())),
        ("actif", ConditionValue::Bool(active)),
    ])
}

/// Toutes les sections (menu lateral et accueil). `section` / `page` :
/// celles qui sont ouvertes.
fn sections(docs: &Docs, section: &str, page: &str) -> ConditionValue {
    liste(docs.sections.iter().enumerate().map(|(i, s)| {
        ConditionValue::map([
            ("id", texte(&s.id)),
            ("numero", texte(&format!("{:02}", i + 1))),
            ("titre", texte(&s.titre)),
            ("resume", texte(&s.resume)),
            ("nb_pages", nombre(s.pages.len())),
            ("nb_exemples", nombre(s.pages.iter().map(|p| p.exemples().count()).sum())),
            ("actif", ConditionValue::Bool(s.id == section)),
            ("pages", liste(s.pages.iter().map(|p| page_courte(p, s.id == section && p.id == page)))),
        ])
    }))
}

/// Un bloc de page, tel que le gabarit le parcourt (le code : ses lignes
/// de morceaux colores `{k, v}`).
pub fn bloc(b: &Bloc) -> ConditionValue {
    match b {
        Bloc::Titre { niveau, texte: t } => ConditionValue::map([("type", texte(&format!("titre{niveau}"))), ("texte", texte(t))]),
        Bloc::Paragraphe(t) => ConditionValue::map([("type", texte("paragraphe")), ("texte", texte(t))]),
        Bloc::Liste(items) => ConditionValue::map([("type", texte("liste")), ("items", liste(items.iter().map(|i| texte(i))))]),
        Bloc::Note { genre, texte: t } => {
            let label = match genre.as_str() {
                "astuce" => "Astuce",
                "attention" => "Attention",
                _ => "Note",
            };
            ConditionValue::map([("type", texte("note")), ("genre", texte(genre)), ("label", texte(label)), ("texte", texte(t))])
        }
        Bloc::Tableau { entetes, lignes } => ConditionValue::map([
            ("type", texte("tableau")),
            ("entetes", liste(entetes.iter().map(|c| texte(c)))),
            ("lignes", liste(lignes.iter().map(|l| liste(l.iter().map(|c| texte(c)))))),
        ]),
        Bloc::Code(e) => {
            // Une liste de lignes, chaque ligne une liste de morceaux colores.
            let lignes = colorer(&e.langage, &e.code).into_iter().map(|l| liste(l.into_iter().map(|(genre, t)| ConditionValue::map([("k", texte(genre)), ("v", texte(&t))]))));
            ConditionValue::map([("type", texte("code")), ("langage", texte(&e.langage)), ("id", texte(&e.id)), ("cle", texte(&cle(&e.id))), ("titre", texte(&e.titre)), ("lignes", liste(lignes))])
        }
        // Le rendu lui-meme est insere apres coup (voir `apercu::inserer`).
        Bloc::Apercu(a) => ConditionValue::map([("type", texte("apercu")), ("n", texte(&a.n.to_string())), ("legende", texte(&a.legende))]),
        Bloc::Demo { nom, texte: t } => ConditionValue::map([("type", texte("demo")), ("nom", texte(nom)), ("texte", texte(t))]),
    }
}

fn totaux(docs: &Docs) -> Context {
    Context::new()
        .with_number("nb_sections", docs.sections.len() as f64)
        .with_number("nb_pages", docs.pages().count() as f64)
        .with_number("nb_exemples", docs.pages().map(|p| p.exemples().count()).sum::<usize>() as f64)
}

pub fn accueil(docs: &Docs) -> Context {
    totaux(docs).with_text("vue", "accueil").with_text("q", "").with_value("sections", sections(docs, "", ""))
}

pub fn section(docs: &Docs, id: &str) -> Context {
    let Some(s) = docs.section(id) else { return accueil(docs) };
    let info = ConditionValue::map([("id", texte(&s.id)), ("titre", texte(&s.titre)), ("resume", texte(&s.resume)), ("pages", liste(s.pages.iter().map(|p| page_courte(p, false))))]);
    totaux(docs).with_text("vue", "section").with_text("q", "").with_value("sections", sections(docs, id, "")).with_value("rubrique", info)
}

pub fn page(docs: &Docs, section_id: &str, id: &str) -> Context {
    let (Some(s), Some(p)) = (docs.section(section_id), docs.page(section_id, id)) else { return accueil(docs) };
    let (avant, apres) = docs.voisines(section_id, id);
    let voisine = |v: Option<&Page>| match v {
        Some(v) => ConditionValue::map([("existe", ConditionValue::Bool(true)), ("id", texte(&v.id)), ("section", texte(&v.section)), ("titre", texte(&v.titre))]),
        None => ConditionValue::map([("existe", ConditionValue::Bool(false))]),
    };
    let info = ConditionValue::map([
        ("id", texte(&p.id)),
        ("titre", texte(&p.titre)),
        ("resume", texte(&p.resume)),
        ("section", texte(&s.id)),
        ("section_titre", texte(&s.titre)),
        ("blocs", liste(p.blocs.iter().map(bloc))),
        ("precedente", voisine(avant)),
        ("suivante", voisine(apres)),
    ]);
    totaux(docs).with_text("vue", "page").with_text("q", "").with_value("sections", sections(docs, section_id, id)).with_value("doc", info)
}

pub fn recherche(docs: &Docs, q: &str) -> Context {
    let resultats = chercher(docs, q);
    let n = resultats.len();
    let items = resultats.into_iter().take(60).map(|r| {
        ConditionValue::map([
            ("genre", texte(if r.genre == Genre::Exemple { "exemple" } else { "page" })),
            ("id", texte(&r.id)),
            ("titre", texte(&r.titre)),
            ("lieu", texte(&r.lieu)),
            ("extrait", texte(&r.extrait)),
            // Bouton : la page du resultat.
            ("cible", texte(&r.chemin.trim_start_matches("/doc/").replacen('/', "__", 1))),
        ])
    });
    totaux(docs).with_text("vue", "recherche").with_text("q", q).with_number("nb_resultats", n as f64).with_value("resultats", liste(items)).with_value("sections", sections(docs, "", ""))
}

/// Les routes de l'app. La documentation est relue a chaque affichage : une
/// page modifiee se voit sans relancer l'app.
pub fn routes(ui: &Path, contenu: &Path) -> RouteTable {
    let rsh = ui.join("docs.rsh").to_string_lossy().into_owned();
    let rsc = ui.join("docs.rsc").to_string_lossy().into_owned();
    let contenu: Arc<Path> = contenu.into();
    let vue = |f: fn(&Docs, &Request) -> Context| {
        let contenu = Arc::clone(&contenu);
        move |request: &Request| match Docs::charger(&contenu) {
            Ok(docs) => f(&docs, request),
            Err(e) => {
                eprintln!("azure-docs : contenu illisible : {e}");
                Context::new().with_text("vue", "erreur").with_text("erreur", &e).with_text("q", "").with_value("sections", liste([]))
            }
        }
    };
    RouteTable::new()
        .view_with("/", &rsh, &rsc, vue(|d, _| accueil(d)))
        .view_with("/section/{section}", &rsh, &rsc, vue(|d, r| section(d, r.param("section").unwrap_or(""))))
        .route("/doc/{section}/{page}", {
            let (rsh, rsc, base) = (rsh.clone(), rsc.clone(), ui.join("apercu.rsc"));
            let contenu = Arc::clone(&contenu);
            move |r| page_avec_apercus(&contenu, &rsh, &rsc, &base, r)
        })
        .view_with("/recherche", &rsh, &rsc, vue(|d, r| recherche(d, &r.payload)))
}

/// L'identifiant d'un exemple dans un `#id` rsH (`rsc.flex.1` ->
/// `rsc_flex_1`) : le point y separe les classes.
pub fn cle(id: &str) -> String {
    id.replace('.', "_")
}

/// Une page de la documentation, avec le rendu de ses apercus rsC.
fn page_avec_apercus(contenu: &Path, rsh: &str, rsc: &str, base: &Path, r: &Request) -> Vec<UiNode> {
    let (section_id, id) = (r.param("section").unwrap_or(""), r.param("page").unwrap_or(""));
    let docs = match Docs::charger(contenu) {
        Ok(docs) => docs,
        Err(e) => {
            eprintln!("azure-docs : contenu illisible : {e}");
            return Vec::new();
        }
    };
    let mut nodes = match load_view(rsh, rsc, r, page(&docs, section_id, id)) {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("azure-docs : page {} : {e}", r.path);
            return Vec::new();
        }
    };
    if let Some(p) = docs.page(section_id, id) {
        let base = std::fs::read_to_string(base).unwrap_or_default();
        apercu::inserer(&mut nodes, p, &base);
    }
    nodes
}

/// Ce qu'un clic demande.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Aller a ce chemin.
    Aller(String),
    /// Chercher le texte du champ `#q`.
    Chercher,
    /// Copier l'exemple de cette cle (voir `cle`).
    Copier(String),
    /// Ouvrir la demonstration `nom` (voir `demos`).
    Demo(String),
}

/// Traduit l'`#id` du bouton clique :
/// `accueil`, `chercher` (ou `q`, Entree dans le champ), `s-<section>`, `p-<section>__<page>`,
/// `copier-<cle>`, `demo-<nom>`.
pub fn action(clique: &str) -> Option<Action> {
    match clique {
        "accueil" => Some(Action::Aller("/".to_string())),
        "chercher" | "q" => Some(Action::Chercher),
        _ => {
            if let Some(c) = clique.strip_prefix("copier-") {
                return Some(Action::Copier(c.to_string()));
            }
            if let Some(d) = clique.strip_prefix("demo-") {
                return Some(Action::Demo(d.to_string()));
            }
            if let Some(s) = clique.strip_prefix("s-") {
                return Some(Action::Aller(format!("/section/{s}")));
            }
            let (s, p) = clique.strip_prefix("p-")?.split_once("__")?;
            Some(Action::Aller(format!("/doc/{s}/{p}")))
        }
    }
}
