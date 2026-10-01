// azure_manager : voir et regler la communication entre apps.
//
//   azure_manager apps                          les apps, actives ou non
//   azure_manager liens                         qui ecoute qui
//   azure_manager evenements                    ce que les apps signalent
//   azure_manager grant  <app> <flux> <autre>   autoriser <autre> a ecouter
//   azure_manager revoke <app> <flux> <autre>   retirer ce droit
//   azure_manager public <app> <flux> oui|non
//   azure_manager reset  <app> <flux>           revenir au manifeste
//   (grant-appel, revoke-appel, public-appel, reset-appel : pareil pour une
//   methode `[provide]`)
//   azure_manager forget <app>                  oublier une app
//   (--socket S pour un autre daemon)
use azure_manager::managers::manager::Kind;
use azure_manager::services::client::ManagerClient;
use azure_service::flux::Value;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let socket = azure_manager::arg_value(&args, "--socket").unwrap_or_else(|| azure_manager::SOCKET_PATH.to_string());
    let words: Vec<&str> = args.iter().map(String::as_str).take_while(|a| *a != "--socket").collect();
    if let Err(e) = run(&words, &socket) {
        eprintln!("azure-manager : {e}");
        std::process::exit(1);
    }
}

fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::Text(t)) => t.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn run(words: &[&str], socket: &str) -> Result<(), String> {
    let mut client = ManagerClient::connect_at(socket)?;
    match words {
        ["apps"] | [] => {
            let state = client.state()?;
            println!("{:<16} {:>6} {:<8} {:>8}  PARTAGE", "APP", "ID", "ETAT", "PID");
            for app in state.get("apps").and_then(Value::as_list).unwrap_or(&[]) {
                let actif = app.get("actif").and_then(Value::as_bool).unwrap_or(false);
                let shares: Vec<String> = app
                    .get("partages")
                    .and_then(Value::as_list)
                    .unwrap_or(&[])
                    .iter()
                    .map(|s| if s.get("public").and_then(Value::as_bool) == Some(true) { format!("{} (public)", text(s.get("flux"))) } else { format!("{} -> {}", text(s.get("flux")), text(s.get("autorises"))) })
                    .collect();
                let pid = if actif { text(app.get("pid")) } else { "-".to_string() };
                println!("{:<16} {:>6} {:<8} {:>8}  {}", text(app.get("nom")), text(app.get("id")), if actif { "actif" } else { "arrete" }, pid, shares.join(" ; "));
            }
            Ok(())
        }
        ["evenements"] => {
            for e in client.state()?.get("evenements").and_then(Value::as_list).unwrap_or(&[]) {
                println!("{} {:<16} {:<10} {}", text(e.get("heure")), text(e.get("app")), text(e.get("niveau")), text(e.get("message")));
            }
            Ok(())
        }
        ["liens"] => {
            for link in client.state()?.get("liens").and_then(Value::as_list).unwrap_or(&[]) {
                println!("{} {} « {} » de {} : {}", text(link.get("vers")), text(link.get("verbe")), text(link.get("nom")), text(link.get("de")), text(link.get("statut")));
            }
            Ok(())
        }
        [verb, owner, name, rest @ ..] if verb.ends_with("-appel") || ["grant", "revoke", "public", "reset"].contains(verb) => {
            let (base, kind) = match verb.strip_suffix("-appel") {
                Some(base) => (base, Kind::Call),
                None => (*verb, Kind::Flux),
            };
            match (base, rest) {
                ("grant", [app]) => client.grant(kind, owner, name, app),
                ("revoke", [app]) => client.revoke(kind, owner, name, app),
                ("public", [choice]) => client.set_public(kind, owner, name, azure_provider::models::config::parse_bool(choice)?),
                ("reset", []) => client.reset(kind, owner, name),
                _ => Err(format!("{verb} : arguments attendus manquants")),
            }
        }
        ["forget", app] => client.forget(app),
        _ => Err("usage : azure_manager apps | liens | evenements | grant|revoke <app> <flux> <autre> | public <app> <flux> oui|non | reset <app> <flux> | forget <app> (et grant-appel... pour une methode)".to_string()),
    }
}
