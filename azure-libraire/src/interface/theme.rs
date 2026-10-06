// Les themes : ce qui donne leurs couleurs et leurs arrondis aux modules.
//
// Un theme est une liste de jetons `nom = valeur` (fichiers `themes/*.theme`).
// Dans une feuille rsC, on ecrit le jeton a la place de la valeur :
//
// ```text
// .az-card { background-color: $surface; border: 1px solid $trait/6; border-radius: $rayon-grand; }
// ```
//
// - `$nom` : la valeur du jeton.
// - `$nom/14` : la couleur du jeton a 14 % d'opacite.
//
// `Theme::appliquer` fait le remplacement. Le theme en cours (`actif`) sert
// a toutes les pages ; le changer (`choisir`, `definir`) fait recompiler les
// feuilles au prochain affichage (`version`).
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

/// Les themes fournis : (nom, source).
const INTEGRES: &[(&str, &str)] = &[
    ("sable", include_str!("themes/sable.theme")),
    ("ivoire", include_str!("themes/ivoire.theme")),
    ("ardoise", include_str!("themes/ardoise.theme")),
];

/// Le theme utilise sans autre choix.
pub const DEFAUT: &str = "sable";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    /// Son nom court (`sable`).
    pub nom: String,
    /// Son nom affiche.
    pub titre: String,
    /// Fond sombre (texte clair) ou l'inverse.
    pub sombre: bool,
    jetons: BTreeMap<String, String>,
}

impl Theme {
    /// Un theme fourni par Azure.
    pub fn integre(nom: &str) -> Option<Theme> {
        let (_, source) = INTEGRES.iter().find(|(n, _)| *n == nom)?;
        Some(Theme::lire(source).unwrap_or_else(|e| panic!("theme integre '{nom}' invalide : {e}")))
    }

    /// Lit un theme : une ligne `jeton = valeur` par jeton, `#` pour un
    /// commentaire. `base = sable` reprend d'abord tous les jetons d'un theme
    /// fourni : un theme d'app n'ecrit que ce qu'il change. `nom`, `titre`
    /// et `sombre` decrivent le theme.
    pub fn lire(source: &str) -> Result<Theme, String> {
        let mut theme = Theme { nom: String::new(), titre: String::new(), sombre: true, jetons: BTreeMap::new() };
        for (n, ligne) in source.lines().enumerate() {
            let ligne = ligne.trim();
            // `#` seul en debut de ligne : une valeur `#1c1b19` n'est pas un commentaire.
            if ligne.is_empty() || ligne.starts_with("# ") || ligne == "#" {
                continue;
            }
            let (cle, valeur) = ligne.split_once('=').ok_or_else(|| format!("ligne {} : 'jeton = valeur' attendu", n + 1))?;
            let (cle, valeur) = (cle.trim(), valeur.trim());
            match cle {
                "base" => {
                    let base = Theme::integre(valeur).ok_or_else(|| format!("ligne {} : theme de base '{valeur}' inconnu", n + 1))?;
                    theme.sombre = base.sombre;
                    for (k, v) in base.jetons {
                        theme.jetons.entry(k).or_insert(v);
                    }
                }
                "nom" => theme.nom = valeur.to_string(),
                "titre" => theme.titre = valeur.to_string(),
                "sombre" => theme.sombre = matches!(valeur, "true" | "oui" | "1"),
                _ if !cle.is_empty() && cle.chars().all(est_de_jeton) => {
                    theme.jetons.insert(cle.to_string(), valeur.to_string());
                }
                _ => return Err(format!("ligne {} : nom de jeton '{cle}' invalide (a-z, 0-9, -)", n + 1)),
            }
        }
        if theme.titre.is_empty() {
            theme.titre = theme.nom.clone();
        }
        Ok(theme)
    }

    /// Lit le theme d'un fichier ; sans `nom`, il prend celui du fichier.
    pub fn fichier(chemin: impl AsRef<Path>) -> Result<Theme, String> {
        let chemin = chemin.as_ref();
        let source = std::fs::read_to_string(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let mut theme = Theme::lire(&source).map_err(|e| format!("{} : {e}", chemin.display()))?;
        if theme.nom.is_empty() {
            theme.nom = chemin.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        }
        Ok(theme)
    }

    /// La valeur d'un jeton.
    pub fn jeton(&self, nom: &str) -> Option<&str> {
        self.jetons.get(nom).map(String::as_str)
    }

    /// Un jeton de couleur en composantes (rouge, vert, bleu).
    pub fn couleur(&self, nom: &str) -> Option<(u8, u8, u8)> {
        self.jeton(nom).and_then(rvb)
    }

    /// Ce theme avec un jeton change : `theme.avec("accent", "#7aa2f7")`.
    pub fn avec(mut self, nom: &str, valeur: &str) -> Theme {
        self.jetons.insert(nom.to_string(), valeur.to_string());
        self
    }

    /// Les jetons, par ordre alphabetique.
    pub fn jetons(&self) -> impl Iterator<Item = (&str, &str)> {
        self.jetons.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Remplace les jetons d'une feuille rsC par leur valeur. Un jeton
    /// inconnu est laisse tel quel : rsC le signalera.
    pub fn appliquer(&self, rsc: &str) -> String {
        let mut out = String::with_capacity(rsc.len() + rsc.len() / 4);
        let mut reste = rsc;
        while let Some(i) = reste.find('$') {
            out.push_str(&reste[..i]);
            let apres = &reste[i + 1..];
            let fin = apres.find(|c: char| !est_de_jeton(c)).unwrap_or(apres.len());
            let nom = apres[..fin].trim_end_matches('-');
            let Some(valeur) = self.jeton(nom).filter(|_| !nom.is_empty()) else {
                out.push('$');
                reste = apres;
                continue;
            };
            let suite = &apres[nom.len()..];
            // `$accent/14` : la couleur a 14 % d'opacite.
            let chiffres = suite.strip_prefix('/').map(|s| &s[..s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len())]).unwrap_or("");
            match (chiffres.parse::<u32>(), rvb(valeur)) {
                (Ok(pour_cent), Some((r, v, b))) if pour_cent <= 100 => {
                    out.push_str(&format!("rgba({r}, {v}, {b}, {}.{:02})", pour_cent / 100, pour_cent % 100));
                    reste = &suite[1 + chiffres.len()..];
                }
                _ => {
                    out.push_str(valeur);
                    reste = suite;
                }
            }
        }
        out.push_str(reste);
        out
    }
}

fn est_de_jeton(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
}

/// `#rgb` ou `#rrggbb` en composantes (rouge, vert, bleu).
pub fn rvb(valeur: &str) -> Option<(u8, u8, u8)> {
    let hex = valeur.strip_prefix('#')?;
    let n = |s: &str| u8::from_str_radix(s, 16).ok();
    match hex.len() {
        3 => Some((n(&hex[0..1])? * 17, n(&hex[1..2])? * 17, n(&hex[2..3])? * 17)),
        6 => Some((n(&hex[0..2])?, n(&hex[2..4])?, n(&hex[4..6])?)),
        _ => None,
    }
}

/// Les noms des themes fournis.
pub fn integres() -> Vec<&'static str> {
    INTEGRES.iter().map(|(n, _)| *n).collect()
}

fn courant() -> &'static RwLock<Arc<Theme>> {
    static COURANT: OnceLock<RwLock<Arc<Theme>>> = OnceLock::new();
    COURANT.get_or_init(|| {
        // `AZURE_THEME=ivoire` : essayer un theme sans toucher a l'app.
        let voulu = std::env::var("AZURE_THEME").ok().and_then(|n| Theme::integre(n.trim()));
        RwLock::new(Arc::new(voulu.unwrap_or_else(|| Theme::integre(DEFAUT).expect("theme par defaut"))))
    })
}

static VERSION: AtomicU64 = AtomicU64::new(0);

/// Le theme en cours.
pub fn actif() -> Arc<Theme> {
    courant().read().map(|t| t.clone()).unwrap_or_else(|e| e.into_inner().clone())
}

/// Change le theme en cours (un theme fourni, lu d'un fichier, ou retouche
/// avec `avec`). Les pages le prennent a leur prochain affichage.
pub fn definir(theme: Theme) {
    let mut courant = courant().write().unwrap_or_else(|e| e.into_inner());
    if **courant != theme {
        *courant = Arc::new(theme);
        VERSION.fetch_add(1, Ordering::Relaxed);
    }
}

/// Passe a un theme fourni : `choisir("ivoire")`.
pub fn choisir(nom: &str) -> Result<(), String> {
    let theme = Theme::integre(nom).ok_or_else(|| format!("theme '{nom}' inconnu (fournis : {})", integres().join(", ")))?;
    definir(theme);
    Ok(())
}

/// Change a chaque changement de theme : une feuille compilee avec une
/// autre version est a refaire.
pub fn version() -> u64 {
    VERSION.load(Ordering::Relaxed)
}
