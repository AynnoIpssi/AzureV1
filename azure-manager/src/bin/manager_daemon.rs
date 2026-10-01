// Daemon azure-manager.
//   manager_daemon [--socket S] [--data D] [--service-socket S] [--provider-socket S] [--admin <exe>]...
use azure_manager::managers::daemon::{run, ManagerConfig};

fn main() {
    // Pas `harden_daemon` : azure-service verifie l'executable du manager
    // (/proc/<pid>/exe), illisible d'un processus non inspectable. Les apps
    // enfermees ne peuvent de toute facon pas l'inspecter (Landlock).
    unsafe { libc::umask(0o077) };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str, default: &str| azure_manager::arg_value(&args, name).unwrap_or_else(|| default.to_string());
    let admins = azure_manager::arg_values(&args, "--admin");
    let config = ManagerConfig {
        socket: value("--socket", &azure_manager::SOCKET_PATH),
        data_dir: azure_manager::arg_value(&args, "--data").map(Into::into).unwrap_or_else(azure_manager::default_data_dir),
        service_socket: value("--service-socket", &azure_service::SOCKET_PATH),
        provider_socket: value("--provider-socket", &azure_provider::SOCKET_PATH),
        admins: if admins.is_empty() { azure_manager::default_admins() } else { admins },
        provider_logs: azure_manager::arg_value(&args, "--provider-logs").map(Into::into).unwrap_or_else(azure_provider::log_dir),
        app_logs: azure_manager::arg_value(&args, "--app-logs").map(Into::into).unwrap_or_else(azure_manager::managers::daemon::default_app_logs),
    };
    println!("azure-manager : socket {}, donnees dans {}", config.socket, config.data_dir.display());
    if let Err(e) = run(config) {
        eprintln!("azure-manager : {e}");
        std::process::exit(1);
    }
}
