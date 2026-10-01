// Ide : app Azure. Manifeste : app.azure ; pages : ui/.
use std::time::Duration;

fn main() {
    if let Err(e) = run() {
        eprintln!("ide : {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // Le manifeste : a cote de l'executable une fois installee, sinon ce dossier.
    let app = azure_foundation::azure_app!()?;
    app.window()?
        .on_click(|ctx| {
            if ctx.clicked == Some("bonjour") {
                ctx.flash("bonjour", "Bonjour !", Duration::from_millis(1200));
            }
        })
        .run();
    Ok(())
}
