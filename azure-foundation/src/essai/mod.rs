//! Essais d'interface : une vraie app Azure, lancee sans fenetre dans un
//! Azure jetable, pilotee comme par quelqu'un (cliquer, remplir, taper,
//! faire defiler) et verifiee sur ce qu'elle montre.
//!
//! ```ignore
//! let env = Environnement::demarrer("mon-essai")?;
//! let mut app = env.lancer(env!("CARGO_BIN_EXE_azure_testeur"))?;
//! app.clic("onglet-atelier");
//! app.attendre_texte("FICHIERS DE TESTS");
//! app.remplir("chemin", "~/Dev/mon-projet");
//! app.verifier_texte("Relier");
//! app.capture("atelier");
//! ```
//!
//! Chaque app pilotee se mesure elle-meme (`AppPilotee::mesures`) : a la
//! fin de l'essai, une ligne `AZURE-SCENARIO` dans la sortie du test donne
//! son processeur, sa memoire, ses dessins et le temps de reponse de chaque
//! action - Azure Benchmark (onglet Scenarios) les lit.
//!
//! L'Azure jetable : les daemons (gestionnaire, stockage, service, routeur)
//! lances depuis ceux d'Azure installes, avec leurs sockets et leurs donnees
//! dans un dossier temporaire (`AZURE_RUNTIME_DIR`) : les vraies donnees ne
//! sont jamais touchees, et deux essais tournent cote a cote. L'app est
//! lancee avec `AZURE_ESSAI` (ni azure-provider, ni bac a sable : voir
//! `app::AppSockets`) et `AZURE_PILOTE` (sa fenetre devient une image en
//! memoire pilotee par un socket, voir `window::models::pilote`).
pub mod protocole;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Delai par defaut de `attendre_texte`.
pub const ATTENTE: Duration = Duration::from_secs(10);

/// Les daemons qu'un essai lance.
const DAEMONS: [&str; 4] = ["manager_daemon", "service_daemon", "stockage_daemon", "routeur_daemon"];

/// Ou un essai trouve les daemons quand le dossier d'Azure lui est ferme :
/// `~/.cache/azure/essai-bin` (voir `preparer_daemons`).
pub fn dossier_daemons_copies() -> PathBuf {
    let cache = std::env::var_os("XDG_CACHE_HOME").filter(|d| !d.is_empty()).map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache"))).unwrap_or_else(std::env::temp_dir);
    cache.join("azure/essai-bin")
}

/// Pour une app qui lance des essais depuis son bac a sable (Azure Testeur,
/// Azure Benchmark) : a appeler AVANT `azure_app!()`. Une fois enfermee,
/// l'app (et les tests qu'elle lance) ne peut plus lire le dossier d'Azure
/// (`~/.local/share/azure`, refuse a tout manifeste) : les daemons sont
/// copies la ou elle peut ecrire (`dossier_daemons_copies`), seulement s'ils
/// ont change.
pub fn preparer_daemons() -> Result<PathBuf, String> {
    let dest = dossier_daemons_copies();
    // Lancee par un essai : ce n'est pas elle qui lancera des scenarios.
    if std::env::var_os("AZURE_ESSAI").is_some() {
        return Ok(dest);
    }
    let source = azure_provider::install_root().join("bin");
    std::fs::create_dir_all(&dest).map_err(|e| format!("{} : {e}", dest.display()))?;
    for d in DAEMONS {
        let (de, vers) = (source.join(d), dest.join(d));
        let meta = std::fs::metadata(&de).map_err(|e| format!("{} : {e} (lancez `azure setup`)", de.display()))?;
        let pareil = std::fs::metadata(&vers).is_ok_and(|m| m.len() == meta.len() && m.modified().ok() >= meta.modified().ok());
        if !pareil {
            let tmp = dest.join(format!(".{d}.copie"));
            std::fs::copy(&de, &tmp).and_then(|_| std::fs::rename(&tmp, &vers)).map_err(|e| format!("{} : {e}", vers.display()))?;
        }
    }
    Ok(dest)
}

/// Les daemons d'Azure : installes (`azure setup`), sinon leur copie.
fn dossier_daemons() -> Result<PathBuf, String> {
    let installes = azure_provider::install_root().join("bin");
    let copies = dossier_daemons_copies();
    [installes.clone(), copies]
        .into_iter()
        // Ouvrable, pas seulement visible : un bac a sable laisse voir un
        // fichier (stat) sans permettre de le lire ni de le lancer.
        .find(|d| DAEMONS.iter().all(|x| std::fs::File::open(d.join(x)).is_ok()))
        .ok_or_else(|| format!("daemons d'Azure introuvables dans {} (lancez `azure setup`) ; depuis le bac a sable d'une app, appelez `essai::preparer_daemons()` avant `azure_app!()`", installes.display()))
}

/// Un Azure jetable : ses daemons tournent tant qu'il existe.
pub struct Environnement {
    pub dossier: PathBuf,
    daemons: Vec<Child>,
}

fn attendre_socket(chemin: &Path, daemon: &mut Child) -> Result<(), String> {
    let fin = Instant::now() + Duration::from_secs(10);
    while UnixStream::connect(chemin).is_err() {
        if let Ok(Some(statut)) = daemon.try_wait() {
            return Err(format!("{} s'est arrete ({statut})", chemin.display()));
        }
        if Instant::now() > fin {
            return Err(format!("{} ne repond pas", chemin.display()));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

impl Environnement {
    /// Lance les daemons d'Azure (ceux installes par `azure setup`) dans un
    /// dossier temporaire propre a cet essai.
    pub fn demarrer(nom: &str) -> Result<Environnement, String> {
        let bin = dossier_daemons()?;
        let dossier = std::env::temp_dir().join(format!("azure-essai-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        let run = dossier.join("run");
        std::fs::create_dir_all(&run).map_err(|e| format!("{} : {e}", run.display()))?;
        let mut env = Environnement { dossier: dossier.clone(), daemons: Vec::new() };
        let d = |x: &str| dossier.join(x).to_string_lossy().into_owned();
        let journaux = std::fs::File::create(dossier.join("daemons.log")).map_err(|e| e.to_string())?;
        let lancer = |env: &mut Environnement, exe: &str, args: Vec<String>, socket: &str| -> Result<(), String> {
            let mut c = Command::new(bin.join(exe));
            c.args(&args).env("AZURE_RUNTIME_DIR", &run).stdin(Stdio::null());
            if let (Ok(a), Ok(b)) = (journaux.try_clone(), journaux.try_clone()) {
                c.stdout(a).stderr(b);
            }
            // Ses daemons meurent avec l'essai, meme s'il plante.
            // SAFETY : un seul appel systeme entre fork et exec.
            unsafe {
                c.pre_exec(|| {
                    libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                    Ok(())
                });
            }
            let mut enfant = c.spawn().map_err(|e| format!("{exe} : {e}"))?;
            let r = attendre_socket(&run.join(format!("{socket}.sock")), &mut enfant);
            env.daemons.push(enfant);
            r
        };
        let manager_sock = run.join("manager.sock").to_string_lossy().into_owned();
        let moi = std::env::current_exe().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        lancer(&mut env, "manager_daemon", vec!["--data".into(), d("manager"), "--admin".into(), moi, "--provider-logs".into(), d("journaux"), "--app-logs".into(), d("journaux-apps")], "manager")?;
        lancer(&mut env, "service_daemon", vec!["--data".into(), d("service"), "--manager-socket".into(), manager_sock.clone()], "service")?;
        lancer(&mut env, "stockage_daemon", vec!["--root".into(), d("stockage"), "--manager-socket".into(), manager_sock.clone()], "stockage")?;
        lancer(&mut env, "routeur_daemon", vec!["--data".into(), d("routeur"), "--manager-socket".into(), manager_sock], "router")?;
        Ok(env)
    }

    /// Lance l'app `exe` (son manifeste : a cote, ou celui de son dossier de
    /// sources) sans fenetre, de 1280 x 820, et la prend en main.
    pub fn lancer(&self, exe: impl AsRef<Path>) -> Result<AppPilotee, String> {
        self.lancer_avec(exe, (1280, 820), &[])
    }

    /// Comme `lancer`, d'une autre taille, avec des variables d'environnement
    /// en plus (`("AZURE_APP_MANIFEST", "...")`...).
    pub fn lancer_avec(&self, exe: impl AsRef<Path>, taille: (u32, u32), vars: &[(&str, &str)]) -> Result<AppPilotee, String> {
        let exe = exe.as_ref();
        let nom = exe.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let socket = self.dossier.join(format!("pilote-{nom}-{}.sock", PILOTES.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
        let journal = self.dossier.join(format!("{nom}.log"));
        let sortie = std::fs::File::create(&journal).map_err(|e| e.to_string())?;
        let mut c = Command::new(exe);
        c.env("AZURE_RUNTIME_DIR", self.dossier.join("run"))
            .env("AZURE_ESSAI", "1")
            .env("AZURE_PILOTE", &socket)
            .env("AZURE_PILOTE_TAILLE", format!("{}x{}", taille.0, taille.1))
            .stdin(Stdio::null())
            .stdout(sortie.try_clone().map_err(|e| e.to_string())?)
            .stderr(sortie);
        for (k, v) in vars {
            c.env(k, v);
        }
        // SAFETY : un seul appel systeme entre fork et exec.
        unsafe {
            c.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                Ok(())
            });
        }
        let mut enfant = c.spawn().map_err(|e| format!("{} : {e}", exe.display()))?;
        let fin = Instant::now() + Duration::from_secs(30);
        let flux = loop {
            if let Ok(f) = UnixStream::connect(&socket) {
                break f;
            }
            if let Ok(Some(statut)) = enfant.try_wait() {
                let journal_txt = std::fs::read_to_string(&journal).unwrap_or_default();
                return Err(format!("{nom} s'est arretee au demarrage ({statut}) :\n{journal_txt}"));
            }
            if Instant::now() > fin {
                let _ = enfant.kill();
                return Err(format!("{nom} n'ouvre pas sa fenetre pilotee (voir {})", journal.display()));
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let lecteur = BufReader::new(flux.try_clone().map_err(|e| e.to_string())?);
        Ok(AppPilotee { nom, mesuree: false, enfant, flux, lecteur, journal })
    }
}

impl Drop for Environnement {
    fn drop(&mut self) {
        for d in &mut self.daemons {
            let _ = d.kill();
            let _ = d.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dossier);
    }
}

/// Ce qu'une app a coute pendant un essai, mesure par elle-meme.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesures {
    pub app: String,
    /// Depuis son lancement.
    pub duree: Duration,
    /// Temps de processeur de l'app, et des programmes qu'elle a lances
    /// (cargo pour Testeur...).
    pub cpu: Duration,
    pub cpu_enfants: Duration,
    pub memoire_max: u64,
    pub memoire_enfants_max: u64,
    /// Fois ou elle s'est endormie / a ete interrompue.
    pub reveils: u64,
    /// Images redessinees, leur cout total et le pire.
    pub dessins: u64,
    pub temps_dessins: Duration,
    pub dessin_max: Duration,
    /// Chaque action (`clic onglet-atelier`) et son temps de reponse.
    pub actions: Vec<(String, Duration)>,
}

/// Marque des lignes de mesures dans la sortie d'un test (lues par Azure
/// Benchmark, onglet Scenarios).
pub const MARQUE: &str = "AZURE-SCENARIO";

impl Mesures {
    fn depuis_reponse(app: &str, texte: &str) -> Mesures {
        let mut m = Mesures { app: app.to_string(), ..Mesures::default() };
        let ms = |v: &str| Duration::from_secs_f64(v.parse::<f64>().unwrap_or(0.0).max(0.0) / 1000.0);
        for l in texte.lines() {
            let Some((k, v)) = l.split_once('=') else { continue };
            match k {
                "duree_ms" => m.duree = ms(v),
                "cpu_ms" => m.cpu = ms(v),
                "cpu_enfants_ms" => m.cpu_enfants = ms(v),
                "memoire_max" => m.memoire_max = v.parse().unwrap_or(0),
                "memoire_enfants_max" => m.memoire_enfants_max = v.parse().unwrap_or(0),
                "reveils" => m.reveils = v.parse().unwrap_or(0),
                "dessins" => m.dessins = v.parse().unwrap_or(0),
                "dessins_ms" => m.temps_dessins = ms(v),
                "dessin_max_ms" => m.dessin_max = ms(v),
                "action" => {
                    if let Some((a, d)) = v.rsplit_once('|') {
                        m.actions.push((a.to_string(), ms(d)));
                    }
                }
                _ => {}
            }
        }
        m
    }

    /// L'action la plus lente et son temps.
    pub fn action_la_plus_lente(&self) -> Option<&(String, Duration)> {
        self.actions.iter().max_by_key(|(_, d)| *d)
    }

    /// Temps de dessin moyen.
    pub fn dessin_moyen(&self) -> Duration {
        if self.dessins == 0 { Duration::ZERO } else { self.temps_dessins / self.dessins as u32 }
    }

    /// Une ligne pour la sortie du test : `AZURE-SCENARIO\t...`.
    pub fn ligne(&self) -> String {
        let ms = |d: Duration| format!("{:.3}", d.as_secs_f64() * 1000.0);
        let actions = self.actions.iter().map(|(a, d)| format!("{a}|{}", ms(*d))).collect::<Vec<_>>().join(";");
        protocole::ligne(&[
            MARQUE,
            &self.app,
            &ms(self.duree),
            &ms(self.cpu),
            &ms(self.cpu_enfants),
            &self.memoire_max.to_string(),
            &self.memoire_enfants_max.to_string(),
            &self.reveils.to_string(),
            &self.dessins.to_string(),
            &ms(self.temps_dessins),
            &ms(self.dessin_max),
            &actions,
        ])
    }

    /// Les mesures ecrites dans la sortie d'un test (une par app lancee).
    /// La marque peut suivre le nom du test sur la meme ligne
    /// (`test x ... AZURE-SCENARIO\t...`) ; le dernier champ peut etre vide :
    /// on ne retire que la fin de ligne, pas les tabulations.
    pub fn dans_la_sortie<'a>(lignes: impl IntoIterator<Item = &'a str>) -> Vec<Mesures> {
        lignes.into_iter().filter_map(|l| l.find(MARQUE).and_then(|i| Mesures::depuis_ligne(l[i..].trim_end_matches(['\n', '\r'])))).collect()
    }

    pub fn depuis_ligne(ligne: &str) -> Option<Mesures> {
        let c = protocole::champs(ligne);
        if c.len() != 12 || c[0] != MARQUE {
            return None;
        }
        let ms = |v: &str| v.parse::<f64>().ok().map(|x| Duration::from_secs_f64(x.max(0.0) / 1000.0));
        let actions = c[11].split(';').filter(|x| !x.is_empty()).filter_map(|x| x.rsplit_once('|').and_then(|(a, d)| Some((a.to_string(), ms(d)?)))).collect();
        Some(Mesures {
            app: c[1].clone(),
            duree: ms(&c[2])?,
            cpu: ms(&c[3])?,
            cpu_enfants: ms(&c[4])?,
            memoire_max: c[5].parse().ok()?,
            memoire_enfants_max: c[6].parse().ok()?,
            reveils: c[7].parse().ok()?,
            dessins: c[8].parse().ok()?,
            temps_dessins: ms(&c[9])?,
            dessin_max: ms(&c[10])?,
            actions,
        })
    }
}

static PILOTES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Une app lancee sans fenetre, prise en main. Les methodes qui agissent
/// (`clic`, `remplir`...) s'arretent sur une erreur claire (« pas de #id a
/// l'ecran (visibles : ...) ») : c'est un essai, l'echec doit dire pourquoi.
/// Les `essayer_*` rendent l'erreur au lieu de s'arreter.
pub struct AppPilotee {
    pub nom: String,
    /// Les mesures ont ete ecrites dans la sortie du test.
    mesuree: bool,
    enfant: Child,
    flux: UnixStream,
    lecteur: BufReader<UnixStream>,
    journal: PathBuf,
}

impl AppPilotee {
    /// Envoie une commande, rend ses donnees.
    pub fn envoyer(&mut self, champs: &[&str]) -> Result<String, String> {
        writeln!(self.flux, "{}", protocole::ligne(champs)).map_err(|e| self.mort(&e.to_string()))?;
        let mut reponse = String::new();
        match self.lecteur.read_line(&mut reponse) {
            Ok(0) | Err(_) => return Err(self.mort("pas de reponse")),
            Ok(_) => {}
        }
        let c = protocole::champs(&reponse);
        match c.first().map(String::as_str) {
            Some("ok") => Ok(c.get(1).cloned().unwrap_or_default()),
            Some("erreur") => Err(c.get(1).cloned().unwrap_or_default()),
            _ => Err(format!("reponse illisible : {reponse}")),
        }
    }

    // L'app est tombee : ce qu'elle a ecrit.
    fn mort(&mut self, quoi: &str) -> String {
        let fin = std::fs::read_to_string(&self.journal).unwrap_or_default();
        let fin: Vec<&str> = fin.lines().rev().take(30).collect();
        let fin: Vec<&str> = fin.into_iter().rev().collect();
        format!("{} ne repond plus ({quoi}) ; son journal :\n{}", self.nom, fin.join("\n"))
    }

    fn faire(&mut self, champs: &[&str]) -> String {
        match self.envoyer(champs) {
            Ok(d) => d,
            Err(e) => panic!("{} : {} : {e}", self.nom, champs.join(" ")),
        }
    }

    /// Clique au milieu de `#id`.
    pub fn clic(&mut self, id: &str) {
        self.faire(&["clic", id]);
    }

    pub fn essayer_clic(&mut self, id: &str) -> Result<(), String> {
        self.envoyer(&["clic", id]).map(drop)
    }

    /// Met la souris sur `#id` (survol, info-bulle).
    pub fn survoler(&mut self, id: &str) {
        self.faire(&["survoler", id]);
    }

    /// Met la souris en `(x, y)` (sans bouton), par exemple sur une boite de
    /// toile (`centre_boite`).
    pub fn souris(&mut self, x: i32, y: i32) {
        self.faire(&["souris", &x.to_string(), &y.to_string()]);
    }

    /// Donne sa valeur au champ `#id` (texte ; case : `true`/`false` ; choix :
    /// la valeur ou le libelle de l'option ; curseur : un nombre).
    pub fn remplir(&mut self, id: &str, valeur: &str) {
        self.faire(&["remplir", id, valeur]);
    }

    /// Choisit `valeur` (valeur ou libelle) dans la liste deroulante `#id`,
    /// comme quelqu'un : l'ouvrir, cliquer l'option (l'app recoit le clic).
    pub fn choisir(&mut self, id: &str, valeur: &str) {
        self.faire(&["choisir", id, valeur]);
    }

    /// Tape `texte` dans le champ qui a le focus (cliquez-le d'abord).
    pub fn taper(&mut self, texte: &str) {
        self.faire(&["taper", texte]);
    }

    /// `entree`, `echap`, `tab`, `espace`, `retour`, `suppr`, `haut`, `bas`,
    /// `gauche`, `droite`.
    pub fn touche(&mut self, nom: &str) {
        self.faire(&["touche", nom]);
    }

    /// La molette au-dessus de `#id` (positif : vers le bas, en pixels).
    pub fn molette(&mut self, id: &str, dy: f64) {
        self.faire(&["molette", id, &dy.to_string()]);
    }

    /// Le centre a l'ecran de la boite `boite` de la toile `#toile`.
    pub fn centre_boite(&mut self, toile: &str, boite: &str) -> (i32, i32) {
        let r = self.faire(&["boite", toile, boite]);
        let (x, y) = r.split_once('\t').unwrap_or(("0", "0"));
        (x.parse().unwrap_or(0), y.parse().unwrap_or(0))
    }

    /// Appuie en `de`, glisse, relache en `vers` (coordonnees ecran).
    pub fn glisser(&mut self, de: (i32, i32), vers: (i32, i32)) {
        self.faire(&["glisser", &de.0.to_string(), &de.1.to_string(), &vers.0.to_string(), &vers.1.to_string()]);
    }

    /// Glisse une boite d'une toile de `(dx, dy)` pixels.
    pub fn deplacer_boite(&mut self, toile: &str, boite: &str, dx: i32, dy: i32) {
        let (x, y) = self.centre_boite(toile, boite);
        self.glisser((x, y), (x + dx, y + dy));
    }

    /// En mode relier : glisse de la boite `de` a la boite `vers`.
    pub fn relier_boites(&mut self, toile: &str, de: &str, vers: &str) {
        let (a, b) = (self.centre_boite(toile, de), self.centre_boite(toile, vers));
        self.glisser(a, b);
    }

    /// Laisse tourner l'app `n` tics (~16 ms chacun : `on_tick`, animations).
    pub fn tics(&mut self, n: usize) {
        self.faire(&["tics", &n.to_string()]);
    }

    /// Les textes visibles a l'ecran, un par ligne.
    pub fn texte(&mut self) -> String {
        self.faire(&["texte"])
    }

    /// Les ids des elements visibles.
    pub fn ids(&mut self) -> Vec<String> {
        self.faire(&["ids"]).lines().map(str::to_string).collect()
    }

    /// Change la taille de la fenetre.
    pub fn taille(&mut self, largeur: u32, hauteur: u32) {
        self.faire(&["taille", &largeur.to_string(), &hauteur.to_string()]);
    }

    /// `texte` est-il visible ?
    pub fn voit(&mut self, texte: &str) -> bool {
        self.texte().contains(texte)
    }

    /// S'arrete si `texte` n'est pas visible (en montrant ce qui l'est).
    pub fn verifier_texte(&mut self, texte: &str) {
        let tout = self.texte();
        assert!(tout.contains(texte), "{} : « {texte} » n'est pas à l'écran. On voit :\n{tout}", self.nom);
    }

    /// S'arrete si `texte` est visible.
    pub fn verifier_absent(&mut self, texte: &str) {
        let tout = self.texte();
        assert!(!tout.contains(texte), "{} : « {texte} » est à l'écran alors qu'il ne devrait pas", self.nom);
    }

    /// Attend (au plus `ATTENTE`) que `texte` soit visible.
    pub fn attendre_texte(&mut self, texte: &str) {
        self.attendre_texte_pendant(texte, ATTENTE);
    }

    pub fn attendre_texte_pendant(&mut self, texte: &str, delai: Duration) {
        let fin = Instant::now() + delai;
        loop {
            let tout = self.texte();
            if tout.contains(texte) {
                return;
            }
            assert!(Instant::now() < fin, "{} : « {texte} » n'est pas apparu en {delai:?}. On voit :\n{tout}", self.nom);
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// L'image de la fenetre, en PPM, dans `chemin` (`.ppm` ajoute si absent).
    pub fn capture_vers(&mut self, chemin: impl AsRef<Path>) -> PathBuf {
        let mut p = chemin.as_ref().to_path_buf();
        if p.extension().is_none() {
            p.set_extension("ppm");
        }
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        self.faire(&["capture", &p.to_string_lossy()]);
        p
    }

    /// L'image de la fenetre dans le dossier temporaire du systeme
    /// (`azure-essais/<app>/<nom>.ppm`) ; rend son chemin.
    pub fn capture(&mut self, nom: &str) -> PathBuf {
        let p = std::env::temp_dir().join("azure-essais").join(&self.nom).join(nom);
        self.capture_vers(p)
    }

    /// Ce que l'app a coute depuis son lancement (processeur, memoire,
    /// dessins, temps de reponse de chaque action).
    pub fn mesures(&mut self) -> Mesures {
        let texte = self.faire(&["mesures"]);
        Mesures::depuis_reponse(&self.nom, &texte)
    }

    // A la fin de l'essai : ses mesures dans la sortie du test (Azure
    // Benchmark les lit), une seule fois.
    fn publier_mesures(&mut self) {
        if self.mesuree {
            return;
        }
        self.mesuree = true;
        if let Ok(texte) = self.envoyer(&["mesures"]) {
            println!("{}", Mesures::depuis_reponse(&self.nom, &texte).ligne());
        }
    }

    /// Ferme l'app comme par son bouton (`on_close` compris).
    pub fn fermer(mut self) {
        self.publier_mesures();
        let _ = writeln!(self.flux, "fermer");
        let fin = Instant::now() + Duration::from_secs(5);
        while Instant::now() < fin {
            if let Ok(Some(_)) = self.enfant.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for AppPilotee {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.publier_mesures();
        }
        if let Ok(None) = self.enfant.try_wait() {
            let _ = self.enfant.kill();
        }
        let _ = self.enfant.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesures_aller_retour() {
        let m = Mesures {
            app: "azure_testeur".into(),
            duree: Duration::from_millis(1500),
            cpu: Duration::from_millis(120),
            cpu_enfants: Duration::from_millis(900),
            memoire_max: 50 << 20,
            memoire_enfants_max: 300 << 20,
            reveils: 420,
            dessins: 12,
            temps_dessins: Duration::from_millis(96),
            dessin_max: Duration::from_millis(31),
            actions: vec![("clic lier".into(), Duration::from_millis(40)), ("remplir chemin".into(), Duration::from_millis(3))],
        };
        let lu = Mesures::dans_la_sortie(["autre chose", &m.ligne(), "test x ... ok"]);
        assert_eq!(lu, vec![m.clone()]);
        // Sans action (dernier champ vide), et apres le nom du test.
        let vide = Mesures { app: "a".into(), ..Mesures::default() };
        let ligne = format!("test x ... {}", vide.ligne());
        assert!(ligne.ends_with('\t'));
        assert_eq!(Mesures::dans_la_sortie([ligne.as_str()]), vec![vide]);
        assert_eq!(m.action_la_plus_lente().map(|a| a.0.as_str()), Some("clic lier"));
        assert_eq!(m.dessin_moyen(), Duration::from_millis(8));
    }
}
