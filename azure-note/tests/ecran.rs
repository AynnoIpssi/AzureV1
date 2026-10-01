// L'ecran v2, comme dans l'app : un clic sur un bouton de l'ecran range les
// champs, fait l'action, puis l'ecran est reconstruit. Images dans
// target/tmp/azure-note/.
use azure_foundation::event::models::app_state::EventState;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::form::{form_values, FieldValue};
use azure_note::carnet::maintenant;
use azure_note::classeur::Classeur;
use azure_note::clics::{agir, cliquer as action, deposer, Lecture, Suite};
use azure_note::ecrans::routes;
use azure_note::page::Etat;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

mod common;
use common::*;

const VUE: (u32, u32, u32, u32) = (0, 0, 1280, 860);

/// L'id de la zone de texte qui a le focus.
/// Les ids des champs de texte a l'ecran.
fn champs_ecran(nodes: &[UiNode]) -> Vec<String> {
    nodes.iter().flat_map(|n| match n {
        UiNode::TextArea(t) => vec![t.id.clone()],
        UiNode::Container(c) => champs_ecran(&c.children),
        _ => Vec::new(),
    }).collect()
}

fn focalise(nodes: &[UiNode]) -> Option<String> {
    nodes.iter().find_map(|n| match n {
        UiNode::TextArea(t) if t.focused => Some(t.id.clone()),
        UiNode::Container(c) => focalise(&c.children),
        _ => None,
    })
}

struct Valeurs(BTreeMap<String, FieldValue>);

impl Lecture for Valeurs {
    fn valeur(&self, id: &str) -> Option<String> {
        self.0.get(id).map(FieldValue::as_text)
    }
}

struct Appli {
    classeur: Classeur,
    etat: Arc<Mutex<Etat>>,
    ecran: EventState,
}

impl Appli {
    fn new() -> Appli {
        let classeur = Classeur::en_memoire();
        let etat = Arc::new(Mutex::new(Etat::default()));
        let ecran = EventState::new(Vec::new());
        let mut a = Appli { classeur, etat, ecran };
        a.redessiner();
        a
    }

    fn redessiner(&mut self) {
        let nodes = routes(&ui(), &self.classeur, Arc::clone(&self.etat)).resolve(&Route::new("/", "")).unwrap();
        self.ecran = EventState::new(nodes);
    }

    /// Clique sur `#id` a l'ecran, comme `on_click` de main.rs.
    fn clic(&mut self, id: &str) {
        let touche = cliquer(&mut self.ecran, id, VUE);
        assert_eq!(touche.as_deref(), Some(id), "clic sur #{id} : {:?}", boutons(&self.ecran.ui_nodes, VUE));
        let lecture = Valeurs(form_values(&self.ecran.ui_nodes));
        let suite = agir(&self.classeur, &self.etat, &lecture, |e, etat| action(e, etat, id, &lecture, maintenant()));
        if suite == Suite::Redessiner {
            self.redessiner();
        }
    }

    /// « + Ajouter une propriété » : dans les options de la page (ou l'en-tete d'une base).
    fn ouvrir_proprietes(&mut self) {
        if !boutons(&self.ecran.ui_nodes, VUE).contains(&"prop-ouvrir".to_string()) {
            self.clic("options");
        }
        self.clic("prop-ouvrir");
    }

    /// Ajoute une propriete comme a la souris : la liste des types, un clic
    /// sur le type (par son libelle), puis son nom et, selon le type, ses
    /// options (« a, b ») ou sa formule ; Entree dans le nom pour finir.
    fn ajouter_propriete(&mut self, nom: &str, libelle: &str, config: &str) {
        use azure_note::modele::Genre;
        let code = *Genre::CODES.iter().find(|c| Genre::depuis(c, "").libelle() == libelle).unwrap_or_else(|| panic!("type {libelle}"));
        self.ouvrir_proprietes();
        self.clic(&format!("prop-nouveau-{code}"));
        assert_eq!(focalise(&self.ecran.ui_nodes).as_deref(), Some("carte-nom"), "le nom est pret a etre tape");
        self.remplir("carte-nom", nom);
        match code {
            "selection" | "etiquettes" => {
                for o in config.split(',').map(str::trim).filter(|o| !o.is_empty()) {
                    self.remplir("carte-opt", o);
                    self.activer("carte-opt");
                }
            }
            "formule" => {
                self.remplir("carte-formule", config);
                self.activer("carte-formule");
            }
            _ => {}
        }
        self.activer("carte-nom");
    }

    /// Choisit `option` pour la propriete `cle` de `page` : ouvrir la valeur,
    /// cliquer l'option.
    fn choisir(&mut self, page: i64, cle: &str, option: &str) {
        use azure_note::modele::Genre;
        let fid = format!("v-{page}-{cle}");
        self.clic(&format!("ouvrir-{fid}"));
        let d = self.classeur.lire(|e| e.definition(page, cle)).unwrap();
        let (Genre::Selection(o) | Genre::Etiquettes(o)) = d.genre else { panic!("{cle} n'a pas d'options") };
        let i = o.iter().position(|x| x == option).unwrap_or_else(|| panic!("{option} pas dans {o:?}"));
        self.clic(&format!("choix-{fid}@{i}"));
    }

    /// Cree (ou reprend) une option en la tapant dans le choix ouvert.
    fn creer_option(&mut self, page: i64, cle: &str, option: &str) {
        let id = format!("creer-v-{page}-{cle}");
        if !champs_ecran(&self.ecran.ui_nodes).contains(&id) {
            self.clic(&format!("ouvrir-v-{page}-{cle}"));
        }
        self.remplir(&id, option);
        self.activer(&id);
    }

    /// Une activation qui vient du clavier (`/`, Entree), comme `on_click`.
    fn activer(&mut self, id: &str) {
        let lecture = Valeurs(form_values(&self.ecran.ui_nodes));
        if agir(&self.classeur, &self.etat, &lecture, |e, etat| action(e, etat, id, &lecture, maintenant())) == Suite::Redessiner {
            self.redessiner();
        }
    }

    fn deposer(&mut self, source: &str, cible: &str, position: usize) {
        let lecture = Valeurs(form_values(&self.ecran.ui_nodes));
        agir(&self.classeur, &self.etat, &lecture, |e, etat| deposer(e, etat, source, cible, position).map(|_| Suite::Redessiner));
        self.redessiner();
    }

    /// Remplit le champ `#id` (texte, ou choix d'une liste).
    fn remplir(&mut self, id: &str, valeur: &str) {
        fn aller(nodes: &mut [UiNode], id: &str, valeur: &str) -> bool {
            for n in nodes {
                match n {
                    UiNode::TextArea(t) if t.id == id => {
                        t.text = valeur.to_string();
                        if let Some(r) = &mut t.rich {
                            r.styles = vec![Default::default(); valeur.chars().count()];
                        }
                        return true;
                    }
                    UiNode::Control(c) if c.id == id => {
                        c.selected = c.options.iter().position(|(v, _)| v == valeur).unwrap_or_else(|| panic!("{valeur} pas dans {:?}", c.options));
                        return true;
                    }
                    UiNode::Container(c) => {
                        if aller(&mut c.children, id, valeur) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }
        assert!(aller(&mut self.ecran.ui_nodes, id, valeur), "champ #{id} absent");
    }

    fn textes(&self) -> Vec<String> {
        textes(&self.ecran.ui_nodes)
    }

    fn page(&self) -> i64 {
        self.etat.lock().unwrap().page.unwrap()
    }

    fn erreur(&self) -> Option<String> {
        self.etat.lock().unwrap().erreur.clone()
    }

    fn capture(&self, nom: &str) {
        capture(&self.ecran.ui_nodes, (VUE.2, VUE.3), nom);
    }
}

#[test]
fn sans_page_on_propose_d_en_creer_une() {
    let a = Appli::new();
    assert!(a.textes().contains(&"Aucune page".to_string()));
    a.capture("v2-vide");
}

#[test]
fn une_page_avec_tous_les_blocs() {
    let mut a = Appli::new();
    a.clic("nouvelle");
    // Pret a ecrire le titre ; Entree descend au 1er bloc.
    assert!(focalise(&a.ecran.ui_nodes).as_deref() == Some("titre"));
    a.remplir("titre", "Projet Azure");
    a.activer("entree-titre@12");
    assert_eq!(focalise(&a.ecran.ui_nodes).as_deref(), Some("b-1"));
    // Chaque bloc par `/` : taper « / », choisir dans le menu.
    let genres = ["titre1", "texte", "puce", "puce", "numero", "numero", "tache", "citation", "code", "tableau", "separateur"];
    let textes = ["Objectifs", "Une page Notion-like, écrite en Rust.", "Pages imbriquées", "Bases et vues", "Étape un", "Étape deux", "Tester l'app", "Rien n'est plus simple que du Rust bien rangé."];
    for (i, genre) in genres.iter().enumerate() {
        let id = i as i64 + 1;
        if i > 0 {
            // Un clic sous les blocs : un nouveau bloc pour ecrire.
            a.clic("zone-ecrire");
        }
        a.remplir(&format!("b-{id}"), "/");
        a.activer(&format!("slash-b-{id}"));
        assert!(a.textes().contains(&"INSÉRER UN BLOC".to_string()));
        if i == 1 {
            a.capture("v2-menu");
        }
        a.clic(&format!("menu-{genre}"));
        assert!(!a.textes().contains(&"INSÉRER UN BLOC".to_string()), "menu ferme");
        if let Some(t) = textes.get(i) {
            a.remplir(&format!("b-{id}"), t);
        }
    }
    a.remplir("b-9", "fn main() {\n    println!(\"salut\"); // bonjour\n}");
    a.remplir("lang-9", "rust");
    a.remplir("c-10-1-0", "Pages");
    a.remplir("c-10-1-1", "fait");
    // Une propriete sur la page.
    a.ajouter_propriete("Avancement", "Nombre", "");
    assert_eq!(a.erreur(), None);
    let p = a.page();
    a.remplir(&format!("v-{p}-avancement"), "40");
    // Le separateur a laisse un texte neuf dessous (12) : une sous-page par `/`.
    a.remplir("b-12", "/");
    a.activer("slash-b-12");
    a.clic("menu-souspage");
    assert_ne!(a.page(), p, "la sous-page est ouverte");
    a.clic(&format!("p-{p}"));
    let t = a.textes();
    for attendu in ["Projet Azure", "Objectifs", "Étape deux", "Avancement", "40", "1.", "2.", "Pages", "Sans titre"] {
        assert!(t.iter().any(|x| x == attendu), "« {attendu} » absent : {t:?}");
    }
    let page = a.classeur.lire(|e| e.page(p).cloned()).unwrap();
    assert_eq!(page.blocs[8].contenu, "fn main() {\n    println!(\"salut\"); // bonjour\n}");
    assert!(page.blocs.iter().all(|b| !azure_note::riche::brut(&b.contenu).contains('/') || b.id == 9), "les / tapes sont partis");
    a.capture("v2-page");

    // Entree au milieu d'un texte : coupe le bloc, la suite part dessous.
    a.remplir("b-2", "Bonjour le monde");
    a.activer("entree-b-2@7");
    let blocs = a.classeur.lire(|e| e.page(p).unwrap().blocs.clone());
    assert_eq!((blocs[1].contenu.as_str(), blocs[2].contenu.as_str()), ("Bonjour", " le monde"));
    assert_eq!(focalise(&a.ecran.ui_nodes), Some(format!("b-{}", blocs[2].id)));
    // Entree dans une puce : une nouvelle puce ; sur une puce vide : du texte.
    a.activer("entree-b-3@16");
    let nouvelle = a.classeur.lire(|e| e.page(p).unwrap().blocs[4].clone());
    assert_eq!(nouvelle.genre, azure_note::modele::GenreBloc::Puce);
    a.activer(&format!("entree-b-{}@0", nouvelle.id));
    assert_eq!(a.classeur.lire(|e| e.page(p).unwrap().blocs[4].genre.clone()), azure_note::modele::GenreBloc::Texte);
    // Le separateur (11) n'est qu'un trait : retour arriere dessous l'enleve.
    let apres = a.classeur.lire(|e| { let b = &e.page(p).unwrap().blocs; b[b.iter().position(|x| x.id == 11).unwrap() + 1].id });
    a.activer(&format!("retour-b-{apres}"));
    assert!(a.classeur.lire(|e| e.page(p).unwrap().blocs.iter().all(|b| b.id != 11)));
    assert!(a.classeur.lire(|e| e.page(p).unwrap().blocs.iter().any(|b| b.id == apres)));
}

#[test]
fn une_base_en_table_kanban_et_galerie() {
    let mut a = Appli::new();
    a.clic("nouvelle-base");
    let base = a.page();
    a.remplir("titre", "Tâches");
    a.ajouter_propriete("Avancement", "Nombre", "");
    a.ajouter_propriete("Progrès", "Formule", "si(compte(enfants) > 0, moyenne(enfants.avancement), avancement)");
    assert_eq!(a.erreur(), None);
    for _ in 0..3 {
        a.clic("ligne-ajouter");
    }
    let l: Vec<i64> = a.classeur.lire(|e| e.enfants(Some(base)).iter().map(|p| p.id).collect());
    for (i, (statut, av)) in [("Fait", "100"), ("En cours", "40"), ("À faire", "0")].iter().enumerate() {
        a.remplir(&format!("v-{}-avancement", l[i]), av);
        a.choisir(l[i], "statut", statut);
    }
    // Des etiquettes creees en les tapant, sous la ligne du tableau.
    a.creer_option(l[0], "étiquettes", "urgent");
    a.creer_option(l[0], "étiquettes", "client");
    a.capture("v2-table-etiquettes");
    assert_eq!(a.classeur.lire(|e| e.affichee(l[0], "étiquettes")), "urgent, client");
    a.clic("ligne-ajouter");
    assert_eq!(a.erreur(), None);
    // Les noms des lignes : ouvrir la 1re, la nommer, revenir.
    a.clic(&format!("p-{}", l[0]));
    a.remplir("titre", "Écrire le moteur de formules");
    a.clic(&format!("p-{base}"));
    let t = a.textes();
    assert!(t.contains(&"Écrire le moteur de formules".to_string()) && t.contains(&"100".to_string()), "{t:?}");
    a.capture("v2-table");

    a.clic("vue-1");
    let t = a.textes();
    for col in ["À faire", "En cours", "Fait", "Sans valeur"] {
        assert!(t.contains(&col.to_string()), "{col} : {t:?}");
    }
    a.capture("v2-kanban");
    // « En cours » (colonne 1) glisse en « Fait » (colonne 2), en tete.
    a.deposer(&format!("k-{}-1", l[1]), "col-2", 0);
    assert_eq!(a.classeur.lire(|e| e.affichee(l[1], "statut")), "Fait");

    a.clic("vue-ajout-galerie");
    a.capture("v2-galerie");
    a.clic("filtres");
    a.remplir("filtre-prop", "Statut");
    a.remplir("filtre-op", "=");
    a.remplir("filtre-val", "Fait");
    a.clic("filtre-ajouter");
    assert_eq!(a.erreur(), None);
    a.capture("v2-filtres");
}

#[test]
fn formule_remonte_depuis_les_sous_pages() {
    let mut a = Appli::new();
    a.clic("nouvelle");
    let parent = a.page();
    a.remplir("titre", "Projet");
    a.ajouter_propriete("Avancement", "Formule", "moyenne(enfants.avancement)");
    for av in ["100", "50"] {
        a.clic(&format!("sous-{parent}"));
        let sous = a.page();
        a.ajouter_propriete("Avancement", "Nombre", "");
        a.remplir(&format!("v-{sous}-avancement"), av);
        a.clic(&format!("p-{parent}"));
    }
    assert!(a.textes().contains(&"75".to_string()), "{:?}", a.textes());
    // Une formule en cycle est refusee avec un message.
    a.clic("prop-menu-avancement");
    a.remplir("carte-formule", "avancement + 1");
    a.activer("carte-formule");
    assert!(a.erreur().is_some_and(|e| e.contains("cycle")), "{:?}", a.erreur());
    a.capture("v2-formule");
}

#[test]
fn clic_droit_et_retour_arriere() {
    use azure_core::rules::window_event::WindowEvent;
    use azure_foundation::event::services::dispatch::handle_event;
    use azure_foundation::ui::services::interact::{self, KeyInput, KeyboardLayout};
    let mut a = Appli::new();
    a.clic("nouvelle");
    // Une page neuve a deja son bloc pour ecrire.
    a.activer("entree-titre@0");
    a.remplir("b-1", "important");
    cliquer(&mut a.ecran, "b-1", VUE);
    let mut clip = String::new();
    interact::type_into_focused(&mut a.ecran.ui_nodes, KeyInput::SelectAll, &mut clip);
    // Clic droit : le panneau ; clic sur le rouge.
    let (x, y) = (a.ecran.mouse_x, a.ecran.mouse_y);
    handle_event(&mut a.ecran, WindowEvent::WindowMouseButton(273, true), KeyboardLayout::Qwerty, VUE);
    let rouge = a.ecran.format_menu.as_ref().expect("panneau").items()[6].rect;
    {
        // L'image avec le panneau (dessine par la fenetre, par-dessus la page).
        let mut canvas = azure_engine::rendering::models::canvas::Canvas::new(VUE.2, VUE.3);
        azure_foundation::ui::services::draw_ui::draw_ui(&a.ecran.ui_nodes, VUE, &mut canvas, -1, -1, true);
        azure_foundation::ui::services::draw_ui::draw_format_menu(a.ecran.format_menu.as_ref().unwrap(), &mut canvas, rouge.0 + 4, rouge.1 + 4);
        let mut ppm = format!("P6\n{} {}\n255\n", VUE.2, VUE.3).into_bytes();
        for px in canvas.buffer.chunks(4) {
            ppm.extend_from_slice(&[px[2], px[1], px[0]]);
        }
        std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-note/v2-clic-droit.ppm"), ppm).unwrap();
    }
    handle_event(&mut a.ecran, WindowEvent::WindowMouseMove(rouge.0 + 4, rouge.1 + 4), KeyboardLayout::Qwerty, VUE);
    handle_event(&mut a.ecran, WindowEvent::WindowMouseButton(272, true), KeyboardLayout::Qwerty, VUE);
    handle_event(&mut a.ecran, WindowEvent::WindowMouseButton(272, false), KeyboardLayout::Qwerty, VUE);
    let _ = (x, y);
    a.clic("zone-ecrire");
    let p = a.page();
    let contenu = a.classeur.lire(|e| e.page(p).unwrap().blocs[0].contenu.clone());
    assert_eq!(azure_note::riche::lire(&contenu)[0].style.couleur, "#e06c75");

    // Retour arriere en tete du bloc vide cree dessous : il part, on remonte.
    let blocs = a.classeur.lire(|e| e.page(p).unwrap().blocs.clone());
    assert_eq!(blocs.len(), 2);
    a.activer(&format!("retour-b-{}", blocs[1].id));
    assert_eq!(a.classeur.lire(|e| e.page(p).unwrap().blocs.len()), 1);
    assert_eq!(focalise(&a.ecran.ui_nodes).as_deref(), Some("b-1"));
    // Un texte non vide se colle au precedent, curseur a la jointure.
    a.clic("zone-ecrire");
    let b2 = a.classeur.lire(|e| e.page(p).unwrap().blocs[1].id);
    a.remplir(&format!("b-{b2}"), " suite");
    a.activer(&format!("retour-b-{b2}"));
    let blocs = a.classeur.lire(|e| e.page(p).unwrap().blocs.clone());
    assert_eq!((blocs.len(), azure_note::riche::brut(&blocs[0].contenu)), (1, "important suite".to_string()));
    let zone = |n: &[UiNode]| -> usize {
        fn f(n: &[UiNode]) -> Option<usize> {
            n.iter().find_map(|x| match x {
                UiNode::TextArea(t) if t.focused => Some(t.cursor),
                UiNode::Container(c) => f(&c.children),
                _ => None,
            })
        }
        f(n).unwrap()
    };
    assert_eq!(zone(&a.ecran.ui_nodes), 9);
    a.capture("v2-page-nue");
}

#[test]
fn slash_au_clavier_et_arbre() {
    use azure_core::rules::window_event::WindowEvent;
    use azure_foundation::event::services::dispatch::handle_event;
    use azure_foundation::ui::services::interact::KeyboardLayout;
    use azure_note::modele::GenreBloc;
    let mut a = Appli::new();
    a.clic("nouvelle");
    a.activer("entree-titre@0");
    cliquer(&mut a.ecran, "b-1", VUE);
    // « /tit » au clavier (QWERTY) : le menu filtre, Entree choisit Titre 1.
    for code in [53, 20, 23, 20] {
        handle_event(&mut a.ecran, WindowEvent::WindowKeyPress(code, true), KeyboardLayout::Qwerty, VUE);
        handle_event(&mut a.ecran, WindowEvent::WindowKeyPress(code, false), KeyboardLayout::Qwerty, VUE);
    }
    let menu = a.ecran.command_menu.clone().expect("menu / ouvert");
    assert_eq!(menu.current().unwrap().code, "titre1");
    assert_eq!(menu.completion(), "re 1");
    {
        let mut canvas = azure_engine::rendering::models::canvas::Canvas::new(VUE.2, VUE.3);
        azure_foundation::ui::services::draw_ui::draw_ui(&a.ecran.ui_nodes, VUE, &mut canvas, -1, -1, true);
        azure_foundation::ui::services::draw_ui::draw_command_menu(&menu, &mut canvas, -1, -1);
        let mut ppm = format!("P6\n{} {}\n255\n", VUE.2, VUE.3).into_bytes();
        for px in canvas.buffer.chunks(4) {
            ppm.extend_from_slice(&[px[2], px[1], px[0]]);
        }
        std::fs::write(std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-note/v2-slash.ppm"), ppm).unwrap();
    }
    handle_event(&mut a.ecran, WindowEvent::WindowKeyPress(28, true), KeyboardLayout::Qwerty, VUE);
    assert!(a.ecran.take_activation());
    let id = a.ecran.clicked_id.clone().unwrap();
    assert_eq!(id, "commande-b-1@titre1@0");
    a.activer(&id);
    let p = a.page();
    let b = a.classeur.lire(|e| e.page(p).unwrap().blocs[0].clone());
    assert_eq!((b.genre, b.contenu.as_str()), (GenreBloc::Titre(1), ""), "le bloc vide devient le titre, sans /tit");

    // L'arbre : une sous-page, replier, supprimer avec confirmation.
    a.clic(&format!("sous-{p}"));
    let sous = a.page();
    assert!(boutons(&a.ecran.ui_nodes, VUE).contains(&format!("arbre-suppr-{sous}")));
    a.clic(&format!("pli-{p}"));
    assert!(!boutons(&a.ecran.ui_nodes, VUE).contains(&format!("arbre-suppr-{sous}")), "repliee");
    a.clic(&format!("pli-{p}"));
    a.clic(&format!("arbre-suppr-{p}"));
    assert!(a.textes().iter().any(|t| t.starts_with("Supprimer « ")));
    a.capture("v2-arbre");
    a.clic("arbre-non");
    assert!(a.classeur.lire(|e| e.page(p).is_some()));
    a.clic(&format!("arbre-suppr-{sous}"));
    a.clic(&format!("arbre-oui-{sous}"));
    assert!(a.classeur.lire(|e| e.page(sous).is_none()));
    assert_eq!(a.page(), p, "on revient au parent");
}

#[test]
fn separateur_et_proprietes() {
    let mut a = Appli::new();
    a.clic("nouvelle");
    a.remplir("titre", "Projet");
    a.activer("entree-titre@6");
    a.remplir("b-1", "Avant le trait");
    a.remplir("b-1", "Avant le trait /");
    a.activer("slash-b-1");
    a.clic("menu-separateur");
    let p = a.page();
    let apres = a.classeur.lire(|e| e.page(p).unwrap().blocs.last().unwrap().id);
    a.remplir(&format!("b-{apres}"), "Après le trait");
    for (nom, genre, config) in [("Statut", "Sélection", "À faire, En cours, Fait"), ("Avancement", "Nombre", ""), ("Échéance", "Date", ""), ("Tags", "Étiquettes", ""), ("Validé", "Case à cocher", ""), ("Liée à", "Relation", "")] {
        a.ajouter_propriete(nom, genre, config);
        assert_eq!(a.erreur(), None, "{nom}");
    }
    a.remplir(&format!("v-{p}-avancement"), "60");
    a.choisir(p, "statut", "En cours");
    a.creer_option(p, "tags", "important");
    a.clic("options");
    a.capture("v2-proprietes");

    // La liste des types, puis une formule creee et nommee sans formulaire.
    a.clic("prop-ouvrir");
    a.capture("v2-prop-ajout");
    a.clic("prop-nouveau-formule");
    a.remplir("carte-nom", "Total");
    a.remplir("carte-formule", "avancement * 2");
    a.capture("v2-prop-nouvelle");
    a.activer("carte-formule");
    assert_eq!(a.erreur(), None);
    assert_eq!(azure_note::page::valeur(&a.classeur.lire(|e| e.clone()), p, "total"), "120");
    a.activer("carte-nom");

    // La carte d'une propriete : renommer, ajouter et retirer une option ;
    // un clic ailleurs applique le nom et ferme.
    a.clic("prop-menu-statut");
    a.capture("v2-prop-carte");
    a.remplir("carte-nom", "État");
    a.remplir("carte-opt", "Bloqué");
    a.activer("carte-opt");
    a.clic("carte-opt-suppr-0");
    a.clic("options");
    assert_eq!(a.erreur(), None);
    let defs = a.classeur.lire(|e| e.definitions(p));
    assert!(defs.iter().any(|d| d.nom == "État" && d.genre == azure_note::modele::Genre::Selection(vec!["En cours".into(), "Fait".into(), "Bloqué".into()])), "{defs:?}");
    assert_eq!(a.classeur.lire(|e| e.affichee(p, "état")), "En cours", "la valeur suit le nouveau nom");
    // Changer de type : les etiquettes deviennent une selection.
    a.clic("prop-menu-tags");
    a.clic("carte-genre-selection");
    assert_eq!(a.classeur.lire(|e| e.affichee(p, "tags")), "important");
    // Supprimer, apres confirmation.
    a.clic("prop-menu-avancement");
    a.clic("carte-suppr");
    a.clic("carte-suppr-oui");
    assert!(a.classeur.lire(|e| e.definitions(p)).iter().all(|d| d.nom != "Avancement"));

    // Une date : le calendrier, « Aujourd'hui ».
    a.clic(&format!("ouvrir-v-{p}-échéance"));
    a.capture("v2-calendrier");
    a.clic(&format!("jour-v-{p}-échéance@auj"));
    let (an, mois, jour) = azure_note::proprietes::aujourdhui();
    assert_eq!(a.classeur.lire(|e| e.affichee(p, "échéance")), format!("{an:04}-{mois:02}-{jour:02}"));
    // Une relation : cocher une autre page.
    a.clic("nouvelle");
    let autre = a.page();
    a.remplir("titre", "Autre page");
    a.clic(&format!("p-{p}"));
    a.clic(&format!("ouvrir-v-{p}-liée_à"));
    a.capture("v2-relation");
    a.clic(&format!("choix-v-{p}-liée_à@{autre}"));
    assert_eq!(a.classeur.lire(|e| e.affichee(p, "liée_à")), "Autre page");
    a.clic("options");
    a.capture("v2-proprietes-fin");
}

#[test]
fn largeur_survol_et_taille_de_police() {
    use azure_core::rules::window_event::WindowEvent;
    use azure_foundation::event::services::dispatch::handle_event;
    use azure_foundation::ui::services::interact::{self, KeyInput, KeyboardLayout};
    let mut a = Appli::new();
    a.clic("nouvelle");
    a.remplir("titre", "Largeur");
    a.activer("entree-titre@7");
    a.remplir("b-1", "Un texte plus grand");
    cliquer(&mut a.ecran, "b-1", VUE);
    let mut clip = String::new();
    interact::type_into_focused(&mut a.ecran.ui_nodes, KeyInput::SelectAll, &mut clip);
    // Clic droit, puis « 24 » sur la ligne des tailles.
    handle_event(&mut a.ecran, WindowEvent::WindowMouseButton(273, true), KeyboardLayout::Qwerty, VUE);
    let items = a.ecran.format_menu.as_ref().expect("panneau").items();
    let t24 = items.iter().find(|i| i.mark == "taille-24").unwrap().rect;
    handle_event(&mut a.ecran, WindowEvent::WindowMouseMove(t24.0 + 4, t24.1 + 4), KeyboardLayout::Qwerty, VUE);
    handle_event(&mut a.ecran, WindowEvent::WindowMouseButton(272, true), KeyboardLayout::Qwerty, VUE);
    handle_event(&mut a.ecran, WindowEvent::WindowMouseButton(272, false), KeyboardLayout::Qwerty, VUE);
    // La zone grandit avec sa police.
    let haut = boite(&a.ecran.ui_nodes, "b-1", VUE).unwrap().3;
    assert!(haut >= 36, "hauteur {haut}");
    a.clic("zone-ecrire");
    let p = a.page();
    let contenu = a.classeur.lire(|e| e.page(p).unwrap().blocs[0].contenu.clone());
    assert_eq!(azure_note::riche::lire(&contenu)[0].style.taille, "24");

    // Poignee et × : visibles au survol du bloc seulement.
    let b = boite(&a.ecran.ui_nodes, "b-1", VUE).unwrap();
    capture(&a.ecran.ui_nodes, (VUE.2, VUE.3), "v2-sans-survol");
    capture_souris(&a.ecran.ui_nodes, (VUE.2, VUE.3), (b.0 + 20, b.1 + 5), "v2-survol-bloc");

    // Largeur de la page : dans les options.
    let etroite = boite(&a.ecran.ui_nodes, "b-1", VUE).unwrap().2;
    a.clic("options");
    a.clic("largeur-pleine");
    assert_eq!(a.classeur.lire(|e| e.page(p).unwrap().largeur.clone()), "pleine");
    let pleine = boite(&a.ecran.ui_nodes, "b-1", VUE).unwrap().2;
    assert!(pleine > etroite + 200, "{etroite} -> {pleine}");
    a.capture("v2-largeur-pleine");
}
