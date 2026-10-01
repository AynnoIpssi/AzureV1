// Ce que font les clics et les glisser-deposer de l'ecran v2.
//
// Avant tout clic, la page ouverte est enregistree : tous ses champs sont
// relus (`enregistrer`). Les ids :
//
//   titre                   titre de la page
//   v-<page>-<cle>          valeur d'une propriete (page ou ligne de base)
//   b-<bloc>                texte (riche) d'un bloc ; t-<bloc> : tache faite
//   lang-<bloc>             langage d'un bloc de code
//   c-<bloc>-<l>-<c>        cellule d'un tableau
//
// Clavier (zones `commandes` / `entree`) : slash-b-<bloc> (`/` tape : menu
// des blocs), entree-b-<bloc>@<curseur> (couper le bloc), entree-titre@<n>.
// Menu `/` : menu-<genre> (voir `page::MENU`), menu-ouvrir, menu-fermer.
//
// Boutons : p-<id> (ouvrir), nouvelle, nouvelle-base, sous-<id>,
// supprimer(-oui/-non), ajout-<genre> (bloc), genre-<bloc>-<genre>,
// bloc-suppr-<bloc>, bloc-ligne-<bloc>, bloc-col-<bloc>, vue-<n>,
// vue-ajout-<affichage>, vue-suppr, groupe,
// tri, tri-sens, filtres, filtre-ajouter (filtre-prop, filtre-op,
// filtre-val), filtre-suppr-<i>, ligne-ajouter, ligne-col-<i>, doc-chercher.
// Les proprietes (carte, valeurs) : voir `proprietes`.
//
// Glisser-deposer (source -> zone) : pa-<page> -> arbre | dans-<page> ;
// k-<ligne>-<colonne> -> col-<i> ; bl-<bloc> -> blocs ; li-<ligne> -> lignes.
use crate::modele::{Affichage, Bloc, Definition, Espace, Filtre, Genre, GenreBloc, Operateur, Vue};
use crate::classeur::Classeur;
use crate::formule::normaliser;
use std::sync::Mutex;
use crate::page::{cellules, champ, ecrire_cellules, tableau_neuf, Etat, VIDE};
use crate::riche::{brut, couper, sans_slash};

/// Les champs de l'ecran au moment du clic (`WindowContext` dans l'app).
pub trait Lecture {
    /// La valeur du champ `id`, `None` s'il n'est pas a l'ecran.
    fn valeur(&self, id: &str) -> Option<String>;
}

/// Ce que la fenetre doit faire apres un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum Suite {
    /// Redessiner l'ecran.
    Redessiner,
    /// Rien a redessiner (clic sans effet).
    Rien,
    /// Ouvrir la fenetre Doc avec la recherche du champ `doc-q`.
    ChercherDoc,
}

/// Une propriete choisie dans une liste (par son nom) : sa cle ; « — » : vide.
fn cle_choisie(l: &dyn Lecture, id: &str) -> String {
    let v = l.valeur(id).unwrap_or_default();
    if v == VIDE { String::new() } else { normaliser(&v) }
}

/// Range ce qui est a l'ecran puis fait `action` (un clic, un depot) ;
/// les erreurs vont dans le bandeau de l'ecran (`etat.erreur`). Ce qui a
/// ete tape est garde meme si l'action echoue. Rend ce que la fenetre doit
/// faire ensuite.
pub fn agir(classeur: &Classeur, etat: &Mutex<Etat>, lecture: &dyn Lecture, action: impl FnOnce(&mut Espace, &mut Etat) -> Result<Suite, String>) -> Suite {
    let mut etat = etat.lock().unwrap_or_else(|e| e.into_inner());
    let resultat = classeur.modifier(|e| {
        let mut erreurs = etat.page.map(|p| enregistrer(e, p, lecture)).unwrap_or_default();
        let suite = action(e, &mut etat).unwrap_or_else(|err| {
            erreurs.push(err);
            Suite::Redessiner
        });
        Ok((erreurs, suite))
    });
    match resultat {
        Ok((erreurs, suite)) => {
            etat.erreur = (!erreurs.is_empty()).then(|| erreurs.join(" · "));
            if etat.erreur.is_some() && suite == Suite::Rien { Suite::Redessiner } else { suite }
        }
        Err(e) => {
            etat.erreur = Some(format!("enregistrement impossible : {e}"));
            Suite::Redessiner
        }
    }
}

fn coche(l: &dyn Lecture, id: &str) -> Option<bool> {
    l.valeur(id).map(|v| v == "true")
}

/// Relit les champs de la page ouverte (et des lignes, pour une base) et
/// les range dans l'espace. Rend les erreurs (valeur refusee...).
pub fn enregistrer(e: &mut Espace, page: i64, l: &dyn Lecture) -> Vec<String> {
    let mut erreurs = Vec::new();
    let Some(p) = e.page(page).cloned() else { return erreurs };
    // Le titre est un texte riche : on n'en garde que le texte, sur une ligne.
    if let Some(titre) = l.valeur("titre") {
        if let Ok(p) = e.page_mut(page) {
            p.titre = brut(&titre).replace('\n', " ");
        }
    }
    let mut cibles = vec![page];
    if p.base {
        cibles.extend(e.enfants(Some(page)).iter().map(|x| x.id));
    }
    for id in cibles {
        for d in e.definitions(id) {
            if let Err(err) = ranger_propriete(e, id, &d, l) {
                erreurs.push(err);
            }
        }
    }
    let mut blocs = p.blocs.clone();
    for bl in &mut blocs {
        let fid = format!("b-{}", bl.id);
        match &bl.genre {
            GenreBloc::Code(_) => {
                if let Some(v) = l.valeur(&fid) {
                    bl.contenu = brut(&v);
                }
                if let Some(langage) = l.valeur(&format!("lang-{}", bl.id)) {
                    bl.genre = GenreBloc::Code(langage.trim().to_string());
                }
            }
            GenreBloc::Tableau => {
                let mut c = cellules(&bl.contenu);
                for (r, ligne) in c.iter_mut().enumerate() {
                    for (k, cellule) in ligne.iter_mut().enumerate() {
                        if let Some(v) = l.valeur(&format!("c-{}-{r}-{k}", bl.id)) {
                            *cellule = v.replace([crate::modele::SEP, crate::modele::SEP_LIGNE], " ");
                        }
                    }
                }
                bl.contenu = ecrire_cellules(&c);
            }
            GenreBloc::SousPage | GenreBloc::Vue | GenreBloc::Separateur => {}
            _ => {
                if let Some(v) = l.valeur(&fid) {
                    bl.contenu = v;
                }
                if let (GenreBloc::Tache(_), Some(fait)) = (&bl.genre, coche(l, &format!("t-{}", bl.id))) {
                    bl.genre = GenreBloc::Tache(fait);
                }
            }
        }
    }
    if let Ok(p) = e.page_mut(page) {
        p.blocs = blocs;
    }
    erreurs
}

fn ranger_propriete(e: &mut Espace, id: i64, d: &Definition, l: &dyn Lecture) -> Result<(), String> {
    let fid = champ(id, &d.cle());
    let brute = match &d.genre {
        Genre::Formule(_) => return Ok(()),
        Genre::Case => match coche(l, &fid) {
            Some(true) => "1".to_string(),
            Some(false) => String::new(),
            None => return Ok(()),
        },
        // Choisies a la souris (voir `proprietes`), pas tapees.
        Genre::Relation | Genre::Selection(_) | Genre::Etiquettes(_) | Genre::Date => return Ok(()),
        _ => match l.valeur(&fid) {
            Some(v) if v == VIDE => String::new(),
            Some(v) => v,
            None => return Ok(()),
        },
    };
    e.changer(id, &d.cle(), &brute)
}

fn nombre(s: &str) -> Option<i64> {
    s.parse().ok()
}

fn nouveau_bloc(e: &Espace, page: i64) -> i64 {
    e.page(page).and_then(|p| p.blocs.iter().map(|b| b.id).max()).unwrap_or(0) + 1
}

fn ajouter_bloc(e: &mut Espace, page: i64, genre: GenreBloc, contenu: String) -> Result<(), String> {
    let id = nouveau_bloc(e, page);
    e.page_mut(page)?.blocs.push(Bloc { id, genre, contenu });
    Ok(())
}

/// La base dont on regarde une vue : la page ouverte si c'en est une.
fn base_ouverte(e: &Espace, etat: &Etat) -> Option<i64> {
    etat.page.filter(|id| e.page(*id).is_some_and(|p| p.base))
}

/// La vue affichee de la base ouverte.
fn vue_ouverte(e: &Espace, etat: &Etat) -> Option<Vue> {
    let base = base_ouverte(e, etat)?;
    e.page(base)?.vues.get(etat.vue(e, base)).cloned()
}

fn avec_vue(e: &mut Espace, etat: &Etat, f: impl FnOnce(&mut Vue)) -> Result<(), String> {
    let base = base_ouverte(e, etat).ok_or("aucune base ouverte")?;
    let n = etat.vue(e, base);
    let b = e.page_mut(base)?;
    if b.vues.is_empty() {
        b.vues.push(Vue::nouvelle(Affichage::Table));
    }
    f(&mut b.vues[n]);
    Ok(())
}

/// Un clic sur `id`. `modifie` : `now` pour dater les nouvelles pages.
pub fn cliquer(e: &mut Espace, etat: &mut Etat, id: &str, l: &dyn Lecture, now: u64) -> Result<Suite, String> {
    // Un clic ailleurs ferme le menu `/` ; le focus ne vaut qu'une fois.
    let menu = etat.menu.take();
    let focus = etat.focus.take();
    etat.focus_pos = None;
    // Une confirmation de suppression dans l'arbre ne vaut que pour le clic suivant.
    let suppr = etat.suppr_arbre.take();
    // La carte d'une propriete applique ce qui y est tape, et se ferme si
    // on clique ailleurs (de meme pour le choix d'une valeur).
    let avant = match etat.page {
        Some(page) => crate::proprietes::avant_clic(e, etat, page, id, l),
        None => Ok(()),
    };
    if let Some(suite) = clavier_et_menu(e, etat, id, menu, now)? {
        return avant.map(|_| suite);
    }
    if let Some(page) = etat.page
        && let Some(suite) = crate::proprietes::cliquer(e, etat, page, id, l)?
    {
        return avant.map(|_| suite);
    }
    avant?;
    // Rien a faire mais un menu ouvert ou un focus demande : redessiner pour
    // les enlever.
    let suite = cliquer_bouton(e, etat, id, l, now)?;
    Ok(if suite == Suite::Rien && (menu.is_some() || focus.is_some() || suppr.is_some()) { Suite::Redessiner } else { suite })
}

/// Un bloc de genre `code` (voir `page::MENU`) : (genre, contenu).
fn bloc_neuf(code: &str) -> Option<(GenreBloc, String)> {
    Some(match code {
        "code" => (GenreBloc::Code(String::new()), String::new()),
        "tableau" => (GenreBloc::Tableau, tableau_neuf()),
        "separateur" => (GenreBloc::Separateur, String::new()),
        c => (genre_texte(c)?, String::new()),
    })
}

/// Insere un bloc juste apres `apres` (a la fin si `None`) ; rend son id.
fn inserer_apres(e: &mut Espace, page: i64, apres: Option<i64>, genre: GenreBloc, contenu: String) -> Result<i64, String> {
    let id = nouveau_bloc(e, page);
    let blocs = &mut e.page_mut(page)?.blocs;
    let i = apres.and_then(|a| blocs.iter().position(|b| b.id == a)).map(|i| i + 1).unwrap_or(blocs.len());
    blocs.insert(i, Bloc { id, genre, contenu });
    Ok(id)
}

/// Ce qui vient du clavier (`slash-`, `entree-`) et le menu `/` (`menu-`).
fn clavier_et_menu(e: &mut Espace, etat: &mut Etat, id: &str, menu: Option<Option<i64>>, now: u64) -> Result<Option<Suite>, String> {
    let Some(page) = etat.page else { return Ok(None) };
    // `/` tape dans un bloc : le menu s'ouvre dessous.
    if let Some(bloc) = id.strip_prefix("slash-b-").and_then(nombre) {
        etat.menu = Some(Some(bloc));
        return Ok(Some(Suite::Redessiner));
    }
    if id == "menu-ouvrir" {
        etat.menu = Some(None);
        return Ok(Some(Suite::Redessiner));
    }
    if id == "menu-fermer" {
        return Ok(Some(Suite::Redessiner));
    }
    if id == "options" {
        etat.options = !etat.options;
        etat.confirmer = false;
        etat.ajout_propriete = false;
        return Ok(Some(Suite::Redessiner));
    }
    // La couleur de la page : choisie dans les options, ou la suivante d'un
    // clic sur la pastille.
    if let Some(x) = id.strip_prefix("teinte-") {
        let x = crate::page::TEINTES.iter().find(|t| **t == x).ok_or("couleur inconnue")?;
        e.page_mut(page)?.icone = x.to_string();
        return Ok(Some(Suite::Redessiner));
    }
    if let Some(x) = id.strip_prefix("largeur-") {
        let x = crate::page::LARGEURS.iter().find(|l| l.0 == x).ok_or("largeur inconnue")?;
        e.page_mut(page)?.largeur = x.0.to_string();
        return Ok(Some(Suite::Redessiner));
    }
    if id == "pastille" {
        let p = e.page(page).ok_or("page introuvable")?;
        let i = crate::page::TEINTES.iter().position(|t| *t == crate::page::teinte(p)).unwrap_or(0);
        e.page_mut(page)?.icone = crate::page::TEINTES[(i + 1) % crate::page::TEINTES.len()].to_string();
        return Ok(Some(Suite::Redessiner));
    }
    if id.starts_with("retour-titre") {
        return Ok(Some(Suite::Rien));
    }
    // Clic sous les blocs : on ecrit a la fin (dans le dernier bloc s'il est
    // un texte vide, sinon dans un nouveau).
    if id == "zone-ecrire" {
        let dernier = e.page(page).and_then(|p| p.blocs.last().cloned());
        etat.focus = Some(match dernier {
            Some(b) if b.genre == GenreBloc::Texte && brut(&b.contenu).is_empty() => b.id,
            _ => inserer_apres(e, page, None, GenreBloc::Texte, String::new())?,
        });
        return Ok(Some(Suite::Redessiner));
    }
    // Retour arriere au debut d'un bloc : vide, il part (on remonte au
    // precedent) ; sinon il se colle au precedent ; tout en haut, un titre ou
    // une liste redevient du texte.
    if let Some(bloc) = id.strip_prefix("retour-b-").and_then(nombre) {
        let blocs = e.page(page).map(|p| p.blocs.clone()).unwrap_or_default();
        let Some(i) = blocs.iter().position(|b| b.id == bloc) else { return Ok(Some(Suite::Rien)) };
        let b = &blocs[i];
        let precedent = i.checked_sub(1).map(|j| &blocs[j]).filter(|p| genre_texte(crate::page::code_bloc(&p.genre)).is_some());
        let vide = brut(&b.contenu).is_empty();
        match precedent {
            Some(p) => {
                let fin = brut(&p.contenu).chars().count();
                let mut segs = crate::riche::lire(&p.contenu);
                segs.extend(crate::riche::lire(&b.contenu));
                let colle = crate::riche::ecrire(&segs);
                let (pid, bid) = (p.id, b.id);
                let page_mut = e.page_mut(page)?;
                if let Some(x) = page_mut.blocs.iter_mut().find(|x| x.id == pid) {
                    x.contenu = colle;
                }
                page_mut.blocs.retain(|x| x.id != bid);
                etat.focus = Some(pid);
                etat.focus_pos = Some(fin);
            }
            // Juste sous un separateur : le trait part.
            None if i > 0 && blocs[i - 1].genre == GenreBloc::Separateur => {
                let sep = blocs[i - 1].id;
                e.page_mut(page)?.blocs.retain(|x| x.id != sep);
                etat.focus = Some(bloc);
                etat.focus_pos = Some(0);
            }
            None if b.genre != GenreBloc::Texte => {
                if let Some(x) = e.page_mut(page)?.blocs.iter_mut().find(|x| x.id == bloc) {
                    x.genre = GenreBloc::Texte;
                }
                etat.focus = Some(bloc);
                etat.focus_pos = Some(0);
            }
            // Un bloc vide sous un bloc special (code, tableau...) : il part.
            None if vide && i > 0 => {
                e.page_mut(page)?.blocs.retain(|x| x.id != bloc);
            }
            None => return Ok(Some(Suite::Rien)),
        }
        return Ok(Some(Suite::Redessiner));
    }
    // Entree dans le titre : on descend au 1er bloc.
    if id.starts_with("entree-titre@") {
        let premier = match e.page(page).and_then(|p| p.blocs.first().map(|b| b.id)) {
            Some(b) => b,
            None => inserer_apres(e, page, None, GenreBloc::Texte, String::new())?,
        };
        etat.focus = Some(premier);
        return Ok(Some(Suite::Redessiner));
    }
    // Entree dans un bloc : il est coupe au curseur, la suite part dans un
    // nouveau bloc (une liste continue la liste ; une puce vide redevient
    // du texte).
    if let Some(reste) = id.strip_prefix("entree-b-") {
        let (bloc, curseur) = reste.split_once('@').ok_or("entree ?")?;
        let (bloc, curseur) = (nombre(bloc).ok_or("bloc ?")?, curseur.parse::<usize>().map_err(|_| "curseur ?")?);
        let Some(b) = e.page(page).and_then(|p| p.blocs.iter().find(|b| b.id == bloc)).cloned() else { return Ok(Some(Suite::Rien)) };
        let liste = matches!(b.genre, GenreBloc::Puce | GenreBloc::Numero | GenreBloc::Tache(_));
        if liste && brut(&b.contenu).trim().is_empty() {
            if let Some(x) = e.page_mut(page)?.blocs.iter_mut().find(|x| x.id == bloc) {
                x.genre = GenreBloc::Texte;
            }
            etat.focus = Some(bloc);
            return Ok(Some(Suite::Redessiner));
        }
        let (avant, apres) = couper(&b.contenu, curseur);
        if let Some(x) = e.page_mut(page)?.blocs.iter_mut().find(|x| x.id == bloc) {
            x.contenu = avant;
        }
        let genre = match b.genre {
            GenreBloc::Tache(_) => GenreBloc::Tache(false),
            g if liste => g,
            _ => GenreBloc::Texte,
        };
        etat.focus = Some(inserer_apres(e, page, Some(bloc), genre, apres)?);
        return Ok(Some(Suite::Redessiner));
    }
    // Une commande choisie au clavier (`/tit` + Entree) : `commande-b-<id>@<code>@<pos>`,
    // le `/tit` est deja parti du texte.
    if let Some(rest) = id.strip_prefix("commande-b-") {
        let mut parts = rest.split('@');
        let bloc = parts.next().and_then(nombre).ok_or("bloc ?")?;
        let code = parts.next().ok_or("commande ?")?;
        return inserer_commande(e, etat, page, Some(bloc), code, false, now).map(Some);
    }
    // Un choix du menu `/` a la souris.
    let Some(code) = id.strip_prefix("menu-") else { return Ok(None) };
    let Some(sous) = menu else { return Ok(Some(Suite::Redessiner)) };
    inserer_commande(e, etat, page, sous, code, true, now).map(Some)
}

/// Le bloc `code` du menu `/`, depuis le bloc `sous` : un bloc vide devient
/// le nouveau bloc, sinon le nouveau se place dessous. `slash` : retirer le
/// `/` tape (menu a la souris).
fn inserer_commande(e: &mut Espace, etat: &mut Etat, page: i64, sous: Option<i64>, code: &str, slash: bool, now: u64) -> Result<Suite, String> {
    let nettoyer = |s: &str| if slash { sans_slash(s) } else { s.to_string() };
    let mut ouvrir = None;
    let (genre, contenu) = match code {
        "souspage" => {
            let p = e.creer(Some(page), "", now)?;
            ouvrir = Some(p);
            (GenreBloc::SousPage, p.to_string())
        }
        "base" => (GenreBloc::Vue, format!("{}/0", e.creer_base(Some(page), "Nouvelle base", now)?)),
        c => bloc_neuf(c).ok_or_else(|| format!("bloc inconnu : {c}"))?,
    };
    // Le `/` tape s'en va ; un bloc vide devient le nouveau bloc, sinon le
    // nouveau se place dessous.
    let vise = sous.and_then(|b| e.page(page).and_then(|p| p.blocs.iter().find(|x| x.id == b)).cloned());
    let bloc = match vise {
        Some(b) if brut(&nettoyer(&b.contenu)).trim().is_empty() => {
            let x = e.page_mut(page)?.blocs.iter_mut().find(|x| x.id == b.id).ok_or("bloc ?")?;
            x.genre = genre.clone();
            x.contenu = contenu;
            b.id
        }
        Some(b) => {
            if let Some(x) = e.page_mut(page)?.blocs.iter_mut().find(|x| x.id == b.id) {
                x.contenu = nettoyer(&x.contenu);
            }
            inserer_apres(e, page, Some(b.id), genre.clone(), contenu)?
        }
        None => inserer_apres(e, page, None, genre.clone(), contenu)?,
    };
    // Apres un separateur, on continue d'ecrire dans un texte neuf.
    etat.focus = Some(if genre == GenreBloc::Separateur { inserer_apres(e, page, Some(bloc), GenreBloc::Texte, String::new())? } else { bloc });
    if let Some(p) = ouvrir {
        etat.page = Some(p);
        etat.focus = Some(0);
    }
    Ok(Suite::Redessiner)
}

fn cliquer_bouton(e: &mut Espace, etat: &mut Etat, id: &str, l: &dyn Lecture, now: u64) -> Result<Suite, String> {
    let page = etat.page;
    // Ce qui n'est pas une action de la page elle-meme.
    match id {
        "doc-chercher" | "doc-q" => return Ok(Suite::ChercherDoc),
        "nouvelle" => {
            let p = e.creer(None, "", now)?;
            ajouter_bloc(e, p, GenreBloc::Texte, String::new())?;
            etat.page = Some(p);
            etat.focus = Some(0);
            return Ok(Suite::Redessiner);
        }
        "nouvelle-base" => {
            etat.page = Some(e.creer_base(None, "Nouvelle base", now)?);
            return Ok(Suite::Redessiner);
        }
        "arbre-non" => {
            etat.suppr_arbre = None;
            return Ok(Suite::Redessiner);
        }
        _ => {}
    }
    // L'arbre : replier, supprimer (apres confirmation).
    if let Some(p) = id.strip_prefix("pli-").and_then(nombre) {
        if !etat.plies.remove(&p) {
            etat.plies.insert(p);
        }
        return Ok(Suite::Redessiner);
    }
    if let Some(p) = id.strip_prefix("arbre-suppr-").and_then(nombre) {
        etat.suppr_arbre = Some(p);
        return Ok(Suite::Redessiner);
    }
    if let Some(p) = id.strip_prefix("arbre-oui-").and_then(nombre) {
        let parent = e.page(p).and_then(|x| x.parent);
        let partis = e.supprimer(p);
        etat.suppr_arbre = None;
        if page.is_some_and(|o| partis.contains(&o)) {
            etat.page = parent.or_else(|| e.enfants(None).first().map(|x| x.id));
            etat.options = false;
            etat.confirmer = false;
        }
        return Ok(Suite::Redessiner);
    }
    if let Some(p) = id.strip_prefix("p-").and_then(nombre) {
        etat.page = Some(p);
        etat.confirmer = false;
        etat.ajout_propriete = false;
        return Ok(Suite::Redessiner);
    }
    if let Some(p) = id.strip_prefix("sous-").and_then(nombre) {
        let sous = e.creer(Some(p), "", now)?;
        ajouter_bloc(e, sous, GenreBloc::Texte, String::new())?;
        etat.page = Some(sous);
        etat.focus = Some(0);
        return Ok(Suite::Redessiner);
    }
    let Some(page) = page else { return Ok(Suite::Rien) };
    match id {
        "supprimer" => etat.confirmer = true,
        "supprimer-non" => etat.confirmer = false,
        "supprimer-oui" => {
            etat.options = false;
            let parent = e.page(page).and_then(|p| p.parent);
            e.supprimer(page);
            etat.confirmer = false;
            etat.page = parent.or_else(|| e.enfants(None).first().map(|p| p.id));
        }
        "ligne-ajouter" => {
            let base = base_ouverte(e, etat).ok_or("aucune base ouverte")?;
            e.creer(Some(base), "", now)?;
        }
        "vue-suppr" => {
            let base = base_ouverte(e, etat).ok_or("aucune base ouverte")?;
            let n = etat.vue(e, base);
            let b = e.page_mut(base)?;
            if b.vues.len() > 1 {
                b.vues.remove(n);
            }
            etat.vues.insert(base, n.saturating_sub(1));
        }
        "filtres" => etat.filtres = !etat.filtres,
        "filtre-ajouter" => {
            let propriete = cle_choisie(l, "filtre-prop");
            if propriete.is_empty() {
                return Err("choisis une propriété à filtrer".into());
            }
            let f = Filtre { propriete, operateur: Operateur::depuis(&l.valeur("filtre-op").unwrap_or_default()), valeur: l.valeur("filtre-val").unwrap_or_default() };
            avec_vue(e, etat, |v| v.filtres.push(f))?;
        }
        // Une liste : le 1er clic l'ouvre (valeur inchangee : ne rien
        // redessiner, sinon elle se refermerait), le 2e choisit.
        "groupe" => {
            let g = cle_choisie(l, "groupe");
            if vue_ouverte(e, etat).is_some_and(|v| v.groupe == g) {
                return Ok(Suite::Rien);
            }
            avec_vue(e, etat, |v| v.groupe = g)?;
        }
        "tri" | "tri-sens" => {
            let p = cle_choisie(l, "tri");
            let desc = matches!(l.valeur("tri-sens").as_deref(), Some("Décroissant" | "desc"));
            let tri = (!p.is_empty()).then_some((p, desc));
            if vue_ouverte(e, etat).is_some_and(|v| v.tri == tri) {
                return Ok(Suite::Rien);
            }
            avec_vue(e, etat, |v| v.tri = tri)?;
        }
        _ => return cliquer_suite(e, etat, page, id, l, now),
    }
    Ok(Suite::Redessiner)
}

// Les boutons qui portent un numero (bloc, vue, colonne...).
fn cliquer_suite(e: &mut Espace, etat: &mut Etat, page: i64, id: &str, l: &dyn Lecture, now: u64) -> Result<Suite, String> {
    if let Some(genre) = id.strip_prefix("ajout-") {
        match genre {
            "code" => ajouter_bloc(e, page, GenreBloc::Code(String::new()), String::new())?,
            "tableau" => ajouter_bloc(e, page, GenreBloc::Tableau, tableau_neuf())?,
            "separateur" => ajouter_bloc(e, page, GenreBloc::Separateur, String::new())?,
            "souspage" => {
                let sous = e.creer(Some(page), "", now)?;
                ajouter_bloc(e, page, GenreBloc::SousPage, sous.to_string())?;
                etat.page = Some(sous);
            }
            "base" => {
                let base = e.creer_base(Some(page), "Nouvelle base", now)?;
                ajouter_bloc(e, page, GenreBloc::Vue, format!("{base}/0"))?;
            }
            g => ajouter_bloc(e, page, genre_texte(g).ok_or_else(|| format!("bloc inconnu : {g}"))?, String::new())?,
        }
        return Ok(Suite::Redessiner);
    }
    if let Some(reste) = id.strip_prefix("genre-") {
        // `genre-<bloc>` : la liste du bloc (choix par libelle) ;
        // `genre-<bloc>-<code>` : un bouton.
        let (bloc, genre) = match reste.split_once('-') {
            Some((bloc, code)) => (nombre(bloc).ok_or("bloc ?")?, genre_texte(code).ok_or("genre ?")?),
            None => {
                let choisi = l.valeur(id).unwrap_or_default();
                let code = crate::page::GENRES_TEXTE.iter().find(|g| g.1 == choisi).map(|g| g.0).ok_or("genre ?")?;
                (nombre(reste).ok_or("bloc ?")?, genre_texte(code).ok_or("genre ?")?)
            }
        };
        let Some(bl) = e.page_mut(page)?.blocs.iter_mut().find(|b| b.id == bloc) else { return Ok(Suite::Rien) };
        // Liste ouverte sans nouveau choix : ne rien redessiner.
        if bl.genre == genre {
            return Ok(Suite::Rien);
        }
        bl.genre = genre;
        return Ok(Suite::Redessiner);
    }
    if let Some(bloc) = id.strip_prefix("bloc-suppr-").and_then(nombre) {
        e.page_mut(page)?.blocs.retain(|b| b.id != bloc);
        return Ok(Suite::Redessiner);
    }
    for (prefixe, colonne) in [("bloc-ligne-", false), ("bloc-col-", true)] {
        if let Some(bloc) = id.strip_prefix(prefixe).and_then(nombre) {
            if let Some(bl) = e.page_mut(page)?.blocs.iter_mut().find(|b| b.id == bloc) {
                let mut c = cellules(&bl.contenu);
                if colonne {
                    let n = c.first().map(Vec::len).unwrap_or(0) + 1;
                    for (r, l) in c.iter_mut().enumerate() {
                        l.push(if r == 0 { format!("Colonne {n}") } else { String::new() });
                    }
                } else {
                    let largeur = c.first().map(Vec::len).unwrap_or(1);
                    c.push(vec![String::new(); largeur]);
                }
                bl.contenu = ecrire_cellules(&c);
            }
            return Ok(Suite::Redessiner);
        }
    }
    if let Some(a) = id.strip_prefix("vue-ajout-") {
        let base = base_ouverte(e, etat).ok_or("aucune base ouverte")?;
        let affichage = Affichage::depuis(a);
        let groupe = e.page(base).and_then(|b| b.schema.iter().find(|d| matches!(d.genre, Genre::Selection(_) | Genre::Etiquettes(_))).map(|d| d.cle())).unwrap_or_default();
        let b = e.page_mut(base)?;
        b.vues.push(Vue { groupe, ..Vue::nouvelle(affichage) });
        etat.vues.insert(base, b.vues.len() - 1);
        return Ok(Suite::Redessiner);
    }
    if let Some(n) = id.strip_prefix("vue-").and_then(|n| n.parse::<usize>().ok()) {
        let base = base_ouverte(e, etat).ok_or("aucune base ouverte")?;
        etat.vues.insert(base, n);
        return Ok(Suite::Redessiner);
    }
    if let Some(i) = id.strip_prefix("filtre-suppr-").and_then(|n| n.parse::<usize>().ok()) {
        avec_vue(e, etat, |v| {
            if i < v.filtres.len() {
                v.filtres.remove(i);
            }
        })?;
        return Ok(Suite::Redessiner);
    }
    // « + » en bas d'une colonne de kanban : la ligne nait dans la colonne.
    if let Some(i) = id.strip_prefix("ligne-col-").and_then(|n| n.parse::<usize>().ok()) {
        let base = base_ouverte(e, etat).ok_or("aucune base ouverte")?;
        let vue = e.page(base).and_then(|b| b.vues.get(etat.vue(e, base)).cloned()).ok_or("vue ?")?;
        let colonne = e.kanban(base, &vue).get(i).map(|c| c.0.clone()).unwrap_or_default();
        let ligne = e.creer(Some(base), "", now)?;
        if colonne != "Sans valeur" {
            e.changer(ligne, &vue.groupe, &colonne)?;
        }
        return Ok(Suite::Redessiner);
    }
    Ok(Suite::Rien)
}

fn genre_texte(code: &str) -> Option<GenreBloc> {
    Some(match code {
        "texte" => GenreBloc::Texte,
        "titre1" => GenreBloc::Titre(1),
        "titre2" => GenreBloc::Titre(2),
        "titre3" => GenreBloc::Titre(3),
        "puce" => GenreBloc::Puce,
        "numero" => GenreBloc::Numero,
        "tache" => GenreBloc::Tache(false),
        "citation" => GenreBloc::Citation,
        _ => return None,
    })
}

/// Range `id` sous `parent`, juste avant `avant` (a la fin si `None`).
fn ranger_avant(e: &mut Espace, id: i64, parent: Option<i64>, avant: Option<i64>) -> Result<(), String> {
    let freres: Vec<i64> = e.enfants(parent).iter().map(|p| p.id).filter(|x| *x != id).collect();
    let position = avant.and_then(|a| freres.iter().position(|x| *x == a)).unwrap_or(freres.len());
    e.deplacer(id, parent, position)
}

/// Les pages de l'arbre (a gauche), dans l'ordre affiche.
fn arbre_a_plat(e: &Espace) -> Vec<i64> {
    fn aller(e: &Espace, parent: Option<i64>, out: &mut Vec<i64>) {
        for p in e.enfants(parent) {
            out.push(p.id);
            if !p.base {
                aller(e, Some(p.id), out);
            }
        }
    }
    let mut out = Vec::new();
    aller(e, None, &mut out);
    out
}

/// Un element lache dans une zone (voir l'en-tete).
pub fn deposer(e: &mut Espace, etat: &Etat, source: &str, cible: &str, position: usize) -> Result<(), String> {
    let (genre, reste) = source.split_once('-').ok_or("source inconnue")?;
    match (genre, cible) {
        ("pa", "arbre") => {
            let id = nombre(reste).ok_or("page ?")?;
            let autres: Vec<i64> = arbre_a_plat(e).into_iter().filter(|x| *x != id).collect();
            match autres.get(position) {
                // Avant cette page, a son niveau.
                Some(&avant) => {
                    let parent = e.page(avant).and_then(|p| p.parent);
                    ranger_avant(e, id, parent, Some(avant))
                }
                None => ranger_avant(e, id, None, None),
            }
        }
        (_, c) if c.starts_with("dans-") => {
            let dans = nombre(&c[5..]).ok_or("page ?")?;
            let id = nombre(reste.split('-').next().unwrap_or("")).ok_or("page ?")?;
            ranger_avant(e, id, Some(dans), None)
        }
        ("k", c) if c.starts_with("col-") => {
            let (id, de) = reste.split_once('-').ok_or("carte ?")?;
            let (id, de, vers) = (nombre(id).ok_or("carte ?")?, de.parse::<usize>().map_err(|_| "colonne ?")?, c[4..].parse::<usize>().map_err(|_| "colonne ?")?);
            let base = e.page(id).and_then(|p| p.parent).ok_or("carte sans base")?;
            let vue = e.page(base).and_then(|b| b.vues.get(etat.vue(e, base)).cloned()).ok_or("vue ?")?;
            let colonnes = e.kanban(base, &vue);
            let nom = |i: usize| colonnes.get(i).map(|c| c.0.clone()).ok_or("colonne ?");
            let (nom_de, nom_vers) = (nom(de)?, nom(vers)?);
            // La carte avant laquelle on lache, parmi celles de la colonne.
            let avant = colonnes[vers].1.iter().copied().filter(|x| *x != id).nth(position);
            e.glisser_carte(id, &vue.groupe, &nom_de, &nom_vers, None)?;
            ranger_avant(e, id, Some(base), avant)
        }
        ("li", "lignes") => {
            let id = nombre(reste).ok_or("ligne ?")?;
            let base = e.page(id).and_then(|p| p.parent).ok_or("ligne sans base")?;
            let vue = e.page(base).and_then(|b| b.vues.get(etat.vue(e, base)).cloned()).ok_or("vue ?")?;
            let avant = e.lignes(base, &vue).into_iter().filter(|x| *x != id).nth(position);
            ranger_avant(e, id, Some(base), avant)
        }
        ("bl", "blocs") => {
            let id = nombre(reste).ok_or("bloc ?")?;
            let page = etat.page.ok_or("aucune page")?;
            let blocs = &mut e.page_mut(page)?.blocs;
            let Some(i) = blocs.iter().position(|b| b.id == id) else { return Ok(()) };
            let bloc = blocs.remove(i);
            blocs.insert(position.min(blocs.len()), bloc);
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Champs(HashMap<String, String>);

    impl Lecture for Champs {
        fn valeur(&self, id: &str) -> Option<String> {
            self.0.get(id).cloned()
        }
    }

    fn champs(v: &[(&str, &str)]) -> Champs {
        Champs(v.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    fn rien() -> Champs {
        champs(&[])
    }

    #[test]
    fn creer_remplir_enregistrer() {
        let mut e = Espace::default();
        let mut etat = Etat::default();
        cliquer(&mut e, &mut etat, "nouvelle", &rien(), 1).unwrap();
        let p = etat.page.unwrap();
        // Une page neuve a deja son bloc de texte (1).
        cliquer(&mut e, &mut etat, "ajout-tache", &rien(), 1).unwrap();
        cliquer(&mut e, &mut etat, "ajout-code", &rien(), 1).unwrap();
        cliquer(&mut e, &mut etat, "ajout-tableau", &rien(), 1).unwrap();
        let l = champs(&[("titre", "Courses"), ("b-1", "du pain"), ("b-2", "lait"), ("t-2", "true"), ("b-3", "\u{1f}#c678dd\u{1f}\u{1f}fn\u{1e}c\u{1f}\u{1f}\u{1f} x"), ("lang-3", "rust"), ("c-4-1-0", "a")]);
        assert!(enregistrer(&mut e, p, &l).is_empty());
        let page = e.page(p).unwrap();
        assert_eq!(page.titre, "Courses");
        assert_eq!(page.blocs[0].contenu, "du pain");
        assert_eq!(page.blocs[1].genre, GenreBloc::Tache(true));
        assert_eq!((page.blocs[2].genre.clone(), page.blocs[2].contenu.as_str()), (GenreBloc::Code("rust".into()), "fn x"));
        assert_eq!(cellules(&page.blocs[3].contenu)[1][0], "a");
        cliquer(&mut e, &mut etat, "bloc-col-4", &rien(), 1).unwrap();
        assert_eq!(cellules(&e.page(p).unwrap().blocs[3].contenu)[0].len(), 4);
        cliquer(&mut e, &mut etat, "genre-1-titre2", &rien(), 1).unwrap();
        assert_eq!(e.page(p).unwrap().blocs[0].genre, GenreBloc::Titre(2));
        cliquer(&mut e, &mut etat, "bloc-suppr-1", &rien(), 1).unwrap();
        assert_eq!(e.page(p).unwrap().blocs.len(), 3);
        deposer(&mut e, &etat, "bl-4", "blocs", 0).unwrap();
        assert_eq!(e.page(p).unwrap().blocs[0].id, 4);
    }

    #[test]
    fn base_proprietes_vues_et_kanban() {
        let mut e = Espace::default();
        let mut etat = Etat::default();
        cliquer(&mut e, &mut etat, "nouvelle-base", &rien(), 1).unwrap();
        let base = etat.page.unwrap();
        // Un clic sur le type cree la propriete ; le nom tape s'applique au clic suivant.
        cliquer(&mut e, &mut etat, "prop-ouvrir", &rien(), 1).unwrap();
        cliquer(&mut e, &mut etat, "prop-nouveau-nombre", &rien(), 1).unwrap();
        assert_eq!(etat.prop_menu.as_deref(), Some("nombre"));
        cliquer(&mut e, &mut etat, "carte-nom", &champs(&[("carte-nom", "Avancement")]), 1).unwrap();
        assert_eq!(etat.prop_menu, None);
        for _ in 0..3 {
            cliquer(&mut e, &mut etat, "ligne-ajouter", &rien(), 1).unwrap();
        }
        let l: Vec<i64> = e.enfants(Some(base)).iter().map(|p| p.id).collect();
        let fid = |id: i64, cle: &str| champ(id, cle);
        let valeurs = [(fid(l[0], "avancement"), "100"), (fid(l[1], "avancement"), "50")];
        let lecture = Champs(valeurs.iter().map(|(k, v)| (k.clone(), v.to_string())).collect());
        assert!(enregistrer(&mut e, base, &lecture).is_empty());
        // Le statut se choisit : ouvrir la valeur, cliquer « Fait » (option 2).
        cliquer(&mut e, &mut etat, &format!("ouvrir-{}", fid(l[0], "statut")), &rien(), 1).unwrap();
        assert!(etat.editeur.is_some());
        cliquer(&mut e, &mut etat, &format!("choix-{}@2", fid(l[0], "statut")), &rien(), 1).unwrap();
        assert_eq!(crate::page::valeur(&e, l[0], "statut"), "Fait");
        assert_eq!(etat.editeur, None);
        // Une formule dans le schema : chaque ligne l'a.
        cliquer(&mut e, &mut etat, "prop-nouveau-formule", &rien(), 1).unwrap();
        cliquer(&mut e, &mut etat, "carte-formule", &champs(&[("carte-nom", "Total"), ("carte-formule", "moyenne(enfants.avancement)")]), 1).unwrap();
        assert_eq!(etat.prop_menu.as_deref(), Some("total"));
        assert!(e.page(base).unwrap().schema.iter().any(|d| d.nom == "Total"));
        // Erreur claire pour une valeur refusee.
        let err = enregistrer(&mut e, base, &champs(&[(&fid(l[2], "avancement"), "beaucoup")]));
        assert_eq!(err.len(), 1);

        // Kanban (vue 1 : groupe = statut) : C passe de « Sans valeur » a « En cours ».
        cliquer(&mut e, &mut etat, "vue-1", &rien(), 1).unwrap();
        deposer(&mut e, &etat, &format!("k-{}-3", l[2]), "col-1", 0).unwrap();
        assert_eq!(crate::page::valeur(&e, l[2], "statut"), "En cours");
        // Une carte lachee en tete de « Fait » passe avant A.
        deposer(&mut e, &etat, &format!("k-{}-1", l[2]), "col-2", 0).unwrap();
        assert_eq!(e.enfants(Some(base))[0].id, l[2]);

        // Filtre et tri sur la vue table.
        cliquer(&mut e, &mut etat, "vue-0", &rien(), 1).unwrap();
        cliquer(&mut e, &mut etat, "filtre-ajouter", &champs(&[("filtre-prop", "statut"), ("filtre-op", "="), ("filtre-val", "fait")]), 1).unwrap();
        let vue = e.page(base).unwrap().vues[0].clone();
        assert_eq!(e.lignes(base, &vue), vec![l[2], l[0]]);
        cliquer(&mut e, &mut etat, "filtre-suppr-0", &rien(), 1).unwrap();
        cliquer(&mut e, &mut etat, "tri", &champs(&[("tri", "avancement"), ("tri-sens", "desc")]), 1).unwrap();
        let vue = e.page(base).unwrap().vues[0].clone();
        assert_eq!(e.lignes(base, &vue), vec![l[0], l[1], l[2]]);
    }

    #[test]
    fn arbre_glisser() {
        let mut e = Espace::default();
        let a = e.creer(None, "A", 0).unwrap();
        let b = e.creer(None, "B", 0).unwrap();
        let a1 = e.creer(Some(a), "A1", 0).unwrap();
        let etat = Etat::default();
        // Arbre : A, A1, B. B lache avant A1 : devient enfant de A, en tete.
        deposer(&mut e, &etat, &format!("pa-{b}"), "arbre", 1).unwrap();
        assert_eq!(e.enfants(Some(a)).iter().map(|p| p.id).collect::<Vec<_>>(), vec![b, a1]);
        // A lache dans sa propre sous-page : refuse.
        assert!(deposer(&mut e, &etat, &format!("pa-{a}"), &format!("dans-{a1}"), 0).is_err());
        // A1 lache dans B.
        deposer(&mut e, &etat, &format!("pa-{a1}"), &format!("dans-{b}"), 0).unwrap();
        assert_eq!(e.page(a1).unwrap().parent, Some(b));
        // Tout en bas : a la racine.
        deposer(&mut e, &etat, &format!("pa-{a1}"), "arbre", 99).unwrap();
        assert_eq!(e.page(a1).unwrap().parent, None);
    }
}
