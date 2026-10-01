// Etat realiste (fabrique par le vrai `Manager`) partage par les tests du
// tableau de bord.
#![allow(dead_code)]
use azure_dashboard::Dashboard;
use azure_manager::managers::manager::Manager;
use azure_manager::models::manifest::Manifest;
use azure_provider::{ServiceStatus, State};
use azure_service::flux::Value;
use std::path::Path;

pub fn state() -> Value {
    let summary = |text: &str| Manifest::parse(text, Path::new("/x")).unwrap().summary();
    let mut m = Manager::in_memory(vec!["/apps/tableau".into()]);
    m.register(summary("[app]\nname = boutique\ntitle = Boutique\nversion = 1.2\n[share panier]\nto = caisse\n[share annonces]\npublic = true\n[provide prix]\nto = caisse\ndescription = Prix d'un produit (en centimes)\n[service sync]\ncommand = x"), "/apps/boutique", None, 4101).unwrap();
    m.register(summary("[app]\nname = caisse\ntitle = Caisse\n[listen panier@boutique]\n[share commandes]\nto = stats\n[use prix@boutique]"), "/apps/caisse", None, 4102).unwrap();
    m.register(summary("[app]\nname = stats\ntitle = Statistiques\n[listen panier@boutique]\n[listen commandes@caisse]\n[use prix@boutique]"), "/apps/stats", None, 4103).unwrap();
    m.register(summary("[app]\nname = tableau-de-bord\ntitle = Azure Manager"), "/apps/tableau", None, 4104).unwrap();
    m.disconnected("stats");
    let service = |name: &str, state: State, pid, restarts, message: &str| ServiceStatus { name: name.into(), state, pid, restarts, uptime_secs: 40, message: message.into() };
    let services = [
        service("rooter", State::Running, Some(3120), 0, ""),
        service("stockage", State::Running, Some(3121), 1, "tue par le signal 9"),
        service("service", State::Running, Some(3122), 0, ""),
        service("manager", State::Running, Some(3123), 0, ""),
        service("boutique-sync", State::Failed, None, 4, "sorti avec le code 2 ; 5 plantages en 60 s, abandon"),
    ];
    m.report("boutique", azure_manager::managers::manager::Level::Error, "paiement refuse (carte expiree)");
    m.report("caisse", azure_manager::managers::manager::Level::Info, "caisse ouverte");
    let b = m.resolve("boutique").unwrap();
    let activity = azure_manager::managers::manager::Activity {
        flux: vec![azure_service::flux::protocol::FluxStats { owner: b, name: "panier".into(), changes: 128, listeners: 2, persist: true }],
        methods: vec![azure_service::flux::protocol::MethodStats { owner: b, method: "prix".into(), calls: 57, errors: 3, timeouts: 1, total_ms: 57 * 4, last_error: "produit inconnu".into(), served: true }],
    };
    m.state_with(&services, true, &activity)
}

pub fn dashboard() -> Dashboard {
    let mut dashboard = Dashboard::default();
    *dashboard.state.lock().unwrap() = state();
    dashboard.logs = Some(std::sync::Arc::new(|name: &str| match name {
        "stockage" => Ok(vec!["azure-stockage : donnees dans /home/ada/.local/share/azure/stockage".into(), "--- azure-provider : demarrage ---".into(), "connexion de l'app 1000".into()]),
        other => Err(format!("{other}.log introuvable")),
    }));
    dashboard
}

