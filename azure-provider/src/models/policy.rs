// Regles de relance : delai croissant apres chaque plantage, abandon quand
// un service plante trop souvent en peu de temps.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct RestartPolicy {
    /// Delai apres le 2e plantage ; double ensuite jusqu'a `max`.
    pub base: Duration,
    pub max: Duration,
    /// Nombre de plantages dans `window` au bout duquel on abandonne.
    pub max_crashes: usize,
    pub window: Duration,
    /// Temps laisse a un service pour que son socket reponde.
    pub start_timeout: Duration,
    /// Intervalle entre deux verifications du socket.
    pub health_interval: Duration,
    /// Verifications ratees d'affilee avant de considerer le service mort.
    pub health_failures: u32,
    /// Temps laisse apres SIGTERM avant SIGKILL.
    pub stop_timeout: Duration,
}

impl Default for RestartPolicy {
    fn default() -> RestartPolicy {
        RestartPolicy {
            base: Duration::from_secs(1),
            max: Duration::from_secs(30),
            max_crashes: 5,
            window: Duration::from_secs(60),
            start_timeout: Duration::from_secs(10),
            health_interval: Duration::from_secs(2),
            health_failures: 3,
            stop_timeout: Duration::from_secs(3),
        }
    }
}

impl RestartPolicy {
    /// Delai avant la relance, `recent` = plantages dans la fenetre (celui-ci
    /// compris) : 1er -> immediat, 2e -> base, 3e -> 2 x base, ... <= max.
    pub fn delay(&self, recent: usize) -> Duration {
        if recent <= 1 {
            return Duration::ZERO;
        }
        let factor = 1u32.checked_shl((recent - 2) as u32).unwrap_or(u32::MAX);
        self.base.saturating_mul(factor).min(self.max)
    }

    pub fn gives_up(&self, recent: usize) -> bool {
        recent >= self.max_crashes
    }
}

/// Dates des plantages recents d'un service.
#[derive(Clone, Debug, Default)]
pub struct CrashHistory {
    times: VecDeque<Instant>,
}

impl CrashHistory {
    /// Note un plantage a `now` et retourne combien il y en a eu dans
    /// `window` (celui-ci compris).
    pub fn record(&mut self, now: Instant, window: Duration) -> usize {
        self.times.push_back(now);
        while self.times.front().is_some_and(|t| now.saturating_duration_since(*t) > window) {
            self.times.pop_front();
        }
        self.times.len()
    }

    pub fn clear(&mut self) {
        self.times.clear();
    }
}
