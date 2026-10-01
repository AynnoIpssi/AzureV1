// `transition` (rsC) : les couleurs de survol / focus d'un bouton ou d'un
// champ changent progressivement au lieu de basculer d'un coup.
//
//   button { background: #333; transition: background-color 0.2s ease; }
//   button:hover { background: #555; }
//
// Chaque widget garde ou il en est (`Animated`) ; tant qu'une transition
// n'est pas finie, le dessin le signale (`request_frame`) et la fenetre
// redessine au tic suivant (voir `AzureWindow::run`).
use azure_engine::rendering::models::color::Color;
use azure_engine::rendering::models::paint::{ColorStop, Fill};
use std::cell::Cell;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    /// `cubic-bezier(x1, y1, x2, y2)` (`ease`, `ease-in`... en sont).
    Bezier(f32, f32, f32, f32),
    /// `steps(n)` : n paliers.
    Steps(u32),
}

impl Easing {
    pub fn from_css(name: &str) -> Option<Easing> {
        Some(match name {
            "linear" => Easing::Linear,
            "ease" => Easing::Bezier(0.25, 0.1, 0.25, 1.0),
            "ease-in" => Easing::Bezier(0.42, 0.0, 1.0, 1.0),
            "ease-out" => Easing::Bezier(0.0, 0.0, 0.58, 1.0),
            "ease-in-out" => Easing::Bezier(0.42, 0.0, 0.58, 1.0),
            "step-start" => Easing::Steps(1),
            "step-end" => Easing::Steps(1),
            _ => return None,
        })
    }

    /// Avancement visible pour un temps ecoule `t` (0 a 1).
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::Steps(n) => ((t * n as f32).floor() / n.max(1) as f32).min(1.0),
            Easing::Bezier(x1, y1, x2, y2) => {
                // x(s) = t, resolu par Newton puis dichotomie ; on rend y(s).
                let bez = |a: f32, b: f32, s: f32| 3.0 * a * s * (1.0 - s).powi(2) + 3.0 * b * s * s * (1.0 - s) + s * s * s;
                let dbez = |a: f32, b: f32, s: f32| 3.0 * a * (1.0 - s).powi(2) + 6.0 * (b - a) * s * (1.0 - s) + 3.0 * (1.0 - b) * s * s;
                let mut s = t;
                for _ in 0..8 {
                    let dx = dbez(x1, x2, s);
                    if dx.abs() < 1e-5 {
                        break;
                    }
                    s = (s - (bez(x1, x2, s) - t) / dx).clamp(0.0, 1.0);
                }
                let (mut lo, mut hi) = (0.0f32, 1.0f32);
                for _ in 0..20 {
                    if (bez(x1, x2, s) - t).abs() < 1e-4 {
                        break;
                    }
                    if bez(x1, x2, s) < t { lo = s } else { hi = s }
                    s = (lo + hi) / 2.0;
                }
                bez(y1, y2, s)
            }
        }
    }
}

/// Duree, delai et courbe (`transition: <propriete> <duree> [courbe] [delai]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    /// En secondes.
    pub duration: f32,
    pub delay: f32,
    pub easing: Easing,
}

/// Ou en est la transition d'un widget : 0 = etat de base, 1 = survole /
/// focalise.
#[derive(Debug, Clone, Default)]
pub struct Animated {
    progress: Cell<f32>,
    last: Cell<Option<Instant>>,
    /// Temps passe a attendre le delai (`transition-delay`) vers la cible.
    waited: Cell<f32>,
    target: Cell<bool>,
}

// L'etat d'animation n'entre pas dans la comparaison de deux widgets.
impl PartialEq for Animated {
    fn eq(&self, _: &Animated) -> bool {
        true
    }
}

impl Animated {
    /// Fait avancer vers `target` (survole ou non) et rend l'avancement
    /// visible (courbe appliquee). Demande une nouvelle image si ce n'est
    /// pas fini.
    pub fn step(&self, transition: &Transition, target: bool) -> f32 {
        let now = Instant::now();
        // Temps reel ecoule depuis l'image precedente ; remis a zero quand
        // la cible change (le widget a pu rester immobile longtemps avant).
        let mut dt = self.last.get().map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.0);
        self.last.set(Some(now));
        if self.target.get() != target {
            self.target.set(target);
            self.waited.set(0.0);
            dt = 0.0;
        }
        let goal = if target { 1.0 } else { 0.0 };
        let mut p = self.progress.get();
        if p != goal {
            if self.waited.get() < transition.delay {
                let wait = (transition.delay - self.waited.get()).min(dt);
                self.waited.set(self.waited.get() + wait);
                dt -= wait;
            }
            let speed = if transition.duration > 0.0 { dt / transition.duration } else { 1.0 };
            p = if goal > p { (p + speed).min(goal) } else { (p - speed).max(goal) };
            self.progress.set(p);
            if p != goal {
                request_frame();
            }
        }
        transition.easing.apply(p)
    }
}

thread_local! {
    static PENDING: Cell<bool> = const { Cell::new(false) };
}

/// Une transition n'est pas finie : il faudra redessiner.
pub fn request_frame() {
    PENDING.with(|p| p.set(true));
}

/// `true` (une seule fois) si une transition attend une nouvelle image.
pub fn take_pending() -> bool {
    PENDING.with(|p| p.replace(false))
}

pub fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    Color::new(l(a.r, b.r), l(a.g, b.g), l(a.b, b.b), l(a.a, b.a))
}

/// Remplissage intermediaire. Couleurs unies, ou degrades de meme forme :
/// melange arret par arret ; sinon bascule a mi-chemin.
pub fn mix_fill(a: &Fill, b: &Fill, t: f32) -> Fill {
    if t <= 0.0 {
        return a.clone();
    }
    if t >= 1.0 {
        return b.clone();
    }
    let stops = |x: &[ColorStop], y: &[ColorStop]| -> Vec<ColorStop> {
        x.iter().zip(y).map(|(p, q)| ColorStop { color: mix_color(p.color, q.color, t), position: p.position + (q.position - p.position) * t }).collect()
    };
    match (a, b) {
        (Fill::Solid(x), Fill::Solid(y)) => Fill::Solid(mix_color(*x, *y, t)),
        (Fill::Linear { angle: aa, stops: sa }, Fill::Linear { angle: ab, stops: sb }) if sa.len() == sb.len() => Fill::Linear { angle: aa + (ab - aa) * t, stops: stops(sa, sb) },
        (Fill::Radial { stops: sa }, Fill::Radial { stops: sb }) if sa.len() == sb.len() => Fill::Radial { stops: stops(sa, sb) },
        // Degrade <-> couleur : la couleur devient un degrade uni.
        (Fill::Solid(c), Fill::Linear { angle, stops: s }) => mix_fill(&Fill::Linear { angle: *angle, stops: s.iter().map(|p| ColorStop { color: *c, position: p.position }).collect() }, b, t),
        (Fill::Linear { angle, stops: s }, Fill::Solid(c)) => mix_fill(a, &Fill::Linear { angle: *angle, stops: s.iter().map(|p| ColorStop { color: *c, position: p.position }).collect() }, t),
        _ => {
            if t < 0.5 {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}
