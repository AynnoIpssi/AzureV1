// Routeur inter-app.
//   routeur_daemon [--data <dossier>]   (messages en attente ; defaut ~/.local/share/azure/rooter)
fn main() {
    // Memoire illisible par les autres processus, fichiers prives.
    azure_core::security::hardening::harden_daemon();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(manager) = args.iter().position(|a| a == "--manager-socket").and_then(|i| args.get(i + 1)) {
        azure_rooter::managers::router::set_manager_socket(manager);
    }
    let data = args.iter().position(|a| a == "--data").and_then(|i| args.get(i + 1)).map(std::path::PathBuf::from).unwrap_or_else(azure_rooter::default_data_dir);
    if let Err(e) = azure_rooter::managers::router::start_router_with(&azure_rooter::SOCKET_PATH, Some(&data)) {
        eprintln!("Erreur: {}", e);
        std::process::exit(1);
    }
}
