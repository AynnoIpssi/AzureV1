// Les projets relies a Testeur : l'environnement Azure lui-meme (ses
// sources, notees par `azure setup`) et les projets choisis par
// l'utilisateur, gardes dans le stockage prive de l'app.
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Projet {
    pub nom: String,
    pub chemin: PathBuf,
    /// Les sources d'Azure.
    pub environnement: bool,
}

/// Ou garder la liste des projets relies.
pub trait Memoire: Send + Sync {
    fn lire(&self) -> Vec<String>;
    fn ecrire(&self, chemins: &[String]) -> Result<(), String>;
}

/// Les sources d'Azure (fichier `source` ecrit par `azure setup`). A lire
/// AVANT que l'app s'enferme : ce dossier-la lui est ensuite interdit.
pub fn environnement() -> Option<Projet> {
    let donnees = match std::env::var_os("XDG_DATA_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".local/share"),
    };
    let texte = std::fs::read_to_string(donnees.join("azure/source")).ok()?;
    let chemin = PathBuf::from(texte.trim());
    chemin.join("Cargo.toml").is_file().then(|| Projet { nom: "Environnement Azure".to_string(), chemin, environnement: true })
}

fn nom_de(chemin: &Path) -> String {
    chemin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| chemin.display().to_string())
}

/// `~/x` -> chemin absolu.
pub fn developper(chemin: &str) -> PathBuf {
    let chemin = chemin.trim();
    match (chemin.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(reste), Some(h)) => PathBuf::from(h).join(reste),
        _ if chemin == "~" => std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default(),
        _ => PathBuf::from(chemin),
    }
}

/// L'environnement (s'il est connu) puis les projets relies.
pub struct Projets {
    pub liste: Vec<Projet>,
    memoire: Box<dyn Memoire>,
}

impl Projets {
    pub fn new(environnement: Option<Projet>, memoire: Box<dyn Memoire>) -> Projets {
        let mut liste: Vec<Projet> = environnement.into_iter().collect();
        for c in memoire.lire() {
            let chemin = PathBuf::from(&c);
            if !liste.iter().any(|p| p.chemin == chemin) {
                liste.push(Projet { nom: nom_de(&chemin), chemin, environnement: false });
            }
        }
        Projets { liste, memoire }
    }

    fn garder(&self) -> Result<(), String> {
        let chemins: Vec<String> = self.liste.iter().filter(|p| !p.environnement).map(|p| p.chemin.to_string_lossy().into_owned()).collect();
        self.memoire.ecrire(&chemins)
    }

    /// Relie le dossier `chemin` ; rend son rang dans la liste.
    pub fn lier(&mut self, chemin: &str) -> Result<usize, String> {
        if chemin.trim().is_empty() {
            return Err("Tapez le chemin du dossier du projet (ex. ~/Dev/mon-projet).".to_string());
        }
        let chemin = developper(chemin);
        match std::fs::read_dir(&chemin) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                return Err(format!("{} : accès refusé. Ajoutez ce dossier à [permissions] (lecture, ecriture) dans app.azure de Testeur, puis réinstallez-le.", chemin.display()));
            }
            Err(e) => return Err(format!("{} : {e}", chemin.display())),
        }
        let chemin = chemin.canonicalize().unwrap_or(chemin);
        if let Some(i) = self.liste.iter().position(|p| p.chemin == chemin) {
            return Ok(i);
        }
        self.liste.push(Projet { nom: nom_de(&chemin), chemin, environnement: false });
        self.garder()?;
        Ok(self.liste.len() - 1)
    }

    /// Oublie le projet `i` (pas l'environnement).
    pub fn delier(&mut self, i: usize) -> Result<(), String> {
        if self.liste.get(i).is_none_or(|p| p.environnement) {
            return Err("L'environnement Azure reste toujours relié.".to_string());
        }
        self.liste.remove(i);
        self.garder()
    }
}

/// Garde les chemins en memoire seulement (tests, stockage indisponible).
#[derive(Default)]
pub struct EnMemoire(pub std::sync::Mutex<Vec<String>>);

impl Memoire for EnMemoire {
    fn lire(&self) -> Vec<String> {
        self.0.lock().map(|v| v.clone()).unwrap_or_default()
    }

    fn ecrire(&self, chemins: &[String]) -> Result<(), String> {
        *self.0.lock().map_err(|_| "verrou")? = chemins.to_vec();
        Ok(())
    }
}
