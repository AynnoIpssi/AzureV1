// azure_provider : surveille les processus d'arriere-plan d'Azure.
//
//   azure_provider [--socket S] [--config F] [--logs D]   lance le provider
//   azure_provider status                                  etat des services
//   azure_provider start|stop|restart <service>
//   azure_provider shutdown                                arrete tout
use azure_provider::managers::daemon::{run, stop_on_signals, ProviderConfig};
use azure_provider::models::config;
use azure_provider::models::policy::RestartPolicy;
use azure_provider::services::client::ProviderClient;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let socket = azure_provider::arg_value(&args, "--socket").unwrap_or_else(|| azure_provider::SOCKET_PATH.to_string());
    let command = args.first().filter(|arg| !arg.starts_with("--")).cloned();
    let result = match command.as_deref() {
        None => serve(&args, socket),
        Some(command) => control(command, args.get(1).filter(|arg| !arg.starts_with("--")), &socket),
    };
    if let Err(e) = result {
        eprintln!("azure-provider : {e}");
        std::process::exit(1);
    }
}

fn serve(args: &[String], socket: String) -> Result<(), String> {
    azure_core::security::hardening::harden_daemon();
    let config_path = azure_provider::arg_value(args, "--config").map(PathBuf::from).or_else(azure_provider::config_path);
    let mut services = config::load(config_path.as_deref())?;
    // Les taches de fond des apps installees : connues des le
    // demarrage, lancees a la demande (sauf `autostart`).
    for spec in config::load_apps(&azure_provider::services_dir()) {
        if !services.iter().any(|known| known.name == spec.name) {
            services.push(spec);
        }
    }
    let log_dir = azure_provider::arg_value(args, "--logs").map(PathBuf::from).unwrap_or_else(azure_provider::log_dir);
    println!("azure-provider : socket {socket}, journaux dans {}", log_dir.display());
    for spec in &services {
        println!("  {} : {}{}", spec.name, spec.command, if spec.autostart { "" } else { " (a la demande)" });
    }
    stop_on_signals();
    run(ProviderConfig { socket, services, policy: RestartPolicy::default(), log_dir })
}

fn control(command: &str, name: Option<&String>, socket: &str) -> Result<(), String> {
    let mut client = ProviderClient::connect_at(socket)?;
    let need_name = || name.cloned().ok_or_else(|| format!("{command} : nom du service attendu"));
    match command {
        "status" => {
            println!("{:<16} {:<18} {:>8} {:>9} {:>8}  DERNIER MESSAGE", "SERVICE", "ETAT", "PID", "RELANCES", "DEPUIS");
            for s in client.status()? {
                let pid = s.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
                let uptime = if s.state.is_ready() { format!("{}s", s.uptime_secs) } else { "-".to_string() };
                println!("{:<16} {:<18} {:>8} {:>9} {:>8}  {}", s.name, s.state.label(), pid, s.restarts, uptime, s.message);
            }
            Ok(())
        }
        "start" => client.start(&need_name()?),
        "stop" => client.stop(&need_name()?),
        "restart" => client.restart(&need_name()?),
        "shutdown" => client.shutdown(),
        other => Err(format!("commande inconnue '{other}' (status, start, stop, restart, shutdown)")),
    }
}
