// Les donnees de l'ecran de Note v2 (ui/note.rsh) : l'arbre des pages a
// gauche, la page ouverte (proprietes, blocs) ou la base ouverte (vues).
//
// Ce module ne construit aucun widget et ne modifie rien : il lit l'espace
// et l'etat de l'ecran (`Etat`) et rend le `Context` du gabarit. Les ids
// des champs et des boutons sont decrits dans `clics`.
use crate::carnet::{depuis, maintenant};
use crate::code::colorer;
use crate::formule::Valeur;
use crate::modele::{cible, Affichage, Definition, Espace, Genre, GenreBloc, Page, Vue, SEP, SEP_LIGNE};
use azure_foundation::compiler::services::condition::{ConditionValue as V, Context};
use std::collections::HashMap;

/// Ce que l'ecran montre, hors donnees : garde par la fenetre entre deux clics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Etat {
    pub page: Option<i64>,
    /// Vue choisie, par base.
    pub vues: HashMap<i64, usize>,
    /// Confirmation de suppression de la page ouverte.
    pub confirmer: bool,
    /// La liste des types (« + Ajouter une propriété ») ouverte.
    pub ajout_propriete: bool,
    /// Panneau des filtres ouvert.
    pub filtres: bool,
    /// Dernier message d'erreur (formule invalide, valeur refusee...).
    pub erreur: Option<String>,
    /// Menu des blocs ouvert (tape `/`) : sous ce bloc, ou en fin de page (`Some(None)`).
    pub menu: Option<Option<i64>>,
    /// Le bloc ou l'on tape, au prochain affichage (0 : le titre).
    pub focus: Option<i64>,
    /// Ou y placer le curseur (sinon a la fin).
    pub focus_pos: Option<usize>,
    /// Le panneau « Options » de la page (proprietes, suppression).
    pub options: bool,
    /// Pages repliees dans l'arbre (leurs sous-pages cachees).
    pub plies: std::collections::BTreeSet<i64>,
    /// Page a supprimer depuis l'arbre, en attente de confirmation.
    pub suppr_arbre: Option<i64>,
    /// La propriete dont la carte (nom, type, options, suppression) est ouverte.
    pub prop_menu: Option<String>,
    /// Elle vient d'etre creee : son nom est selectionne, pret a remplacer.
    pub nom_neuf: bool,
    /// Suppression de la propriete ouverte, en attente de confirmation.
    pub suppr_prop: bool,
    /// La valeur dont le choix est ouvert (`v-<page>-<cle>`, voir `proprietes`).
    pub editeur: Option<String>,
    /// Le mois montre par le calendrier d'une date.
    pub mois: Option<(i32, u32)>,
    /// Le filtre des pages proposees pour une relation.
    pub filtre_rel: String,
}

/// Les mots qui trouvent aussi une commande du menu `/` au clavier.
const MOTS: [(&str, &str); 13] = [
    ("texte", "text paragraphe p"),
    ("titre1", "h1 heading titre"),
    ("titre2", "h2 heading titre"),
    ("titre3", "h3 heading titre"),
    ("puce", "ul bullet liste"),
    ("numero", "ol numero liste"),
    ("tache", "todo case checkbox"),
    ("citation", "quote"),
    ("code", "pre programme"),
    ("tableau", "table grille"),
    ("separateur", "hr ligne trait divider"),
    ("souspage", "page"),
    ("base", "database db kanban table galerie"),
];

/// Le menu `/` pour `<richtext commandes>` : `code|Nom|description|mots;...`.
pub fn commandes() -> String {
    MENU.iter().map(|(c, n, d)| format!("{c}|{n}|{d}|{}", MOTS.iter().find(|m| m.0 == *c).map(|m| m.1).unwrap_or(""))).collect::<Vec<_>>().join(";")
}

/// Le menu `/` : (code, nom, description).
pub const MENU: [(&str, &str, &str); 13] = [
    ("texte", "Texte", "Un paragraphe"),
    ("titre1", "Titre 1", "Grand titre de section"),
    ("titre2", "Titre 2", "Titre moyen"),
    ("titre3", "Titre 3", "Petit titre"),
    ("puce", "Liste à puces", "Une liste simple"),
    ("numero", "Liste numérotée", "Une liste avec des numéros"),
    ("tache", "Tâche", "Une case à cocher"),
    ("citation", "Citation", "Un texte mis en avant"),
    ("code", "Code", "Du code coloré"),
    ("tableau", "Tableau", "Des lignes et des colonnes"),
    ("separateur", "Séparateur", "Un trait entre deux parties"),
    ("souspage", "Sous-page", "Une page dans cette page"),
    ("base", "Base", "Table, kanban, liste ou galerie"),
];

impl Etat {
    /// La vue choisie de la base `base` (bornee a ses vues).
    pub fn vue(&self, espace: &Espace, base: i64) -> usize {
        let n = espace.page(base).map(|b| b.vues.len()).unwrap_or(0);
        self.vues.get(&base).copied().unwrap_or(0).min(n.saturating_sub(1))
    }
}

/// Les couleurs d'une page (son identite : couverture, pastille, puce
/// dans l'arbre). Rangee dans `Page::icone` ; sans choix, tiree de l'id.
pub const TEINTES: [&str; 8] = ["sable", "terre", "olive", "mousse", "prune", "ardoise", "rose", "ocre"];

pub fn teinte(p: &Page) -> &'static str {
    TEINTES.iter().find(|x| **x == p.icone).copied().unwrap_or(TEINTES[p.id.rem_euclid(TEINTES.len() as i64) as usize])
}

/// La lettre de la pastille : l'initiale du titre.
pub fn initiale(p: &Page) -> String {
    p.nom().chars().find(|c| c.is_alphanumeric()).map(|c| c.to_uppercase().collect()).unwrap_or_else(|| "·".to_string())
}

/// Largeurs de la colonne d'une page (code, nom), de `.col-page.<code>`.
pub const LARGEURS: [(&str, &str); 4] = [("etroite", "Étroite"), ("moyenne", "Moyenne"), ("large", "Large"), ("pleine", "Pleine")];

/// La largeur choisie ; sans choix, large pour une base, etroite sinon.
pub fn largeur(p: &Page) -> &'static str {
    LARGEURS.iter().map(|l| l.0).find(|l| *l == p.largeur).unwrap_or(if p.base { "large" } else { "etroite" })
}

/// Le petit symbole d'un type de propriete (dessine en monospace : Sora
/// n'a pas ces glyphes).
pub fn icone_genre(code: &str) -> &'static str {
    match code {
        "nombre" => "#",
        "selection" => "◉",
        "etiquettes" => "◆",
        "date" => "◷",
        "case" => "☑",
        "relation" => "↗",
        "formule" => "∑",
        _ => "≡",
    }
}

/// Le choix « aucun » d'une liste.
pub const VIDE: &str = "—";

/// « —, a, b » : les choix d'un <select>, « aucun » en tete.
pub fn liste<'a>(choix: impl Iterator<Item = &'a str>) -> String {
    std::iter::once(VIDE).chain(choix).collect::<Vec<_>>().join(", ")
}

fn t(s: &str) -> V {
    V::Text(s.to_string())
}

fn b(v: bool) -> V {
    V::Bool(v)
}

fn map<const N: usize>(entries: [(&str, V); N]) -> V {
    V::map(entries)
}

/// Le code d'un genre de bloc pour le gabarit (`titre1`, `code`...).
pub fn code_bloc(g: &GenreBloc) -> &'static str {
    match g {
        GenreBloc::Titre(1) => "titre1",
        GenreBloc::Titre(2) => "titre2",
        GenreBloc::Titre(_) => "titre3",
        GenreBloc::Texte => "texte",
        GenreBloc::Puce => "puce",
        GenreBloc::Numero => "numero",
        GenreBloc::Tache(_) => "tache",
        GenreBloc::Citation => "citation",
        GenreBloc::Code(_) => "code",
        GenreBloc::Tableau => "tableau",
        GenreBloc::Separateur => "separateur",
        GenreBloc::SousPage => "souspage",
        GenreBloc::Vue => "vue",
    }
}

/// Les genres de bloc qu'on peut choisir pour un bloc de texte.
pub const GENRES_TEXTE: [(&str, &str); 8] = [("texte", "Texte"), ("titre1", "Titre 1"), ("titre2", "Titre 2"), ("titre3", "Titre 3"), ("puce", "Liste"), ("numero", "Liste numérotée"), ("tache", "Tâche"), ("citation", "Citation")];

/// Les cellules d'un bloc tableau.
pub fn cellules(contenu: &str) -> Vec<Vec<String>> {
    let mut v: Vec<Vec<String>> = contenu.split(SEP_LIGNE).map(|l| l.split(SEP).map(str::to_string).collect()).collect();
    let largeur = v.iter().map(Vec::len).max().unwrap_or(1).max(1);
    for l in &mut v {
        l.resize(largeur, String::new());
    }
    v
}

pub fn ecrire_cellules(c: &[Vec<String>]) -> String {
    c.iter().map(|l| l.join(&SEP.to_string())).collect::<Vec<_>>().join(&SEP_LIGNE.to_string())
}

/// Un tableau neuf : en-tete + 2 lignes, 3 colonnes.
pub fn tableau_neuf() -> String {
    ecrire_cellules(&[vec!["Colonne 1".into(), "Colonne 2".into(), "Colonne 3".into()], vec![String::new(); 3], vec![String::new(); 3]])
}

/// L'arbre des pages, a plat avec leur profondeur (les bases ne montrent
/// pas leurs lignes, trop nombreuses).
fn arbre(e: &Espace, ouverte: Option<i64>, etat: &Etat) -> Vec<V> {
    fn aller(e: &Espace, parent: Option<i64>, niveau: usize, ouverte: Option<i64>, etat: &Etat, out: &mut Vec<V>) {
        for p in e.enfants(parent) {
            let enfants = !p.base && !e.enfants(Some(p.id)).is_empty();
            let plie = etat.plies.contains(&p.id);
            out.push(map([
                ("id", t(&p.id.to_string())),
                ("nom", t(p.nom())),
                // Classes : .n0 ... .n6 (retrait), .on (ouverte), .base.
                ("niveau", t(&format!("n{}", niveau.min(6)))),
                ("etat", t(if Some(p.id) == ouverte { "on" } else { "off" })),
                ("sorte", t(if p.base { "sorte-base" } else { "sorte-page" })),
                ("base", b(p.base)),
                ("teinte", t(teinte(p))),
                // Le triangle pour replier / deplier ses sous-pages.
                ("a_enfants", b(enfants)),
                ("pli", t(if plie { "▸" } else { "▾" })),
                ("confirmer", b(etat.suppr_arbre == Some(p.id))),
            ]));
            if enfants && !plie {
                aller(e, Some(p.id), niveau + 1, ouverte, etat, out);
            }
        }
    }
    let mut out = Vec::new();
    aller(e, None, 0, ouverte, etat, &mut out);
    out
}

/// L'id du champ de la propriete `cle` de la page `id`.
pub fn champ(id: i64, cle: &str) -> String {
    format!("v-{id}-{cle}")
}

/// Une propriete de `id` pour le gabarit : de quoi afficher sa valeur
/// (champ, pastilles, date lisible...). `menu` : sa carte est ouverte ;
/// `ouvert` : le choix de sa valeur est ouvert.
fn propriete(e: &Espace, id: i64, d: &Definition, menu: bool, ouvert: bool) -> V {
    let cle = d.cle();
    let brute = e.page(id).and_then(|p| p.valeurs.get(&cle).cloned()).unwrap_or_default();
    let affichee = e.affichee(id, &cle);
    let pastilles = crate::proprietes::pastilles(e, d, &brute);
    map([
        ("cle", t(&cle)),
        ("nom", t(&d.nom)),
        ("genre", t(d.genre.code())),
        ("libelle", t(d.genre.libelle())),
        ("fid", t(&champ(id, &cle))),
        ("valeur", t(&brute)),
        // Le ⚠ des erreurs n'existe pas dans la police : la couleur suffit.
        ("texte", t(affichee.trim_start_matches('⚠').trim_start())),
        ("date", t(&crate::proprietes::date_lisible(&brute))),
        ("vide", b(brute.is_empty())),
        ("coche", b(brute == "1")),
        ("a_pastilles", b(!pastilles.is_empty())),
        ("pastilles", V::List(pastilles)),
        ("erreur", b(affichee.starts_with('⚠'))),
        ("icone", t(icone_genre(d.genre.code()))),
        ("menu", b(menu)),
        ("ouvert", b(ouvert)),
    ])
}

/// Les blocs de la page pour le gabarit.
fn blocs(e: &Espace, page: &Page, etat: &Etat) -> Vec<V> {
    let mut numero = 0;
    page.blocs
        .iter()
        .map(|bl| {
            numero = if bl.genre == GenreBloc::Numero { numero + 1 } else { 0 };
            let code = code_bloc(&bl.genre);
            let mut m = vec![
                ("id", t(&bl.id.to_string())),
                ("genre", t(code)),
                ("texte_riche", b(GENRES_TEXTE.iter().any(|g| g.0 == code))),
                ("valeur", t(&bl.contenu)),
                ("numero", t(&numero.to_string())),
                ("fait", b(bl.genre == GenreBloc::Tache(true))),
                ("menu", b(etat.menu == Some(Some(bl.id)))),
                // `focus` du <richtext> : "true" (fin), une position, ou "false".
                ("focus", t(&match (etat.focus == Some(bl.id), etat.focus_pos) {
                    (true, Some(p)) => p.to_string(),
                    (true, None) => "true".into(),
                    _ => "false".into(),
                })),
                // Le <select> du genre : les libelles, celui du bloc choisi.
                ("choix", t(&GENRES_TEXTE.iter().map(|g| g.1).collect::<Vec<_>>().join(", "))),
                ("libelle", t(GENRES_TEXTE.iter().find(|g| g.0 == code).map(|g| g.1).unwrap_or(""))),
            ];
            match &bl.genre {
                GenreBloc::Code(langage) => {
                    m.push(("valeur", t(&colorer(&bl.contenu))));
                    m.push(("langage", t(langage)));
                }
                GenreBloc::Tableau => {
                    let lignes = cellules(&bl.contenu)
                        .iter()
                        .enumerate()
                        .map(|(r, l)| map([("r", t(&r.to_string())), ("entete", b(r == 0)), ("cellules", V::List(l.iter().enumerate().map(|(c, v)| map([("c", t(&c.to_string())), ("v", t(v)), ("cl", t(if r == 0 { "entete" } else { "cellule" }))])).collect()))]))
                        .collect();
                    m.push(("lignes", V::List(lignes)));
                }
                GenreBloc::SousPage => {
                    let p = cible(bl).and_then(|i| e.page(i));
                    m.push(("cible", t(&p.map(|p| p.id.to_string()).unwrap_or_default())));
                    m.push(("nom", t(p.map(|p| p.nom()).unwrap_or("Page supprimée"))));
                    m.push(("base", b(p.is_some_and(|p| p.base))));
                    m.push(("teinte", t(p.map(teinte).unwrap_or("ardoise"))));
                }
                GenreBloc::Vue => {
                    let base = cible(bl).and_then(|i| e.page(i)).filter(|p| p.base);
                    m.push(("cible", t(&base.map(|p| p.id.to_string()).unwrap_or_default())));
                    m.push(("nom", t(base.map(|p| p.nom()).unwrap_or("Base supprimée"))));
                    if let Some(base) = base {
                        let vue = Vue::nouvelle(Affichage::Table);
                        m.push(("colonnes", V::List(base.schema.iter().take(4).map(|d| map([("nom", t(&d.nom))])).collect())));
                        m.push(("lignes", V::List(lignes_table(e, base, &vue, 4, etat))));
                    }
                }
                _ => {}
            }
            V::Map(m.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
        })
        .collect()
}

/// Les lignes d'une base : id, nom, et une cellule par propriete du schema
/// (au plus `max`).
fn lignes_table(e: &Espace, base: &Page, vue: &Vue, max: usize, etat: &Etat) -> Vec<V> {
    e.lignes(base.id, vue)
        .into_iter()
        .filter_map(|id| e.page(id))
        .map(|l| {
            let ouvert = |d: &Definition| etat.editeur.as_deref() == Some(champ(l.id, &d.cle()).as_str());
            let cellules = base.schema.iter().take(max).map(|d| propriete(e, l.id, d, false, ouvert(d))).collect();
            map([("id", t(&l.id.to_string())), ("nom", t(l.nom())), ("cellules", V::List(cellules)), ("ouvert", b(base.schema.iter().take(max).any(ouvert)))])
        })
        .collect()
}

/// Les donnees d'une base ouverte : ses vues, la vue choisie, ses lignes.
fn donnees_base(ctx: Context, e: &Espace, base: &Page, etat: &Etat) -> Context {
    let n = etat.vue(e, base.id);
    let vue = base.vues.get(n).cloned().unwrap_or_else(|| Vue::nouvelle(Affichage::Table));
    let onglets = base.vues.iter().enumerate().map(|(i, v)| map([("n", t(&i.to_string())), ("nom", t(&v.nom)), ("actif", b(i == n))])).collect();
    let colonnes = base.schema.iter().map(|d| map([("cle", t(&d.cle())), ("nom", t(&d.nom)), ("libelle", t(d.genre.libelle())), ("icone", t(icone_genre(d.genre.code()))), ("on", t(if etat.prop_menu.as_deref() == Some(d.cle().as_str()) { "on" } else { "off" }))])).collect();
    let nom_de = |cle: &str| base.schema.iter().find(|d| d.cle() == cle).map(|d| d.nom.clone()).unwrap_or_else(|| VIDE.to_string());
    let groupables = liste(base.schema.iter().filter(|d| matches!(d.genre, Genre::Selection(_) | Genre::Etiquettes(_))).map(|d| d.nom.as_str()));
    let props = liste(base.schema.iter().map(|d| d.nom.as_str()));
    let filtres = vue
        .filtres
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let nom = base.schema.iter().find(|d| d.cle() == f.propriete).map(|d| d.nom.clone()).unwrap_or(f.propriete.clone());
            map([("i", t(&i.to_string())), ("texte", t(&format!("{nom} {} {}", f.operateur.code(), f.valeur).trim().to_string()))])
        })
        .collect();
    let kanban = e
        .kanban(base.id, &vue)
        .into_iter()
        .enumerate()
        .map(|(i, (nom, ids))| {
            let cartes = ids
                .iter()
                .filter_map(|id| e.page(*id))
                .map(|p| {
                    let details: Vec<V> = base
                        .schema
                        .iter()
                        .filter(|d| d.cle() != vue.groupe)
                        .map(|d| (d.nom.clone(), e.affichee(p.id, &d.cle())))
                        .filter(|(_, v)| !v.is_empty())
                        .take(3)
                        .map(|(k, v)| map([("nom", t(&k)), ("v", t(&v))]))
                        .collect();
                    map([("id", t(&p.id.to_string())), ("nom", t(p.nom())), ("details", V::List(details))])
                })
                .collect::<Vec<_>>();
            map([("i", t(&i.to_string())), ("nom", t(&nom)), ("nb", t(&cartes.len().to_string())), ("cartes", V::List(cartes))])
        })
        .collect();
    let (tri, desc) = vue.tri.clone().unwrap_or_default();
    ctx.with_value("vues", V::List(onglets))
        .with_value("affichage", t(vue.affichage.code()))
        .with_value("colonnes", V::List(colonnes))
        .with_value("lignes", V::List(lignes_table(e, base, &vue, usize::MAX, etat)))
        .with_value("kanban", V::List(kanban))
        .with_value("groupables", t(&groupables))
        .with_value("groupe", t(&nom_de(&vue.groupe)))
        .with_value("props_base", t(&props))
        .with_value("tri", t(&nom_de(&tri)))
        .with_value("tri_sens", t(if desc { "Décroissant" } else { "Croissant" }))
        .with_value("operateurs", t(&crate::modele::Operateur::TOUS.iter().map(|o| o.code()).collect::<Vec<_>>().join(", ")))
        .with_value("filtres", V::List(filtres))
        .with_bool("filtres_ouverts", etat.filtres)
        .with_number("nb_lignes", e.enfants(Some(base.id)).len() as f64)
}

/// Le `Context` complet de l'ecran.
pub fn ecran(e: &Espace, etat: &Etat) -> Context {
    let ouverte = etat.page.and_then(|id| e.page(id)).or_else(|| e.enfants(None).into_iter().next());
    let mut ctx = Context::new()
        .with_value("arbre", V::List(arbre(e, ouverte.map(|p| p.id), etat)))
        .with_number("nb_pages", e.pages.len() as f64)
        .with_bool("a_erreur", etat.erreur.is_some())
        .with_value("erreur", t(etat.erreur.as_deref().unwrap_or("")))
        .with_value("genres_prop", t(&Genre::CODES.iter().map(|c| Genre::depuis(c, "").libelle()).collect::<Vec<_>>().join(", ")));
    let Some(p) = ouverte else {
        return ctx.with_bool("a_page", false);
    };
    let parent_base = p.parent.and_then(|x| e.page(x)).is_some_and(|x| x.base);
    let chemin = e.chemin(p.id).iter().map(|c| map([("id", t(&c.id.to_string())), ("nom", t(c.nom()))])).collect();
    let props: Vec<V> = e.definitions(p.id).iter().map(|d| propriete(e, p.id, d, etat.prop_menu.as_deref() == Some(d.cle().as_str()), etat.editeur.as_deref() == Some(champ(p.id, &d.cle()).as_str()))).collect();
    let sous_pages: Vec<V> = if p.base {
        Vec::new()
    } else {
        e.enfants(Some(p.id)).iter().filter(|c| !p.blocs.iter().any(|bl| cible(bl) == Some(c.id))).map(|c| map([("id", t(&c.id.to_string())), ("nom", t(c.nom())), ("base", b(c.base)), ("teinte", t(teinte(c)))])).collect()
    };
    ctx = ctx
        .with_bool("a_page", true)
        .with_value("page", map([("id", t(&p.id.to_string())), ("titre", t(&p.titre)), ("nom", t(p.nom())), ("depuis", t(&depuis(p.modifie, maintenant()))), ("base", b(p.base)), ("ligne", b(parent_base)), ("teinte", t(teinte(p))), ("initiale", t(&initiale(p))), ("largeur", t(largeur(p)))]))
        .with_value("chemin", V::List(chemin))
        .with_value("props", V::List(props.clone()))
        .with_bool("a_props", (!props.is_empty() || etat.ajout_propriete) && !p.base)
        .with_value("blocs", V::List(blocs(e, p, etat)))
        .with_bool("menu_fin", etat.menu == Some(None))
        .with_bool("focus_titre", etat.focus == Some(0))
        .with_value("commandes", t(&commandes()))
        .with_value("menu", V::List(MENU.iter().map(|(c, n, d)| map([("code", t(c)), ("nom", t(n)), ("desc", t(d))])).collect()))
        .with_bool("a_sous_pages", !sous_pages.is_empty())
        .with_value("sous_pages", V::List(sous_pages))
        .with_value("largeurs", V::List(LARGEURS.iter().map(|(code, nom)| map([("code", t(code)), ("nom", t(nom)), ("on", t(if *code == largeur(p) { "on" } else { "off" }))])).collect()))
        .with_value("teintes", V::List(TEINTES.iter().map(|x| map([("nom", t(x)), ("on", t(if *x == teinte(p) { "on" } else { "off" }))])).collect()))
        .with_value("types", crate::proprietes::types())
        .with_value("ed", crate::proprietes::editeur(e, etat))
        .with_value("carte", crate::proprietes::carte(e, etat, p.id))
        .with_bool("a_carte", etat.prop_menu.is_some())
        .with_bool("confirmer", etat.confirmer)
        .with_bool("ajout_propriete", etat.ajout_propriete)
        .with_bool("options", etat.options || etat.confirmer);
    if p.base {
        ctx = donnees_base(ctx, e, p, etat);
    }
    ctx
}

/// Pour les tests et le debogage : la valeur de `nom` sur `id`, en texte.
pub fn valeur(e: &Espace, id: i64, nom: &str) -> String {
    match e.valeur(id, nom) {
        Ok(Valeur::Vide) | Err(_) => String::new(),
        Ok(v) => v.to_string(),
    }
}
