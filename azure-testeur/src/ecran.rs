// L'ecran (ui/testeur.rsh, une seule route `/`) : ce qu'il montre vient
// des projets, de l'analyse du projet ouvert, de l'execution en cours et
// de l'atelier. Les clics sont dans `clics`.
use crate::composants::{Bibliotheque, Garde, GardeMemoire, Genre, Id};
use crate::execution::Lanceur;
use crate::langage::{self, Analyse, FichierTests, Langage, Statut, Test};
use crate::projet::{Projet, Projets};
use azure_foundation::compiler::services::condition::{ConditionValue as V, Context};
use azure_foundation::navigation::models::route_table::RouteTable;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Onglet {
    #[default]
    Tests,
    Atelier,
    /// La bibliotheque de composants (barre de gauche).
    Composants,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Vue {
    #[default]
    Tous,
    Echecs,
    Reussis,
    Jamais,
}

impl Vue {
    pub const TOUTES: [(Vue, &'static str, &'static str); 4] = [(Vue::Tous, "tous", "Tous"), (Vue::Echecs, "echecs", "Échecs"), (Vue::Reussis, "reussis", "Réussis"), (Vue::Jamais, "jamais", "Pas lancés")];

    fn garde(self, s: Statut) -> bool {
        match self {
            Vue::Tous => true,
            Vue::Echecs => matches!(s, Statut::Echoue | Statut::Erreur),
            Vue::Reussis => s == Statut::Reussi,
            Vue::Jamais => matches!(s, Statut::Jamais | Statut::Ignore),
        }
    }
}

/// Le bloc de l'atelier ouvert dans l'editeur de code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ouvert {
    Generique,
    Nouveau,
    /// Le test de ce nom.
    Test(String),
}

/// Lignes de tests affichees au plus (le reste : filtrer ou replier) ;
/// chaque ligne coute a construire, dessiner et survoler.
pub const LIGNES_MAX: usize = 150;

/// Lignes de sortie montrees dans la console.
pub const CONSOLE_MAX: usize = 200;

/// Au-dela, les fichiers sont replies au depart.
pub const DEPLIES_JUSQUA: usize = 60;

/// Ce qu'on regarde et tape dans la bibliotheque de composants. Garde
/// quand on change de projet.
#[derive(Clone, Debug, Default)]
pub struct EtatComposants {
    /// Le noeud ouvert a droite.
    pub choisi: Option<Id>,
    /// Noeuds dont les enfants sont caches dans l'arbre.
    pub plies: HashSet<Id>,
    /// Confirmer la suppression du noeud ouvert.
    pub supprimer: bool,
    /// Nom et code tapes, pas encore enregistres (pour le noeud ouvert).
    pub nom: Option<(Id, String)>,
    pub code: Option<(Id, String)>,
    /// Le champ du nom prend le focus (tout selectionne) au prochain dessin.
    pub nommer: bool,
}

/// Ce que l'utilisateur regarde et a commence a ecrire.
#[derive(Default)]
pub struct Etat {
    pub onglet: Onglet,
    pub projet: Option<usize>,
    pub langage: Option<Arc<dyn Langage>>,
    pub analyse: Analyse,
    /// Filtre applique (Entree) et ce qui est tape dans le champ.
    pub filtre: String,
    pub saisie_filtre: String,
    pub saisie_chemin: String,
    pub vue: Vue,
    /// Fichiers dont on a change le pli (par rapport au depart).
    pub bascules: HashSet<String>,
    /// Cle du test montre a droite.
    pub choisi: Option<String>,
    pub console: bool,
    pub message: String,
    pub erreur: String,
    // --- Atelier ---
    pub fichier: Option<String>,
    pub lu: Option<FichierTests>,
    /// Ce qui est tape et pas encore ecrit dans le code, par fichier puis
    /// par champ (`generique`, `nom@<test>`, `code@<test>`, `opt@<test>@<option>`,
    /// `nouveau-nom`...).
    pub brouillons: HashMap<String, BTreeMap<String, String>>,
    /// Test dont on confirme la suppression.
    pub supprimer: Option<String>,
    /// Ce qui est ouvert dans l'editeur (un seul a la fois, le reste est une
    /// liste legere) : voir `Ouvert`.
    pub ouvert: Option<Ouvert>,
    pub nouveau_fichier: bool,
    /// Nom tape pour un nouveau fichier.
    pub nf_nom: String,
    pub composants: EtatComposants,
}

impl Etat {
    pub fn brouillon(&self, champ: &str) -> Option<&String> {
        self.brouillons.get(self.fichier.as_deref()?)?.get(champ)
    }

    pub fn fixer_brouillon(&mut self, champ: &str, valeur: Option<String>) {
        let Some(f) = self.fichier.clone() else { return };
        let b = self.brouillons.entry(f).or_default();
        match valeur {
            Some(v) => {
                b.insert(champ.to_string(), v);
            }
            None => {
                b.remove(champ);
            }
        }
    }

    pub fn oublier_brouillons(&mut self, prefixe: &str) {
        if let Some(b) = self.fichier.as_ref().and_then(|f| self.brouillons.get_mut(f)) {
            b.retain(|k, _| !k.starts_with(prefixe));
        }
    }
}

/// Tout ce que l'app partage entre la fenetre, les clics et l'execution.
pub struct Testeur {
    pub projets: Mutex<Projets>,
    pub etat: Mutex<Etat>,
    pub lanceur: Lanceur,
    /// Les composants de l'utilisateur (propres a Testeur).
    pub bibliotheque: Mutex<Bibliotheque>,
}

impl Testeur {
    /// Composants en memoire seulement : voir `avec`.
    pub fn new(projets: Projets) -> Arc<Testeur> {
        Testeur::avec(projets, Box::new(GardeMemoire::default()))
    }

    /// Composants gardes par `garde` (le stockage prive de l'app).
    pub fn avec(projets: Projets, garde: Box<dyn Garde>) -> Arc<Testeur> {
        let t = Arc::new(Testeur { projets: Mutex::new(projets), etat: Mutex::new(Etat::default()), lanceur: Lanceur::default(), bibliotheque: Mutex::new(Bibliotheque::new(garde)) });
        // Le premier projet (l'environnement s'il est connu) s'ouvre seul.
        if !t.projets().liste.is_empty() {
            t.ouvrir(0);
        }
        t
    }

    pub fn projets(&self) -> MutexGuard<'_, Projets> {
        self.projets.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn etat(&self) -> MutexGuard<'_, Etat> {
        self.etat.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A prendre APRES `etat()` si les deux sont tenus ensemble.
    pub fn bibliotheque(&self) -> MutexGuard<'_, Bibliotheque> {
        self.bibliotheque.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn projet(&self) -> Option<Projet> {
        let i = self.etat().projet?;
        self.projets().liste.get(i).cloned()
    }

    /// Ouvre le projet `i` et l'analyse.
    pub fn ouvrir(&self, i: usize) {
        let projet = self.projets().liste.get(i).cloned();
        let mut e = self.etat();
        let garde_console = e.console;
        let saisie_chemin = std::mem::take(&mut e.saisie_chemin);
        let composants = std::mem::take(&mut e.composants);
        *e = Etat { projet: Some(i), onglet: e.onglet, console: garde_console, saisie_chemin, composants, ..Etat::default() };
        drop(e);
        if let Some(p) = projet {
            self.analyser(&p);
        }
    }

    /// (Re)lit les tests du projet ouvert.
    pub fn analyser(&self, p: &Projet) {
        let langage = langage::detecter(&p.chemin);
        let analyse = langage.as_ref().map(|l| l.analyser(&p.chemin)).unwrap_or_default();
        let mut e = self.etat();
        if langage.is_none() {
            e.erreur = format!("Aucun langage reconnu dans {} (Testeur connaît : {}).", p.chemin.display(), langage::langages().iter().map(|l| l.nom()).collect::<Vec<_>>().join(", "));
        }
        e.langage = langage;
        e.analyse = analyse;
        // Le fichier ouvert dans l'atelier est relu.
        if let (Some(f), Some(l)) = (e.fichier.clone(), e.langage.clone()) {
            e.lu = l.lire_fichier(&p.chemin, &f).ok();
        }
    }
}

// ------------------------------------------------------------------ vues

fn t(s: &str) -> V {
    V::Text(s.to_string())
}

fn b(x: bool) -> V {
    V::Bool(x)
}

fn n(x: usize) -> V {
    V::Number(x as f64)
}

fn map<const N: usize>(entrees: [(&str, V); N]) -> V {
    V::map(entrees)
}

fn lignes(l: &[String]) -> V {
    V::List(l.iter().map(|x| t(x)).collect())
}

/// Code montre dans un bloc de code facon Azure Note (colore).
pub fn code_riche(code: &str) -> String {
    azure_note::code::colorer(code)
}

/// Les tests visibles (filtre + vue), avec leur rang dans l'analyse.
pub fn visibles<'a>(e: &'a Etat, statut: &dyn Fn(&str) -> Statut) -> Vec<(usize, &'a Test)> {
    let filtre = e.filtre.trim().to_lowercase();
    e.analyse
        .tests
        .iter()
        .enumerate()
        .filter(|(_, x)| filtre.is_empty() || x.chemin.to_lowercase().contains(&filtre) || x.fichier.to_lowercase().contains(&filtre))
        .filter(|(_, x)| e.vue.garde(statut(&x.cle())))
        .collect()
}

/// Le fichier est-il deplie ?
pub fn deplie(e: &Etat, fichier: &str, nb_visibles: usize) -> bool {
    let depart = nb_visibles <= DEPLIES_JUSQUA || !e.filtre.trim().is_empty();
    depart != e.bascules.contains(fichier)
}

/// Les fichiers de tests proposes dans l'atelier (et le nombre de tests).
pub fn fichiers_atelier(e: &Etat) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for x in &e.analyse.tests {
        match out.iter_mut().find(|f| f.0 == x.fichier) {
            Some(f) => f.1 += 1,
            None => out.push((x.fichier.clone(), 1)),
        }
    }
    if let Some(f) = &e.fichier
        && !out.iter().any(|x| &x.0 == f)
    {
        out.push((f.clone(), 0));
    }
    out
}

/// Les donnees de l'ecran.
pub fn contexte(testeur: &Testeur) -> Context {
    let projets = testeur.projets().liste.clone();
    let exec = testeur.lanceur.lire();
    let e = testeur.etat();
    let statut = |cle: &str| exec.statut(cle);
    let liste = projets.iter().enumerate().map(|(i, p)| map([("i", n(i)), ("nom", t(&p.nom)), ("chemin", t(&p.chemin.to_string_lossy())), ("env", b(p.environnement)), ("actif", b(e.projet == Some(i)))])).collect();
    let mut ctx = Context::new()
        .with_value("projets", V::List(liste))
        .with_text(
            "onglet",
            match e.onglet {
                Onglet::Tests => "tests",
                Onglet::Atelier => "atelier",
                Onglet::Composants => "composants",
            },
        )
        .with_text("saisie_chemin", &e.saisie_chemin)
        .with_text("message", &e.message)
        .with_text("erreur", &e.erreur)
        .with_bool("en_cours", exec.en_cours)
        .with_text("exec_titre", &exec.titre)
        .with_text("exec_etape", &if exec.nb_etapes > 1 { format!("étape {}/{}", exec.etape, exec.nb_etapes) } else { String::new() })
        .with_text("bilan", &exec.bilan)
        .with_bool("echec_commande", exec.echec_commande)
        .with_bool("console", e.console)
        .with_value("console_lignes", lignes(&exec.lignes[exec.lignes.len().saturating_sub(CONSOLE_MAX)..]))
        .with_bool("a_sortie", !exec.lignes.is_empty());
    ctx = vue_composants(ctx, &e, &testeur.bibliotheque());
    let Some(p) = e.projet.and_then(|i| projets.get(i)) else {
        return ctx.with_bool("a_projet", false);
    };
    let langage = e.langage.as_ref().map(|l| l.nom()).unwrap_or("langage inconnu");
    let compte = |s: Statut| e.analyse.tests.iter().filter(|x| statut(&x.cle()) == s).count();
    ctx = ctx
        .with_bool("a_projet", true)
        .with_value("projet", map([("nom", t(&p.nom)), ("chemin", t(&p.chemin.to_string_lossy())), ("langage", t(langage)), ("env", b(p.environnement)), ("nb_paquets", n(e.analyse.paquets.len()))]))
        .with_number("nb_tests", e.analyse.tests.len() as f64)
        .with_number("nb_reussis", compte(Statut::Reussi) as f64)
        .with_number("nb_echoues", (compte(Statut::Echoue) + compte(Statut::Erreur)) as f64)
        .with_number("nb_ignores", compte(Statut::Ignore) as f64)
        .with_number("nb_jamais", compte(Statut::Jamais) as f64);
    match e.onglet {
        Onglet::Tests => vue_tests(ctx, &e, &exec, &p.chemin),
        Onglet::Atelier => vue_atelier(ctx, &e, &exec, &p.chemin),
        Onglet::Composants => ctx,
    }
}

fn vue_tests(ctx: Context, e: &Etat, exec: &crate::execution::Execution, projet: &Path) -> Context {
    let statut = |cle: &str| exec.statut(cle);
    let visibles = visibles(e, &statut);
    let mut groupes: Vec<(String, Vec<(usize, &Test)>)> = Vec::new();
    for (i, x) in &visibles {
        match groupes.last_mut() {
            Some(g) if g.0 == x.fichier => g.1.push((*i, x)),
            _ => groupes.push((x.fichier.clone(), vec![(*i, x)])),
        }
    }
    let mut reste = LIGNES_MAX;
    let mut caches = 0usize;
    let groupes: Vec<V> = groupes
        .iter()
        .map(|(fichier, tests)| {
            let deplie_ici = deplie(e, fichier, visibles.len());
            let montres = if deplie_ici { tests.len().min(reste) } else { 0 };
            if deplie_ici {
                caches += tests.len() - montres;
                reste -= montres;
            }
            let ouvert = montres > 0;
            let echecs = tests.iter().filter(|(_, x)| matches!(statut(&x.cle()), Statut::Echoue | Statut::Erreur)).count();
            let reussis = tests.iter().filter(|(_, x)| statut(&x.cle()) == Statut::Reussi).count();
            let lignes: Vec<V> = if ouvert {
                tests
                    .iter()
                    .take(montres)
                    .map(|(i, x)| {
                        let s = statut(&x.cle());
                        map([("i", n(*i)), ("nom", t(&x.chemin)), ("ligne", n(x.ligne)), ("statut", t(s.code())), ("libelle", t(s.libelle())), ("choisi", b(e.choisi.as_deref() == Some(x.cle().as_str()))), ("ignore", b(x.options.contains_key("ignore")))])
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let (dossier, nom) = fichier.rsplit_once('/').map(|(d, n)| (format!("{d}/"), n.to_string())).unwrap_or((String::new(), fichier.clone()));
            map([("premier", n(tests[0].0)), ("dossier", t(&dossier)), ("nom", t(&nom)), ("nb", n(tests.len())), ("ouvert", b(ouvert)), ("coupe", b(deplie_ici && montres < tests.len())), ("nb_echecs", n(echecs)), ("nb_reussis", n(reussis)), ("tests", V::List(lignes))])
        })
        .collect();
    let vues = Vue::TOUTES.iter().map(|(v, code, nom)| map([("code", t(code)), ("nom", t(nom)), ("on", b(*v == e.vue))])).collect();
    let mut ctx = ctx
        .with_value("groupes", V::List(groupes))
        .with_number("nb_visibles", visibles.len() as f64)
        .with_number("nb_caches", caches as f64)
        .with_text("filtre", &e.filtre)
        .with_text("saisie_filtre", &e.saisie_filtre)
        .with_value("vues", V::List(vues))
        .with_value("avertissements", lignes(&e.analyse.avertissements));
    // Le test choisi : a droite, avec son code (bloc facon Azure Note).
    let choisi = e.choisi.as_ref().and_then(|c| e.analyse.tests.iter().enumerate().find(|(_, x)| &x.cle() == c));
    ctx = match choisi {
        Some((i, x)) => {
            let r = exec.resultats.get(&x.cle()).cloned().unwrap_or_default();
            let code = e.langage.as_ref().and_then(|l| l.lire_fichier(projet, &x.fichier).ok()).and_then(|f| f.tests.into_iter().find(|s| s.ligne == x.ligne || s.nom == x.nom)).map(|s| s.code).unwrap_or_default();
            ctx.with_bool("a_detail", true).with_value(
                "detail",
                map([("i", n(i)), ("nom", t(&x.nom)), ("chemin", t(&x.chemin)), ("fichier", t(&x.fichier)), ("ligne", n(x.ligne)), ("cible", t(&x.cible)), ("statut", t(r.statut.code())), ("libelle", t(r.statut.libelle())), ("sortie", lignes(&r.sortie)), ("a_sortie", b(!r.sortie.is_empty())), ("code", t(&code_riche(&code)))]),
            )
        }
        None => ctx.with_bool("a_detail", false),
    };
    ctx
}

fn vue_atelier(ctx: Context, e: &Etat, exec: &crate::execution::Execution, _projet: &Path) -> Context {
    let fichiers: Vec<V> = fichiers_atelier(e).iter().enumerate().map(|(k, (f, nb))| map([("k", n(k)), ("chemin", t(f)), ("nb", n(*nb)), ("actif", b(e.fichier.as_deref() == Some(f.as_str())))])).collect();
    let paquets = e.analyse.paquets.iter().map(|p| if p.dossier.is_empty() { p.nom.clone() } else { format!("{} ({})", p.nom, p.dossier) }).collect::<Vec<_>>().join(", ");
    let options = e.langage.as_ref().map(|l| l.options()).unwrap_or(&[]);
    let mut ctx = ctx
        .with_value("fichiers", V::List(fichiers))
        .with_bool("nouveau_fichier", e.nouveau_fichier)
        .with_text("paquets", &paquets)
        .with_text("nf_nom", &e.nf_nom);
    let (Some(f), Some(lu)) = (&e.fichier, &e.lu) else {
        return ctx.with_bool("a_fichier", false);
    };
    let val = |champ: &str, defaut: &str| e.brouillon(champ).cloned().unwrap_or_else(|| defaut.to_string());
    let ouvert = e.ouvert.as_ref();
    let tests: Vec<V> = lu
        .tests
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let cle_test = e.analyse.tests.iter().find(|x| &x.fichier == f && x.nom == s.nom).map(|x| x.cle()).unwrap_or_default();
            let st = exec.statut(&cle_test);
            let sortie = exec.resultats.get(&cle_test).map(|r| r.sortie.clone()).unwrap_or_default();
            let opts: Vec<V> = options
                .iter()
                .map(|o| {
                    let coche = val(&format!("opt@{}@{}", s.nom, o.code), if s.options.contains_key(o.code) { "true" } else { "false" }) == "true";
                    map([("code", t(o.code)), ("libelle", t(o.libelle)), ("coche", t(if coche { "true" } else { "false" }))])
                })
                .collect();
            let modifie = e.brouillons.get(f).is_some_and(|b| b.keys().any(|k| k.split('@').nth(1) == Some(s.nom.as_str())));
            map([
                ("i", n(i)),
                ("nom", t(&val(&format!("nom@{}", s.nom), &s.nom))),
                ("vrai_nom", t(&s.nom)),
                ("ouvert", b(ouvert == Some(&Ouvert::Test(s.nom.clone())))),
                // Seul le test ouvert a son editeur (texte riche mis en page a
                // chaque image : tous a la fois figeraient la fenetre).
                ("code", t(&if ouvert == Some(&Ouvert::Test(s.nom.clone())) { code_riche(&val(&format!("code@{}", s.nom), &s.code)) } else { String::new() })),
                ("ligne", n(s.ligne)),
                ("options", V::List(opts)),
                ("statut", t(st.code())),
                ("libelle", t(st.libelle())),
                ("sortie", lignes(&sortie)),
                ("a_sortie", b(!sortie.is_empty())),
                ("modifie", b(modifie)),
                ("confirmer", b(e.supprimer.as_deref() == Some(s.nom.as_str()))),
            ])
        })
        .collect();
    let modele = e.langage.as_ref().map(|l| l.modele_test()).unwrap_or("");
    let nouveau_opts: Vec<V> = options.iter().map(|o| map([("code", t(o.code)), ("libelle", t(o.libelle)), ("coche", t(&val(&format!("nouveau-opt@{}", o.code), "false")))])).collect();
    ctx = ctx
        .with_bool("a_fichier", true)
        .with_value(
            "atelier",
            map([
                ("chemin", t(f)),
                ("generique_ouvert", b(ouvert == Some(&Ouvert::Generique))),
                ("generique_lignes", n(lu.generique.lines().count())),
                ("generique", t(&if ouvert == Some(&Ouvert::Generique) { code_riche(&val("generique", &lu.generique)) } else { String::new() })),
                ("generique_modifie", b(e.brouillon("generique").is_some())),
                ("tests", V::List(tests)),
                ("nb", n(lu.tests.len())),
                ("nouveau_nom", t(&val("nouveau-nom", ""))),
                ("nouveau_ouvert", b(ouvert == Some(&Ouvert::Nouveau))),
                ("nouveau_code", t(&if ouvert == Some(&Ouvert::Nouveau) { code_riche(&val("nouveau-code", modele)) } else { String::new() })),
                ("nouveau_options", V::List(nouveau_opts)),
            ]),
        );
    ctx
}

/// Quand redessiner pendant une execution (appele a chaque tic). Un
/// redessin de la liste coute cher, et bien plus quand cargo et les tests
/// occupent tous les coeurs : on ne redessine que si ce qui est a l'ecran a
/// change - un resultat (la sortie seulement si la console est ouverte) -
/// au plus tous les `RYTHME`, et dans l'atelier seulement a la fin (pour ne
/// pas couper la frappe). Le rythme s'allonge avec le cout mesure du
/// dernier redessin : la fenetre passe au plus 1/`PART` de son temps a
/// redessiner et reste libre pour les clics.
#[derive(Debug)]
pub struct Rafraichir {
    vu: u64,
    tournait: bool,
    dernier: std::time::Instant,
    /// Duree du dernier redessin (mesuree au tic suivant).
    cout: std::time::Duration,
    mesurer: bool,
}

impl Rafraichir {
    pub const RYTHME: std::time::Duration = std::time::Duration::from_millis(500);
    pub const PART: u32 = 8;

    pub fn new() -> Rafraichir {
        Rafraichir { vu: 0, tournait: false, dernier: std::time::Instant::now(), cout: std::time::Duration::ZERO, mesurer: false }
    }

    /// L'attente minimale entre deux redessins.
    pub fn rythme(&self) -> std::time::Duration {
        Self::RYTHME.max(self.cout * Self::PART)
    }

    /// Faut-il redessiner maintenant ? (Si oui : c'est note comme fait.)
    pub fn maintenant(&mut self, t: &Testeur) -> bool {
        // Premier tic apres un redessin : il a dure (a peu pres) jusqu'ici.
        if std::mem::take(&mut self.mesurer) {
            self.cout = self.dernier.elapsed();
        }
        let (console, atelier) = {
            let e = t.etat();
            (e.console, e.onglet == Onglet::Atelier)
        };
        let (version, en_cours) = {
            let x = t.lanceur.lire();
            (if console { x.version } else { x.version_resultats }, x.en_cours)
        };
        if version == self.vu {
            return false;
        }
        self.tournait |= en_cours;
        let fini = self.tournait && !en_cours;
        if !fini && (atelier || self.dernier.elapsed() < self.rythme()) {
            return false;
        }
        self.vu = version;
        self.tournait = en_cours;
        self.dernier = std::time::Instant::now();
        self.mesurer = true;
        true
    }
}

impl Default for Rafraichir {
    fn default() -> Rafraichir {
        Rafraichir::new()
    }
}

/// Profondeur montree dans l'arbre (au-dela, meme retrait).
pub const PROFONDEUR_MAX: usize = 8;

/// Valeur d'une option de liste pour le noeud `id` : `12. Stockage › nom`
/// (les virgules separent les options : retirees).
pub fn option_noeud(id: Id, chemin: &str) -> String {
    format!("{id}. {}", chemin.replace(',', " "))
}

/// Le noeud d'une option choisie (voir `option_noeud`).
pub fn noeud_de_option(valeur: &str) -> Option<Id> {
    valeur.split_once('.')?.0.trim().parse().ok()
}

/// La barre de gauche (arbre), la page du noeud ouvert, et la liste des
/// composants a inserer dans un nouveau test.
fn vue_composants(ctx: Context, e: &Etat, bib: &Bibliotheque) -> Context {
    let ec = &e.composants;
    let arbre: Vec<V> = bib
        .arbre(&ec.plies)
        .into_iter()
        .map(|(x, prof)| {
            let a_enfants = bib.enfants(Some(x.id)).next().is_some();
            map([
                ("id", n(x.id as usize)),
                ("nom", t(&x.nom)),
                ("genre", t(x.genre.code())),
                ("prof", n(prof.min(PROFONDEUR_MAX))),
                ("a_enfants", b(a_enfants)),
                ("plie", b(ec.plies.contains(&x.id))),
                ("on", b(ec.choisi == Some(x.id) && e.onglet == Onglet::Composants)),
            ])
        })
        .collect();
    let composants = bib.composants();
    let options = composants.iter().map(|(id, c)| option_noeud(*id, c)).collect::<Vec<_>>().join(", ");
    let mut ctx = ctx
        .with_value("comp_arbre", V::List(arbre))
        .with_bool("a_composants_arbre", !bib.noeuds.is_empty())
        .with_text("composants_options", &options)
        .with_bool("a_composants", !composants.is_empty());
    let Some(x) = ec.choisi.and_then(|id| bib.noeud(id)) else {
        return ctx.with_bool("comp_a_choix", false);
    };
    let ancetres = bib.chemin(x.id);
    let chemin: Vec<V> = ancetres.iter().enumerate().map(|(k, c)| map([("id", n(c.id as usize)), ("nom", t(&c.nom)), ("dernier", b(k + 1 == ancetres.len()))])).collect();
    let enfants: Vec<V> = {
        let mut v: Vec<_> = bib.enfants(Some(x.id)).collect();
        v.sort_by_key(|c| c.genre == Genre::Composant);
        v.into_iter().map(|c| map([("id", n(c.id as usize)), ("nom", t(&c.nom)), ("genre", t(c.genre.code())), ("libelle", t(c.genre.libelle())), ("nb", n(bib.descendants(c.id).len() - 1))])).collect()
    };
    // Ou le deplacer : tout contenant hors de lui-meme et de ses descendants.
    let interdits = bib.descendants(x.id);
    let destinations = bib.arbre(&HashSet::new()).into_iter().filter(|(c, _)| c.genre.contenant() && !interdits.contains(&c.id) && Some(c.id) != x.parent).map(|(c, _)| option_noeud(c.id, &bib.chemin_texte(c.id))).collect::<Vec<_>>().join(", ");
    let nom = ec.nom.as_ref().filter(|(id, _)| *id == x.id).map(|(_, v)| v.clone()).unwrap_or_else(|| x.nom.clone());
    let code_tape = ec.code.as_ref().filter(|(id, _)| *id == x.id).map(|(_, v)| v.clone());
    let code = code_tape.clone().unwrap_or_else(|| x.code.clone());
    ctx = ctx.with_bool("comp_a_choix", true).with_value(
        "comp",
        map([
            ("id", n(x.id as usize)),
            ("genre", t(x.genre.code())),
            ("libelle", t(x.genre.libelle())),
            ("nom", t(&nom)),
            ("nommer", t(if ec.nommer { "tout" } else { "false" })),
            ("chemin", V::List(chemin)),
            ("contenant", b(x.genre.contenant())),
            ("enfants", V::List(enfants)),
            ("a_enfants", b(bib.enfants(Some(x.id)).next().is_some())),
            ("nb_dedans", n(interdits.len() - 1)),
            ("code", t(&if x.genre == Genre::Composant { code_riche(&code) } else { String::new() })),
            ("lignes", n(code.lines().count())),
            ("modifie", b(code_tape.is_some())),
            ("confirmer", b(ec.supprimer)),
            ("destinations", t(&destinations)),
            ("a_destinations", b(!destinations.is_empty())),
        ]),
    );
    ctx
}

/// La table de routes de la fenetre.
pub fn routes(ui: &Path, testeur: &Arc<Testeur>) -> RouteTable {
    let rsh = ui.join("testeur.rsh").to_string_lossy().into_owned();
    let rsc = ui.join("testeur.rsc").to_string_lossy().into_owned();
    let testeur = Arc::clone(testeur);
    RouteTable::new().view_with("/", &rsh, &rsc, move |_| contexte(&testeur))
}
