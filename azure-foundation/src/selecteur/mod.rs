// La boite « Ouvrir » d'Azure : choisir un dossier (ou un fichier) en le
// parcourant, au lieu de taper son chemin. La meme pour toutes les apps.
//
// Une app la demande depuis un clic :
//
//     Some("parcourir") => ctx.choisir(Selecteur::dossier().titre("Relier un projet").dans("chemin").puis("lier")),
//
// La boite s'ouvre par-dessus la page (voir `window::models::window`,
// `LoopState::selecteur`). Au choix : le chemin est ecrit dans le champ
// `dans`, puis `on_click` est appele comme pour un clic sur `puis`, avec
// le chemin dans `ctx.choix()`. Annuler ne fait rien.
//
// Une app enfermee (voir SECURITE.md) ne parcourt que ce que ses
// `[permissions]` lui laissent lire : ses dossiers autorises sont ses
// « lieux » (voir `definir_lieux`), un dossier interdit le dit.
//
// mod.rs          la demande (`Selecteur`) et la boite ouverte (`Ouvert`)
// selecteur.rsh   la boite, en rsH
// selecteur.rsc   son style
use crate::compiler::rsc::mangers::parser::parse as parse_rsc;
use crate::compiler::rsc::models::rule::RscStylesheet;
use crate::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use crate::compiler::rsh::mangers::parser::{parse as parse_rsh, AstNode};
use crate::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use crate::compiler::services::codegen::StyleSource;
use crate::compiler::services::condition::{ConditionValue as V, Context};
use crate::compiler::services::interpreter::build_ui_with_context;
use crate::ui::models::ui_node::UiNode;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const RSH: &str = include_str!("selecteur.rsh");
const RSC: &str = include_str!("selecteur.rsc");

/// Au-dela, la liste est coupee (un dossier de milliers d'entrees figerait
/// la mise en page) : le reste est annonce en bas.
const MAX_ENTREES: usize = 400;

/// Ce qu'on choisit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genre {
    Dossier,
    Fichier,
}

/// La demande d'une app (voir `WindowContext::choisir`).
#[derive(Debug, Clone, PartialEq)]
pub struct Selecteur {
    genre: Genre,
    titre: String,
    depart: Option<PathBuf>,
    champ: Option<String>,
    puis: Option<String>,
    extensions: Vec<String>,
}

impl Selecteur {
    /// Choisir un dossier.
    pub fn dossier() -> Selecteur {
        Selecteur { genre: Genre::Dossier, titre: "Choisir un dossier".to_string(), depart: None, champ: None, puis: None, extensions: Vec::new() }
    }

    /// Choisir un fichier qui existe.
    pub fn fichier() -> Selecteur {
        Selecteur { genre: Genre::Fichier, titre: "Ouvrir un fichier".to_string(), ..Selecteur::dossier() }
    }

    /// Le titre de la boite.
    pub fn titre(mut self, titre: &str) -> Selecteur {
        self.titre = titre.to_string();
        self
    }

    /// Ou la boite s'ouvre (`~/Dev`, ou ce qui est deja tape dans le
    /// champ). Un fichier, ou un chemin qui n'existe pas : son premier
    /// dossier parent qui existe. Vide, ou sans appel : le premier lieu.
    pub fn depart(mut self, chemin: &str) -> Selecteur {
        let chemin = chemin.trim();
        self.depart = (!chemin.is_empty()).then(|| developper(chemin));
        self
    }

    /// Le champ `<input#id>` de la page ou ecrire le chemin choisi.
    pub fn dans(mut self, id: &str) -> Selecteur {
        self.champ = Some(id.to_string());
        self
    }

    /// Le bouton `#id` a « cliquer » une fois le choix fait : `on_click`
    /// est appele avec `ctx.clicked == Some(id)`.
    pub fn puis(mut self, id: &str) -> Selecteur {
        self.puis = Some(id.to_string());
        self
    }

    /// Pour un fichier : ne montrer que ces extensions (`&["merise", "sql"]`).
    pub fn extensions(mut self, extensions: &[&str]) -> Selecteur {
        self.extensions = extensions.iter().map(|e| e.trim_start_matches('.').to_lowercase()).collect();
        self
    }

    pub fn genre(&self) -> Genre {
        self.genre
    }

    pub(crate) fn champ(&self) -> Option<&str> {
        self.champ.as_deref()
    }

    pub(crate) fn suite(&self) -> Option<&str> {
        self.puis.as_deref()
    }
}

/// `~/x` -> chemin absolu.
pub fn developper(chemin: &str) -> PathBuf {
    let maison = std::env::var_os("HOME").map(PathBuf::from);
    match (chemin.strip_prefix("~/"), maison) {
        (Some(reste), Some(maison)) => maison.join(reste),
        (None, Some(maison)) if chemin == "~" => maison,
        _ => PathBuf::from(chemin),
    }
}

/// L'inverse, pour l'affichage : `/home/moi/Dev` -> `~/Dev`.
pub fn abreger(chemin: &Path) -> String {
    let maison = std::env::var_os("HOME").map(PathBuf::from).filter(|m| m != Path::new("/"));
    match maison.as_ref().and_then(|m| chemin.strip_prefix(m).ok()) {
        Some(reste) if reste.as_os_str().is_empty() => "~".to_string(),
        Some(reste) => format!("~/{}", reste.display()),
        None => chemin.display().to_string(),
    }
}

// Les dossiers que l'app declare dans ses `[permissions]` (voir
// `definir_lieux`).
static LIEUX: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Les dossiers autorises de l'app, montres en premier dans les « lieux »
/// de la boite. Appele par `AzureApp` avec les `[permissions]` du manifeste.
pub fn definir_lieux(dossiers: Vec<PathBuf>) {
    if let Ok(mut lieux) = LIEUX.lock() {
        *lieux = dossiers;
    }
}

// Les lieux a proposer : ceux de l'app, puis les dossiers habituels - en
// ne gardant que ceux que l'app peut reellement lire (une app enfermee ne
// lit pas `~`).
fn lieux() -> Vec<PathBuf> {
    let mut candidats = LIEUX.lock().map(|l| l.clone()).unwrap_or_default();
    if let Some(maison) = std::env::var_os("HOME").map(PathBuf::from) {
        candidats.push(maison.clone());
        for nom in ["Bureau", "Desktop", "Documents", "Téléchargements", "Downloads", "Dev"] {
            candidats.push(maison.join(nom));
        }
    }
    let mut lieux: Vec<PathBuf> = Vec::new();
    for c in candidats {
        let c = c.canonicalize().unwrap_or(c);
        if !lieux.contains(&c) && std::fs::read_dir(&c).is_ok() {
            lieux.push(c);
        }
    }
    lieux
}

/// Une ligne du dossier ouvert.
#[derive(Debug, Clone, PartialEq)]
pub struct Entree {
    pub nom: String,
    pub dossier: bool,
}

/// Ce qu'un clic dans la boite demande a la fenetre.
#[derive(Debug, Clone, PartialEq)]
pub enum Suite {
    /// Rien n'a change.
    Rien,
    /// Un autre dossier est ouvert : redessiner, liste en haut.
    Liste,
    /// Le meme dossier : redessiner sans bouger la liste.
    Garder,
    /// Fermer sans rien choisir.
    Annuler,
    /// Fermer : ce chemin (absolu) est choisi.
    Choisi(PathBuf),
}

/// La boite ouverte : le dossier parcouru et ce qui y est choisi.
#[derive(Debug)]
pub struct Ouvert {
    demande: Selecteur,
    dossier: PathBuf,
    entrees: Vec<Entree>,
    // Entrees non montrees (voir `MAX_ENTREES`).
    reste: usize,
    // Mode fichier : le rang du fichier choisi dans `entrees`.
    choisi: Option<usize>,
    erreur: String,
    caches: bool,
    lieux: Vec<PathBuf>,
}

impl Ouvert {
    pub fn new(demande: Selecteur) -> Ouvert {
        let lieux = lieux();
        Ouvert::avec_lieux(demande, lieux)
    }

    /// Comme `new`, avec ces lieux (les essais n'ont pas ceux du systeme).
    pub fn avec_lieux(demande: Selecteur, lieux: Vec<PathBuf>) -> Ouvert {
        let depart = demande
            .depart
            .as_deref()
            .and_then(|d| d.ancestors().find(|a| a.is_dir()))
            .map(Path::to_path_buf)
            // Rien de demande, ou un depart que l'app ne peut pas lire.
            .filter(|d| std::fs::read_dir(d).is_ok())
            .or_else(|| lieux.first().cloned())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("/"));
        let mut ouvert = Ouvert { demande, dossier: PathBuf::new(), entrees: Vec::new(), reste: 0, choisi: None, erreur: String::new(), caches: false, lieux };
        ouvert.aller(depart);
        ouvert
    }

    pub fn demande(&self) -> &Selecteur {
        &self.demande
    }

    pub fn dossier(&self) -> &Path {
        &self.dossier
    }

    pub fn entrees(&self) -> &[Entree] {
        &self.entrees
    }

    pub fn erreur(&self) -> &str {
        &self.erreur
    }

    // Ouvre `dossier` et lit ce qu'il contient.
    fn aller(&mut self, dossier: PathBuf) {
        self.dossier = dossier.canonicalize().unwrap_or(dossier);
        self.lire();
    }

    fn lire(&mut self) {
        self.entrees.clear();
        self.erreur.clear();
        self.choisi = None;
        self.reste = 0;
        let lecture = match std::fs::read_dir(&self.dossier) {
            Ok(l) => l,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                self.erreur = "Cette app n'a pas le droit de lire ce dossier. Pour l'autoriser, ajoutez-le aux [permissions] de son app.azure, puis réinstallez-la.".to_string();
                return;
            }
            Err(e) => {
                self.erreur = format!("{} : {e}", self.dossier.display());
                return;
            }
        };
        let mut entrees: Vec<Entree> = lecture
            .flatten()
            .filter_map(|e| {
                let nom = e.file_name().to_string_lossy().into_owned();
                // Un lien est ce vers quoi il pointe.
                let dossier = e.path().is_dir();
                (self.montre(&nom, dossier)).then_some(Entree { nom, dossier })
            })
            .collect();
        // Les dossiers d'abord, puis par nom sans regarder la casse.
        entrees.sort_by(|a, b| b.dossier.cmp(&a.dossier).then_with(|| a.nom.to_lowercase().cmp(&b.nom.to_lowercase())).then_with(|| a.nom.cmp(&b.nom)));
        self.reste = entrees.len().saturating_sub(MAX_ENTREES);
        entrees.truncate(MAX_ENTREES);
        self.entrees = entrees;
    }

    fn montre(&self, nom: &str, dossier: bool) -> bool {
        if nom.starts_with('.') && !self.caches {
            return false;
        }
        if dossier || self.demande.genre == Genre::Dossier || self.demande.extensions.is_empty() {
            return true;
        }
        let extension = Path::new(nom).extension().map(|e| e.to_string_lossy().to_lowercase());
        extension.is_some_and(|e| self.demande.extensions.contains(&e))
    }

    // Le fil d'Ariane : du haut jusqu'au dossier ouvert. `~` remplace le
    // dossier personnel et ce qui est au-dessus.
    fn fil(&self) -> Vec<(String, PathBuf)> {
        let maison = std::env::var_os("HOME").map(PathBuf::from).filter(|m| m != Path::new("/") && self.dossier.starts_with(m));
        let mut fil: Vec<(String, PathBuf)> = self
            .dossier
            .ancestors()
            .take_while(|a| maison.as_deref().is_none_or(|m| a.starts_with(m)))
            .map(|a| {
                let nom = match a.file_name() {
                    _ if Some(a) == maison.as_deref() => "~".to_string(),
                    Some(n) => n.to_string_lossy().into_owned(),
                    None => "/".to_string(),
                };
                (nom, a.to_path_buf())
            })
            .collect();
        fil.reverse();
        fil
    }

    // Ce que la boite rendrait si on validait maintenant.
    fn choix(&self) -> Option<PathBuf> {
        if !self.erreur.is_empty() {
            return None;
        }
        match self.demande.genre {
            Genre::Dossier => Some(self.dossier.clone()),
            Genre::Fichier => self.choisi.and_then(|i| self.entrees.get(i)).map(|e| self.dossier.join(&e.nom)),
        }
    }

    /// Un clic sur l'element `id` de la boite ; `double` : c'est un
    /// double-clic.
    pub fn cliquer(&mut self, id: &str, double: bool) -> Suite {
        let rang = |prefixe: &str| id.strip_prefix(prefixe).and_then(|n| n.parse::<usize>().ok());
        match id {
            "selecteur-fermer" | "sel-annuler" => return Suite::Annuler,
            "sel-choisir" => return self.choix().map(Suite::Choisi).unwrap_or(Suite::Rien),
            "sel-parent" => {
                let Some(parent) = self.dossier.parent().map(Path::to_path_buf) else { return Suite::Rien };
                self.aller(parent);
                return Suite::Liste;
            }
            "sel-caches" => {
                self.caches = !self.caches;
                self.lire();
                return Suite::Liste;
            }
            _ => {}
        }
        if let Some(i) = rang("sel-fil-") {
            let Some((_, chemin)) = self.fil().into_iter().nth(i) else { return Suite::Rien };
            self.aller(chemin);
            return Suite::Liste;
        }
        if let Some(i) = rang("sel-lieu-") {
            let Some(lieu) = self.lieux.get(i).cloned() else { return Suite::Rien };
            self.aller(lieu);
            return Suite::Liste;
        }
        if let Some(i) = rang("sel-entree-") {
            let Some(entree) = self.entrees.get(i) else { return Suite::Rien };
            if entree.dossier {
                let dossier = self.dossier.join(&entree.nom);
                self.aller(dossier);
                return Suite::Liste;
            }
            if self.demande.genre == Genre::Dossier {
                return Suite::Rien;
            }
            // Double-clic sur un fichier : c'est lui.
            if double && self.choisi == Some(i) {
                return self.choix().map(Suite::Choisi).unwrap_or(Suite::Rien);
            }
            self.choisi = Some(i);
            return Suite::Garder;
        }
        Suite::Rien
    }

    // Les donnees de la boite pour selecteur.rsh.
    fn contexte(&self) -> Context {
        let fichiers = self.demande.genre == Genre::Fichier;
        let fil = self.fil();
        let dernier = fil.len().saturating_sub(1);
        let fil: Vec<V> = fil.iter().enumerate().map(|(i, (nom, _))| V::map([("i", V::from(i as u32)), ("nom", V::from(nom.as_str())), ("dernier", V::from(i == dernier))])).collect();
        let lieux: Vec<V> = self.lieux.iter().enumerate().map(|(i, l)| V::map([("i", V::from(i as u32)), ("nom", V::from(abreger(l))), ("ici", V::from(*l == self.dossier))])).collect();
        let entrees: Vec<V> = self
            .entrees
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let etat = match e.dossier {
                    false if !fichiers => "inactif",
                    false if self.choisi == Some(i) => "choisi",
                    _ => "",
                };
                V::map([("i", V::from(i as u32)), ("nom", V::from(e.nom.as_str())), ("genre", V::from(if e.dossier { "dossier" } else { "fichier" })), ("etat", V::from(etat))])
            })
            .collect();
        let choix = match (self.choix(), fichiers) {
            (Some(c), _) => abreger(&c),
            (None, true) if self.erreur.is_empty() => "Aucun fichier choisi".to_string(),
            (None, _) => abreger(&self.dossier),
        };
        let vide_texte = if fichiers && !self.demande.extensions.is_empty() {
            format!("Rien à ouvrir ici ({}).", self.demande.extensions.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(", "))
        } else {
            "Ce dossier est vide.".to_string()
        };
        Context::new()
            .with_text("titre", &self.demande.titre)
            .with_bool("caches", self.caches)
            .with_bool("a_lieux", !lieux.is_empty())
            .with_bool("a_erreur", !self.erreur.is_empty())
            .with_text("erreur", &self.erreur)
            .with_bool("vide", self.erreur.is_empty() && entrees.is_empty())
            .with_text("vide_texte", &vide_texte)
            .with_bool("tronque", self.reste > 0)
            .with_text("reste", &format!("… et {} autre(s), non montré(s).", self.reste))
            .with_text("choix", &choix)
            .with_bool("pret", self.choix().is_some())
            .with_text("action", if fichiers { "Ouvrir" } else { "Choisir ce dossier" })
            .with_list("fil", fil)
            .with_list("lieux", lieux)
            .with_list("entrees", entrees)
    }

    /// La boite a dessiner par-dessus la page.
    pub fn noeuds(&self) -> Vec<UiNode> {
        let (ast, feuille) = modele();
        build_ui_with_context(ast, &StyleSource::Rsc(feuille), &self.contexte())
    }
}

// selecteur.rsh et selecteur.rsc, analyses une seule fois.
fn modele() -> &'static (Vec<AstNode>, RscStylesheet) {
    static MODELE: OnceLock<(Vec<AstNode>, RscStylesheet)> = OnceLock::new();
    MODELE.get_or_init(|| {
        let ast = parse_rsh(tokenize_rsh(RSH)).expect("selecteur.rsh invalide");
        let feuille = parse_rsc(tokenize_rsc(RSC)).expect("selecteur.rsc invalide");
        (ast, feuille)
    })
}
