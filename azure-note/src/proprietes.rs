// Les proprietes, cote ecran : leurs valeurs (pastilles, calendrier, choix
// des pages liees) et leur carte (nom, type, options, formule).
//
// Tout se fait sans formulaire a valider :
//
// - « + Ajouter une propriete » ouvre la liste des types ; un clic sur un
//   type cree la propriete (nommee d'apres le type) et ouvre sa carte, le
//   nom deja selectionne ;
// - la carte applique ce qui est tape (nom, formule) au clic suivant, quel
//   qu'il soit, et se ferme des qu'on clique ailleurs ;
// - une valeur de selection, d'etiquettes, de date ou de relation s'ouvre en
//   dessous (une seule a la fois) et se choisit en un clic.
//
// Ids (fid = v-<page>-<cle>, voir `page::champ`) :
//
//   prop-ouvrir, prop-nouveau-<type>     la liste des types, creer
//   prop-menu-<cle>                      ouvrir / fermer la carte
//   carte-nom, carte-formule, carte-opt  champs de la carte (Entree : valider)
//   carte-genre-<type>                   changer le type
//   carte-opt-suppr-<i>                  retirer une option
//   carte-suppr(-oui/-non), carte-fermer
//   ouvrir-<fid>                         ouvrir / fermer le choix d'une valeur
//   choix-<fid>@<i|->                    option i (ou aucune) ; page i (relation)
//   creer-<fid>                          champ « nouvelle option » / filtre
//   jour-<fid>@<AAAA-MM-JJ|>             date (vide : effacer)
//   mois-<fid>@<-1|+1|auj>               mois affiche
//   retirer-<fid>@<i>                    retire une etiquette / une page liee
use crate::carnet::maintenant;
use crate::clics::{Lecture, Suite};
use crate::modele::{Definition, Espace, Genre, SEP};
use crate::page::{icone_genre, Etat};
use azure_foundation::compiler::services::condition::ConditionValue as V;

/// Les types, dans la liste « + Ajouter une propriete » : (code, description).
pub const TYPES: [(&str, &str); 8] = [
    ("texte", "Du texte libre"),
    ("nombre", "Un nombre, pour calculer"),
    ("selection", "Une option dans une liste"),
    ("etiquettes", "Plusieurs étiquettes"),
    ("date", "Un jour du calendrier"),
    ("case", "Oui ou non"),
    ("relation", "Des liens vers d'autres pages"),
    ("formule", "Calculée depuis les autres"),
];

const MOIS: [&str; 12] = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
const MOIS_LONGS: [&str; 12] = ["Janvier", "Février", "Mars", "Avril", "Mai", "Juin", "Juillet", "Août", "Septembre", "Octobre", "Novembre", "Décembre"];

fn t(s: &str) -> V {
    V::Text(s.to_string())
}

fn b(v: bool) -> V {
    V::Bool(v)
}

fn map<const N: usize>(entries: [(&str, V); N]) -> V {
    V::map(entries)
}

/// La couleur d'une option (classe `.c0` ... `.c7`), tiree de son nom.
pub fn teinte_option(nom: &str) -> String {
    format!("c{}", nom.to_lowercase().chars().map(|c| c as u32).sum::<u32>() % 8)
}

/// `v-<page>-<cle>` -> (page, cle).
pub fn decouper_fid(fid: &str) -> Option<(i64, String)> {
    let (id, cle) = fid.strip_prefix("v-")?.split_once('-')?;
    Some((id.parse().ok()?, cle.to_string()))
}

// ---- dates ----------------------------------------------------------------

/// Jours depuis le 1970-01-01.
fn jours(a: i32, m: u32, j: u32) -> i64 {
    let a = if m <= 2 { a - 1 } else { a } as i64;
    let ere = a.div_euclid(400);
    let an = a - ere * 400;
    let mp = (m as i64 + 9) % 12;
    let jour_an = (153 * mp + 2) / 5 + j as i64 - 1;
    let jour_ere = an * 365 + an / 4 - an / 100 + jour_an;
    ere * 146097 + jour_ere - 719468
}

/// L'inverse de `jours`.
fn civil(n: i64) -> (i32, u32, u32) {
    let z = n + 719468;
    let ere = z.div_euclid(146097);
    let jour_ere = z - ere * 146097;
    let an = (jour_ere - jour_ere / 1460 + jour_ere / 36524 - jour_ere / 146096) / 365;
    let jour_an = jour_ere - (365 * an + an / 4 - an / 100);
    let mp = (5 * jour_an + 2) / 153;
    let j = (jour_an - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((an + ere * 400) as i32 + i32::from(m <= 2), m, j)
}

fn date_texte((a, m, j): (i32, u32, u32)) -> String {
    format!("{a:04}-{m:02}-{j:02}")
}

pub fn aujourdhui() -> (i32, u32, u32) {
    civil((maintenant() / 86400) as i64)
}

/// « 29 sept. 2026 » (le texte tel quel s'il n'est pas une date).
pub fn date_lisible(s: &str) -> String {
    match crate::modele::date_valide(s) {
        Some((a, m, j)) => format!("{j} {} {a}", MOIS[(m - 1) as usize]),
        None => s.to_string(),
    }
}

/// Le mois voisin : `(a, m)` decale de `d` mois.
fn decaler((a, m): (i32, u32), d: i32) -> (i32, u32) {
    let n = a * 12 + m as i32 - 1 + d;
    (n.div_euclid(12), n.rem_euclid(12) as u32 + 1)
}

/// Les semaines du mois (lundi en tete), avec les jours voisins pour
/// completer : (jour, date, classes).
fn calendrier(fid: &str, (a, m): (i32, u32), choisie: &str) -> Vec<V> {
    let premier = jours(a, m, 1);
    // 1970-01-01 etait un jeudi : lundi = 0.
    let decalage = (premier + 3).rem_euclid(7);
    let auj = date_texte(aujourdhui());
    let mut semaines = Vec::new();
    let mut n = premier - decalage;
    for _ in 0..6 {
        let mut s = Vec::new();
        for _ in 0..7 {
            let d = civil(n);
            let date = date_texte(d);
            let mut cl = vec!["jour"];
            if d.1 != m {
                cl.push("hors");
            }
            if date == auj {
                cl.push("auj");
            }
            if date == choisie {
                cl.push("on");
            }
            s.push(map([("j", t(&d.2.to_string())), ("id", t(&format!("jour-{fid}@{date}"))), ("date", t(&date)), ("cl", t(&cl.join(" ")))]));
            n += 1;
        }
        semaines.push(map([("jours", V::List(s))]));
        // Plus rien de ce mois : on s'arrete.
        if civil(n).1 != m {
            break;
        }
    }
    semaines
}

// ---- donnees de l'ecran ---------------------------------------------------

/// Les pastilles d'une valeur (selection, etiquettes, pages liees).
pub fn pastilles(e: &Espace, d: &Definition, brute: &str) -> Vec<V> {
    match &d.genre {
        Genre::Selection(_) | Genre::Etiquettes(_) => brute.split(SEP).filter(|x| !x.is_empty()).map(|x| map([("nom", t(x)), ("teinte", t(&teinte_option(x))), ("id", t(""))])).collect(),
        Genre::Relation => brute
            .split(',')
            .filter_map(|x| x.trim().parse::<i64>().ok())
            .filter_map(|x| e.page(x))
            .map(|p| map([("nom", t(p.nom())), ("teinte", t("rel")), ("id", t(&p.id.to_string()))]))
            .collect(),
        _ => Vec::new(),
    }
}

/// Le choix ouvert sous une valeur (`Etat::editeur`), pour `<editeur-valeur>`.
pub fn editeur(e: &Espace, etat: &Etat) -> V {
    let vide = map([("genre", t(""))]);
    let Some(fid) = etat.editeur.as_deref() else { return vide };
    let Some((id, cle)) = decouper_fid(fid) else { return vide };
    let Some(d) = e.definition(id, &cle) else { return vide };
    let brute = e.page(id).and_then(|p| p.valeurs.get(&cle).cloned()).unwrap_or_default();
    let choisies: Vec<&str> = brute.split(SEP).filter(|x| !x.is_empty()).collect();
    let mut m = vec![("fid", t(fid)), ("genre", t(d.genre.code())), ("nom", t(&d.nom))];
    match &d.genre {
        Genre::Selection(o) | Genre::Etiquettes(o) => {
            let options: Vec<V> = o.iter().enumerate().map(|(i, x)| map([("id", t(&format!("choix-{fid}@{i}"))), ("nom", t(x)), ("teinte", t(&teinte_option(x))), ("on", t(if choisies.contains(&x.as_str()) { "on" } else { "off" }))])).collect();
            m.push(("a_options", b(!options.is_empty())));
            m.push(("options", V::List(options)));
            m.push(("aucune", t(&format!("choix-{fid}@-"))));
            m.push(("retirer", b(!brute.is_empty() && matches!(d.genre, Genre::Selection(_)))));
            m.push(("aide", t(if matches!(d.genre, Genre::Selection(_)) { "Choisir ou créer une option…" } else { "Ajouter ou créer une étiquette…" })));
        }
        Genre::Date => {
            let mois = etat.mois.or_else(|| crate::modele::date_valide(&brute).map(|(a, m, _)| (a, m))).unwrap_or_else(|| {
                let (a, m, _) = aujourdhui();
                (a, m)
            });
            m.push(("titre", t(&format!("{} {}", MOIS_LONGS[(mois.1 - 1) as usize], mois.0))));
            m.push(("semaines", V::List(calendrier(fid, mois, &brute))));
            m.push(("prec", t(&format!("mois-{fid}@-1"))));
            m.push(("suiv", t(&format!("mois-{fid}@1"))));
            m.push(("auj", t(&format!("jour-{fid}@auj"))));
            m.push(("effacer", t(&format!("jour-{fid}@"))));
            m.push(("a_date", b(!brute.is_empty())));
        }
        Genre::Relation => {
            let liees: Vec<i64> = brute.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            let filtre = etat.filtre_rel.to_lowercase();
            let mut pages: Vec<_> = e.pages.values().filter(|p| p.id != id && !p.base).filter(|p| filtre.is_empty() || p.nom().to_lowercase().contains(&filtre)).collect();
            // Les pages liees d'abord, puis par nom.
            pages.sort_by_key(|p| (!liees.contains(&p.id), p.nom().to_lowercase()));
            let pages: Vec<V> = pages.iter().take(40).map(|p| map([("id", t(&p.id.to_string())), ("choix", t(&format!("choix-{fid}@{}", p.id))), ("nom", t(p.nom())), ("teinte", t(crate::page::teinte(p))), ("on", t(if liees.contains(&p.id) { "on" } else { "off" }))])).collect();
            m.push(("a_pages", b(!pages.is_empty())));
            m.push(("pages", V::List(pages)));
            m.push(("filtre", t(&etat.filtre_rel)));
        }
        _ => {}
    }
    V::Map(m.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// La carte de la propriete ouverte (`Etat::prop_menu`), pour `<carte-prop>`.
pub fn carte(e: &Espace, etat: &Etat, page: i64) -> V {
    let Some(d) = etat.prop_menu.as_deref().and_then(|cle| e.definition(page, cle).or_else(|| e.page(page).and_then(|p| p.schema.iter().find(|d| d.cle() == cle).cloned()))) else {
        return map([("cle", t(""))]);
    };
    let types = TYPES.iter().map(|(c, _)| map([("code", t(c)), ("libelle", t(Genre::depuis(c, "").libelle())), ("icone", t(icone_genre(c))), ("on", t(if *c == d.genre.code() { "on" } else { "off" }))])).collect();
    let options: Vec<V> = match &d.genre {
        Genre::Selection(o) | Genre::Etiquettes(o) => o.iter().enumerate().map(|(i, x)| map([("i", t(&i.to_string())), ("nom", t(x)), ("teinte", t(&teinte_option(x)))])).collect(),
        _ => Vec::new(),
    };
    map([
        ("cle", t(&d.cle())),
        ("nom", t(&d.nom)),
        ("icone", t(icone_genre(d.genre.code()))),
        ("types", V::List(types)),
        ("a_options", b(matches!(d.genre, Genre::Selection(_) | Genre::Etiquettes(_)))),
        ("options", V::List(options)),
        ("a_formule", b(matches!(d.genre, Genre::Formule(_)))),
        ("formule", t(&d.genre.config())),
        ("focus_nom", t(if etat.nom_neuf { "tout" } else { "false" })),
        ("focus_formule", t(if !etat.nom_neuf && matches!(&d.genre, Genre::Formule(f) if f.is_empty()) { "true" } else { "false" })),
        ("confirmer", b(etat.suppr_prop)),
    ])
}

/// La liste « + Ajouter une propriete ».
pub fn types() -> V {
    V::List(TYPES.iter().map(|(c, desc)| map([("code", t(c)), ("libelle", t(Genre::depuis(c, "").libelle())), ("icone", t(icone_genre(c))), ("desc", t(desc))])).collect())
}

// ---- clics -----------------------------------------------------------------

/// Ou vit la propriete `cle` vue depuis `page` : la page elle-meme, ou le
/// schema de sa base (celle qu'on regarde, ou la base parente d'une ligne).
pub fn proprietaire(e: &Espace, page: i64, cle: &str) -> Option<(i64, bool)> {
    let p = e.page(page)?;
    if p.propres.iter().any(|d| d.cle() == cle) {
        return Some((page, false));
    }
    if p.base && p.schema.iter().any(|d| d.cle() == cle) {
        return Some((page, true));
    }
    let parent = e.page(p.parent?)?;
    (parent.base && parent.schema.iter().any(|d| d.cle() == cle)).then_some((parent.id, true))
}

fn definition_de(e: &Espace, proprio: i64, dans_schema: bool, cle: &str) -> Option<Definition> {
    let p = e.page(proprio)?;
    let defs = if dans_schema { &p.schema } else { &p.propres };
    defs.iter().find(|d| d.cle() == cle).cloned()
}

/// Les pages qui portent une valeur de cette propriete.
fn porteurs(e: &Espace, proprio: i64, dans_schema: bool) -> Vec<i64> {
    if dans_schema { e.enfants(Some(proprio)).iter().map(|p| p.id).collect() } else { vec![proprio] }
}

/// Avant chaque clic : ce qui est tape dans la carte ouverte (nom, formule)
/// s'applique ; la carte et le choix d'une valeur se ferment si on clique
/// ailleurs. Rend les erreurs (nom deja pris, formule invalide).
pub fn avant_clic(e: &mut Espace, etat: &mut Etat, page: i64, id: &str, l: &dyn Lecture) -> Result<(), String> {
    let mut erreur = Ok(());
    if let Some(cle) = etat.prop_menu.clone() {
        if let Some((proprio, dans_schema)) = proprietaire(e, page, &cle) {
            if let Some(ancienne) = definition_de(e, proprio, dans_schema, &cle) {
                let nom = l.valeur("carte-nom").map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or(ancienne.nom.clone());
                let genre = match (&ancienne.genre, l.valeur("carte-formule")) {
                    (Genre::Formule(_), Some(f)) => Genre::Formule(f.trim().to_string()),
                    (g, _) => g.clone(),
                };
                if nom != ancienne.nom || genre != ancienne.genre {
                    let neuve = Definition { nom, genre };
                    let cle_neuve = neuve.cle();
                    match e.renommer_definition(proprio, &cle, neuve, dans_schema) {
                        Ok(()) => etat.prop_menu = Some(cle_neuve),
                        Err(err) => erreur = Err(err),
                    }
                }
            }
        }
        etat.nom_neuf = false;
        if !(id.starts_with("carte-") || id.starts_with("prop-menu-")) {
            etat.prop_menu = None;
            etat.suppr_prop = false;
        }
    }
    if etat.editeur.is_some() && !["ouvrir-", "choix-", "creer-", "jour-", "mois-", "retirer-"].iter().any(|x| id.starts_with(x)) {
        etat.editeur = None;
    }
    if !id.starts_with("prop-nouveau-") && id != "prop-ouvrir" {
        etat.ajout_propriete = false;
    }
    erreur
}

/// Un nom libre pour une nouvelle propriete (« Nombre », « Nombre 2 »...).
fn nom_libre(e: &Espace, page: i64, base: &str) -> String {
    let pris: Vec<String> = e.page(page).map(|p| p.schema.iter().chain(&p.propres).map(|d| d.cle()).collect()).unwrap_or_default();
    let pris = |n: &str| pris.contains(&crate::formule::normaliser(n)) || e.definition(page, n).is_some();
    (1..).map(|i| if i == 1 { base.to_string() } else { format!("{base} {i}") }).find(|n| !pris(n)).unwrap_or_default()
}

/// Change le type d'une propriete : les options se gardent entre selection
/// et etiquettes (sinon elles viennent des valeurs deja tapees), les valeurs
/// qui n'ont plus de sens partent.
fn changer_genre(e: &mut Espace, proprio: i64, dans_schema: bool, cle: &str, code: &str) -> Result<(), String> {
    let ancienne = definition_de(e, proprio, dans_schema, cle).ok_or("propriété introuvable")?;
    if ancienne.genre.code() == code {
        return Ok(());
    }
    let pages = porteurs(e, proprio, dans_schema);
    let valeurs: Vec<(i64, String)> = pages.iter().filter_map(|p| e.page(*p).and_then(|x| x.valeurs.get(cle)).map(|v| (*p, v.clone()))).collect();
    let options = match &ancienne.genre {
        Genre::Selection(o) | Genre::Etiquettes(o) => o.clone(),
        _ => {
            let mut o: Vec<String> = Vec::new();
            for (_, v) in &valeurs {
                for x in v.split([SEP, ',']).map(str::trim).filter(|x| !x.is_empty()) {
                    if !o.iter().any(|y| y.eq_ignore_ascii_case(x)) {
                        o.push(x.to_string());
                    }
                }
            }
            o
        }
    };
    let genre = match code {
        "selection" => Genre::Selection(options),
        "etiquettes" => Genre::Etiquettes(options),
        "formule" => Genre::Formule(String::new()),
        c => Genre::depuis(c, ""),
    };
    e.definir(proprio, Definition { nom: ancienne.nom, genre }, dans_schema)?;
    // Chaque valeur repasse par la verification du nouveau type.
    for (p, v) in valeurs {
        let v = match code {
            "selection" => v.split([SEP, ',']).next().unwrap_or("").trim().to_string(),
            _ => v,
        };
        if code == "formule" || e.changer(p, cle, &v).is_err() {
            if let Ok(page) = e.page_mut(p) {
                page.valeurs.remove(cle);
            }
        }
    }
    Ok(())
}

/// Les options de la propriete (selection, etiquettes) : `f` les change.
fn avec_options(e: &mut Espace, proprio: i64, dans_schema: bool, cle: &str, f: impl FnOnce(&mut Vec<String>)) -> Result<(), String> {
    let p = e.page_mut(proprio)?;
    let defs = if dans_schema { &mut p.schema } else { &mut p.propres };
    if let Some(Definition { genre: Genre::Selection(o) | Genre::Etiquettes(o), .. }) = defs.iter_mut().find(|d| d.cle() == cle) {
        f(o);
    }
    Ok(())
}

/// Les clics de la carte et des valeurs ; `None` si l'id n'en est pas un.
pub fn cliquer(e: &mut Espace, etat: &mut Etat, page: i64, id: &str, l: &dyn Lecture) -> Result<Option<Suite>, String> {
    let suite = Some(Suite::Redessiner);
    // La liste des types, puis la creation d'un clic.
    if id == "prop-ouvrir" {
        etat.ajout_propriete = !etat.ajout_propriete;
        etat.prop_menu = None;
        return Ok(suite);
    }
    if let Some(code) = id.strip_prefix("prop-nouveau-") {
        let libelle = Genre::depuis(code, "").libelle();
        let nom = nom_libre(e, page, libelle);
        let dans_schema = e.page(page).is_some_and(|p| p.base);
        let genre = if code == "formule" { Genre::Formule(String::new()) } else { Genre::depuis(code, "") };
        let d = Definition { nom, genre };
        let cle = d.cle();
        e.definir(page, d, dans_schema)?;
        etat.ajout_propriete = false;
        etat.prop_menu = Some(cle);
        etat.nom_neuf = true;
        return Ok(suite);
    }
    if let Some(cle) = id.strip_prefix("prop-menu-") {
        // Le nom vient peut-etre de changer (`avant_clic`) : l'ancienne cle
        // n'existe plus, c'est la carte ouverte qu'on referme.
        let existe = proprietaire(e, page, cle).is_some();
        etat.prop_menu = if etat.prop_menu.as_deref() == Some(cle) || !existe { None } else { Some(cle.to_string()) };
        etat.suppr_prop = false;
        return Ok(suite);
    }
    if let Some(action) = id.strip_prefix("carte-") {
        return carte_clic(e, etat, page, action, l).map(|_| suite);
    }
    valeur_clic(e, etat, id, l)
}

fn carte_clic(e: &mut Espace, etat: &mut Etat, page: i64, action: &str, l: &dyn Lecture) -> Result<(), String> {
    let Some(cle) = etat.prop_menu.clone() else { return Ok(()) };
    let (proprio, dans_schema) = proprietaire(e, page, &cle).ok_or("propriété introuvable")?;
    match action {
        // Entree dans le nom : c'est fini.
        "nom" | "fermer" => {
            etat.prop_menu = None;
            etat.suppr_prop = false;
        }
        // Entree dans la formule : deja appliquee par `avant_clic`.
        "formule" => {}
        "opt" => {
            let nouvelle = l.valeur("carte-opt").unwrap_or_default().trim().to_string();
            if !nouvelle.is_empty() {
                avec_options(e, proprio, dans_schema, &cle, |o| {
                    if !o.iter().any(|x| x.eq_ignore_ascii_case(&nouvelle)) {
                        o.push(nouvelle);
                    }
                })?;
            }
        }
        "suppr" => etat.suppr_prop = true,
        "suppr-non" => etat.suppr_prop = false,
        "suppr-oui" => {
            e.retirer_definition(proprio, &cle, dans_schema)?;
            etat.prop_menu = None;
            etat.suppr_prop = false;
        }
        a => {
            if let Some(code) = a.strip_prefix("genre-") {
                changer_genre(e, proprio, dans_schema, &cle, code)?;
            } else if let Some(i) = a.strip_prefix("opt-suppr-").and_then(|i| i.parse::<usize>().ok()) {
                let mut retiree = None;
                avec_options(e, proprio, dans_schema, &cle, |o| {
                    if i < o.len() {
                        retiree = Some(o.remove(i));
                    }
                })?;
                // L'option part aussi des valeurs.
                if let Some(r) = retiree {
                    for p in porteurs(e, proprio, dans_schema) {
                        let Some(v) = e.page(p).and_then(|x| x.valeurs.get(&cle)).cloned() else { continue };
                        let reste: Vec<&str> = v.split(SEP).filter(|x| *x != r).collect();
                        let page = e.page_mut(p)?;
                        if reste.is_empty() {
                            page.valeurs.remove(&cle);
                        } else {
                            page.valeurs.insert(cle.clone(), reste.join(&SEP.to_string()));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn valeur_clic(e: &mut Espace, etat: &mut Etat, id: &str, l: &dyn Lecture) -> Result<Option<Suite>, String> {
    let suite = Ok(Some(Suite::Redessiner));
    if let Some(fid) = id.strip_prefix("ouvrir-") {
        etat.editeur = if etat.editeur.as_deref() == Some(fid) { None } else { Some(fid.to_string()) };
        etat.mois = None;
        etat.filtre_rel.clear();
        return suite;
    }
    let Some((action, reste)) = ["choix-", "creer-", "jour-", "mois-", "retirer-"].iter().find_map(|a| id.strip_prefix(a).map(|r| (*a, r))) else { return Ok(None) };
    let (fid, arg) = reste.split_once('@').unwrap_or((reste, ""));
    let (page, cle) = decouper_fid(fid).ok_or("valeur ?")?;
    let d = e.definition(page, &cle).ok_or("propriété introuvable")?;
    let brute = e.page(page).and_then(|p| p.valeurs.get(&cle).cloned()).unwrap_or_default();
    let options = match &d.genre {
        Genre::Selection(o) | Genre::Etiquettes(o) => o.clone(),
        _ => Vec::new(),
    };
    let mut liste: Vec<String> = match d.genre {
        Genre::Relation => brute.split(',').filter(|x| !x.is_empty()).map(str::to_string).collect(),
        _ => brute.split(SEP).filter(|x| !x.is_empty()).map(str::to_string).collect(),
    };
    let basculer = |liste: &mut Vec<String>, x: String| {
        if let Some(i) = liste.iter().position(|y| *y == x) {
            liste.remove(i);
        } else {
            liste.push(x);
        }
    };
    let ecrire = |e: &mut Espace, liste: &[String]| {
        let sep = if matches!(d.genre, Genre::Relation) { ",".to_string() } else { SEP.to_string() };
        e.changer(page, &cle, &liste.join(&sep))
    };
    match action {
        "choix-" => match &d.genre {
            Genre::Selection(_) => {
                let v = arg.parse::<usize>().ok().and_then(|i| options.get(i).cloned()).unwrap_or_default();
                e.changer(page, &cle, &v)?;
                etat.editeur = None;
            }
            Genre::Etiquettes(_) => {
                if let Some(x) = arg.parse::<usize>().ok().and_then(|i| options.get(i).cloned()) {
                    basculer(&mut liste, x);
                    ecrire(e, &liste)?;
                }
            }
            Genre::Relation => {
                basculer(&mut liste, arg.to_string());
                ecrire(e, &liste)?;
            }
            _ => {}
        },
        // Entree dans le champ du choix : une option creee (et choisie), ou
        // le filtre des pages.
        "creer-" => {
            let tape = l.valeur(id).unwrap_or_default().trim().to_string();
            match &d.genre {
                Genre::Relation => etat.filtre_rel = tape,
                _ if tape.is_empty() => {}
                Genre::Selection(_) => {
                    // Une option qui existe deja (sans la casse) est reprise telle quelle.
                    let v = options.iter().find(|o| o.eq_ignore_ascii_case(&tape)).cloned().unwrap_or(tape);
                    e.changer(page, &cle, &v)?;
                    etat.editeur = None;
                }
                _ => {
                    let v = options.iter().find(|o| o.eq_ignore_ascii_case(&tape)).cloned().unwrap_or(tape);
                    if !liste.contains(&v) {
                        liste.push(v);
                    }
                    ecrire(e, &liste)?;
                }
            }
        }
        "jour-" => {
            let v = if arg == "auj" { date_texte(aujourdhui()) } else { arg.to_string() };
            e.changer(page, &cle, &v)?;
            etat.editeur = None;
        }
        "mois-" => {
            let courant = etat.mois.or_else(|| crate::modele::date_valide(&brute).map(|(a, m, _)| (a, m))).unwrap_or_else(|| {
                let (a, m, _) = aujourdhui();
                (a, m)
            });
            etat.mois = Some(match arg {
                "auj" => {
                    let (a, m, _) = aujourdhui();
                    (a, m)
                }
                d => decaler(courant, d.parse().unwrap_or(0)),
            });
        }
        _ => {
            // retirer- : l'element `arg` (son texte, ou l'id d'une page).
            liste.retain(|x| x != arg);
            ecrire(e, &liste)?;
        }
    }
    suite
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(jours(1970, 1, 1), 0);
        assert_eq!(civil(jours(2026, 9, 29)), (2026, 9, 29));
        assert_eq!(civil(jours(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(decaler((2026, 12), 1), (2027, 1));
        assert_eq!(decaler((2026, 1), -1), (2025, 12));
        assert_eq!(date_lisible("2026-09-29"), "29 sept. 2026");
        // Septembre 2026 commence un mardi : lundi 31 aout en tete.
        let s = calendrier("v-1-d", (2026, 9), "2026-09-29");
        let V::Map(premiere) = &s[0] else { panic!() };
        let V::List(j) = &premiere["jours"] else { panic!() };
        let V::Map(lundi) = &j[0] else { panic!() };
        assert_eq!(lundi["date"], t("2026-08-31"));
        assert_eq!(s.len(), 5);
    }
}
