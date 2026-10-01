// Chronometre, releve des mesures et outils partages par les parties du banc.
use azure_engine::rendering::models::canvas::Canvas;
use azure_foundation::navigation::models::route::Route;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::draw_ui::draw_ui;
use azure_foundation::ui::services::form::{form_values, FieldValue};
use azure_foundation::ui::services::interact;
use azure_testeur::clics::Lecture;
use azure_testeur::ecran::{routes, Testeur};
use azure_testeur::projet::{self, EnMemoire, Projet, Projets};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Une image a 60 Hz.
pub const IMAGE: Duration = Duration::from_millis(16);
/// Au-dela, la fenetre est figee (meme seuil que `AzureWindow : lent`).
pub const GEL: Duration = Duration::from_millis(100);

pub const VUE: (u32, u32) = (1280, 820);

#[derive(Clone)]
pub struct Mesure {
    pub zone: &'static str,
    pub nom: String,
    pub duree: Duration,
    /// Tourne sur le thread de la fenetre (sinon : thread a part).
    pub fenetre: bool,
}

static MESURES: Mutex<Vec<Mesure>> = Mutex::new(Vec::new());

pub fn noter(zone: &'static str, nom: impl Into<String>, fenetre: bool, duree: Duration) -> Duration {
    let nom = nom.into();
    eprintln!("  {:<9} {:<58} {:>10.2?}{}", zone, nom, duree, marque(duree, fenetre));
    MESURES.lock().unwrap().push(Mesure { zone, nom, duree, fenetre });
    duree
}

fn marque(d: Duration, fenetre: bool) -> &'static str {
    match (fenetre, d >= GEL, d >= IMAGE) {
        (false, _, _) => "  (thread a part)",
        (true, true, _) => "  <<< GEL",
        (true, false, true) => "  < saccade",
        _ => "",
    }
}

pub fn chrono<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let debut = Instant::now();
    let r = f();
    (r, debut.elapsed())
}

/// Le plus court de `n` essais (la machine peut etre occupee ailleurs).
pub fn meilleur(n: usize, mut f: impl FnMut()) -> Duration {
    (0..n).map(|_| chrono(&mut f).1).min().unwrap_or_default()
}

/// Mesure `f` (meilleur de 3) et la note.
pub fn mesure(zone: &'static str, nom: &str, fenetre: bool, f: impl FnMut()) -> Duration {
    noter(zone, nom, fenetre, meilleur(3, f))
}

/// Les mesures (d'une zone ou toutes), de la plus longue a la plus courte.
pub fn rapport(zone: Option<&str>) -> String {
    let mut m: Vec<Mesure> = MESURES.lock().unwrap().iter().filter(|m| zone.is_none_or(|z| m.zone == z)).cloned().collect();
    m.sort_by(|a, b| b.duree.cmp(&a.duree));
    let gels = m.iter().filter(|x| x.fenetre && x.duree >= GEL).count();
    let saccades = m.iter().filter(|x| x.fenetre && x.duree >= IMAGE && x.duree < GEL).count();
    let mut out = format!("{} mesures : {gels} gel(s) (>= 100 ms), {saccades} saccade(s) (>= 16 ms) sur le thread de la fenetre\n", m.len());
    for x in &m {
        out.push_str(&format!("{:>10.2?}  {:<9} {}{}\n", x.duree, x.zone, x.nom, marque(x.duree, x.fenetre)));
    }
    out
}

pub fn ecrire_rapport(nom: &str, texte: &str) {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join("azure-testeur");
    let _ = std::fs::create_dir_all(&d);
    let _ = std::fs::write(d.join(format!("banc-{nom}.txt")), texte);
}

/// Imprime le classement de la zone, puis echoue (en release) si quelque
/// chose fige la fenetre.
pub fn fin_de_zone(zone: &'static str) {
    let r = rapport(Some(zone));
    eprintln!("\n--- {zone} : classement ---\n{r}");
    ecrire_rapport(zone, &r);
    verifier_gels(Some(zone));
}

pub fn verifier_gels(zone: Option<&str>) {
    if cfg!(debug_assertions) || std::env::var_os("AZURE_BANC_SANS_SEUIL").is_some() {
        return;
    }
    let gels: Vec<String> = MESURES.lock().unwrap().iter().filter(|m| zone.is_none_or(|z| m.zone == z) && m.fenetre && m.duree >= GEL).map(|m| format!("{} {} ({:.0?})", m.zone, m.nom, m.duree)).collect();
    assert!(gels.is_empty(), "ce qui fige la fenetre :\n  {}", gels.join("\n  "));
}

// ------------------------------------------------------------------ app

pub fn environnement() -> Option<Projet> {
    let env = projet::environnement();
    if env.is_none() {
        eprintln!("(pas d'environnement Azure : `azure setup` n'a pas note les sources, partie sautee)");
    }
    env
}

pub fn testeur(env: Option<Projet>) -> Arc<Testeur> {
    Testeur::new(Projets::new(env, Box::new(EnMemoire::default())))
}

pub fn ui() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("ui")
}

/// L'ecran tel que la fenetre le construit (donnees + rsH + rsC).
pub fn construire(t: &Arc<Testeur>) -> Vec<UiNode> {
    routes(&ui(), t).resolve(&Route::new("/", "")).expect("rsH ou rsC invalide")
}

pub fn dessiner(nodes: &[UiNode]) {
    let mut c = Canvas::new(VUE.0, VUE.1);
    draw_ui(nodes, (0, 0, VUE.0, VUE.1), &mut c, -1, -1, false);
}

/// Les ids cliquables a l'ecran, dans l'ordre.
pub fn ids(nodes: &[UiNode]) -> Vec<String> {
    let mut out = Vec::new();
    interact::walk(nodes, (0, 0, VUE.0, 100_000), &mut |n, _| match n {
        UiNode::Button(b) if !b.id.is_empty() => out.push(b.id.clone()),
        UiNode::TextArea(t) if !t.id.is_empty() => out.push(t.id.clone()),
        UiNode::Control(c) if !c.id.is_empty() => out.push(c.id.clone()),
        _ => {}
    });
    out
}

/// Centre de l'element `id` a l'ecran.
pub fn position(nodes: &[UiNode], id: &str) -> Option<(i32, i32)> {
    let mut out = None;
    interact::walk(nodes, (0, 0, VUE.0, VUE.1), &mut |n, r| {
        let ok = match n {
            UiNode::Button(b) => b.id == id,
            UiNode::TextArea(t) => t.id == id,
            UiNode::Control(c) => c.id == id,
            _ => false,
        };
        if ok && out.is_none() {
            out = Some((r.0 + r.2 as i32 / 2, r.1 + r.3 as i32 / 2));
        }
    });
    out
}

/// Les champs de l'ecran, comme `WindowContext::value`.
pub struct Valeurs(pub BTreeMap<String, FieldValue>);

impl Valeurs {
    pub fn de(nodes: &[UiNode]) -> Valeurs {
        Valeurs(form_values(nodes))
    }
}

impl Lecture for Valeurs {
    fn valeur(&self, id: &str) -> Option<String> {
        self.0.get(id).map(FieldValue::as_text)
    }

    fn coche(&self, id: &str) -> bool {
        matches!(self.0.get(id), Some(FieldValue::Bool(true)))
    }
}

/// Un petit crate : un test qui passe, un qui echoue.
pub fn crate_demo(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("azure-testeur-banc-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::write(d.join("Cargo.toml"), format!("[package]\nname = \"{nom}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n")).unwrap();
    std::fs::write(
        d.join("src/lib.rs"),
        "pub fn double(x: i32) -> i32 {\n    x * 2\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn double_ok() {\n        assert_eq!(double(2), 4);\n    }\n\n    #[test]\n    fn double_faux() {\n        assert_eq!(double(2), 5);\n    }\n}\n",
    )
    .unwrap();
    d
}

/// Attend la fin de l'execution lancee (au plus `max`).
pub fn attendre(t: &Testeur, max: Duration) {
    let fin = Instant::now() + max;
    while t.lanceur.lire().en_cours && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(50));
    }
}
