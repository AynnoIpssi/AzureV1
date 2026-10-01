// Evenements et compteurs survivent a un redemarrage du manager, et les
// compteurs d'azure-service restent cumulatifs quand lui redemarre.
use azure_manager::managers::manager::{Activity, Level, Manager};
use azure_service::flux::protocol::{FluxStats, MethodStats};
use std::path::PathBuf;

fn method(calls: u64, errors: u64) -> MethodStats {
    MethodStats { owner: 1000, method: "prix".into(), calls, errors, timeouts: 0, total_ms: calls * 2, last_error: String::new(), served: true }
}

fn flux(changes: u64) -> FluxStats {
    FluxStats { owner: 1000, name: "panier".into(), changes, listeners: 1, persist: true }
}

#[test]
fn events_and_counters_survive_restarts() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("manager-activite");
    let _ = std::fs::remove_dir_all(&dir);
    {
        let mut m = Manager::open(&dir, vec![]).unwrap();
        m.report("boutique", Level::Error, "stock introuvable");
        let a = m.absorb(Activity { flux: vec![flux(5)], methods: vec![method(10, 1)] });
        assert_eq!((a.flux[0].changes, a.methods[0].calls), (5, 10));
        // azure-service redemarre : ses compteurs repartent de zero.
        let a = m.absorb(Activity { flux: vec![flux(2)], methods: vec![method(3, 0)] });
        assert_eq!((a.flux[0].changes, a.methods[0].calls, a.methods[0].errors, a.methods[0].total_ms), (7, 13, 1, 26));
        m.save_activity().unwrap();
    }
    // Le manager redemarre.
    let mut m = Manager::open(&dir, vec![]).unwrap();
    let events: Vec<_> = m.events().map(|e| (e.app.clone(), e.message.clone())).collect();
    assert_eq!(events, vec![("boutique".to_string(), "stock introuvable".to_string())]);
    let a = m.absorb(Activity { flux: vec![flux(4)], methods: vec![method(5, 0)] });
    assert_eq!((a.flux[0].changes, a.methods[0].calls), (9, 15), "suite du total, sans recompter");
    // Chiffre sur disque.
    assert_eq!(&std::fs::read(dir.join("activite.bin")).unwrap()[..4], b"AZS1");
}
