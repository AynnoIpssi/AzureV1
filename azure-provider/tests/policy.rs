use azure_provider::models::policy::{CrashHistory, RestartPolicy};
use std::time::{Duration, Instant};

#[test]
fn delay_grows_then_is_capped() {
    let policy = RestartPolicy::default();
    let delays: Vec<u64> = (1..=8).map(|n| policy.delay(n).as_secs()).collect();
    assert_eq!(delays, [0, 1, 2, 4, 8, 16, 30, 30]);
    // Pas de debordement avec un nombre enorme.
    assert_eq!(policy.delay(500), Duration::from_secs(30));
}

#[test]
fn gives_up_after_five_crashes_in_a_minute() {
    let policy = RestartPolicy::default();
    let mut history = CrashHistory::default();
    let t0 = Instant::now();
    let counts: Vec<usize> = (0..5).map(|i| history.record(t0 + Duration::from_secs(i * 10), policy.window)).collect();
    assert_eq!(counts, [1, 2, 3, 4, 5]);
    assert!(!policy.gives_up(4));
    assert!(policy.gives_up(5));
}

#[test]
fn old_crashes_are_forgotten() {
    let window = Duration::from_secs(60);
    let mut history = CrashHistory::default();
    let t0 = Instant::now();
    history.record(t0, window);
    history.record(t0 + Duration::from_secs(30), window);
    // 61 s apres le premier : il sort de la fenetre, pas le deuxieme.
    assert_eq!(history.record(t0 + Duration::from_secs(61), window), 2);
    // Longtemps apres : repart de 1.
    assert_eq!(history.record(t0 + Duration::from_secs(1000), window), 1);
}
