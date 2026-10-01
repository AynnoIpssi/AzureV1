// azure_dashboard : le tableau de bord d'Azure. Montre les apps, qui parle
// a qui et l'etat des services ; regle les partages. L'etat vient en temps
// reel du flux `etat` d'azure-manager ; les reglages lui sont envoyes.
use azure_dashboard::{action, routes, Action, Dashboard};
use azure_manager::services::client::ManagerClient;
use azure_service::flux::FluxEvent;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn text_of(value: &azure_service::flux::Value, path: &str) -> String {
    match value.get(path) {
        Some(azure_service::flux::Value::Int(n)) => n.to_string(),
        Some(other) => format!("{other:?}"),
        None => "?".to_string(),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("azure-dashboard : {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // Installe (`azure install azure-dashboard`) ou lance par cargo run.
    let app = azure_foundation::azure_app!()?;
    let dir = app.manifest().dir.clone();
    let admin = Arc::new(Mutex::new(ManagerClient::connect()?));
    let mut dashboard = Dashboard::default();
    let reader = Arc::clone(&admin);
    dashboard.logs = Some(Arc::new(move |name: &str| reader.lock().map_err(|_| "manager indisponible".to_string())?.logs(name, 300)));
    // Le terminal : le manager lance `azure` hors du bac a sable.
    // Une connexion par commande : une compilation ne bloque pas les autres
    // actions du tableau de bord.
    dashboard.runner = Some(Arc::new(|args: &[String]| ManagerClient::connect()?.azure_command(args)));
    *dashboard.state.lock().map_err(|_| "etat indisponible")? = admin.lock().map_err(|_| "manager indisponible")?.state()?;

    // Le manager partage `etat` avec le tableau de bord juste apres son
    // enregistrement : on laisse un instant au partage.
    let deadline = Instant::now() + Duration::from_secs(3);
    let listener = loop {
        match app.flux()?.listen(azure_manager::MANAGER_ID, azure_manager::STATE_FLUX).start() {
            Ok(listener) => break listener,
            Err(e) if Instant::now() >= deadline => return Err(e),
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    };

    let table = routes(&dir.join("ui"), &dashboard);
    let (live, clicks, errors, ticks) = (dashboard.clone(), dashboard.clone(), dashboard.clone(), dashboard.clone());
    let mut fenetre = app.window_with(table)?;
    if let Some(icone) = app.manifest().icon.as_ref().and_then(|p| p.to_str()) {
        fenetre = fenetre.icon(icone);
    }
    fenetre
        .flux(listener, move |ctx, event, etat| {
            // Le temps reel s'arrete sur un refus ; une coupure se rattrape
            // seule. Note au journal pour comprendre un etat fige.
            match event {
                FluxEvent::Update { seq, .. } | FluxEvent::Snapshot { seq } => {
                    if std::env::var_os("AZURE_DEBUG_ETAT").is_some() {
                        eprintln!("azure-dashboard : etat recu (n° {seq}, {} app(s) active(s))", text_of(etat, "nb_actives"));
                    }
                }
                other => eprintln!("azure-dashboard : flux etat : {other:?}"),
            }
            if let Ok(mut state) = live.state.lock() {
                *state = etat.clone();
            }
            // Redessine la page ouverte avec le nouvel etat.
            ctx.goto(&live.current(), "");
        })
        .on_click(move |ctx| {
            let Some(clicked) = ctx.clicked else { return };
            let Some(todo) = action(clicked, &clicks.current(), &clicks.state()) else { return };
            // Terminal : `execute` passe lui-meme par le manager.
            match todo {
                Action::Execute => {
                    if let Ok(mut draft) = clicks.draft.lock() {
                        draft.clear();
                    }
                    clicks.start(&ctx.value("commande").unwrap_or_default());
                    ctx.goto("/terminal", "");
                    ctx.scroll_to("commande");
                    return;
                }
                Action::Command(line) => {
                    if let Ok(mut draft) = clicks.draft.lock() {
                        *draft = ctx.value("commande").unwrap_or_default();
                    }
                    clicks.start(&line);
                    ctx.goto("/terminal", "");
                    ctx.scroll_to("commande");
                    return;
                }
                Action::Build(name) => {
                    match clicks.start_build(&name) {
                        Ok(()) => errors.set_error(""),
                        Err(e) => errors.set_error(&format!("Compilation impossible : {e}")),
                    }
                    ctx.goto(&clicks.current(), "");
                    return;
                }
                _ => {}
            }
            let Ok(mut admin) = admin.lock() else { return };
            let result = match todo {
                Action::Goto(path) => {
                    ctx.goto(&path, "");
                    Ok(())
                }
                Action::Grant { kind, owner, name, app } => admin.grant(kind, &owner, &name, &app),
                Action::Revoke { kind, owner, name, app } => admin.revoke(kind, &owner, &name, &app),
                Action::SetPublic { kind, owner, name, public } => admin.set_public(kind, &owner, &name, public),
                Action::Reset { kind, owner, name } => admin.reset(kind, &owner, &name),
                Action::Restart(service) => admin.restart_service(&service),
                Action::AskForget(name) => {
                    ctx.goto(&format!("/app/{name}"), "oublier");
                    Ok(())
                }
                Action::Forget(name) => admin.forget(&name).map(|()| {
                    ctx.goto("/", "");
                }),
                Action::Execute | Action::Command(_) | Action::Build(_) => Ok(()),
            };
            // Une action refusee s'affiche en haut de la page ; la suivante
            // qui reussit l'efface.
            match result {
                Ok(()) => errors.set_error(""),
                Err(e) => {
                    eprintln!("azure-dashboard : {e}");
                    errors.set_error(&format!("Action refusée : {e}"));
                    ctx.goto(&errors.current(), "");
                }
            }
        })
        // Une commande du terminal (ou une compilation) a fini (thread a
        // part) : on redessine la page, en gardant ce qui est tape.
        .on_tick(move |ctx| {
            let current = ticks.current();
            if current.starts_with("/app/") {
                if ticks.take_refresh() {
                    ctx.goto(&current, "");
                }
                return;
            }
            if current != "/terminal" || !ticks.take_refresh() {
                return;
            }
            if let Ok(mut draft) = ticks.draft.lock() {
                *draft = ctx.value("commande").unwrap_or_default();
            }
            ctx.goto("/terminal", "");
            ctx.scroll_to("commande");
        })
        .run();
    Ok(())
}
