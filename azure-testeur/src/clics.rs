// Ce que font les clics (et Entree dans un champ). Avant toute action, les
// champs de l'ecran sont ranges (`ranger`) : ce qui est tape dans l'atelier
// survit au redessin, jusqu'a ce qu'on l'ecrive dans le code du projet.
use crate::composants::{Genre, Id};
use crate::ecran::{deplie, fichiers_atelier, noeud_de_option, visibles, Onglet, Ouvert, Testeur, Vue};
use crate::langage::{Operation, Statut, Test, TestSource};
use std::collections::BTreeMap;

/// Les champs de l'ecran au moment du clic.
pub trait Lecture {
    fn valeur(&self, id: &str) -> Option<String>;
    fn coche(&self, id: &str) -> bool {
        self.valeur(id).as_deref() == Some("true")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redessin {
    Rien,
    /// Meme page, les zones qui defilent restent ou elles sont.
    Garder,
    /// Page changee : on remonte en haut.
    Haut,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reponse {
    pub redessin: Redessin,
    /// Faire defiler jusqu'a cet element apres le redessin.
    pub defiler: Option<String>,
}

fn rep(redessin: Redessin) -> Reponse {
    Reponse { redessin, defiler: None }
}

/// Texte d'une zone de code (texte riche d'Azure Note) : sans les styles.
fn brut(v: &str) -> String {
    azure_note::riche::brut(v).trim_end().to_string()
}

/// Range ce qui est tape (champs de saisie, atelier) dans l'etat.
pub fn ranger(t: &Testeur, champs: &dyn Lecture) {
    let mut e = t.etat();
    if let Some(v) = champs.valeur("filtre") {
        e.saisie_filtre = v;
    }
    if let Some(v) = champs.valeur("chemin") {
        e.saisie_chemin = v;
    }
    if let Some(v) = champs.valeur("nf-nom") {
        e.nf_nom = v;
    }
    ranger_composant(t, &mut e, champs);
    let Some(lu) = e.lu.clone() else { return };
    let garder = |e: &mut crate::ecran::Etat, champ: &str, lu: Option<String>, fichier: &str| {
        if let Some(v) = lu {
            let change = v != fichier;
            e.fixer_brouillon(champ, change.then_some(v));
        }
    };
    garder(&mut e, "generique", champs.valeur("generique").map(|v| brut(&v)), &lu.generique);
    for (i, s) in lu.tests.iter().enumerate() {
        garder(&mut e, &format!("nom@{}", s.nom), champs.valeur(&format!("nom-{i}")).map(|v| v.trim().to_string()), &s.nom);
        garder(&mut e, &format!("code@{}", s.nom), champs.valeur(&format!("code-{i}")).map(|v| brut(&v)), s.code.trim_end());
        let options = e.langage.as_ref().map(|l| l.options()).unwrap_or(&[]);
        for o in options {
            let id = format!("opt-{i}-{}", o.code);
            let lu_champ = champs.valeur(&id).map(|_| champs.coche(&id).to_string());
            garder(&mut e, &format!("opt@{}@{}", s.nom, o.code), lu_champ, if s.options.contains_key(o.code) { "true" } else { "false" });
        }
    }
    let modele = e.langage.as_ref().map(|l| l.modele_test()).unwrap_or("");
    garder(&mut e, "nouveau-nom", champs.valeur("nouveau-nom").map(|v| v.trim().to_string()), "");
    garder(&mut e, "nouveau-code", champs.valeur("nouveau-code").map(|v| brut(&v)), modele);
    let options = e.langage.as_ref().map(|l| l.options()).unwrap_or(&[]);
    for o in options {
        let id = format!("nouveau-opt-{}", o.code);
        let lu_champ = champs.valeur(&id).map(|_| champs.coche(&id).to_string());
        garder(&mut e, &format!("nouveau-opt@{}", o.code), lu_champ, "false");
    }
}

/// Le nom et le code tapes pour le noeud ouvert de la bibliotheque (gardes
/// tant qu'ils different de ce qui est enregistre).
fn ranger_composant(t: &Testeur, e: &mut crate::ecran::Etat, champs: &dyn Lecture) {
    let Some(id) = e.composants.choisi else { return };
    let Some((nom, code)) = t.bibliotheque().noeud(id).map(|n| (n.nom.clone(), n.code.clone())) else { return };
    if let Some(v) = champs.valeur("comp-nom") {
        e.composants.nom = (v != nom).then_some((id, v));
    }
    if let Some(v) = champs.valeur("comp-code").map(|v| brut(&v)) {
        e.composants.code = (v != code.trim_end()).then_some((id, v));
    }
}

/// Ouvre le noeud `id` de la bibliotheque a droite.
fn choisir_noeud(t: &Testeur, id: Option<Id>, nommer: bool) {
    let mut e = t.etat();
    let c = &mut e.composants;
    if c.choisi != id {
        c.nom = None;
        c.code = None;
    }
    c.choisi = id;
    c.supprimer = false;
    c.nommer = nommer;
    // Ses parents sont deplies : on le voit dans l'arbre.
    if let Some(id) = id {
        for p in t.bibliotheque().chemin(id) {
            if p.id != id {
                c.plies.remove(&p.id);
            }
        }
    }
    e.onglet = Onglet::Composants;
}

/// Cree un noeud sous le noeud ouvert (sous son parent si c'est un
/// composant) et l'ouvre, nom a taper.
fn creer_noeud(t: &Testeur, genre: Genre) -> Result<(), String> {
    let choisi = t.etat().composants.choisi;
    let id = {
        let mut b = t.bibliotheque();
        let parent = match (genre, choisi.and_then(|c| b.noeud(c))) {
            (Genre::Stockage, _) => None,
            (_, Some(n)) if n.genre.contenant() => Some(n.id),
            (_, Some(n)) => n.parent,
            (_, None) => None,
        };
        let nom = match genre {
            Genre::Stockage => "Nouveau stockage",
            Genre::Dossier => "Nouveau dossier",
            Genre::Composant => "nouveau_composant",
        };
        b.creer(parent, genre, nom)?
    };
    choisir_noeud(t, Some(id), true);
    Ok(())
}

/// Les actions de la bibliotheque de composants (`None` : pas la sienne).
fn action_composants(t: &Testeur, id: &str, champs: &dyn Lecture) -> Option<Result<Reponse, String>> {
    let garder = Ok(rep(Redessin::Garder));
    // Le focus sur le nom ne vaut que pour le dessin juste apres une creation.
    let choisi = {
        let mut e = t.etat();
        e.composants.nommer = false;
        e.composants.choisi
    };
    let r = match id {
        "onglet-composants" => {
            t.etat().onglet = Onglet::Composants;
            return Some(Ok(rep(Redessin::Haut)));
        }
        "comp-nouveau-stockage" => creer_noeud(t, Genre::Stockage).and(garder),
        "comp-dossier" => creer_noeud(t, Genre::Dossier).and(garder),
        "comp-composant" => creer_noeud(t, Genre::Composant).and(garder),
        "comp-nom" | "comp-renommer" => (|| {
            let id = choisi.ok_or("Aucun élément ouvert.")?;
            let nom = champs.valeur("comp-nom").unwrap_or_default();
            t.bibliotheque().renommer(id, &nom)?;
            let mut e = t.etat();
            e.composants.nom = None;
            e.composants.nommer = false;
            e.message = format!("Renommé en « {} ».", nom.trim());
            garder.clone()
        })(),
        "comp-enregistrer" => (|| {
            let id = choisi.ok_or("Aucun composant ouvert.")?;
            let code = t.etat().composants.code.as_ref().filter(|(i, _)| *i == id).map(|(_, c)| c.clone());
            if let Some(code) = code {
                t.bibliotheque().fixer_code(id, &code)?;
            }
            let mut e = t.etat();
            e.composants.code = None;
            e.message = "Composant enregistré.".to_string();
            garder.clone()
        })(),
        "comp-annuler" => {
            let mut e = t.etat();
            e.composants.code = None;
            e.composants.nom = None;
            garder.clone()
        }
        "comp-supprimer" => {
            t.etat().composants.supprimer = true;
            garder.clone()
        }
        "comp-supprimer-non" => {
            t.etat().composants.supprimer = false;
            garder.clone()
        }
        "comp-supprimer-oui" => (|| {
            let id = choisi.ok_or("Aucun élément ouvert.")?;
            let (parent, nb) = {
                let mut b = t.bibliotheque();
                let parent = b.noeud(id).and_then(|n| n.parent);
                (parent, b.supprimer(id)?)
            };
            choisir_noeud(t, parent, false);
            t.etat().message = format!("{nb} élément(s) supprimé(s).");
            garder.clone()
        })(),
        "comp-deplacer" => (|| {
            let id = choisi.ok_or("Aucun élément ouvert.")?;
            let vers = champs.valeur("comp-destination").as_deref().and_then(noeud_de_option).ok_or("Choisissez où le ranger.")?;
            t.bibliotheque().deplacer(id, vers)?;
            let chemin = t.bibliotheque().chemin_texte(id);
            choisir_noeud(t, Some(id), false);
            t.etat().message = format!("Rangé dans {chemin}.");
            garder.clone()
        })(),
        "inserer-composant" => (|| {
            let comp = champs.valeur("composant-choix").as_deref().and_then(noeud_de_option).ok_or("Choisissez un composant.")?;
            let (nom, code) = t.bibliotheque().noeud(comp).map(|n| (n.nom.clone(), n.code.clone())).ok_or("Ce composant n'existe plus.")?;
            let mut e = t.etat();
            let modele = e.langage.as_ref().map(|l| l.modele_test()).unwrap_or("").to_string();
            let avant = e.brouillon("nouveau-code").cloned().unwrap_or(modele.clone());
            // Le modele vide ne sert plus : le composant le remplace.
            let texte = if avant.trim().is_empty() || avant == modele { code.trim_end().to_string() } else { format!("{}\n{}", avant.trim_end(), code.trim_end()) };
            e.fixer_brouillon("nouveau-code", Some(texte));
            e.message = format!("Composant « {nom} » ajouté au test.");
            garder.clone()
        })(),
        // Listes : le premier clic les ouvre, rien a redessiner.
        "comp-destination" | "composant-choix" => Ok(rep(Redessin::Rien)),
        _ => {
            if let Some(n) = numero(id, "noeud-") {
                choisir_noeud(t, Some(n as Id), false);
                return Some(Ok(rep(Redessin::Garder)));
            }
            if let Some(n) = numero(id, "plier-") {
                let mut e = t.etat();
                let n = n as Id;
                if !e.composants.plies.remove(&n) {
                    e.composants.plies.insert(n);
                }
                return Some(Ok(rep(Redessin::Garder)));
            }
            return None;
        }
    };
    Some(r)
}

fn numero(id: &str, prefixe: &str) -> Option<usize> {
    id.strip_prefix(prefixe)?.parse().ok()
}

/// Lance `tests` (meme s'ils sont dans plusieurs cibles).
fn lancer(t: &Testeur, tests: &[Test], titre: &str) -> Result<(), String> {
    let p = t.projet().ok_or("Aucun projet ouvert.")?;
    let l = t.etat().langage.clone().ok_or("Langage du projet inconnu.")?;
    if tests.is_empty() {
        return Err("Aucun test à lancer.".to_string());
    }
    let refs: Vec<&Test> = tests.iter().collect();
    let etapes = l.etape_tests(&refs);
    t.lanceur.lancer(p.chemin, l, titre, etapes, tests.iter().map(Test::cle).collect())
}

/// Tous les tests du projet, paquet par paquet.
fn tout_lancer(t: &Testeur) -> Result<(), String> {
    let p = t.projet().ok_or("Aucun projet ouvert.")?;
    let e = t.etat();
    let l = e.langage.clone().ok_or("Langage du projet inconnu.")?;
    let mut etapes = Vec::new();
    for paquet in &e.analyse.paquets {
        if e.analyse.tests.iter().any(|x| x.paquet == paquet.dossier) {
            etapes.push(crate::langage::Etape { titre: format!("tests de {}", paquet.nom), paquet: paquet.dossier.clone(), ..Default::default() });
        }
    }
    let attendus = e.analyse.tests.iter().map(Test::cle).collect();
    drop(e);
    if etapes.is_empty() {
        return Err("Aucun test dans ce projet.".to_string());
    }
    t.lanceur.lancer(p.chemin, l, &format!("Tous les tests de {}", p.nom), etapes, attendus)
}

/// Applique une modification du code, puis relit le projet.
fn modifier(t: &Testeur, op: Operation) -> Result<String, String> {
    let p = t.projet().ok_or("Aucun projet ouvert.")?;
    let l = t.etat().langage.clone().ok_or("Langage du projet inconnu.")?;
    let fichier = l.appliquer(&p.chemin, &op)?;
    t.analyser(&p);
    Ok(fichier)
}

/// Le test `i` de l'atelier tel qu'il est tape (brouillon) ou ecrit.
fn test_tape(t: &Testeur, i: usize) -> Option<(String, TestSource)> {
    let e = t.etat();
    let s = e.lu.as_ref()?.tests.get(i)?.clone();
    let nom = e.brouillon(&format!("nom@{}", s.nom)).cloned().unwrap_or_else(|| s.nom.clone());
    let code = e.brouillon(&format!("code@{}", s.nom)).cloned().unwrap_or_else(|| s.code.clone());
    let mut options = BTreeMap::new();
    for o in e.langage.as_ref().map(|l| l.options()).unwrap_or(&[]) {
        let defaut = if s.options.contains_key(o.code) { "true" } else { "false" };
        if e.brouillon(&format!("opt@{}@{}", s.nom, o.code)).map(String::as_str).unwrap_or(defaut) == "true" {
            options.insert(o.code.to_string(), "oui".to_string());
        }
    }
    Some((s.nom.clone(), TestSource { nom, code, options, ligne: s.ligne }))
}

fn oublier_test(t: &Testeur, nom: &str) {
    let mut e = t.etat();
    e.fixer_brouillon(&format!("nom@{nom}"), None);
    e.fixer_brouillon(&format!("code@{nom}"), None);
    e.oublier_brouillons(&format!("opt@{nom}@"));
}

fn test_du_fichier(t: &Testeur, nom: &str) -> Option<Test> {
    let e = t.etat();
    let f = e.fichier.clone()?;
    e.analyse.tests.iter().find(|x| x.fichier == f && x.nom == nom).cloned()
}

/// Ecrit le test `i` de l'atelier dans le code ; rend son nom.
fn enregistrer(t: &Testeur, i: usize) -> Result<String, String> {
    let (ancien, test) = test_tape(t, i).ok_or("Ce test n'existe plus.")?;
    let fichier = t.etat().fichier.clone().ok_or("Aucun fichier ouvert.")?;
    let nom = test.nom.clone();
    modifier(t, Operation::ModifierTest { fichier, ancien: ancien.clone(), test })?;
    oublier_test(t, &ancien);
    let mut e = t.etat();
    if e.ouvert == Some(Ouvert::Test(ancien)) {
        e.ouvert = Some(Ouvert::Test(nom.clone()));
    }
    Ok(nom)
}

/// Ouvre `fichier` dans l'atelier.
pub fn ouvrir_fichier(t: &Testeur, fichier: &str) {
    let p = t.projet();
    let mut e = t.etat();
    e.onglet = Onglet::Atelier;
    if e.fichier.as_deref() != Some(fichier) {
        e.ouvert = None;
    }
    e.fichier = Some(fichier.to_string());
    e.supprimer = None;
    e.lu = match (&p, &e.langage) {
        (Some(p), Some(l)) => match l.lire_fichier(&p.chemin, fichier) {
            Ok(f) => Some(f),
            Err(err) => {
                e.erreur = err;
                None
            }
        },
        _ => None,
    };
}

/// Le clic `id`.
pub fn cliquer(t: &Testeur, id: &str, champs: &dyn Lecture) -> Reponse {
    ranger(t, champs);
    // Les boutons du panneau de detail doublent ceux de la liste.
    let id = id.strip_prefix("d-").unwrap_or(id);
    // Une case cochee : rangee, rien a redessiner.
    if id.starts_with("opt-") || id.starts_with("nouveau-opt-") || id == "nf-paquet" || id == "comp-destination" || id == "composant-choix" {
        return rep(Redessin::Rien);
    }
    {
        let mut e = t.etat();
        e.message.clear();
        e.erreur.clear();
    }
    match action(t, id, champs) {
        Ok(r) => r,
        Err(err) => {
            t.etat().erreur = err;
            rep(Redessin::Garder)
        }
    }
}

fn action(t: &Testeur, id: &str, champs: &dyn Lecture) -> Result<Reponse, String> {
    if let Some(r) = action_composants(t, id, champs) {
        return r;
    }
    let garder = Ok(rep(Redessin::Garder));
    let haut = Ok(rep(Redessin::Haut));
    let tests = || t.etat().analyse.tests.clone();
    match id {
        "onglet-tests" => {
            t.etat().onglet = Onglet::Tests;
            return haut;
        }
        "onglet-atelier" => {
            let premier = {
                let e = t.etat();
                if e.fichier.is_some() {
                    None
                } else {
                    let choisi = e.choisi.as_ref().and_then(|c| e.analyse.tests.iter().find(|x| &x.cle() == c)).map(|x| x.fichier.clone());
                    choisi.or_else(|| fichiers_atelier(&e).first().map(|f| f.0.clone()))
                }
            };
            t.etat().onglet = Onglet::Atelier;
            if let Some(f) = premier {
                ouvrir_fichier(t, &f);
            }
            return haut;
        }
        "fermer-message" => return garder,
        "lier" | "chemin" => {
            let chemin = champs.valeur("chemin").unwrap_or_default();
            let i = t.projets().lier(&chemin)?;
            t.etat().saisie_chemin.clear();
            t.ouvrir(i);
            let nb = t.etat().analyse.tests.len();
            t.etat().message = format!("Projet relié : {nb} test(s) trouvé(s).");
            return haut;
        }
        "analyser" => {
            let p = t.projet().ok_or("Aucun projet ouvert.")?;
            t.analyser(&p);
            let e = &mut *t.etat();
            e.message = format!("{} test(s) trouvé(s) dans {} paquet(s).", e.analyse.tests.len(), e.analyse.paquets.len());
            return garder;
        }
        "tout-lancer" => {
            tout_lancer(t)?;
            return garder;
        }
        "relancer-echecs" => {
            let echecs: Vec<Test> = {
                let exec = t.lanceur.lire();
                tests().into_iter().filter(|x| matches!(exec.statut(&x.cle()), Statut::Echoue | Statut::Erreur)).collect()
            };
            lancer(t, &echecs, "Relance des échecs")?;
            return garder;
        }
        "lancer-visibles" => {
            let v: Vec<Test> = {
                let exec = t.lanceur.lire();
                let e = t.etat();
                visibles(&e, &|c| exec.statut(c)).into_iter().map(|(_, x)| x.clone()).collect()
            };
            lancer(t, &v, "Tests affichés")?;
            return garder;
        }
        "arreter" => {
            t.lanceur.arreter();
            return garder;
        }
        "filtre" | "filtrer" => {
            let mut e = t.etat();
            e.filtre = e.saisie_filtre.clone();
            return haut;
        }
        "effacer-filtre" => {
            let mut e = t.etat();
            e.filtre.clear();
            e.saisie_filtre.clear();
            return haut;
        }
        "ouvrir-tout" | "fermer-tout" => {
            let ouvrir = id == "ouvrir-tout";
            let exec = t.lanceur.lire();
            let mut e = t.etat();
            let v = visibles(&e, &|c| exec.statut(c));
            let mut fichiers: Vec<String> = v.iter().map(|(_, x)| x.fichier.clone()).collect();
            fichiers.dedup();
            let nb = v.len();
            for f in fichiers {
                if deplie(&e, &f, nb) != ouvrir {
                    if !e.bascules.remove(&f) {
                        e.bascules.insert(f);
                    }
                }
            }
            return garder;
        }
        "console" => {
            let mut e = t.etat();
            e.console = !e.console;
            return garder;
        }
        "fermer-detail" => {
            t.etat().choisi = None;
            return garder;
        }
        "nouveau-fichier" => {
            let mut e = t.etat();
            e.nouveau_fichier = !e.nouveau_fichier;
            return garder;
        }
        "nf-creer" | "nf-nom" => {
            let nom = champs.valeur("nf-nom").unwrap_or_default().trim().to_string();
            let (paquet, generique) = {
                let e = t.etat();
                let l = e.langage.clone().ok_or("Langage du projet inconnu.")?;
                let choix = champs.valeur("nf-paquet").unwrap_or_default();
                let p = e.analyse.paquets.iter().find(|p| choix == p.nom || choix.starts_with(&format!("{} (", p.nom))).or(e.analyse.paquets.first()).cloned().ok_or("Aucun paquet dans ce projet.")?;
                (p.dossier.clone(), l.modele_generique(&p))
            };
            let fichier = modifier(t, Operation::CreerFichier { paquet, nom, generique })?;
            {
                let mut e = t.etat();
                e.nouveau_fichier = false;
                e.nf_nom.clear();
                e.message = format!("{fichier} créé : écrivez son code générique puis ses tests.");
            }
            ouvrir_fichier(t, &fichier);
            return haut;
        }
        "generique-enregistrer" => {
            let (fichier, code) = {
                let e = t.etat();
                let f = e.fichier.clone().ok_or("Aucun fichier ouvert.")?;
                let code = e.brouillon("generique").cloned().or_else(|| e.lu.as_ref().map(|l| l.generique.clone())).unwrap_or_default();
                (f, code)
            };
            modifier(t, Operation::Generique { fichier: fichier.clone(), code })?;
            let mut e = t.etat();
            e.fixer_brouillon("generique", None);
            e.message = format!("Code générique écrit dans {fichier}.");
            return garder;
        }
        "generique-annuler" => {
            t.etat().fixer_brouillon("generique", None);
            return garder;
        }
        "creer-test" | "nouveau-nom" => {
            let (fichier, test) = {
                let e = t.etat();
                let f = e.fichier.clone().ok_or("Ouvrez ou créez d'abord un fichier de tests.")?;
                let modele = e.langage.as_ref().map(|l| l.modele_test()).unwrap_or("");
                let nom = e.brouillon("nouveau-nom").cloned().unwrap_or_default();
                if nom.is_empty() {
                    return Err("Donnez un nom au test (ex. calcule_la_somme).".to_string());
                }
                let code = e.brouillon("nouveau-code").cloned().unwrap_or_else(|| modele.to_string());
                let mut options = BTreeMap::new();
                for o in e.langage.as_ref().map(|l| l.options()).unwrap_or(&[]) {
                    if e.brouillon(&format!("nouveau-opt@{}", o.code)).map(String::as_str) == Some("true") {
                        options.insert(o.code.to_string(), "oui".to_string());
                    }
                }
                (f, TestSource { nom, code, options, ligne: 0 })
            };
            let nom = test.nom.clone();
            modifier(t, Operation::CreerTest { fichier: fichier.clone(), test })?;
            let mut e = t.etat();
            e.oublier_brouillons("nouveau-");
            e.ouvert = Some(Ouvert::Test(nom.clone()));
            let rang = e.lu.as_ref().and_then(|l| l.tests.iter().position(|s| s.nom == nom));
            let ligne = rang.and_then(|r| e.lu.as_ref().map(|l| l.tests[r].ligne)).unwrap_or(0);
            e.message = format!("Test « {nom} » créé dans {fichier} (ligne {ligne}).");
            return Ok(Reponse { redessin: Redessin::Garder, defiler: rang.map(|r| format!("at-{r}")) });
        }
        "lancer-fichier" => {
            let f = t.etat().fichier.clone().ok_or("Aucun fichier ouvert.")?;
            let liste: Vec<Test> = tests().into_iter().filter(|x| x.fichier == f).collect();
            lancer(t, &liste, &f)?;
            return garder;
        }
        "ouvrir-generique" | "ouvrir-nouveau" | "fermer-editeur" => {
            let mut e = t.etat();
            let voulu = match id {
                "ouvrir-generique" => Some(Ouvert::Generique),
                "ouvrir-nouveau" => Some(Ouvert::Nouveau),
                _ => None,
            };
            e.ouvert = if e.ouvert == voulu { None } else { voulu };
            return garder;
        }
        "supprimer-non" => {
            t.etat().supprimer = None;
            return garder;
        }
        _ => {}
    }
    if let Some(i) = numero(id, "projet-") {
        t.ouvrir(i);
        return haut;
    }
    if let Some(i) = numero(id, "delier-") {
        let actif = t.etat().projet;
        t.projets().delier(i)?;
        match actif {
            Some(a) if a == i => t.ouvrir(0),
            Some(a) if a > i => t.etat().projet = Some(a - 1),
            _ => {}
        }
        return haut;
    }
    if let Some(code) = id.strip_prefix("vue-") {
        let v = Vue::TOUTES.iter().find(|v| v.1 == code).map(|v| v.0).unwrap_or_default();
        t.etat().vue = v;
        return haut;
    }
    if let Some(i) = numero(id, "groupe-") {
        let f = tests().get(i).map(|x| x.fichier.clone()).ok_or("Fichier introuvable.")?;
        let mut e = t.etat();
        if !e.bascules.remove(&f) {
            e.bascules.insert(f);
        }
        return garder;
    }
    if let Some(i) = numero(id, "lancer-groupe-") {
        let tous = tests();
        let f = tous.get(i).map(|x| x.fichier.clone()).ok_or("Fichier introuvable.")?;
        let liste: Vec<Test> = tous.into_iter().filter(|x| x.fichier == f).collect();
        lancer(t, &liste, &f)?;
        return garder;
    }
    if let Some(i) = numero(id, "voir-") {
        let cle = tests().get(i).map(Test::cle);
        let mut e = t.etat();
        e.choisi = if e.choisi == cle { None } else { cle };
        return garder;
    }
    if let Some(i) = numero(id, "lancer-") {
        let x = tests().get(i).cloned().ok_or("Test introuvable.")?;
        t.etat().choisi = Some(x.cle());
        lancer(t, std::slice::from_ref(&x), &x.chemin)?;
        return garder;
    }
    if let Some(i) = numero(id, "ecrire-") {
        let x = tests().get(i).cloned().ok_or("Test introuvable.")?;
        ouvrir_fichier(t, &x.fichier);
        t.etat().ouvert = Some(Ouvert::Test(x.nom.clone()));
        let rang = t.etat().lu.as_ref().and_then(|l| l.tests.iter().position(|s| s.nom == x.nom));
        return Ok(Reponse { redessin: Redessin::Haut, defiler: rang.map(|r| format!("at-{r}")) });
    }
    if let Some(i) = numero(id, "ouvrir-") {
        let mut e = t.etat();
        let voulu = e.lu.as_ref().and_then(|l| l.tests.get(i)).map(|s| Ouvert::Test(s.nom.clone()));
        e.ouvert = if e.ouvert == voulu { None } else { voulu };
        e.supprimer = None;
        return Ok(Reponse { redessin: Redessin::Garder, defiler: Some(format!("at-{i}")) });
    }
    if let Some(k) = numero(id, "fichier-") {
        let f = fichiers_atelier(&t.etat()).get(k).map(|f| f.0.clone()).ok_or("Fichier introuvable.")?;
        ouvrir_fichier(t, &f);
        return haut;
    }
    if let Some(i) = numero(id, "enregistrer-").or_else(|| numero(id, "nom-")) {
        let nom = enregistrer(t, i)?;
        t.etat().message = format!("Test « {nom} » écrit dans le code.");
        return garder;
    }
    if let Some(i) = numero(id, "annuler-") {
        let nom = t.etat().lu.as_ref().and_then(|l| l.tests.get(i)).map(|s| s.nom.clone());
        if let Some(nom) = nom {
            oublier_test(t, &nom);
        }
        return garder;
    }
    if let Some(i) = numero(id, "supprimer-oui-") {
        let (fichier, nom) = {
            let e = t.etat();
            (e.fichier.clone().ok_or("Aucun fichier ouvert.")?, e.lu.as_ref().and_then(|l| l.tests.get(i)).map(|s| s.nom.clone()).ok_or("Ce test n'existe plus.")?)
        };
        modifier(t, Operation::SupprimerTest { fichier: fichier.clone(), nom: nom.clone() })?;
        oublier_test(t, &nom);
        let mut e = t.etat();
        e.supprimer = None;
        e.ouvert = None;
        e.message = format!("Test « {nom} » retiré de {fichier}.");
        return garder;
    }
    if let Some(i) = numero(id, "supprimer-") {
        let mut e = t.etat();
        e.supprimer = e.lu.as_ref().and_then(|l| l.tests.get(i)).map(|s| s.nom.clone());
        return garder;
    }
    if let Some(i) = numero(id, "lancer-at-") {
        // Ce qui est tape est d'abord ecrit : on lance le test tel qu'on le voit.
        let modifie = {
            let e = t.etat();
            let nom = e.lu.as_ref().and_then(|l| l.tests.get(i)).map(|s| s.nom.clone()).unwrap_or_default();
            e.fichier.as_ref().and_then(|f| e.brouillons.get(f)).is_some_and(|b| b.keys().any(|k| k.split('@').nth(1) == Some(nom.as_str())))
        };
        let nom = if modifie { enregistrer(t, i)? } else { t.etat().lu.as_ref().and_then(|l| l.tests.get(i)).map(|s| s.nom.clone()).ok_or("Ce test n'existe plus.")? };
        let x = test_du_fichier(t, &nom).ok_or("Test introuvable après analyse.")?;
        lancer(t, std::slice::from_ref(&x), &x.chemin)?;
        return garder;
    }
    Ok(rep(Redessin::Rien))
}
