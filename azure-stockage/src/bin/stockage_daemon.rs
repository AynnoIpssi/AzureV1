// Daemon de stockage Azure.
//   stockage_daemon [--root <dossier>] [--socket <chemin>] [--admin <exe>]
// Sans --root : AZURE_STOCKAGE_ROOT, puis ~/.config/azure/stockage.conf
// (`root = ...`), puis ~/.local/share/azure/stockage (voir `resolve_root`).
fn main() {
    // La cle maitre est en memoire : illisible par les autres processus.
    azure_core::security::hardening::harden_daemon();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(manager) = azure_stockage::arg_value(&args, "--manager-socket") {
        azure_stockage::managers::daemon::set_manager_socket(&manager);
    }
    let socket = azure_stockage::arg_value(&args, "--socket").unwrap_or_else(|| azure_stockage::SOCKET_PATH.to_string());
    let result = azure_stockage::resolve_root(&args).and_then(|root| {
        println!("azure-stockage : donnees dans {}, socket {socket}", root.display());
        azure_stockage::managers::daemon::start_daemon_with(&socket, &root, azure_stockage::managers::daemon::Options::from_args(&args))
    });
    if let Err(e) = result {
        eprintln!("azure-stockage : {e}");
        std::process::exit(1);
    }
}
