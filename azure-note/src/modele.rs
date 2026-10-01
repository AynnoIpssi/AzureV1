// Les pages de Note v2, en memoire (le stockage est dans `classeur`).
//
// - Une page a un parent (ou aucun : racine), des blocs et des proprietes.
// - Une page « base » est une base de donnees : ses sous-pages sont ses
//   lignes et partagent son `schema` ; elle a des vues (table, kanban...).
// - Toute page peut aussi avoir ses proprietes `propres`.
// - Une propriete Formule se calcule a la lecture, a partir de la page et de
//   ses sous-pages : modifier un enfant change donc aussitot le parent, et
//   ainsi de suite jusqu'en haut. Les cycles sont signales, pas suivis.
use crate::formule::{normaliser, Contexte, Formule, Valeur};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// Separe les champs d'une valeur rangee en texte.
pub const SEP: char = '\u{1f}';
/// Separe les lignes (d'un tableau, des options...).
pub const SEP_LIGNE: char = '\u{1e}';

#[derive(Debug, Clone, PartialEq)]
pub enum Genre {
    Texte,
    Nombre,
    /// Une option parmi la liste.
    Selection(Vec<String>),
    /// Plusieurs etiquettes parmi la liste (on peut en ajouter).
    Etiquettes(Vec<String>),
    /// AAAA-MM-JJ.
    Date,
    Case,
    /// Des ids de pages.
    Relation,
    Formule(String),
}

impl Genre {
    pub const CODES: [&'static str; 8] = ["texte", "nombre", "selection", "etiquettes", "date", "case", "relation", "formule"];

    pub fn code(&self) -> &'static str {
        match self {
            Genre::Texte => "texte",
            Genre::Nombre => "nombre",
            Genre::Selection(_) => "selection",
            Genre::Etiquettes(_) => "etiquettes",
            Genre::Date => "date",
            Genre::Case => "case",
            Genre::Relation => "relation",
            Genre::Formule(_) => "formule",
        }
    }

    /// Les options, ou le texte de la formule.
    pub fn config(&self) -> String {
        match self {
            Genre::Selection(o) | Genre::Etiquettes(o) => o.join(&SEP.to_string()),
            Genre::Formule(f) => f.clone(),
            _ => String::new(),
        }
    }

    pub fn depuis(code: &str, config: &str) -> Genre {
        let options = || config.split(SEP).filter(|o| !o.is_empty()).map(str::to_string).collect();
        match code {
            "nombre" => Genre::Nombre,
            "selection" => Genre::Selection(options()),
            "etiquettes" => Genre::Etiquettes(options()),
            "date" => Genre::Date,
            "case" => Genre::Case,
            "relation" => Genre::Relation,
            "formule" => Genre::Formule(config.to_string()),
            _ => Genre::Texte,
        }
    }

    pub fn libelle(&self) -> &'static str {
        match self {
            Genre::Texte => "Texte",
            Genre::Nombre => "Nombre",
            Genre::Selection(_) => "Sélection",
            Genre::Etiquettes(_) => "Étiquettes",
            Genre::Date => "Date",
            Genre::Case => "Case à cocher",
            Genre::Relation => "Relation",
            Genre::Formule(_) => "Formule",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub nom: String,
    pub genre: Genre,
}

impl Definition {
    pub fn cle(&self) -> String {
        normaliser(&self.nom)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenreBloc {
    Titre(u8),
    /// Texte riche (voir `riche`).
    Texte,
    Puce,
    Numero,
    Tache(bool),
    Citation,
    /// Le langage, pour la coloration.
    Code(String),
    /// Cellules : lignes separees par SEP_LIGNE, cellules par SEP ; la 1re
    /// ligne est l'en-tete.
    Tableau,
    Separateur,
    /// Lien vers une sous-page (son id dans `contenu`).
    SousPage,
    /// Une vue d'une base (« <id base>/<n° de vue> » dans `contenu`).
    Vue,
}

impl GenreBloc {
    pub fn code(&self) -> String {
        match self {
            GenreBloc::Titre(n) => format!("titre{n}"),
            GenreBloc::Texte => "texte".into(),
            GenreBloc::Puce => "puce".into(),
            GenreBloc::Numero => "numero".into(),
            GenreBloc::Tache(fait) => if *fait { "tache+" } else { "tache" }.into(),
            GenreBloc::Citation => "citation".into(),
            GenreBloc::Code(l) => format!("code:{l}"),
            GenreBloc::Tableau => "tableau".into(),
            GenreBloc::Separateur => "separateur".into(),
            GenreBloc::SousPage => "souspage".into(),
            GenreBloc::Vue => "vue".into(),
        }
    }

    pub fn depuis(code: &str) -> GenreBloc {
        match code {
            "titre1" => GenreBloc::Titre(1),
            "titre2" => GenreBloc::Titre(2),
            "titre3" => GenreBloc::Titre(3),
            "puce" => GenreBloc::Puce,
            "numero" => GenreBloc::Numero,
            "tache" => GenreBloc::Tache(false),
            "tache+" => GenreBloc::Tache(true),
            "citation" => GenreBloc::Citation,
            "tableau" => GenreBloc::Tableau,
            "separateur" => GenreBloc::Separateur,
            "souspage" => GenreBloc::SousPage,
            "vue" => GenreBloc::Vue,
            c if c.starts_with("code:") => GenreBloc::Code(c[5..].to_string()),
            _ => GenreBloc::Texte,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bloc {
    pub id: i64,
    pub genre: GenreBloc,
    pub contenu: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Affichage {
    Table,
    Kanban,
    Liste,
    Galerie,
}

impl Affichage {
    pub const TOUS: [Affichage; 4] = [Affichage::Table, Affichage::Kanban, Affichage::Liste, Affichage::Galerie];

    pub fn code(&self) -> &'static str {
        match self {
            Affichage::Table => "table",
            Affichage::Kanban => "kanban",
            Affichage::Liste => "liste",
            Affichage::Galerie => "galerie",
        }
    }

    pub fn depuis(code: &str) -> Affichage {
        Affichage::TOUS.into_iter().find(|a| a.code() == code).unwrap_or(Affichage::Table)
    }

    pub fn libelle(&self) -> &'static str {
        match self {
            Affichage::Table => "Table",
            Affichage::Kanban => "Kanban",
            Affichage::Liste => "Liste",
            Affichage::Galerie => "Galerie",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operateur {
    Egal,
    Different,
    Contient,
    Plus,
    Moins,
    Vide,
    NonVide,
}

impl Operateur {
    pub const TOUS: [Operateur; 7] = [Operateur::Egal, Operateur::Different, Operateur::Contient, Operateur::Plus, Operateur::Moins, Operateur::Vide, Operateur::NonVide];

    pub fn code(&self) -> &'static str {
        match self {
            Operateur::Egal => "=",
            Operateur::Different => "!=",
            Operateur::Contient => "contient",
            Operateur::Plus => ">",
            Operateur::Moins => "<",
            Operateur::Vide => "vide",
            Operateur::NonVide => "non vide",
        }
    }

    pub fn depuis(code: &str) -> Operateur {
        Operateur::TOUS.into_iter().find(|o| o.code() == code).unwrap_or(Operateur::Egal)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filtre {
    pub propriete: String,
    pub operateur: Operateur,
    pub valeur: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vue {
    pub nom: String,
    pub affichage: Affichage,
    /// Kanban : la propriete (selection ou etiquettes) qui fait les colonnes.
    pub groupe: String,
    pub filtres: Vec<Filtre>,
    /// Propriete de tri et sens (true = decroissant).
    pub tri: Option<(String, bool)>,
}

impl Vue {
    pub fn nouvelle(affichage: Affichage) -> Vue {
        Vue { nom: affichage.libelle().to_string(), affichage, groupe: String::new(), filtres: Vec::new(), tri: None }
    }

    /// nom SEP affichage SEP groupe SEP tri SEP desc, puis un filtre par ligne.
    pub fn ecrire(&self) -> String {
        let (tri, desc) = self.tri.clone().unwrap_or_default();
        let mut lignes = vec![[self.nom.as_str(), self.affichage.code(), &self.groupe, &tri, if desc { "1" } else { "0" }].join(&SEP.to_string())];
        for f in &self.filtres {
            lignes.push([f.propriete.as_str(), f.operateur.code(), &f.valeur].join(&SEP.to_string()));
        }
        lignes.join(&SEP_LIGNE.to_string())
    }

    pub fn lire(s: &str) -> Vue {
        let mut lignes = s.split(SEP_LIGNE);
        let tete: Vec<&str> = lignes.next().unwrap_or("").split(SEP).collect();
        let champ = |i: usize| tete.get(i).copied().unwrap_or("").to_string();
        let tri = champ(3);
        let filtres = lignes
            .map(|l| {
                let f: Vec<&str> = l.split(SEP).collect();
                Filtre { propriete: f.first().unwrap_or(&"").to_string(), operateur: Operateur::depuis(f.get(1).unwrap_or(&"")), valeur: f.get(2).unwrap_or(&"").to_string() }
            })
            .collect();
        Vue { nom: champ(0), affichage: Affichage::depuis(&champ(1)), groupe: champ(2), filtres, tri: (!tri.is_empty()).then(|| (tri, champ(4) == "1")) }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Page {
    pub id: i64,
    pub parent: Option<i64>,
    pub ordre: i64,
    pub titre: String,
    pub icone: String,
    /// Largeur de la colonne (`page::LARGEURS`) ; vide : celle par defaut.
    pub largeur: String,
    pub base: bool,
    /// Base : les proprietes de ses lignes.
    pub schema: Vec<Definition>,
    pub vues: Vec<Vue>,
    /// Les proprietes de la page elle-meme.
    pub propres: Vec<Definition>,
    /// Valeurs brutes, par cle normalisee.
    pub valeurs: BTreeMap<String, String>,
    pub blocs: Vec<Bloc>,
    pub modifie: u64,
}

impl Page {
    pub fn nom(&self) -> &str {
        let t = self.titre.trim();
        if t.is_empty() { "Sans titre" } else { t }
    }
}

/// Toutes les pages.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Espace {
    pub pages: BTreeMap<i64, Page>,
}

impl Espace {
    pub fn page(&self, id: i64) -> Option<&Page> {
        self.pages.get(&id)
    }

    pub fn page_mut(&mut self, id: i64) -> Result<&mut Page, String> {
        self.pages.get_mut(&id).ok_or_else(|| format!("page {id} introuvable"))
    }

    /// Les sous-pages de `parent` (None : les racines), dans l'ordre.
    pub fn enfants(&self, parent: Option<i64>) -> Vec<&Page> {
        let mut v: Vec<&Page> = self.pages.values().filter(|p| p.parent == parent).collect();
        v.sort_by_key(|p| (p.ordre, p.id));
        v
    }

    /// Le chemin depuis la racine jusqu'a `id` compris.
    pub fn chemin(&self, id: i64) -> Vec<&Page> {
        let mut v = Vec::new();
        let mut c = self.page(id);
        while let Some(p) = c {
            if v.iter().any(|q: &&Page| q.id == p.id) {
                break;
            }
            v.push(p);
            c = p.parent.and_then(|x| self.page(x));
        }
        v.reverse();
        v
    }

    /// `id` et tous ses descendants.
    pub fn descendants(&self, id: i64) -> Vec<i64> {
        let mut v = vec![id];
        let mut i = 0;
        while i < v.len() {
            let p = v[i];
            v.extend(self.pages.values().filter(|x| x.parent == Some(p) && !v.contains(&x.id)).map(|x| x.id).collect::<Vec<_>>());
            i += 1;
        }
        v
    }

    fn nouvel_id(&self) -> i64 {
        self.pages.keys().max().copied().unwrap_or(0) + 1
    }

    /// Nouvelle page a la fin de `parent` ; rend son id. Dans une base, la
    /// ligne recoit la 1re option des selections de la vue kanban si
    /// `valeurs` ne dit rien.
    pub fn creer(&mut self, parent: Option<i64>, titre: &str, modifie: u64) -> Result<i64, String> {
        if let Some(p) = parent {
            self.page(p).ok_or_else(|| format!("page {p} introuvable"))?;
        }
        let id = self.nouvel_id();
        let ordre = self.enfants(parent).last().map(|p| p.ordre + 1).unwrap_or(0);
        self.pages.insert(id, Page { id, parent, ordre, titre: titre.to_string(), modifie, ..Page::default() });
        Ok(id)
    }

    /// Nouvelle base de donnees (avec un schema et une vue table de depart).
    pub fn creer_base(&mut self, parent: Option<i64>, titre: &str, modifie: u64) -> Result<i64, String> {
        let id = self.creer(parent, titre, modifie)?;
        let p = self.page_mut(id)?;
        p.base = true;
        p.schema = vec![Definition { nom: "Statut".into(), genre: Genre::Selection(vec!["À faire".into(), "En cours".into(), "Fait".into()]) }, Definition { nom: "Étiquettes".into(), genre: Genre::Etiquettes(Vec::new()) }];
        p.vues = vec![Vue::nouvelle(Affichage::Table), Vue { groupe: "statut".into(), ..Vue::nouvelle(Affichage::Kanban) }];
        Ok(id)
    }

    /// Supprime `id` et ses descendants ; rend les ids supprimes.
    pub fn supprimer(&mut self, id: i64) -> Vec<i64> {
        if self.page(id).is_none() {
            return Vec::new();
        }
        let ids = self.descendants(id);
        for i in &ids {
            self.pages.remove(i);
        }
        // Les blocs qui pointaient vers elles disparaissent aussi.
        for p in self.pages.values_mut() {
            p.blocs.retain(|b| !matches!(b.genre, GenreBloc::SousPage | GenreBloc::Vue) || !ids.iter().any(|i| cible(b) == Some(*i)));
        }
        ids
    }

    /// Range `id` sous `parent`, a la position `position` parmi ses freres.
    /// Refuse de mettre une page dans sa propre descendance.
    pub fn deplacer(&mut self, id: i64, parent: Option<i64>, position: usize) -> Result<(), String> {
        self.page(id).ok_or_else(|| format!("page {id} introuvable"))?;
        if let Some(p) = parent {
            if self.descendants(id).contains(&p) {
                return Err("impossible de ranger une page dans sa propre sous-page".into());
            }
            self.page(p).ok_or_else(|| format!("page {p} introuvable"))?;
        }
        let mut freres: Vec<i64> = self.enfants(parent).iter().map(|p| p.id).filter(|x| *x != id).collect();
        freres.insert(position.min(freres.len()), id);
        self.page_mut(id)?.parent = parent;
        for (i, f) in freres.into_iter().enumerate() {
            self.page_mut(f)?.ordre = i as i64;
        }
        Ok(())
    }

    /// Les definitions de proprietes de `id` : celles de sa base (si son
    /// parent en est une) puis les siennes.
    pub fn definitions(&self, id: i64) -> Vec<Definition> {
        let Some(p) = self.page(id) else { return Vec::new() };
        let mut v: Vec<Definition> = p.parent.and_then(|x| self.page(x)).filter(|b| b.base).map(|b| b.schema.clone()).unwrap_or_default();
        for d in &p.propres {
            if !v.iter().any(|x| x.cle() == d.cle()) {
                v.push(d.clone());
            }
        }
        v
    }

    pub fn definition(&self, id: i64, nom: &str) -> Option<Definition> {
        let cle = normaliser(nom);
        self.definitions(id).into_iter().find(|d| d.cle() == cle)
    }

    /// Change la valeur brute de `nom` sur `id` (verifiee selon le genre).
    /// Les formules ne se remplissent pas.
    pub fn changer(&mut self, id: i64, nom: &str, brute: &str) -> Result<(), String> {
        let d = self.definition(id, nom).ok_or_else(|| format!("propriété inconnue : {nom}"))?;
        let brute = brute.trim();
        let propre = match &d.genre {
            Genre::Formule(_) => return Err(format!("{} est calculée par sa formule", d.nom)),
            Genre::Nombre if !brute.is_empty() => {
                let n: f64 = brute.replace(',', ".").parse().map_err(|_| format!("{} : « {brute} » n'est pas un nombre", d.nom))?;
                Valeur::Nombre(n).to_string()
            }
            Genre::Case => if matches!(brute, "1" | "true" | "vrai" | "oui") { "1" } else { "" }.to_string(),
            Genre::Date if !brute.is_empty() => {
                date_valide(brute).ok_or_else(|| format!("{} : date attendue au format AAAA-MM-JJ", d.nom))?;
                brute.to_string()
            }
            Genre::Etiquettes(_) => {
                let mut tags: Vec<String> = Vec::new();
                for t in brute.split([',', SEP]).map(str::trim).filter(|t| !t.is_empty()) {
                    if !tags.iter().any(|x| x.eq_ignore_ascii_case(t)) {
                        tags.push(t.to_string());
                    }
                }
                self.ajouter_options(id, &d, &tags)?;
                tags.join(&SEP.to_string())
            }
            Genre::Selection(_) if !brute.is_empty() => {
                self.ajouter_options(id, &d, &[brute.to_string()])?;
                brute.to_string()
            }
            Genre::Relation => brute.split([',', SEP]).filter_map(|x| x.trim().parse::<i64>().ok()).filter(|x| self.pages.contains_key(x)).map(|x| x.to_string()).collect::<Vec<_>>().join(","),
            _ => brute.to_string(),
        };
        let p = self.page_mut(id)?;
        if propre.is_empty() {
            p.valeurs.remove(&d.cle());
        } else {
            p.valeurs.insert(d.cle(), propre);
        }
        Ok(())
    }

    /// Une option tapee qui n'existe pas encore rejoint la liste (dans le
    /// schema de la base, ou dans les proprietes de la page).
    fn ajouter_options(&mut self, id: i64, d: &Definition, nouvelles: &[String]) -> Result<(), String> {
        let parent = self.page(id).and_then(|p| p.parent);
        let base = parent.filter(|x| self.page(*x).is_some_and(|b| b.base && b.schema.iter().any(|s| s.cle() == d.cle())));
        let defs = match base {
            Some(b) => &mut self.page_mut(b)?.schema,
            None => &mut self.page_mut(id)?.propres,
        };
        if let Some(def) = defs.iter_mut().find(|x| x.cle() == d.cle()) {
            if let Genre::Selection(o) | Genre::Etiquettes(o) = &mut def.genre {
                for n in nouvelles {
                    if !o.iter().any(|x| x.eq_ignore_ascii_case(n)) {
                        o.push(n.clone());
                    }
                }
            }
        }
        Ok(())
    }

    /// Ajoute (ou remplace, meme nom) une definition. `dans_schema` : sur la
    /// base `id`, pour toutes ses lignes ; sinon, propre a la page. Une
    /// formule invalide ou qui fait un cycle est refusee.
    pub fn definir(&mut self, id: i64, def: Definition, dans_schema: bool) -> Result<(), String> {
        if def.nom.trim().is_empty() {
            return Err("nom de propriété vide".into());
        }
        if def.cle() == "titre" || def.cle() == "enfants" {
            return Err(format!("« {} » est réservé", def.nom));
        }
        // Une formule vide (pas encore tapee) vaut vide.
        if let Genre::Formule(src) = &def.genre
            && !src.trim().is_empty()
        {
            Formule::analyser(src)?;
        }
        let ancien = self.clone();
        let p = self.page_mut(id)?;
        if dans_schema && !p.base {
            return Err("seule une base a un schéma".into());
        }
        let defs = if dans_schema { &mut p.schema } else { &mut p.propres };
        match defs.iter_mut().find(|d| d.cle() == def.cle()) {
            Some(d) => *d = def.clone(),
            None => defs.push(def.clone()),
        }
        // Un cycle se voit en calculant : on essaie sur les pages touchees.
        let touchees: Vec<i64> = if dans_schema { self.enfants(Some(id)).iter().map(|p| p.id).collect() } else { vec![id] };
        for t in touchees {
            if let Err(e) = self.valeur(t, &def.nom) {
                if e.starts_with("cycle") {
                    *self = ancien;
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// Change la definition `cle` (nom, options, formule) ; les valeurs
    /// suivent un changement de nom.
    pub fn renommer_definition(&mut self, id: i64, cle: &str, def: Definition, dans_schema: bool) -> Result<(), String> {
        if def.cle() == cle {
            return self.definir(id, def, dans_schema);
        }
        let lignes: Vec<i64> = if dans_schema { self.enfants(Some(id)).iter().map(|p| p.id).collect() } else { vec![id] };
        {
            let p = self.page(id).ok_or("page introuvable")?;
            let defs = if dans_schema { &p.schema } else { &p.propres };
            if defs.iter().any(|d| d.cle() == def.cle()) {
                return Err(format!("« {} » existe déjà", def.nom));
            }
        }
        let ancien = self.clone();
        let p = self.page_mut(id)?;
        let defs = if dans_schema { &mut p.schema } else { &mut p.propres };
        let place = defs.iter().position(|d| d.cle() == cle).ok_or("propriété introuvable")?;
        defs[place] = def.clone();
        for l in lignes {
            let page = self.page_mut(l)?;
            if let Some(v) = page.valeurs.remove(cle) {
                page.valeurs.insert(def.cle(), v);
            }
        }
        // Meme verifications qu'a la creation (formule valide, pas de cycle).
        if let Err(e) = self.definir(id, def, dans_schema) {
            *self = ancien;
            return Err(e);
        }
        Ok(())
    }

    /// Retire une definition (et les valeurs rangees sous ce nom).
    pub fn retirer_definition(&mut self, id: i64, nom: &str, dans_schema: bool) -> Result<(), String> {
        let cle = normaliser(nom);
        let lignes: Vec<i64> = if dans_schema { self.enfants(Some(id)).iter().map(|p| p.id).collect() } else { vec![id] };
        let p = self.page_mut(id)?;
        let defs = if dans_schema { &mut p.schema } else { &mut p.propres };
        defs.retain(|d| d.cle() != cle);
        for l in lignes {
            self.page_mut(l)?.valeurs.remove(&cle);
        }
        Ok(())
    }

    /// La valeur de la propriete `nom` de `id` (calculee si c'est une formule).
    pub fn valeur(&self, id: i64, nom: &str) -> Result<Valeur, String> {
        let pile = RefCell::new(Vec::new());
        let ctx = Ctx { espace: self, id, pile: &pile };
        ctx.propriete(&normaliser(nom))?.ok_or_else(|| format!("propriété inconnue : {nom}"))
    }

    /// Le texte a afficher pour `nom` sur `id` (« ⚠ ... » si la formule echoue).
    pub fn affichee(&self, id: i64, nom: &str) -> String {
        match self.valeur(id, nom) {
            Ok(Valeur::Booleen(b)) => if b { "✓" } else { "" }.into(),
            Ok(v) => v.to_string(),
            Err(e) => format!("⚠ {e}"),
        }
    }

    /// Les options (etiquettes ou selection) de la valeur de `nom`.
    pub fn options(&self, id: i64, nom: &str) -> Vec<String> {
        match self.valeur(id, nom) {
            Ok(Valeur::Liste(l)) => l.iter().map(|v| v.to_string()).collect(),
            Ok(Valeur::Vide) | Err(_) => Vec::new(),
            Ok(v) => vec![v.to_string()],
        }
    }

    /// Les lignes de la base `base` que montre `vue` (filtrees, triees).
    pub fn lignes(&self, base: i64, vue: &Vue) -> Vec<i64> {
        let mut v: Vec<i64> = self.enfants(Some(base)).iter().map(|p| p.id).filter(|id| vue.filtres.iter().all(|f| self.passe(*id, f))).collect();
        if let Some((prop, desc)) = &vue.tri {
            let cle = |id: &i64| self.valeur(*id, prop).unwrap_or(Valeur::Vide);
            // Le vide reste en dernier, meme en ordre decroissant.
            v.sort_by(|a, b| {
                let (x, y) = (cle(a), cle(b));
                match (x == Valeur::Vide, y == Valeur::Vide) {
                    (false, false) if *desc => comparer(&x, &y).reverse(),
                    _ => comparer(&x, &y),
                }
            });
        }
        v
    }

    fn passe(&self, id: i64, f: &Filtre) -> bool {
        let v = self.valeur(id, &f.propriete).unwrap_or(Valeur::Vide);
        let vide = match &v {
            Valeur::Vide => true,
            Valeur::Texte(t) => t.is_empty(),
            Valeur::Liste(l) => l.is_empty(),
            Valeur::Booleen(b) => !b,
            _ => false,
        };
        let cible = Valeur::Texte(f.valeur.clone());
        let egal = |x: &Valeur| comparer(x, &cible) == std::cmp::Ordering::Equal;
        match f.operateur {
            Operateur::Vide => vide,
            Operateur::NonVide => !vide,
            Operateur::Egal => match &v {
                Valeur::Liste(l) => l.iter().any(egal),
                Valeur::Booleen(b) => *b == matches!(f.valeur.as_str(), "1" | "vrai" | "oui" | "true"),
                _ => egal(&v),
            },
            Operateur::Different => match &v {
                Valeur::Liste(l) => !l.iter().any(egal),
                _ => !egal(&v),
            },
            Operateur::Contient => v.to_string().to_lowercase().contains(&f.valeur.to_lowercase()),
            Operateur::Plus => !vide && comparer(&v, &cible) == std::cmp::Ordering::Greater,
            Operateur::Moins => !vide && comparer(&v, &cible) == std::cmp::Ordering::Less,
        }
    }

    /// Les colonnes d'un kanban : chaque option de `groupe` (puis « Sans
    /// valeur ») et ses lignes. Une ligne a plusieurs etiquettes apparait
    /// dans chacune.
    pub fn kanban(&self, base: i64, vue: &Vue) -> Vec<(String, Vec<i64>)> {
        let options = match self.page(base).and_then(|b| b.schema.iter().find(|d| d.cle() == normaliser(&vue.groupe)).cloned()) {
            Some(Definition { genre: Genre::Selection(o) | Genre::Etiquettes(o), .. }) => o,
            _ => Vec::new(),
        };
        let mut cols: Vec<(String, Vec<i64>)> = options.iter().map(|o| (o.clone(), Vec::new())).collect();
        let mut sans = Vec::new();
        for id in self.lignes(base, vue) {
            let les_siennes = self.options(id, &vue.groupe);
            let mut placee = false;
            for (o, ids) in cols.iter_mut() {
                if les_siennes.iter().any(|x| x.eq_ignore_ascii_case(o)) {
                    ids.push(id);
                    placee = true;
                }
            }
            if !placee {
                sans.push(id);
            }
        }
        cols.push(("Sans valeur".into(), sans));
        cols
    }

    /// Glisser une carte de kanban : `id` passe de la colonne `de` a la
    /// colonne `vers` (« Sans valeur » vide la propriete), a la position
    /// `position` parmi les lignes de la base.
    pub fn glisser_carte(&mut self, id: i64, groupe: &str, de: &str, vers: &str, position: Option<usize>) -> Result<(), String> {
        let d = self.definition(id, groupe).ok_or_else(|| format!("propriété inconnue : {groupe}"))?;
        let vers = if vers == "Sans valeur" { "" } else { vers };
        let brute = match d.genre {
            Genre::Etiquettes(_) => {
                let mut tags = self.options(id, groupe);
                tags.retain(|t| !t.eq_ignore_ascii_case(de));
                if !vers.is_empty() && !tags.iter().any(|t| t.eq_ignore_ascii_case(vers)) {
                    tags.push(vers.to_string());
                }
                tags.join(",")
            }
            _ => vers.to_string(),
        };
        self.changer(id, groupe, &brute)?;
        if let (Some(pos), Some(parent)) = (position, self.page(id).and_then(|p| p.parent)) {
            self.deplacer(id, Some(parent), pos)?;
        }
        Ok(())
    }
}

/// L'id vise par un bloc sous-page / vue.
pub fn cible(b: &Bloc) -> Option<i64> {
    b.contenu.split('/').next()?.trim().parse().ok()
}

/// AAAA-MM-JJ plausible.
pub fn date_valide(s: &str) -> Option<(i32, u32, u32)> {
    let p: Vec<&str> = s.split('-').collect();
    if p.len() != 3 || p[0].len() != 4 {
        return None;
    }
    let (a, m, j) = (p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?);
    ((1..=12).contains(&m) && (1..=31).contains(&j)).then_some((a, m, j))
}

/// Ordre pour trier et filtrer : nombres entre eux, sinon texte sans casse ;
/// le vide en dernier.
pub fn comparer(a: &Valeur, b: &Valeur) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Valeur::Vide, Valeur::Vide) => Ordering::Equal,
        (Valeur::Vide, _) => Ordering::Greater,
        (_, Valeur::Vide) => Ordering::Less,
        _ => match (a.nombre(), b.nombre()) {
            (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
            _ => a.to_string().to_lowercase().cmp(&b.to_string().to_lowercase()),
        },
    }
}

/// Une page vue par le moteur de formules. `pile` : les (page, propriete) en
/// cours de calcul, pour voir les cycles.
struct Ctx<'a> {
    espace: &'a Espace,
    id: i64,
    pile: &'a RefCell<Vec<(i64, String)>>,
}

impl Contexte for Ctx<'_> {
    fn propriete(&self, nom: &str) -> Result<Option<Valeur>, String> {
        let Some(page) = self.espace.page(self.id) else { return Ok(None) };
        if nom == "titre" {
            return Ok(Some(Valeur::Texte(page.nom().to_string())));
        }
        let Some(def) = self.espace.definitions(self.id).into_iter().find(|d| d.cle() == nom) else { return Ok(None) };
        let brute = page.valeurs.get(nom).cloned().unwrap_or_default();
        Ok(Some(match &def.genre {
            Genre::Formule(src) => {
                let cle = (self.id, nom.to_string());
                if self.pile.borrow().contains(&cle) {
                    let mut noms: Vec<String> = self.pile.borrow().iter().skip_while(|x| **x != cle).map(|(i, n)| format!("{}.{n}", self.espace.page(*i).map(|p| p.nom()).unwrap_or("?"))).collect();
                    noms.push(format!("{}.{nom}", page.nom()));
                    return Err(format!("cycle : {}", noms.join(" › ")));
                }
                self.pile.borrow_mut().push(cle);
                let r = if src.trim().is_empty() { Ok(Valeur::Vide) } else { Formule::analyser(src).and_then(|f| f.evaluer(self)) };
                self.pile.borrow_mut().pop();
                r?
            }
            _ if brute.is_empty() => match def.genre {
                Genre::Case => Valeur::Booleen(false),
                Genre::Etiquettes(_) | Genre::Relation => Valeur::Liste(Vec::new()),
                _ => Valeur::Vide,
            },
            Genre::Nombre => brute.parse().map(Valeur::Nombre).unwrap_or(Valeur::Vide),
            Genre::Case => Valeur::Booleen(brute == "1"),
            Genre::Etiquettes(_) => Valeur::Liste(brute.split(SEP).map(|t| Valeur::Texte(t.to_string())).collect()),
            Genre::Relation => Valeur::Liste(brute.split(',').filter_map(|x| x.parse::<i64>().ok()).filter_map(|x| self.espace.page(x)).map(|p| Valeur::Texte(p.nom().to_string())).collect()),
            _ => Valeur::Texte(brute),
        }))
    }

    fn enfants(&self) -> Vec<Box<dyn Contexte + '_>> {
        self.espace.enfants(Some(self.id)).iter().map(|p| Box::new(Ctx { espace: self.espace, id: p.id, pile: self.pile }) as Box<dyn Contexte + '_>).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(nom: &str, genre: Genre) -> Definition {
        Definition { nom: nom.into(), genre }
    }

    /// Projet (base) > 3 taches ; la 1re a 2 sous-taches.
    fn exemple() -> (Espace, i64, [i64; 3], [i64; 2]) {
        let mut e = Espace::default();
        let projet = e.creer_base(None, "Projet", 0).unwrap();
        e.definir(projet, def("Avancement", Genre::Nombre), true).unwrap();
        let t: Vec<i64> = ["A", "B", "C"].iter().map(|n| e.creer(Some(projet), n, 0).unwrap()).collect();
        e.changer(t[1], "avancement", "50").unwrap();
        e.changer(t[2], "Avancement", "0").unwrap();
        e.changer(t[1], "statut", "En cours").unwrap();
        // La tache A calcule son avancement depuis ses sous-taches.
        e.definir(t[0], def("Fini", Genre::Formule("moyenne(enfants.fait) * 100".into())), false).unwrap();
        let s: Vec<i64> = ["a1", "a2"].iter().map(|n| e.creer(Some(t[0]), n, 0).unwrap()).collect();
        for x in &s {
            e.definir(*x, def("Fait", Genre::Case), false).unwrap();
        }
        (e, projet, [t[0], t[1], t[2]], [s[0], s[1]])
    }

    #[test]
    fn formule_remonte_la_hierarchie() {
        let (mut e, projet, t, s) = exemple();
        e.definir(projet, def("Total", Genre::Formule("moyenne(enfants, si(fini != vide, fini, avancement))".into())), false).unwrap();
        // A : 0 % (aucune sous-tache faite), B 50, C 0.
        assert_eq!(e.affichee(t[0], "fini"), "0");
        assert_eq!(e.affichee(projet, "total"), "16.67");
        e.changer(s[0], "fait", "1").unwrap();
        assert_eq!(e.affichee(t[0], "fini"), "50");
        assert_eq!(e.affichee(projet, "total"), "33.33");
        e.changer(s[1], "fait", "oui").unwrap();
        assert_eq!(e.affichee(projet, "total"), "50");
    }

    #[test]
    fn cycle_refuse() {
        let (mut e, projet, _, _) = exemple();
        e.definir(projet, def("a", Genre::Formule("1".into())), false).unwrap();
        e.definir(projet, def("b", Genre::Formule("a + 1".into())), false).unwrap();
        let err = e.definir(projet, def("a", Genre::Formule("b * 2".into())), false).unwrap_err();
        assert!(err.starts_with("cycle"), "{err}");
        // L'ancienne formule est gardee.
        assert_eq!(e.affichee(projet, "b"), "2");
    }

    #[test]
    fn valeurs_verifiees() {
        let (mut e, projet, t, _) = exemple();
        assert!(e.changer(t[0], "avancement", "beaucoup").is_err());
        assert!(e.changer(t[0], "fini", "3").is_err());
        e.changer(t[0], "avancement", "12,5").unwrap();
        assert_eq!(e.valeur(t[0], "avancement").unwrap(), Valeur::Nombre(12.5));
        // Une etiquette nouvelle rejoint le schema de la base.
        e.changer(t[0], "étiquettes", "urgent, client, Urgent").unwrap();
        assert_eq!(e.options(t[0], "étiquettes"), vec!["urgent", "client"]);
        match &e.page(projet).unwrap().schema[1].genre {
            Genre::Etiquettes(o) => assert_eq!(o, &vec!["urgent".to_string(), "client".to_string()]),
            g => panic!("{g:?}"),
        }
        assert!(e.changer(t[0], "inconnue", "x").is_err());
    }

    #[test]
    fn vues_filtres_tri_kanban() {
        let (mut e, projet, t, _) = exemple();
        let mut vue = Vue::nouvelle(Affichage::Table);
        vue.tri = Some(("avancement".into(), true));
        assert_eq!(e.lignes(projet, &vue), vec![t[1], t[2], t[0]]);
        vue.filtres.push(Filtre { propriete: "statut".into(), operateur: Operateur::Egal, valeur: "en cours".into() });
        assert_eq!(e.lignes(projet, &vue), vec![t[1]]);

        let k = Vue { groupe: "statut".into(), ..Vue::nouvelle(Affichage::Kanban) };
        let cols = e.kanban(projet, &k);
        assert_eq!(cols.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(), vec!["À faire", "En cours", "Fait", "Sans valeur"]);
        assert_eq!(cols[1].1, vec![t[1]]);
        assert_eq!(cols[3].1, vec![t[0], t[2]]);
        e.glisser_carte(t[1], "statut", "En cours", "Fait", Some(0)).unwrap();
        let cols = e.kanban(projet, &k);
        assert_eq!(cols[2].1, vec![t[1]]);
        assert_eq!(e.enfants(Some(projet))[0].id, t[1]);
        assert_eq!(Vue::lire(&vue.ecrire()), vue);
    }

    #[test]
    fn deplacer_et_supprimer() {
        let (mut e, projet, t, s) = exemple();
        assert!(e.deplacer(t[0], Some(s[0]), 0).is_err());
        e.deplacer(s[1], None, 0).unwrap();
        assert_eq!(e.enfants(None).iter().map(|p| p.id).collect::<Vec<_>>(), vec![s[1], projet]);
        e.page_mut(projet).unwrap().blocs.push(Bloc { id: 1, genre: GenreBloc::SousPage, contenu: t[0].to_string() });
        let partis = e.supprimer(t[0]);
        assert_eq!(partis.len(), 2);
        assert!(e.page(s[0]).is_none());
        assert!(e.page(projet).unwrap().blocs.is_empty());
        assert_eq!(e.chemin(t[1]).iter().map(|p| p.nom()).collect::<Vec<_>>(), vec!["Projet", "B"]);
    }
}
