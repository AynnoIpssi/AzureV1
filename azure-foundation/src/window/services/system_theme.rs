// Detecte le theme du bureau (sombre/clair, disposition des boutons de
// fenetre) via `gsettings`, exactement comme `ui::services::interact::detect_keyboard_layout`
// lit `localectl` : on interroge le reglage que GNOME/KDE (via son pont
// gsettings) utilisent eux-memes, plutot que d'essayer de parser le CSS
// d'un theme GTK arbitraire (hors de portee ici, azure-foundation ne lie
// aucun moteur CSS/GTK). Une disposition de boutons a gauche (voir
// `ButtonLayout::on_left`) est le signal le plus fiable qu'un theme "a la
// mac" (WhiteSur, McMojave...) est installe : ces themes reconfigurent
// systematiquement `button-layout` pour imiter macOS, et c'est ce reglage,
// pas le nom du theme, que `draw_header::draw_traffic_light` suit pour
// basculer sur un rendu "feux tricolores".
use crate::window::models::header_bar::{ButtonLayout, HeaderButton};

pub struct SystemTheme {
    pub dark: bool,
    pub layout: ButtonLayout,
}

/// Interroge le bureau une seule fois au demarrage de la fenetre (voir
/// `AzureWindow::run`) - pas a chaque redessin, un changement de theme en
/// cours de session n'est pas suivi. Retombe silencieusement sur
/// `dark: false` + `ButtonLayout::default_right()` si `gsettings` est
/// absent (bureau non-GNOME sans pont gsettings, ex. certaines
/// installations Sway/i3) ou si les cles interrogees n'existent pas.
pub fn detect() -> SystemTheme {
    SystemTheme {
        dark: detect_dark_mode(),
        layout: detect_button_layout(),
    }
}

fn run_gsettings(schema: &str, key: &str) -> Option<String> {
    let output = std::process::Command::new("gsettings").arg("get").arg(schema).arg(key).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().trim_matches('\'').to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn detect_dark_mode() -> bool {
    if let Some(scheme) = run_gsettings("org.gnome.desktop.interface", "color-scheme") {
        if scheme.contains("dark") {
            return true;
        }
        if scheme.contains("light") {
            return false;
        }
    }
    // Bureaux plus anciens (avant `color-scheme`, GNOME <42) : seul le nom
    // du theme GTK laisse deviner quelque chose.
    if let Some(theme) = run_gsettings("org.gnome.desktop.interface", "gtk-theme")
        && theme.to_lowercase().contains("dark") {
            return true;
        }
    false
}

fn detect_button_layout() -> ButtonLayout {
    match run_gsettings("org.gnome.desktop.wm.preferences", "button-layout") {
        Some(raw) => parse_button_layout(&raw).unwrap_or_else(ButtonLayout::default_right),
        None => ButtonLayout::default_right(),
    }
}

// Format `button-layout` : deux listes separees par `:` (boutons a gauche,
// boutons a droite de la barre de titre), chacune une liste de tokens
// separes par des virgules - `close`, `minimize`, `maximize` sont les
// seuls qu'on sait dessiner (voir `HeaderButton`) ; `appmenu`/`icon`/
// `spacer` (menu d'application, icone, espaceur) sont ignores, on n'a rien
// d'equivalent a dessiner pour eux. Un theme "a la mac" type WhiteSur
// donne typiquement `"close,minimize,maximize:"` (tout a gauche, rien a
// droite) - c'est ce cas precis, cote gauche non vide, qui determine
// `on_left`.
pub fn parse_button_layout(raw: &str) -> Option<ButtonLayout> {
    let mut sides = raw.splitn(2, ':');
    let left = sides.next().unwrap_or("");
    let right = sides.next().unwrap_or("");

    let parse_side = |side: &str| -> Vec<HeaderButton> {
        side.split(',')
            .filter_map(|token| match token.trim() {
                "close" => Some(HeaderButton::Close),
                "minimize" => Some(HeaderButton::Minimize),
                "maximize" => Some(HeaderButton::Fullscreen),
                _ => None,
            })
            .collect()
    };

    let left_buttons = parse_side(left);
    if !left_buttons.is_empty() {
        return Some(ButtonLayout { order: left_buttons, on_left: true });
    }
    let right_buttons = parse_side(right);
    if !right_buttons.is_empty() {
        return Some(ButtonLayout { order: right_buttons, on_left: false });
    }
    None
}
