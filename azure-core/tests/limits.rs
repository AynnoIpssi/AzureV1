// Plafond de connexions : au-dela, refuse ; une place rendue se reprend.
use azure_core::security::limits::ConnectionGate;
use std::os::unix::net::UnixStream;

#[test]
fn connections_are_capped_per_process_and_released() {
    let gate = ConnectionGate::new(2, 10);
    let pairs: Vec<(UnixStream, UnixStream)> = (0..3).map(|_| UnixStream::pair().unwrap()).collect();
    let a = gate.admit(&pairs[0].0).expect("1re");
    let _b = gate.admit(&pairs[1].0).expect("2e");
    assert!(gate.admit(&pairs[2].0).is_none(), "plafond par processus");
    drop(a);
    assert!(gate.admit(&pairs[2].0).is_some(), "place rendue");
    assert_eq!(gate.open_connections(), 1, "la derniere place a ete rendue a la fin de l'expression");
}

#[test]
fn the_total_cap_applies_too() {
    let gate = ConnectionGate::new(100, 1);
    let (x, _) = UnixStream::pair().unwrap();
    let _p = gate.admit(&x).unwrap();
    assert!(gate.admit(&x).is_none());
}
