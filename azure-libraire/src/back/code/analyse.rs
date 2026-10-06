// Analyser un projet : parcourir ses dossiers, lire chaque fichier source,
// puis resoudre les appels (quelle fonction de quel fichier) et les imports
// (quel fichier). Rust : les crates (Cargo.toml), les modules (chemin du
// fichier), les `use` (re-exports suivis), les `impl`. JS : les chemins
// relatifs des imports.
//
// Une resolution prudente : un appel qu'on ne sait pas attribuer surement
// (une methode `.len()` qui pourrait etre celle de Vec) n'est pas relie.
use super::{js, rust, Appel, Fichier, Import, Langage};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Dossiers jamais lus.
const IGNORES: [&str; 12] = ["target", "node_modules", "dist", "build", "out", "vendor", "__pycache__", "coverage", "venv", "env", "bin_cache", "obj"];
const MAX_FICHIERS: usize = 6000;
const MAX_OCTETS: u64 = 1_500_000;

/// Une fonction : (fichier, fonction).
pub type RefFn = (usize, usize);

#[derive(Clone, Debug, Default)]
pub struct Projet {
    pub racine: String,
    pub nom: String,
    pub fichiers: Vec<Fichier>,
    /// Appels resolus : (de, vers) -> nombre d'appels.
    pub appels: BTreeMap<(RefFn, RefFn), usize>,
    /// Imports resolus : (fichier, fichier) -> nombre de noms importes.
    pub imports: BTreeMap<(usize, usize), usize>,
    /// Appels vers une fonction du projet qu'on n'a pas su attribuer.
    pub ambigus: usize,
    /// Fichiers sautes (trop gros, illisibles, au-dela de la limite).
    pub sautes: usize,
}

impl Projet {
    pub fn fichier(&self, chemin: &str) -> Option<usize> {
        self.fichiers.iter().position(|f| f.chemin == chemin)
    }

    pub fn nb_lignes(&self) -> usize {
        self.fichiers.iter().map(|f| f.lignes).sum()
    }

    pub fn nb_fonctions(&self) -> usize {
        self.fichiers.iter().map(|f| f.fonctions.len()).sum()
    }

    pub fn langages(&self) -> Vec<(Langage, usize)> {
        let mut out: Vec<(Langage, usize)> = Vec::new();
        for f in &self.fichiers {
            match out.iter_mut().find(|x| x.0 == f.langage) {
                Some(x) => x.1 += 1,
                None => out.push((f.langage, 1)),
            }
        }
        out
    }

    /// Le meme projet sans ses fichiers de test (indices renumerotes) : la
    /// carte du code sans le banc de test.
    pub fn sans_tests(&self) -> Projet {
        let mut nouveau = vec![None; self.fichiers.len()];
        let mut fichiers = Vec::new();
        for (i, f) in self.fichiers.iter().enumerate() {
            if !f.test {
                nouveau[i] = Some(fichiers.len());
                fichiers.push(f.clone());
            }
        }
        let appels = self.appels.iter().filter_map(|(((a, ja), (b, jb)), n)| Some((((nouveau[*a]?, *ja), (nouveau[*b]?, *jb)), *n))).collect();
        let imports = self.imports.iter().filter_map(|((a, b), n)| Some(((nouveau[*a]?, nouveau[*b]?), *n))).collect();
        Projet { racine: self.racine.clone(), nom: self.nom.clone(), fichiers, appels, imports, ambigus: self.ambigus, sautes: self.sautes }
    }

    /// Les fonctions atteintes par les tests (appelees par une fonction d'un
    /// fichier de test, directement ou de proche en proche), hors tests.
    pub fn atteintes_par_les_tests(&self) -> (std::collections::HashSet<RefFn>, std::collections::HashSet<RefFn>) {
        let mut directes = std::collections::HashSet::new();
        let mut suivants: HashMap<RefFn, Vec<RefFn>> = HashMap::new();
        for (a, b) in self.appels.keys() {
            suivants.entry(*a).or_default().push(*b);
            if self.fichiers[a.0].test && !self.fichiers[b.0].test {
                directes.insert(*b);
            }
        }
        let mut toutes = directes.clone();
        let mut pile: Vec<RefFn> = directes.iter().copied().collect();
        while let Some(f) = pile.pop() {
            for s in suivants.get(&f).into_iter().flatten() {
                if !self.fichiers[s.0].test && toutes.insert(*s) {
                    pile.push(*s);
                }
            }
        }
        (directes, toutes)
    }

    /// Les liens entre fichiers : appels + noms importes, hors d'un fichier
    /// vers lui-meme.
    pub fn liens_fichiers(&self) -> BTreeMap<(usize, usize), usize> {
        let mut out = BTreeMap::new();
        for (((a, _), (b, _)), n) in &self.appels {
            if a != b {
                *out.entry((*a, *b)).or_insert(0) += n;
            }
        }
        for ((a, b), n) in &self.imports {
            if a != b {
                *out.entry((*a, *b)).or_insert(0) += n;
            }
        }
        out
    }
}

/// Lit le projet du dossier `racine`.
pub fn analyser(racine: &Path) -> Result<Projet, String> {
    if !racine.is_dir() {
        return Err(format!("{} n'est pas un dossier.", racine.display()));
    }
    let mut chemins = Vec::new();
    let mut cargos = Vec::new();
    let mut sautes = 0;
    parcourir(racine, racine, &mut chemins, &mut cargos, &mut sautes);
    chemins.sort_by(|a, b| a.0.cmp(&b.0));
    if chemins.len() > MAX_FICHIERS {
        sautes += chemins.len() - MAX_FICHIERS;
        chemins.truncate(MAX_FICHIERS);
    }
    let mut fichiers = Vec::new();
    for (rel, abs, langage) in chemins {
        match std::fs::read_to_string(&abs) {
            Ok(source) => fichiers.push(match langage {
                Langage::Rust => rust::lire(&rel, &source),
                Langage::Js => js::lire(&rel, &source),
            }),
            Err(_) => sautes += 1,
        }
    }
    if fichiers.is_empty() {
        return Err(format!("Aucun fichier source (Rust, JavaScript, TypeScript) dans {}.", racine.display()));
    }
    let nom = racine.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| racine.display().to_string());
    let mut p = Projet { racine: racine.display().to_string(), nom, fichiers, sautes, ..Projet::default() };
    resoudre(&mut p, &cargos);
    Ok(p)
}

fn parcourir(racine: &Path, dossier: &Path, out: &mut Vec<(String, PathBuf, Langage)>, cargos: &mut Vec<Crate>, sautes: &mut usize) {
    let Ok(entrees) = std::fs::read_dir(dossier) else { return };
    for e in entrees.flatten() {
        let nom = e.file_name().to_string_lossy().into_owned();
        let chemin = e.path();
        let Ok(genre) = e.file_type() else { continue };
        if genre.is_symlink() {
            continue;
        }
        if genre.is_dir() {
            if nom.starts_with('.') || IGNORES.contains(&nom.as_str()) {
                continue;
            }
            parcourir(racine, &chemin, out, cargos, sautes);
        } else if nom == "Cargo.toml" {
            if let Ok(texte) = std::fs::read_to_string(&chemin)
                && let Some(n) = nom_de_crate(&texte)
            {
                cargos.push(Crate { dossier: relatif(racine, dossier), nom: n, dependances: dependances(&texte) });
            }
        } else if let Some(l) = chemin.extension().and_then(|x| x.to_str()).and_then(Langage::depuis_extension) {
            // Les fichiers minifies (.min.js) et les declarations (.d.ts) : sautes.
            if nom.ends_with(".min.js") || nom.ends_with(".d.ts") {
                continue;
            }
            if e.metadata().map(|m| m.len()).unwrap_or(0) > MAX_OCTETS {
                *sautes += 1;
                continue;
            }
            out.push((relatif(racine, &chemin), chemin, l));
        }
    }
}

fn relatif(racine: &Path, p: &Path) -> String {
    p.strip_prefix(racine).unwrap_or(p).components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// Une crate du projet (un Cargo.toml).
#[derive(Clone, Debug)]
pub struct Crate {
    pub dossier: String,
    pub nom: String,
    /// Les crates dont elle depend (dev et build compris), avec des `_`.
    pub dependances: Vec<String>,
}

/// Les dependances d'un Cargo.toml : `x = ...` sous [dependencies],
/// [dev-dependencies], [build-dependencies] (et `[dependencies.x]`).
fn dependances(toml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut dans = false;
    for l in toml.lines() {
        let l = l.trim();
        if l.starts_with('[') {
            let titre = l.trim_matches(|c| c == '[' || c == ']');
            let titre = titre.rsplit_once("target.").map_or(titre, |_| titre.rsplit('.').next().unwrap_or(titre));
            dans = titre.ends_with("dependencies");
            for prefixe in ["dependencies.", "dev-dependencies.", "build-dependencies."] {
                if let Some(nom) = l.trim_matches(|c| c == '[' || c == ']').strip_prefix(prefixe) {
                    out.push(nom.trim().replace('-', "_"));
                }
            }
            continue;
        }
        if dans && let Some((nom, _)) = l.split_once('=') {
            let nom = nom.trim().trim_matches('"');
            if !nom.is_empty() && !nom.starts_with('#') {
                out.push(nom.replace('-', "_"));
            }
        }
    }
    out
}

/// Le nom de la crate d'un Cargo.toml ([lib] name, sinon [package] name),
/// avec des `_`.
fn nom_de_crate(toml: &str) -> Option<String> {
    let mut section = "";
    let (mut paquet, mut lib) = (None, None);
    for l in toml.lines() {
        let l = l.trim();
        if l.starts_with('[') {
            section = if l == "[package]" { "package" } else if l == "[lib]" { "lib" } else { "" };
            continue;
        }
        if let Some(v) = l.strip_prefix("name").map(str::trim_start).and_then(|r| r.strip_prefix('=')) {
            let v = v.trim().trim_matches('"').replace('-', "_");
            match section {
                "package" => paquet = Some(v),
                "lib" => lib = Some(v),
                _ => {}
            }
        }
    }
    lib.or(paquet)
}

/// Ou est un fichier Rust : sa crate (cle) et son module.
#[derive(Clone, Debug, PartialEq)]
struct Place {
    cle: String,
    module: Vec<String>,
}

/// La crate (la plus profonde) qui contient un fichier.
fn crate_de<'a>(chemin: &str, crates: &'a [Crate]) -> Option<&'a Crate> {
    crates.iter().filter(|c| c.dossier.is_empty() || chemin.starts_with(&format!("{}/", c.dossier))).max_by_key(|c| c.dossier.len())
}

fn place_rust(chemin: &str, crates: &[Crate]) -> Place {
    let Some(Crate { dossier, nom, .. }) = crate_de(chemin, crates) else {
        return Place { cle: chemin.to_string(), module: Vec::new() };
    };
    let rel = if dossier.is_empty() { chemin } else { &chemin[dossier.len() + 1..] };
    let sans_ext = rel.trim_end_matches(".rs");
    let segments: Vec<&str> = sans_ext.split('/').collect();
    match segments.as_slice() {
        ["src", "lib"] | ["src", "main"] => Place { cle: nom.clone(), module: Vec::new() },
        // Binaires, tests, exemples : chacun sa racine.
        ["src", "bin", ..] | ["tests", ..] | ["examples", ..] | ["benches", ..] => Place { cle: format!("{dossier}/{rel}"), module: Vec::new() },
        ["src", reste @ ..] => {
            let mut m: Vec<String> = reste.iter().map(|s| s.to_string()).collect();
            if m.last().is_some_and(|d| d == "mod") {
                m.pop();
            }
            Place { cle: nom.clone(), module: m }
        }
        _ => Place { cle: chemin.to_string(), module: Vec::new() },
    }
}

/// Ce qu'un nom local designe dans un fichier.
#[derive(Clone, Debug)]
struct Cible {
    fichier: usize,
    /// None : le nom designe le module (le fichier) lui-meme.
    item: Option<String>,
}

/// Les noms communs des types de la bibliotheque standard : une methode du
/// projet qui porte ce nom n'est reliee que si on sait que c'est la sienne.
const COMMUNS: &[&str] = &[
    "new", "get", "get_mut", "set", "len", "push", "pop", "iter", "iter_mut", "map", "clone", "insert", "remove", "contains", "is_empty", "unwrap", "from", "into", "to_string", "as_str", "next", "default", "fmt", "eq", "write", "read", "find", "filter", "join", "split", "parse", "run", "start", "stop", "draw", "update", "open", "close", "send", "recv", "lock", "value", "text", "clear", "extend", "take", "replace", "apply", "build", "add", "sub", "first", "last", "keys", "values", "entry", "flush", "drop", "hash", "cmp", "partial_cmp", "as_ref", "borrow", "deref", "load", "store", "save", "name", "id", "kind", "size", "width", "height", "then", "catch", "forEach", "toString", "push_str", "lines", "chars", "trim", "contains_key", "with_capacity", "copy", "call", "bind", "test", "render",
    "last_mut", "first_mut", "push_back", "push_front", "pop_back", "pop_front", "retain", "sort", "sort_by", "sort_by_key", "dedup", "truncate", "drain", "chunks", "windows", "zip", "rev", "enumerate", "collect", "fold", "any", "all", "position", "max", "min", "sum", "count", "skip", "step_by", "flat_map", "filter_map", "cloned", "copied", "unwrap_or", "unwrap_or_default", "unwrap_or_else", "expect", "ok", "err", "ok_or", "ok_or_else", "and_then", "or_else", "map_err", "is_some", "is_none", "is_ok", "is_err", "as_mut", "as_deref", "to_owned", "to_vec", "starts_with", "ends_with", "strip_prefix", "strip_suffix", "trim_start", "trim_end", "to_lowercase", "to_uppercase", "split_once", "bytes", "display", "exists", "is_dir", "is_file", "parent", "file_name", "extension", "metadata", "kill", "wait", "spawn", "elapsed", "now", "as_bytes", "write_all", "read_to_string", "read_line", "resize", "swap", "reverse", "concat", "or_insert", "or_insert_with", "or_default", "get_or_insert_with", "abs", "round", "floor", "ceil", "sqrt", "pow", "clamp", "min_by_key", "max_by_key", "chain", "peekable", "peek", "last", "nth", "map_or", "map_or_else", "filter", "inspect", "lock", "try_lock", "read", "write", "join", "sleep", "duration_since", "as_secs", "as_millis", "contains", "replace", "split_whitespace", "parse", "format", "fill", "resize_with", "into_iter", "into_inner", "set_nonblocking", "accept", "connect", "shutdown", "bind", "listen", "incoming", "addEventListener", "querySelector", "setState", "useState", "useEffect", "json", "fetch", "log",
];

fn resoudre(p: &mut Projet, crates: &[Crate]) {
    let n = p.fichiers.len();
    // Ce qu'un fichier peut appeler : sa crate et ses dependances.
    let crates_fichiers: Vec<Option<&Crate>> = p.fichiers.iter().map(|f| crate_de(&f.chemin, crates)).collect();
    let permis = |de: usize, vers: usize| match (crates_fichiers[de], crates_fichiers[vers]) {
        (Some(a), Some(b)) => a.nom == b.nom || a.dependances.contains(&b.nom),
        _ => true,
    };
    let places: Vec<Option<Place>> = p.fichiers.iter().map(|f| (f.langage == Langage::Rust).then(|| place_rust(&f.chemin, crates))).collect();
    let mut modules: HashMap<(String, Vec<String>), usize> = HashMap::new();
    for (i, pl) in places.iter().enumerate() {
        if let Some(pl) = pl {
            modules.entry((pl.cle.clone(), pl.module.clone())).or_insert(i);
        }
    }
    let noms_crates: HashSet<String> = crates.iter().map(|c| c.nom.clone()).collect();
    let par_chemin: HashMap<&str, usize> = p.fichiers.iter().enumerate().map(|(i, f)| (f.chemin.as_str(), i)).collect();
    // Les fonctions par nom.
    let mut par_nom: HashMap<&str, Vec<RefFn>> = HashMap::new();
    for (i, f) in p.fichiers.iter().enumerate() {
        for (j, g) in f.fonctions.iter().enumerate() {
            par_nom.entry(g.nom.as_str()).or_default().push((i, j));
        }
    }
    let definit = |i: usize, nom: &str| p.fichiers[i].types.iter().any(|t| t == nom) || p.fichiers[i].fonctions.iter().any(|g| g.nom == nom && g.proprietaire.is_empty());

    // Rust : un chemin de `use` (ou d'appel) -> fichier et element.
    let resoudre_chemin = |de: usize, chemin: &[String]| -> Option<Cible> {
        let pl = places[de].as_ref()?;
        let (mut cle, mut m, reste): (String, Vec<String>, &[String]) = match chemin.first()?.as_str() {
            "crate" => (pl.cle.clone(), Vec::new(), &chemin[1..]),
            "self" => (pl.cle.clone(), pl.module.clone(), &chemin[1..]),
            "super" => {
                let mut m = pl.module.clone();
                let mut k = 0;
                while chemin.get(k).is_some_and(|s| s == "super") {
                    m.pop();
                    k += 1;
                }
                (pl.cle.clone(), m, &chemin[k..])
            }
            premier if noms_crates.contains(premier) && !modules.contains_key(&(pl.cle.clone(), [pl.module.clone(), vec![premier.to_string()]].concat())) => (premier.to_string(), Vec::new(), &chemin[1..]),
            premier => {
                // Un module enfant du module courant (`mod x;` + `use x::...`).
                let enfant = [pl.module.clone(), vec![premier.to_string()]].concat();
                if modules.contains_key(&(pl.cle.clone(), enfant.clone())) {
                    (pl.cle.clone(), enfant, &chemin[1..])
                } else {
                    return None;
                }
            }
        };
        // Un binaire ou un test qui fait `use crate::` reste dans sa racine.
        if !modules.contains_key(&(cle.clone(), m.clone())) {
            if modules.contains_key(&(pl.cle.clone(), m.clone())) {
                cle = pl.cle.clone();
            } else {
                return None;
            }
        }
        let mut item = None;
        for s in reste {
            let suivant = [m.clone(), vec![s.clone()]].concat();
            if item.is_none() && modules.contains_key(&(cle.clone(), suivant.clone())) {
                m = suivant;
            } else {
                item.get_or_insert_with(|| s.clone());
                break;
            }
        }
        Some(Cible { fichier: *modules.get(&(cle, m))?, item })
    };

    // JS : un module relatif -> fichier.
    let resoudre_module = |de: usize, module: &str| -> Option<usize> {
        let dossier = p.fichiers[de].dossier();
        let base = if let Some(r) = module.strip_prefix("@/") {
            format!("src/{r}")
        } else if module.starts_with('.') {
            let mut parts: Vec<&str> = if dossier.is_empty() { Vec::new() } else { dossier.split('/').collect() };
            for s in module.split('/') {
                match s {
                    "." | "" => {}
                    ".." => {
                        parts.pop();
                    }
                    s => parts.push(s),
                }
            }
            parts.join("/")
        } else {
            return None;
        };
        let essais = ["", ".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs", "/index.js", "/index.jsx", "/index.ts", "/index.tsx"];
        essais.iter().find_map(|e| par_chemin.get(format!("{base}{e}").as_str()).copied())
    };

    // La portee de chaque fichier : nom local -> cible ; et les `*`.
    let mut portees: Vec<HashMap<String, Cible>> = vec![HashMap::new(); n];
    let mut globs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for imp in &p.fichiers[i].imports {
            match imp {
                Import::Rust { chemin, local } => {
                    let Some(c) = resoudre_chemin(i, chemin) else { continue };
                    if local.is_empty() {
                        if c.item.is_none() {
                            globs[i].push(c.fichier);
                        }
                    } else {
                        portees[i].insert(local.clone(), c);
                    }
                }
                Import::Js { module, nom, local } => {
                    let Some(f) = resoudre_module(i, module) else { continue };
                    if local.is_empty() {
                        if nom == "*" {
                            globs[i].push(f);
                        } else {
                            // import 'm' (effet de bord) : un lien de fichier.
                            portees[i].insert(format!("\u{0}{module}"), Cible { fichier: f, item: None });
                        }
                    } else {
                        portees[i].insert(local.clone(), Cible { fichier: f, item: if nom == "*" { None } else { Some(nom.clone()) } });
                    }
                }
            }
        }
        // `mod x;` : x designe le module enfant.
        if let Some(pl) = &places[i] {
            for m in &p.fichiers[i].modules {
                if let Some(&f) = modules.get(&(pl.cle.clone(), [pl.module.clone(), vec![m.clone()]].concat())) {
                    portees[i].entry(m.clone()).or_insert(Cible { fichier: f, item: None });
                }
            }
        }
    }
    // Suivre les re-exports : `pub use x::Y;` dans le fichier vise.
    let suivre = |c: &Cible| -> Cible {
        let mut c = c.clone();
        for _ in 0..5 {
            let Some(item) = c.item.clone() else { break };
            if definit(c.fichier, &item) {
                break;
            }
            match portees[c.fichier].get(&item) {
                Some(suivant) if suivant.fichier != c.fichier => c = Cible { fichier: suivant.fichier, item: suivant.item.clone().or(Some(item)) },
                _ => break,
            }
        }
        c
    };

    // Les imports : fichier -> fichier.
    let mut imports: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for (i, portee) in portees.iter().enumerate() {
        for c in portee.values() {
            let c = suivre(c);
            if c.fichier != i {
                *imports.entry((i, c.fichier)).or_insert(0) += 1;
            }
        }
        for g in &globs[i] {
            if *g != i {
                imports.entry((i, *g)).or_insert(1);
            }
        }
    }

    // Les appels.
    let mut appels: BTreeMap<(RefFn, RefFn), usize> = BTreeMap::new();
    let mut ambigus = 0;
    // Les fichiers « connus » d'un fichier : ceux qu'il importe.
    let connus: Vec<HashSet<usize>> = (0..n).map(|i| imports.range((i, 0)..(i + 1, 0)).map(|((_, b), _)| *b).collect()).collect();
    for i in 0..n {
        let f = &p.fichiers[i];
        for (j, g) in f.fonctions.iter().enumerate() {
            for a in &g.appels {
                let Some(cands) = par_nom.get(a.nom.as_str()) else { continue };
                let cands: Vec<RefFn> = cands.iter().copied().filter(|c| permis(i, c.0)).collect();
                let choisis = choisir(p, i, &g.proprietaire, a, &cands, &portees[i], &globs[i], &connus[i], &resoudre_chemin, &suivre, &places);
                match choisis {
                    Some(v) => {
                        for c in v {
                            *appels.entry(((i, j), c)).or_insert(0) += 1;
                        }
                    }
                    None => ambigus += 1,
                }
            }
        }
    }
    p.appels = appels;
    p.imports = imports;
    p.ambigus = ambigus;
}

// Les fonctions visees par l'appel `a` (de la fonction de type `proprio`
// dans le fichier `i`). None : des candidats, mais aucun sur.
#[allow(clippy::too_many_arguments)]
fn choisir(
    p: &Projet,
    i: usize,
    proprio: &str,
    a: &Appel,
    cands: &[RefFn],
    portee: &HashMap<String, Cible>,
    globs: &[usize],
    connus: &HashSet<usize>,
    resoudre_chemin: &dyn Fn(usize, &[String]) -> Option<Cible>,
    suivre: &dyn Fn(&Cible) -> Cible,
    places: &[Option<Place>],
) -> Option<Vec<RefFn>> {
    let fct = |c: &RefFn| &p.fichiers[c.0].fonctions[c.1];
    let langage = p.fichiers[i].langage;
    let cands: Vec<RefFn> = cands.iter().copied().filter(|c| p.fichiers[c.0].langage == langage).collect();
    if cands.is_empty() {
        return Some(Vec::new());
    }
    let un = |v: Vec<RefFn>| -> Option<Vec<RefFn>> { if v.is_empty() { None } else { Some(v) } };
    let dans = |v: &[RefFn], fichier: usize| -> Vec<RefFn> { v.iter().copied().filter(|c| c.0 == fichier).collect() };
    let libres: Vec<RefFn> = cands.iter().copied().filter(|c| fct(c).proprietaire.is_empty()).collect();
    let methodes: Vec<RefFn> = cands.iter().copied().filter(|c| !fct(c).proprietaire.is_empty()).collect();
    // Une cible de la portee (import) -> les fonctions qu'elle designe.
    let vers_cible = |c: &Cible, nom: &str| -> Vec<RefFn> {
        let c = suivre(c);
        let nom = c.item.as_deref().filter(|x| *x != "default").unwrap_or(nom);
        let v: Vec<RefFn> = cands.iter().copied().filter(|x| x.0 == c.fichier && fct(x).nom == nom).collect();
        if v.is_empty() && c.item.as_deref() == Some("default") {
            return cands.iter().copied().filter(|x| x.0 == c.fichier && fct(x).defaut).collect();
        }
        v
    };
    match langage {
        Langage::Js => {
            if a.chemin.first().is_some_and(|c| c == "this") {
                return un(dans(&methodes, i).into_iter().filter(|c| fct(c).proprietaire == proprio).collect());
            }
            if a.methode {
                // ns.f() : ns importe en `* as ns`.
                if let Some(c) = a.chemin.first().and_then(|ns| portee.get(ns)) {
                    let c = suivre(c);
                    return un(cands.iter().copied().filter(|x| x.0 == c.fichier && fct(x).nom == a.nom).collect());
                }
                let meme = dans(&methodes, i);
                return if meme.is_empty() { if methodes.is_empty() { Some(Vec::new()) } else { None } } else { Some(meme) };
            }
            let meme = dans(&libres, i);
            if !meme.is_empty() {
                return Some(meme);
            }
            if let Some(c) = portee.get(&a.nom) {
                return un(vers_cible(c, &a.nom));
            }
            for g in globs {
                let v = dans(&libres, *g);
                if !v.is_empty() {
                    return Some(v);
                }
            }
            if libres.is_empty() { Some(Vec::new()) } else { None }
        }
        Langage::Rust => {
            if a.methode {
                if methodes.is_empty() {
                    return Some(Vec::new());
                }
                // Le type du proprietaire connu ici : defini dans ce fichier,
                // importe, ou dans un fichier importe.
                let types_connus = |c: &RefFn| {
                    let t = &fct(c).proprietaire;
                    c.0 == i || p.fichiers[i].types.contains(t) || portee.get(t).is_some_and(|x| suivre(x).fichier == c.0 || x.item.as_deref() == Some(t.as_str())) || connus.contains(&c.0)
                };
                let surs: Vec<RefFn> = methodes.iter().copied().filter(|c| types_connus(c)).collect();
                if !surs.is_empty() {
                    let meme = dans(&surs, i);
                    return Some(if meme.is_empty() { surs.into_iter().take(4).collect() } else { meme });
                }
                // Une seule methode de ce nom dans tout le projet, au nom peu
                // commun, dans la meme crate (ou une crate importee).
                if methodes.len() == 1 && !COMMUNS.contains(&a.nom.as_str()) {
                    let c = methodes[0];
                    let meme_crate = match (&places[i], &places[c.0]) {
                        (Some(x), Some(y)) => x.cle == y.cle || connus.iter().any(|k| places[*k].as_ref().is_some_and(|z| z.cle == y.cle)),
                        _ => false,
                    };
                    if meme_crate {
                        return Some(vec![c]);
                    }
                }
                return None;
            }
            if let Some(q) = a.chemin.last() {
                if q == "Self" {
                    let v: Vec<RefFn> = methodes.iter().copied().filter(|c| fct(c).proprietaire == proprio).collect();
                    let meme = dans(&v, i);
                    return un(if meme.is_empty() { v } else { meme });
                }
                if q.starts_with(|c: char| c.is_uppercase()) {
                    // Type::f : le type, peut-etre renomme a l'import.
                    let (nom_type, indice) = match portee.get(q) {
                        Some(c) => {
                            let c = suivre(c);
                            (c.item.clone().unwrap_or_else(|| q.clone()), Some(c.fichier))
                        }
                        None => (q.clone(), None),
                    };
                    let v: Vec<RefFn> = methodes.iter().copied().filter(|c| fct(c).proprietaire == nom_type).collect();
                    if v.is_empty() {
                        return if methodes.iter().any(|c| fct(c).proprietaire == *q) { None } else { Some(Vec::new()) };
                    }
                    let meme = dans(&v, i);
                    if !meme.is_empty() {
                        return Some(meme);
                    }
                    // Le fichier qui definit le type, s'il a la methode.
                    let definisseur: Vec<RefFn> = v.iter().copied().filter(|c| p.fichiers[c.0].types.contains(&nom_type) || Some(c.0) == indice).collect();
                    return Some(if definisseur.is_empty() { v.into_iter().take(4).collect() } else { definisseur });
                }
                // module::f
                let mut chemin = a.chemin.clone();
                chemin.push(a.nom.clone());
                if let Some(c) = resoudre_chemin(i, &chemin) {
                    return un(vers_cible(&c, &a.nom));
                }
                if let Some(c) = portee.get(q) {
                    let c = suivre(c);
                    let v: Vec<RefFn> = libres.iter().copied().filter(|x| x.0 == c.fichier).collect();
                    if !v.is_empty() {
                        return Some(v);
                    }
                    // Le module re-exporte : son `use` vers le vrai fichier.
                    if c.item.is_none() {
                        return un(vers_cible(&Cible { fichier: c.fichier, item: Some(a.nom.clone()) }, &a.nom));
                    }
                }
                // Externe (std::fs::read...) si aucun module du projet ne s'appelle q.
                let v: Vec<RefFn> = libres.iter().copied().filter(|c| places[c.0].as_ref().is_some_and(|pl| pl.module.last() == Some(q))).collect();
                return if v.is_empty() { Some(Vec::new()) } else { Some(v) };
            }
            // f(...) : ce fichier, un import, un `*`.
            let meme = dans(&libres, i);
            if !meme.is_empty() {
                return Some(meme);
            }
            if let Some(c) = portee.get(&a.nom) {
                return un(vers_cible(c, &a.nom));
            }
            for g in globs {
                let v = vers_cible(&Cible { fichier: *g, item: Some(a.nom.clone()) }, &a.nom);
                if !v.is_empty() {
                    return Some(v);
                }
            }
            // Une variable (closure) qui porte le nom d'une fonction ailleurs.
            Some(Vec::new())
        }
    }
}
