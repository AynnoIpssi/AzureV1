// Ce que coute chaque fonctionnalite d'une app, mesure par la fondation :
// les clics (par bouton), les pages construites (donnees de l'app, puis
// construction), le dessin, les evenements, les taches de fond (`on_tick`),
// les flux. Une app peut nommer ses propres taches avec `mesurer`.
//
// Pour chaque mesure : combien de fois, le temps (mur, avec ce qu'elle a
// appele) et le temps de processeur du fil, SANS ce que les mesures
// imbriquees ont deja compte (un clic qui reconstruit la page : la page est
// comptee a part, le clic garde le reste). Les temps de processeur
// s'additionnent donc sans compter deux fois.
//
// La fenetre publie le tout toutes les 2 secondes dans
// /tmp/azure-perf-<uid>/<app>.txt (dossier a soi, 0700), lu par Azure
// Benchmark. /tmp est accessible a toutes les apps, enfermees ou non.
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Une fonctionnalite mesuree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stat {
    pub fois: u64,
    /// Temps ecoule, ce qu'elle a appele compris.
    pub mur: Duration,
    /// Temps de processeur a elle (hors mesures imbriquees).
    pub cpu: Duration,
    /// La plus longue fois (temps ecoule).
    pub max: Duration,
}

struct Profil {
    app: Option<String>,
    stats: BTreeMap<(String, String), Stat>,
    version: u64,
    publie: Option<(Instant, u64)>,
    debut: Instant,
}

static PROFIL: Mutex<Option<Profil>> = Mutex::new(None);

thread_local! {
    /// Les mesures en cours sur ce fil : (temps, processeur) deja comptes
    /// par leurs mesures imbriquees.
    static PILE: RefCell<Vec<(Duration, Duration)>> = const { RefCell::new(Vec::new()) };
}

fn profil<T>(f: impl FnOnce(&mut Profil) -> T) -> T {
    let mut g = PROFIL.lock().unwrap_or_else(|e| e.into_inner());
    let p = g.get_or_insert_with(|| Profil { app: None, stats: BTreeMap::new(), version: 0, publie: None, debut: Instant::now() });
    f(p)
}

/// Temps de processeur du fil courant.
pub fn cpu_du_fil() -> Duration {
    // SAFETY : clock_gettime remplit `ts`, une structure C sans pointeur.
    let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    Duration::new(ts.tv_sec.max(0) as u64, ts.tv_nsec.max(0) as u32)
}

/// Le nom sous lequel l'app publie (celui de son manifeste).
pub fn nommer(app: &str) {
    profil(|p| p.app = Some(app.to_string()));
}

/// `lancer-12` -> `lancer-#` : un bouton par ligne compte comme une seule
/// fonctionnalite.
pub fn generaliser(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    let mut chiffre = false;
    for c in id.chars() {
        if c.is_ascii_digit() {
            if !chiffre {
                out.push('#');
            }
            chiffre = true;
        } else {
            out.push(c);
            chiffre = false;
        }
    }
    out
}

/// Mesure `f` comme la fonctionnalite `nom` de la `categorie` (`Clic`,
/// `Page`, ou une categorie a soi).
pub fn mesurer<T>(categorie: &str, nom: &str, f: impl FnOnce() -> T) -> T {
    let (debut, cpu_debut) = (Instant::now(), cpu_du_fil());
    PILE.with(|p| p.borrow_mut().push((Duration::ZERO, Duration::ZERO)));
    let r = f();
    let (mur, cpu) = (debut.elapsed(), cpu_du_fil().saturating_sub(cpu_debut));
    let enfants = PILE.with(|p| {
        let mut p = p.borrow_mut();
        let enfants = p.pop().unwrap_or_default();
        if let Some(parent) = p.last_mut() {
            parent.0 += mur;
            parent.1 += cpu;
        }
        enfants
    });
    profil(|p| {
        let s = p.stats.entry((categorie.to_string(), nom.to_string())).or_default();
        s.fois += 1;
        s.mur += mur;
        s.cpu += cpu.saturating_sub(enfants.1);
        s.max = s.max.max(mur);
        p.version += 1;
    });
    r
}

/// Tout ce qui a ete mesure : (categorie, nom, stat).
pub fn instantane() -> Vec<(String, String, Stat)> {
    profil(|p| p.stats.iter().map(|((c, n), s)| (c.clone(), n.clone(), *s)).collect())
}

/// Le dossier des publications de cet utilisateur.
pub fn dossier() -> PathBuf {
    // SAFETY : getuid n'echoue jamais.
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/azure-perf-{uid}"))
}

/// Ce qu'une app a publie.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Publication {
    pub app: String,
    /// Quand (secondes depuis 1970) et depuis combien de temps elle mesure.
    pub quand: u64,
    pub depuis: Duration,
    pub stats: Vec<(String, String, Stat)>,
}

impl Publication {
    pub fn texte(&self) -> String {
        let mut s = format!("azure-perf 1\t{}\t{}\t{}\n", self.app, self.quand, self.depuis.as_millis());
        for (c, n, x) in &self.stats {
            let propre = |t: &str| t.replace(['\t', '\n'], " ");
            s.push_str(&format!("{}\t{}\t{}\t{}\t{}\t{}\n", propre(c), propre(n), x.fois, x.mur.as_micros(), x.cpu.as_micros(), x.max.as_micros()));
        }
        s
    }

    pub fn depuis_texte(texte: &str) -> Option<Publication> {
        let mut lignes = texte.lines();
        let tete: Vec<&str> = lignes.next()?.split('\t').collect();
        if tete.len() != 4 || tete[0] != "azure-perf 1" {
            return None;
        }
        let us = |x: &str| x.parse::<u64>().ok().map(Duration::from_micros);
        let stats = lignes
            .filter_map(|l| {
                let c: Vec<&str> = l.split('\t').collect();
                if c.len() != 6 {
                    return None;
                }
                Some((c[0].to_string(), c[1].to_string(), Stat { fois: c[2].parse().ok()?, mur: us(c[3])?, cpu: us(c[4])?, max: us(c[5])? }))
            })
            .collect();
        Some(Publication { app: tete[1].to_string(), quand: tete[2].parse().ok()?, depuis: Duration::from_millis(tete[3].parse().ok()?), stats })
    }

    /// Ce que l'app `app` a publie (`None` : rien, ou illisible).
    pub fn lire(app: &str) -> Option<Publication> {
        if app.contains('/') || app.starts_with('.') {
            return None;
        }
        Publication::depuis_texte(&std::fs::read_to_string(dossier().join(format!("{app}.txt"))).ok()?)
    }
}

/// Publie si 2 secondes ont passe et que quelque chose a change (appele
/// par la fenetre a chaque tic).
pub fn publier_si_temps() {
    const RYTHME: Duration = Duration::from_secs(2);
    // Une app pilotee par un essai ne se montre pas dans Azure Benchmark.
    if std::env::var_os("AZURE_PILOTE").is_some() {
        return;
    }
    let a_publier = profil(|p| {
        let app = p.app.clone()?;
        if p.publie.is_some_and(|(quand, v)| quand.elapsed() < RYTHME || v == p.version) {
            return None;
        }
        p.publie = Some((Instant::now(), p.version));
        let quand = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        Some(Publication { app, quand, depuis: p.debut.elapsed(), stats: p.stats.iter().map(|((c, n), s)| (c.clone(), n.clone(), *s)).collect() })
    });
    if let Some(p) = a_publier
        && let Err(e) = ecrire(&p)
    {
        // Une fois suffit : sans dossier, on ne publie plus.
        eprintln!("perf : publication impossible ({e})");
        profil(|x| x.app = None);
    }
}

fn ecrire(p: &Publication) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    let d = dossier();
    let _ = std::fs::DirBuilder::new().mode(0o700).create(&d);
    // Un dossier a quelqu'un d'autre (cree avant nous) : on n'y ecrit pas.
    // SAFETY : getuid n'echoue jamais.
    if std::fs::symlink_metadata(&d)?.uid() != unsafe { libc::getuid() } {
        return Err(std::io::Error::other(format!("{} n'est pas a nous", d.display())));
    }
    let tmp = d.join(format!(".{}.{}", p.app, std::process::id()));
    std::fs::write(&tmp, p.texte())?;
    std::fs::rename(tmp, d.join(format!("{}.txt", p.app)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generalise_les_identifiants() {
        assert_eq!(generaliser("lancer-12"), "lancer-#");
        assert_eq!(generaliser("commande-code-0@5@14"), "commande-code-#@#@#");
        assert_eq!(generaliser("onglet-tests"), "onglet-tests");
    }

    /// Le temps d'une mesure imbriquee n'est pas compte deux fois.
    #[test]
    fn les_mesures_imbriquees_comptent_a_part() {
        let travail = |ms| {
            let d = Instant::now();
            let mut x = 0u64;
            while d.elapsed() < Duration::from_millis(ms) {
                x = std::hint::black_box(x.wrapping_add(1));
            }
        };
        mesurer("Test-imbrique", "clic", || {
            travail(30);
            mesurer("Test-imbrique", "page", || travail(60));
        });
        let s: BTreeMap<String, Stat> = instantane().into_iter().filter(|x| x.0 == "Test-imbrique").map(|x| (x.1, x.2)).collect();
        let (clic, page) = (s["clic"], s["page"]);
        assert!(clic.mur >= Duration::from_millis(90) && page.mur >= Duration::from_millis(60), "{s:?}");
        assert!(clic.cpu < Duration::from_millis(55) && page.cpu >= Duration::from_millis(45), "le clic sans la page : {s:?}");
    }

    #[test]
    fn la_publication_se_relit() {
        let p = Publication { app: "note".into(), quand: 1_790_000_000, depuis: Duration::from_secs(12), stats: vec![("Clic".into(), "lancer-#".into(), Stat { fois: 3, mur: Duration::from_micros(1500), cpu: Duration::from_micros(900), max: Duration::from_micros(700) })] };
        assert_eq!(Publication::depuis_texte(&p.texte()), Some(p));
        assert_eq!(Publication::depuis_texte("autre chose"), None);
        assert_eq!(Publication::lire("../etc/passwd"), None);
    }
}
