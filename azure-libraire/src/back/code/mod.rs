// Lire du code source. On lit les fichiers d'un projet (Rust, JavaScript /
// TypeScript), on en tire les fonctions, leurs appels et les imports
// (`rust`, `js`), puis on resout qui appelle qui (`analyse`).
// `coloration` decoupe un code en morceaux a colorer (rsH, rsC, RsS, Rust,
// TOML, shell).
pub mod analyse;
pub mod coloration;
pub mod js;
pub mod rust;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Langage {
    Rust,
    Js,
}

impl Langage {
    pub fn depuis_extension(ext: &str) -> Option<Langage> {
        match ext {
            "rs" => Some(Langage::Rust),
            "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" => Some(Langage::Js),
            _ => None,
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            Langage::Rust => "Rust",
            Langage::Js => "JS / TS",
        }
    }
}

/// Un appel (ou une reference) dans le corps d'une fonction, tel qu'ecrit.
#[derive(Clone, Debug, PartialEq)]
pub struct Appel {
    pub nom: String,
    /// Ce qui le qualifie : `Toile::cadrer` -> ["Toile"], `interact::walk` ->
    /// ["interact"], `ns.f` (JS) -> ["ns"].
    pub chemin: Vec<String>,
    /// `x.nom(...)` : une methode.
    pub methode: bool,
    pub ligne: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fonction {
    pub nom: String,
    /// Le type (impl / trait / classe) d'une methode, vide sinon.
    pub proprietaire: String,
    pub ligne: usize,
    pub fin: usize,
    pub appels: Vec<Appel>,
    /// JS : exportee par defaut.
    pub defaut: bool,
}

impl Fonction {
    pub fn nom_complet(&self) -> String {
        if self.proprietaire.is_empty() { self.nom.clone() } else { format!("{}::{}", self.proprietaire, self.nom) }
    }
}

/// Un import tel qu'ecrit.
#[derive(Clone, Debug, PartialEq)]
pub enum Import {
    /// Rust : `use a::b::c as d;` -> chemin [a, b, c], nom local d ; `*` :
    /// nom local vide.
    Rust { chemin: Vec<String>, local: String },
    /// JS : `import { a as b } from './x'` -> module ./x, nom a, local b ;
    /// `import X from` : nom "default" ; `import * as ns` : nom "*".
    Js { module: String, nom: String, local: String },
}

/// Ce qu'on tire d'un fichier.
#[derive(Clone, Debug, PartialEq)]
pub struct Fichier {
    /// Relatif a la racine du projet, avec des `/`.
    pub chemin: String,
    pub langage: Langage,
    pub lignes: usize,
    pub fonctions: Vec<Fonction>,
    pub imports: Vec<Import>,
    /// Rust : `mod x;` declares ici.
    pub modules: Vec<String>,
    /// Rust : les types (struct, enum, trait) definis ici.
    pub types: Vec<String>,
    /// Un fichier de test (voir `est_test`).
    pub test: bool,
}

/// Un fichier de test : dans un dossier tests/, test/, __tests__/, spec/,
/// ou nomme tests.rs, *_test.rs, *.test.js, *.spec.ts...
pub fn est_test(chemin: &str) -> bool {
    let parts: Vec<&str> = chemin.split('/').collect();
    let nom = parts.last().copied().unwrap_or("");
    parts[..parts.len().saturating_sub(1)].iter().any(|d| matches!(*d, "tests" | "test" | "__tests__" | "spec" | "specs"))
        || nom == "tests.rs"
        || nom.ends_with("_test.rs")
        || nom.ends_with("_tests.rs")
        || nom.contains(".test.")
        || nom.contains(".spec.")
}

impl Fichier {
    pub fn dossier(&self) -> &str {
        self.chemin.rsplit_once('/').map_or("", |(d, _)| d)
    }

    pub fn nom(&self) -> &str {
        self.chemin.rsplit('/').next().unwrap_or(&self.chemin)
    }
}

/// Mots cles qui precedent une parenthese sans etre un appel.
pub fn mot_cle(m: &str) -> bool {
    matches!(
        m,
        "if" | "while" | "for" | "match" | "return" | "fn" | "loop" | "in" | "as" | "let" | "mut" | "ref" | "move" | "async" | "await" | "where" | "impl" | "dyn" | "pub" | "crate" | "super" | "self" | "Self" | "use" | "mod" | "unsafe" | "else" | "break" | "continue" | "function" | "switch" | "catch" | "typeof" | "new" | "delete" | "void" | "yield" | "constructor" | "super_" | "do" | "try" | "throw" | "with" | "instanceof"
    )
}
