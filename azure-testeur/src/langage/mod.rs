// Ce que Testeur sait faire d'un langage : trouver ses tests, les lancer,
// lire la sortie, et ecrire des tests dans le code du projet. Rust (cargo)
// est le premier ; un autre langage = un module de plus qui implemente
// `Langage`, ajoute a `langages()`. Le reste de l'app n'en sait rien.
pub mod rust;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

/// Un paquet du projet (un crate pour Rust).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Paquet {
    /// Dossier, relatif au projet ("" : le projet lui-meme).
    pub dossier: String,
    pub nom: String,
    /// A une bibliotheque que les tests peuvent importer.
    pub bibliotheque: bool,
}

/// Un test trouve dans le code.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Test {
    /// Dossier du paquet, relatif au projet.
    pub paquet: String,
    /// Fichier du test, relatif au projet.
    pub fichier: String,
    /// La cible qui le compile (pour Rust : son fichier racine, relatif au
    /// paquet : `src/lib.rs`, `tests/ecran.rs`...).
    pub cible: String,
    /// Nom complet, tel que le lanceur l'affiche (`modele::tests::somme`).
    pub chemin: String,
    /// Nom seul (`somme`).
    pub nom: String,
    /// Ligne de la declaration (1...).
    pub ligne: usize,
    /// Options reconnues (voir `Langage::options`), `"oui"` si presentes.
    pub options: BTreeMap<String, String>,
}

impl Test {
    /// Identifiant stable : paquet, cible et nom complet.
    pub fn cle(&self) -> String {
        cle(&self.paquet, &self.cible, &self.chemin)
    }
}

pub fn cle(paquet: &str, cible: &str, chemin: &str) -> String {
    format!("{paquet}|{cible}|{chemin}")
}

/// Ce que l'analyse d'un projet a trouve.
#[derive(Clone, Debug, Default)]
pub struct Analyse {
    pub paquets: Vec<Paquet>,
    pub tests: Vec<Test>,
    /// Fichiers illisibles, etc.
    pub avertissements: Vec<String>,
}

/// Une commande de test a lancer : tout un paquet, une cible, ou des tests
/// precis.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Etape {
    /// Ce qui s'affiche pendant qu'elle tourne.
    pub titre: String,
    pub paquet: String,
    /// `None` : toutes les cibles de test du paquet.
    pub cible: Option<String>,
    /// Noms complets (vide : tous).
    pub filtres: Vec<String>,
    /// Les filtres sont des noms exacts (sinon : prefixes).
    pub exact: bool,
    /// Lancer aussi les tests ignores par defaut.
    pub avec_ignores: bool,
}

/// Issue d'un test.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Statut {
    #[default]
    Jamais,
    EnCours,
    Reussi,
    Echoue,
    Ignore,
    /// Pas execute : la compilation a echoue ou l'execution a ete arretee.
    Erreur,
}

impl Statut {
    pub fn code(self) -> &'static str {
        match self {
            Statut::Jamais => "jamais",
            Statut::EnCours => "en_cours",
            Statut::Reussi => "reussi",
            Statut::Echoue => "echoue",
            Statut::Ignore => "ignore",
            Statut::Erreur => "erreur",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Statut::Jamais => "pas lancé",
            Statut::EnCours => "en cours",
            Statut::Reussi => "réussi",
            Statut::Echoue => "échoué",
            Statut::Ignore => "ignoré",
            Statut::Erreur => "non exécuté",
        }
    }
}

/// Ce que le lecteur reconnait dans la sortie.
#[derive(Clone, Debug, PartialEq)]
pub enum Evenement {
    Statut { cle: String, statut: Statut },
    /// Sortie d'un test en echec (message, panique...).
    Sortie { cle: String, lignes: Vec<String> },
}

/// Lit la sortie d'une etape, ligne par ligne.
pub trait Lecteur: Send {
    fn ligne(&mut self, ligne: &str) -> Vec<Evenement>;
    /// Fin de la sortie : ce qui restait en attente.
    fn fin(&mut self) -> Vec<Evenement>;
}

/// Une option d'un test, montree en case a cocher dans l'atelier.
#[derive(Clone, Copy, Debug)]
pub struct OptionTest {
    pub code: &'static str,
    pub libelle: &'static str,
}

/// Un test tel qu'il est ecrit dans son fichier (pour l'atelier).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TestSource {
    pub nom: String,
    /// Le corps, sans l'indentation du fichier.
    pub code: String,
    pub options: BTreeMap<String, String>,
    pub ligne: usize,
}

/// Un fichier de tests tel que l'atelier le montre : son code generique
/// (ce qui precede les tests : imports, fonctions d'aide) et ses tests.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FichierTests {
    /// Relatif au projet.
    pub chemin: String,
    pub generique: String,
    pub tests: Vec<TestSource>,
}

/// Une modification du code du projet, faite par l'atelier. Une nouvelle
/// facon d'editer = une variante de plus, que chaque langage applique.
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    /// Cree un fichier de tests dans le paquet `paquet`, avec son code
    /// generique.
    CreerFichier { paquet: String, nom: String, generique: String },
    /// Remplace le code generique du fichier.
    Generique { fichier: String, code: String },
    /// Ajoute un test a la fin du fichier.
    CreerTest { fichier: String, test: TestSource },
    /// Remplace le test `ancien` (nom, corps, options).
    ModifierTest { fichier: String, ancien: String, test: TestSource },
    SupprimerTest { fichier: String, nom: String },
}

pub trait Langage: Send + Sync {
    /// `rust`
    fn code(&self) -> &'static str;
    /// `Rust · cargo`
    fn nom(&self) -> &'static str;
    /// Le projet est-il de ce langage ?
    fn reconnait(&self, projet: &Path) -> bool;
    fn analyser(&self, projet: &Path) -> Analyse;
    /// La commande qui lance l'etape.
    fn commande(&self, projet: &Path, etape: &Etape) -> Result<Command, String>;
    fn lecteur(&self, projet: &Path, etape: &Etape) -> Box<dyn Lecteur>;
    /// L'etape qui lance exactement ces tests (meme paquet et meme cible).
    fn etape_tests(&self, tests: &[&Test]) -> Vec<Etape> {
        let mut groupes: BTreeMap<(String, String), Vec<&Test>> = BTreeMap::new();
        for t in tests {
            groupes.entry((t.paquet.clone(), t.cible.clone())).or_default().push(t);
        }
        groupes
            .into_iter()
            .map(|((paquet, cible), tests)| Etape {
                titre: if tests.len() == 1 { tests[0].chemin.clone() } else { format!("{} test(s) de {cible}", tests.len()) },
                paquet,
                cible: Some(cible),
                filtres: tests.iter().map(|t| t.chemin.clone()).collect(),
                exact: true,
                avec_ignores: tests.iter().all(|t| t.options.contains_key("ignore")),
            })
            .collect()
    }

    // --- Atelier -------------------------------------------------------
    /// Options d'un test (cases a cocher).
    fn options(&self) -> &'static [OptionTest];
    /// Corps d'un nouveau test.
    fn modele_test(&self) -> &'static str;
    /// Code generique d'un nouveau fichier, pour le paquet `paquet`.
    fn modele_generique(&self, paquet: &Paquet) -> String;
    /// Le nom d'un test ou d'un fichier est-il acceptable ?
    fn valider_nom(&self, nom: &str) -> Result<(), String>;
    /// Chemin (relatif au projet) du fichier `nom` cree dans `paquet`.
    fn chemin_fichier(&self, paquet: &str, nom: &str) -> String;
    fn lire_fichier(&self, projet: &Path, fichier: &str) -> Result<FichierTests, String>;
    /// Applique `op` au code du projet ; rend le fichier modifie.
    fn appliquer(&self, projet: &Path, op: &Operation) -> Result<String, String>;
}

/// Tous les langages connus.
pub fn langages() -> Vec<Arc<dyn Langage>> {
    vec![Arc::new(rust::Rust)]
}

/// Le langage du projet, s'il est connu.
pub fn detecter(projet: &Path) -> Option<Arc<dyn Langage>> {
    langages().into_iter().find(|l| l.reconnait(projet))
}
