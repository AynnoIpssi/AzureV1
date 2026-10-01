// Les pages v2, gardees en memoire et rangees dans la base RsS de l'app :
//
//   pages       (id, parent, ordre, titre, icone, est_base, vues, modifie)
//   blocs       (page, ordre, id, genre, contenu)
//   proprietes  (page, ordre, nom, du_schema, genre, config)
//   valeurs     (page, cle, valeur)
//   apparence   (page, largeur)
//
// Tout changement passe par `modifier` : on compare l'espace avant/apres et
// seules les pages qui ont bouge sont reecrites (une transaction). Si
// l'ecriture echoue, la memoire revient en arriere.
//
// Au premier lancement, les notes v1 deviennent des pages (les cles v1 ne
// sont pas effacees).
use crate::carnet::{maintenant, Carnet};
use crate::modele::{Bloc, Definition, Espace, Genre, GenreBloc, Page, Vue};
use azure_foundation::storage::models::db::Db;
use azure_foundation::storage::models::stockage::Stockage;
use azure_foundation::storage::Value;
use std::sync::{Arc, Mutex, MutexGuard};

/// Separe les vues d'une base (chaque vue utilise SEP et SEP_LIGNE).
const SEP_VUE: char = '\u{1d}';

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS pages (id INT PRIMARY KEY, parent INT, ordre INT, titre TEXT, icone TEXT, est_base BOOL, vues TEXT, modifie INT);
CREATE TABLE IF NOT EXISTS blocs (page INT, ordre INT, id INT, genre TEXT, contenu TEXT);
CREATE TABLE IF NOT EXISTS proprietes (page INT, ordre INT, nom TEXT, du_schema BOOL, genre TEXT, config TEXT);
CREATE TABLE IF NOT EXISTS valeurs (page INT, cle TEXT, valeur TEXT);
CREATE TABLE IF NOT EXISTS apparence (page INT, largeur TEXT)
";

/// Cle du stockage prive : la migration v1 est faite.
const MIGRE: &str = "v2.migre";

#[derive(Clone, Default)]
pub struct Classeur {
    espace: Arc<Mutex<Espace>>,
    db: Option<Db>,
}

impl Classeur {
    /// Sans stockage (tests, daemon absent).
    pub fn en_memoire() -> Classeur {
        Classeur::default()
    }

    /// Lit les pages ; convertit les notes v1 la premiere fois.
    pub fn ouvrir(store: Stockage) -> Result<Classeur, String> {
        let db = store.db();
        db.script(SCHEMA)?;
        let espace = charger(&db)?;
        let classeur = Classeur { espace: Arc::new(Mutex::new(espace)), db: Some(db) };
        if store.get::<String>(MIGRE)?.is_none() {
            let notes = Carnet::charger(store.clone())?.notes();
            classeur.modifier(|e| {
                // La plus recente en haut, comme en v1.
                for n in &notes {
                    let id = e.creer(None, &n.titre, n.modifie)?;
                    let p = e.page_mut(id)?;
                    p.blocs = blocs_depuis_texte(&n.texte);
                }
                Ok(())
            })?;
            store.set(MIGRE, "1")?;
        }
        Ok(classeur)
    }

    fn verrou(&self) -> MutexGuard<'_, Espace> {
        self.espace.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Une copie de l'espace, pour afficher.
    pub fn espace(&self) -> Espace {
        self.verrou().clone()
    }

    /// Lit sans copier.
    pub fn lire<T>(&self, f: impl FnOnce(&Espace) -> T) -> T {
        f(&self.verrou())
    }

    /// Applique `f` puis range les pages changees (datees de maintenant).
    pub fn modifier<T>(&self, f: impl FnOnce(&mut Espace) -> Result<T, String>) -> Result<T, String> {
        let mut espace = self.verrou();
        let avant = espace.clone();
        let r = match f(&mut espace) {
            Ok(r) => r,
            Err(e) => {
                *espace = avant;
                return Err(e);
            }
        };
        let ids = changees(&avant, &espace);
        let t = maintenant();
        for id in &ids {
            let Some(p) = espace.pages.get_mut(id) else { continue };
            match avant.page(*id) {
                // Une page creee garde la date donnee (notes v1).
                None if p.modifie == 0 => p.modifie = t,
                Some(a) if contenu_change(a, p) => p.modifie = t,
                _ => {}
            }
        }
        if let Some(db) = &self.db {
            if let Err(e) = ecrire(db, &espace, &ids) {
                *espace = avant;
                return Err(e);
            }
        }
        Ok(r)
    }
}

/// Les pages ajoutees, retirees ou changees.
pub fn changees(avant: &Espace, apres: &Espace) -> Vec<i64> {
    let mut ids: Vec<i64> = apres.pages.iter().filter(|(id, p)| avant.pages.get(id) != Some(p)).map(|(id, _)| *id).collect();
    ids.extend(avant.pages.keys().filter(|id| !apres.pages.contains_key(id)));
    ids
}

/// Plus que la place (ordre, parent) : merite une nouvelle date.
fn contenu_change(a: &Page, b: &Page) -> bool {
    a.titre != b.titre || a.icone != b.icone || a.blocs != b.blocs || a.valeurs != b.valeurs || a.propres != b.propres || a.schema != b.schema || a.vues != b.vues
}

/// Une note v1 en blocs : une ligne = un bloc ; `# `, `## `, `### `, `- `,
/// `* `, `1. `, `[ ] `, `[x] `, `> ` et ``` donnent le bloc correspondant.
pub fn blocs_depuis_texte(texte: &str) -> Vec<Bloc> {
    let mut blocs: Vec<Bloc> = Vec::new();
    let mut code: Option<(String, Vec<&str>)> = None;
    for ligne in texte.lines() {
        if let Some((langage, lignes)) = &mut code {
            if ligne.trim_start().starts_with("```") {
                blocs.push(Bloc { id: 0, genre: GenreBloc::Code(langage.clone()), contenu: lignes.join("\n") });
                code = None;
            } else {
                lignes.push(ligne);
            }
            continue;
        }
        let l = ligne.trim();
        if l.is_empty() {
            continue;
        }
        if let Some(langage) = l.strip_prefix("```") {
            code = Some((langage.trim().to_string(), Vec::new()));
            continue;
        }
        let numero = l.split_once(". ").filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        let (genre, reste) = if let Some(r) = l.strip_prefix("### ") {
            (GenreBloc::Titre(3), r)
        } else if let Some(r) = l.strip_prefix("## ") {
            (GenreBloc::Titre(2), r)
        } else if let Some(r) = l.strip_prefix("# ") {
            (GenreBloc::Titre(1), r)
        } else if let Some(r) = l.strip_prefix("- [ ] ").or_else(|| l.strip_prefix("[ ] ")) {
            (GenreBloc::Tache(false), r)
        } else if let Some(r) = l.strip_prefix("- [x] ").or_else(|| l.strip_prefix("[x] ")) {
            (GenreBloc::Tache(true), r)
        } else if let Some(r) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
            (GenreBloc::Puce, r)
        } else if let Some((_, r)) = numero {
            (GenreBloc::Numero, r)
        } else if let Some(r) = l.strip_prefix("> ") {
            (GenreBloc::Citation, r)
        } else if l == "---" {
            (GenreBloc::Separateur, "")
        } else {
            (GenreBloc::Texte, l)
        };
        blocs.push(Bloc { id: 0, genre, contenu: reste.to_string() });
    }
    // Un ``` jamais ferme garde quand meme son code.
    if let Some((langage, lignes)) = code {
        blocs.push(Bloc { id: 0, genre: GenreBloc::Code(langage), contenu: lignes.join("\n") });
    }
    for (i, b) in blocs.iter_mut().enumerate() {
        b.id = i as i64 + 1;
    }
    blocs
}

fn charger(db: &Db) -> Result<Espace, String> {
    let mut espace = Espace::default();
    for r in db.query("SELECT id, parent, ordre, titre, icone, est_base, vues, modifie FROM pages", &[])? {
        let vues: String = r.get_or("vues", String::new());
        let page = Page {
            id: r.get("id")?,
            parent: r.get("parent")?,
            ordre: r.get_or("ordre", 0),
            titre: r.get_or("titre", String::new()),
            icone: r.get_or("icone", String::new()),
            base: r.get_or("est_base", false),
            vues: vues.split(SEP_VUE).filter(|v| !v.is_empty()).map(Vue::lire).collect(),
            modifie: r.get_or::<i64>("modifie", 0).max(0) as u64,
            ..Page::default()
        };
        espace.pages.insert(page.id, page);
    }
    let mut blocs: Vec<(i64, i64, Bloc)> = Vec::new();
    for r in db.query("SELECT page, ordre, id, genre, contenu FROM blocs", &[])? {
        blocs.push((r.get("page")?, r.get_or("ordre", 0), Bloc { id: r.get_or("id", 0), genre: GenreBloc::depuis(&r.get_or("genre", String::new())), contenu: r.get_or("contenu", String::new()) }));
    }
    blocs.sort_by_key(|b| (b.0, b.1));
    for (page, _, bloc) in blocs {
        if let Some(p) = espace.pages.get_mut(&page) {
            p.blocs.push(bloc);
        }
    }
    let mut props: Vec<(i64, i64, bool, Definition)> = Vec::new();
    for r in db.query("SELECT page, ordre, nom, du_schema, genre, config FROM proprietes", &[])? {
        let genre = Genre::depuis(&r.get_or("genre", String::new()), &r.get_or("config", String::new()));
        props.push((r.get("page")?, r.get_or("ordre", 0), r.get_or("du_schema", false), Definition { nom: r.get_or("nom", String::new()), genre }));
    }
    props.sort_by_key(|p| (p.0, p.1));
    for (page, _, du_schema, d) in props {
        if let Some(p) = espace.pages.get_mut(&page) {
            let defs = if du_schema { &mut p.schema } else { &mut p.propres };
            defs.push(d);
        }
    }
    for r in db.query("SELECT page, cle, valeur FROM valeurs", &[])? {
        let page: i64 = r.get("page")?;
        if let Some(p) = espace.pages.get_mut(&page) {
            p.valeurs.insert(r.get("cle")?, r.get_or("valeur", String::new()));
        }
    }
    for r in db.query("SELECT page, largeur FROM apparence", &[])? {
        let page: i64 = r.get("page")?;
        if let Some(p) = espace.pages.get_mut(&page) {
            p.largeur = r.get_or("largeur", String::new());
        }
    }
    Ok(espace)
}

fn ecrire(db: &Db, espace: &Espace, ids: &[i64]) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }
    db.transaction(|db| {
        for id in ids {
            let id = Value::from(*id);
            db.run("DELETE FROM pages WHERE id = ?", std::slice::from_ref(&id))?;
            for table in ["blocs", "proprietes", "valeurs", "apparence"] {
                db.run(&format!("DELETE FROM {table} WHERE page = ?"), std::slice::from_ref(&id))?;
            }
        }
        for p in ids.iter().filter_map(|id| espace.page(*id)) {
            let vues = p.vues.iter().map(Vue::ecrire).collect::<Vec<_>>().join(&SEP_VUE.to_string());
            db.run(
                "INSERT INTO pages (id, parent, ordre, titre, icone, est_base, vues, modifie) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                &[Value::from(p.id), Value::from(p.parent), Value::from(p.ordre), Value::from(&p.titre), Value::from(&p.icone), Value::from(p.base), Value::from(vues), Value::from(p.modifie as i64)],
            )?;
            for (i, b) in p.blocs.iter().enumerate() {
                db.run("INSERT INTO blocs (page, ordre, id, genre, contenu) VALUES (?, ?, ?, ?, ?)", &[Value::from(p.id), Value::from(i), Value::from(b.id), Value::from(b.genre.code()), Value::from(&b.contenu)])?;
            }
            let defs = p.schema.iter().map(|d| (true, d)).chain(p.propres.iter().map(|d| (false, d)));
            for (i, (du_schema, d)) in defs.enumerate() {
                db.run(
                    "INSERT INTO proprietes (page, ordre, nom, du_schema, genre, config) VALUES (?, ?, ?, ?, ?, ?)",
                    &[Value::from(p.id), Value::from(i), Value::from(&d.nom), Value::from(du_schema), Value::from(d.genre.code()), Value::from(d.genre.config())],
                )?;
            }
            for (cle, valeur) in &p.valeurs {
                db.run("INSERT INTO valeurs (page, cle, valeur) VALUES (?, ?, ?)", &[Value::from(p.id), Value::from(cle), Value::from(valeur)])?;
            }
            if !p.largeur.is_empty() {
                db.run("INSERT INTO apparence (page, largeur) VALUES (?, ?)", &[Value::from(p.id), Value::from(&p.largeur)])?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_v1_en_blocs() {
        let b = blocs_depuis_texte("# Courses\n\n- pain\n[x] lait\n2. œufs\n```rust\nfn main() {}\n```\nfin");
        let g: Vec<(GenreBloc, &str)> = b.iter().map(|b| (b.genre.clone(), b.contenu.as_str())).collect();
        assert_eq!(g, vec![
            (GenreBloc::Titre(1), "Courses"),
            (GenreBloc::Puce, "pain"),
            (GenreBloc::Tache(true), "lait"),
            (GenreBloc::Numero, "œufs"),
            (GenreBloc::Code("rust".into()), "fn main() {}"),
            (GenreBloc::Texte, "fin"),
        ]);
        assert_eq!(b.iter().map(|b| b.id).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn modifier_annule_sur_erreur_et_voit_les_changements() {
        let c = Classeur::en_memoire();
        let id = c.modifier(|e| e.creer(None, "A", 0)).unwrap();
        let avant = c.espace();
        assert!(c.modifier(|e| {
            e.page_mut(id)?.titre = "B".into();
            Err::<(), _>("non".into())
        })
        .is_err());
        assert_eq!(c.espace(), avant);

        let b = c.modifier(|e| e.creer(None, "B", 0)).unwrap();
        let avant = c.espace();
        c.modifier(|e| e.deplacer(b, Some(id), 0)).unwrap();
        assert_eq!(changees(&avant, &c.espace()), vec![b]);
        let avant = c.espace();
        c.modifier(|e| Ok(e.supprimer(id))).unwrap();
        assert_eq!(changees(&avant, &c.espace()), vec![id, b]);
    }
}
