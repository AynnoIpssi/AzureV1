// Ce qu'une app peut recevoir du routeur inter-app (voir
// `navigation_manager::listen`) : une navigation vers un de ses ecrans, ou
// une fenetre envoyee par une autre app.
use crate::navigation::models::route::Route;
use crate::window::models::shared_window::SharedWindow;

pub enum Incoming {
    Route(Route),
    Window(SharedWindow),
}

impl Incoming {
    /// Une fenetre mal formee est ignoree (message sur stderr) plutot que
    /// prise pour une `Route` : son texte n'a aucun sens comme chemin.
    pub fn decode(raw: &str) -> Option<Incoming> {
        if !SharedWindow::is_shared_window(raw) {
            return Some(Incoming::Route(Route::decode(raw)));
        }
        match SharedWindow::decode(raw) {
            Ok(window) => Some(Incoming::Window(window)),
            Err(err) => {
                eprintln!("Fenetre recue ignoree : {err}");
                None
            }
        }
    }
}
