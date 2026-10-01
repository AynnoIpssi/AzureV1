// Portfolio : app Azure. Manifeste : app.azure ; pages : ui/ (voir lib.rs).
use azure_portfolio::{link, routes, target};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    if let Err(e) = run() {
        eprintln!("portfolio : {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // Le manifeste : a cote de l'executable une fois installee, sinon ce dossier.
    let app = Arc::new(azure_foundation::azure_app!()?);
    let routes = routes(app.routes(), &app.manifest().dir.join("ui"));
    let window = app.window_with(routes)?;
    window
        .on_click(move |ctx| {
            let Some(clicked) = ctx.clicked else { return };
            if let Some(path) = target(clicked) {
                ctx.goto(path, "");
            } else if let Some((to, path)) = link(clicked) {
                // Par le routeur (et le manager si l'app est fermee).
                // Le jeton de ce clic : l'app ouverte passe au premier plan.
                let token = ctx.activation_token();
                let text = match app.open(to, &path, "", token.as_deref()) {
                    Ok(true) => "Lancement…",
                    Ok(false) => "Ouvert",
                    Err(e) => {
                        eprintln!("portfolio : ouvrir {to} : {e}");
                        "Indisponible"
                    }
                };
                ctx.flash(clicked, text, Duration::from_millis(1500));
            }
        })
        .run();
    Ok(())
}
