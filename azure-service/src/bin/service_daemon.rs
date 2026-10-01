// Daemon azure-service.
//   service_daemon [--socket <chemin>] [--data <dossier>] [--manager <executable>]
fn main() {
    // Memoire illisible par les autres processus, fichiers prives.
    azure_core::security::hardening::harden_daemon();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(manager) = azure_service::arg_value(&args, "--manager-socket") {
        azure_service::managers::daemon::set_manager_socket(&manager);
    }
    let socket = azure_service::arg_value(&args, "--socket").unwrap_or_else(|| azure_service::SOCKET_PATH.to_string());
    let data = azure_service::arg_value(&args, "--data").map(std::path::PathBuf::from).unwrap_or_else(azure_service::default_data_dir);
    println!("azure-service : socket {socket}, donnees dans {}", data.display());
    let manager = azure_service::arg_value(&args, "--manager").or_else(azure_service::managers::daemon::default_manager_exe);
    if let Err(e) = azure_service::managers::daemon::start_daemon_with(&socket, &data, manager) {
        eprintln!("azure-service : {e}");
        std::process::exit(1);
    }
}
