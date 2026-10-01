// Les composants de Testeur : des morceaux de code de test faits par
// l'utilisateur, ranges dans des stockages (racines) et des dossiers
// imbriques (chaque noeud connait son parent). Ils sont propres a l'app :
// gardes dans son stockage prive, jamais dans les projets. L'atelier les
// insere dans un nouveau test.
use std::collections::HashSet;

pub type Id = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genre {
    /// Une racine : n'a pas de parent.
    Stockage,
    Dossier,
    Composant,
}

impl Genre {
    pub fn code(self) -> &'static str {
        match self {
            Genre::Stockage => "stockage",
            Genre::Dossier => "dossier",
            Genre::Composant => "composant",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Genre::Stockage => "Stockage",
            Genre::Dossier => "Dossier",
            Genre::Composant => "Composant",
        }
    }

    fn depuis(code: &str) -> Option<Genre> {
        [Genre::Stockage, Genre::Dossier, Genre::Composant].into_iter().find(|g| g.code() == code)
    }

    /// Peut contenir d'autres noeuds.
    pub fn contenant(self) -> bool {
        self != Genre::Composant
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Noeud {
    pub id: Id,
    pub parent: Option<Id>,
    pub genre: Genre,
    pub nom: String,
    /// Le code (composants seulement).
    pub code: String,
}

/// Ou garder la bibliotheque (le stockage prive de l'app, ou la memoire).
pub trait Garde: Send + Sync {
    fn charger(&self) -> String;
    fn garder(&self, texte: &str) -> Result<(), String>;
}

/// En memoire seulement (tests, stockage indisponible).
#[derive(Default)]
pub struct GardeMemoire(pub std::sync::Mutex<String>);

impl Garde for GardeMemoire {
    fn charger(&self) -> String {
        self.0.lock().map(|t| t.clone()).unwrap_or_default()
    }

    fn garder(&self, texte: &str) -> Result<(), String> {
        *self.0.lock().map_err(|_| "verrou")? = texte.to_string();
        Ok(())
    }
}

pub struct Bibliotheque {
    /// Dans l'ordre de creation (les enfants s'affichent dans cet ordre).
    pub noeuds: Vec<Noeud>,
    garde: Box<dyn Garde>,
}

// Une ligne par noeud : id, parent, genre, nom, code - separes par des
// tabulations, avec \\, \t et \n echappes.
fn echapper(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
}

fn desechapper(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

fn lire(texte: &str) -> Vec<Noeud> {
    texte
        .lines()
        .filter_map(|l| {
            let mut c = l.split('\t');
            let id = c.next()?.parse().ok()?;
            let parent = c.next()?;
            let parent = if parent.is_empty() { None } else { Some(parent.parse().ok()?) };
            let genre = Genre::depuis(c.next()?)?;
            let nom = desechapper(c.next()?);
            let code = c.next().map(desechapper).unwrap_or_default();
            Some(Noeud { id, parent, genre, nom, code })
        })
        .collect()
}

fn ecrire(noeuds: &[Noeud]) -> String {
    noeuds
        .iter()
        .map(|n| format!("{}\t{}\t{}\t{}\t{}\n", n.id, n.parent.map(|p| p.to_string()).unwrap_or_default(), n.genre.code(), echapper(&n.nom), echapper(&n.code)))
        .collect()
}

impl Bibliotheque {
    pub fn new(garde: Box<dyn Garde>) -> Bibliotheque {
        let mut noeuds = lire(&garde.charger());
        // Un noeud dont le parent a disparu (fichier abime) redevient une racine.
        let ids: HashSet<Id> = noeuds.iter().map(|n| n.id).collect();
        for n in &mut noeuds {
            if n.parent.is_some_and(|p| !ids.contains(&p)) {
                n.parent = None;
                n.genre = if n.genre == Genre::Composant { Genre::Composant } else { Genre::Stockage };
            }
        }
        Bibliotheque { noeuds, garde }
    }

    fn sauver(&self) -> Result<(), String> {
        self.garde.garder(&ecrire(&self.noeuds))
    }

    pub fn noeud(&self, id: Id) -> Option<&Noeud> {
        self.noeuds.iter().find(|n| n.id == id)
    }

    pub fn enfants(&self, id: Option<Id>) -> impl Iterator<Item = &Noeud> {
        self.noeuds.iter().filter(move |n| n.parent == id)
    }

    /// Les ancetres de `id`, de la racine a lui compris.
    pub fn chemin(&self, id: Id) -> Vec<&Noeud> {
        let mut out = Vec::new();
        let mut courant = self.noeud(id);
        while let Some(n) = courant {
            out.push(n);
            // Garde-fou contre un cycle (fichier abime).
            if out.len() > self.noeuds.len() {
                break;
            }
            courant = n.parent.and_then(|p| self.noeud(p));
        }
        out.reverse();
        out
    }

    /// `Stockage › Dossier › nom`
    pub fn chemin_texte(&self, id: Id) -> String {
        self.chemin(id).iter().map(|n| n.nom.as_str()).collect::<Vec<_>>().join(" › ")
    }

    /// `id` et tous ses descendants.
    pub fn descendants(&self, id: Id) -> HashSet<Id> {
        let mut out = HashSet::from([id]);
        loop {
            let avant = out.len();
            for n in &self.noeuds {
                if n.parent.is_some_and(|p| out.contains(&p)) {
                    out.insert(n.id);
                }
            }
            if out.len() == avant {
                return out;
            }
        }
    }

    /// Cree un noeud sous `parent` (un stockage n'a pas de parent, les
    /// autres en ont un qui peut contenir). Rend son id.
    pub fn creer(&mut self, parent: Option<Id>, genre: Genre, nom: &str) -> Result<Id, String> {
        match (genre, parent) {
            (Genre::Stockage, Some(_)) => return Err("Un stockage est toujours à la racine.".to_string()),
            (Genre::Dossier | Genre::Composant, None) => return Err("Un dossier ou un composant se range dans un stockage.".to_string()),
            (_, Some(p)) if !self.noeud(p).is_some_and(|n| n.genre.contenant()) => return Err("On ne range rien dans un composant.".to_string()),
            _ => {}
        }
        let nom = nom.trim();
        let nom = if nom.is_empty() { genre.libelle() } else { nom };
        let id = self.noeuds.iter().map(|n| n.id).max().unwrap_or(0) + 1;
        self.noeuds.push(Noeud { id, parent, genre, nom: nom.to_string(), code: String::new() });
        self.sauver()?;
        Ok(id)
    }

    pub fn renommer(&mut self, id: Id, nom: &str) -> Result<(), String> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err("Le nom ne peut pas être vide.".to_string());
        }
        self.noeuds.iter_mut().find(|n| n.id == id).ok_or("Élément introuvable.")?.nom = nom.to_string();
        self.sauver()
    }

    pub fn fixer_code(&mut self, id: Id, code: &str) -> Result<(), String> {
        let n = self.noeuds.iter_mut().find(|n| n.id == id).ok_or("Composant introuvable.")?;
        if n.genre != Genre::Composant {
            return Err("Seul un composant a du code.".to_string());
        }
        n.code = code.to_string();
        self.sauver()
    }

    /// Deplace `id` sous `parent` (pas sous lui-meme ni un descendant).
    pub fn deplacer(&mut self, id: Id, parent: Id) -> Result<(), String> {
        if self.descendants(id).contains(&parent) {
            return Err("On ne range pas un dossier dans lui-même.".to_string());
        }
        if !self.noeud(parent).is_some_and(|n| n.genre.contenant()) {
            return Err("On ne range rien dans un composant.".to_string());
        }
        let n = self.noeuds.iter_mut().find(|n| n.id == id).ok_or("Élément introuvable.")?;
        if n.genre == Genre::Stockage {
            n.genre = Genre::Dossier;
        }
        n.parent = Some(parent);
        self.sauver()
    }

    /// Supprime `id` et tout ce qu'il contient ; rend le nombre d'elements
    /// supprimes.
    pub fn supprimer(&mut self, id: Id) -> Result<usize, String> {
        let partis = self.descendants(id);
        let avant = self.noeuds.len();
        self.noeuds.retain(|n| !partis.contains(&n.id));
        self.sauver()?;
        Ok(avant - self.noeuds.len())
    }

    /// Tous les composants, dans l'ordre de l'arbre, avec leur chemin.
    pub fn composants(&self) -> Vec<(Id, String)> {
        self.arbre(&HashSet::new()).into_iter().filter(|(n, _)| n.genre == Genre::Composant).map(|(n, _)| (n.id, self.chemin_texte(n.id))).collect()
    }

    /// L'arbre a plat (parent puis enfants), avec la profondeur ; les
    /// enfants des noeuds de `plies` sont caches.
    pub fn arbre(&self, plies: &HashSet<Id>) -> Vec<(&Noeud, usize)> {
        fn aller<'a>(b: &'a Bibliotheque, parent: Option<Id>, prof: usize, plies: &HashSet<Id>, out: &mut Vec<(&'a Noeud, usize)>) {
            // Dossiers d'abord, puis composants.
            let mut enfants: Vec<&Noeud> = b.enfants(parent).collect();
            enfants.sort_by_key(|n| n.genre == Genre::Composant);
            for n in enfants {
                out.push((n, prof));
                if !plies.contains(&n.id) && prof < 32 {
                    aller(b, Some(n.id), prof + 1, plies, out);
                }
            }
        }
        let mut out = Vec::new();
        aller(self, None, 0, plies, &mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn biblio() -> Bibliotheque {
        Bibliotheque::new(Box::new(GardeMemoire::default()))
    }

    #[test]
    fn dossiers_imbriques_et_suppression_en_cascade() {
        let mut b = biblio();
        let s = b.creer(None, Genre::Stockage, "Mes modèles").unwrap();
        let d = b.creer(Some(s), Genre::Dossier, "Réseau").unwrap();
        let dd = b.creer(Some(d), Genre::Dossier, "HTTP").unwrap();
        let c = b.creer(Some(dd), Genre::Composant, "requete_ok").unwrap();
        b.fixer_code(c, "let r = get(\"/\");\n\tassert!(r.ok());").unwrap();
        assert_eq!(b.chemin_texte(c), "Mes modèles › Réseau › HTTP › requete_ok");
        assert!(b.creer(Some(c), Genre::Dossier, "x").is_err(), "rien dans un composant");
        assert!(b.creer(None, Genre::Dossier, "x").is_err(), "un dossier a un parent");
        assert!(b.deplacer(d, dd).is_err(), "pas dans son propre descendant");
        let plie: Vec<Id> = b.arbre(&HashSet::from([d])).iter().map(|(n, _)| n.id).collect();
        assert_eq!(plie, vec![s, d]);
        assert_eq!(b.arbre(&HashSet::new()).iter().map(|(_, p)| *p).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
        assert_eq!(b.supprimer(d).unwrap(), 3);
        assert_eq!(b.noeuds.len(), 1);
    }

    #[test]
    fn relu_tel_quel_depuis_le_stockage() {
        let garde = std::sync::Arc::new(GardeMemoire::default());
        struct Partage(std::sync::Arc<GardeMemoire>);
        impl Garde for Partage {
            fn charger(&self) -> String {
                self.0.charger()
            }
            fn garder(&self, t: &str) -> Result<(), String> {
                self.0.garder(t)
            }
        }
        let mut b = Bibliotheque::new(Box::new(Partage(garde.clone())));
        let s = b.creer(None, Genre::Stockage, "A\tb\\c").unwrap();
        let c = b.creer(Some(s), Genre::Composant, "comp").unwrap();
        b.fixer_code(c, "ligne 1\n\tligne 2 \\n pas un saut").unwrap();
        let relu = Bibliotheque::new(Box::new(Partage(garde)));
        assert_eq!(relu.noeuds, b.noeuds);
    }
}
