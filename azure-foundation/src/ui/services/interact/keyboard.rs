// Touches du clavier -> `KeyInput`, selon la carte du clavier envoyee par
// le compositeur (`KeyboardLayout::Xkb`, voir `keymap`), ou a defaut une
// table fixe (QWERTY, AZERTY, QWERTZ).
use super::keymap::{self, Keymap, Modifiers};

/// Une frappe clavier deja traduite (voir `key_to_input`), independante de
/// la representation bas niveau du clavier. Le `bool` sur les
/// deplacements/Home/End porte l'etat de Shift au moment de la frappe :
/// `true` etend (ou demarre) la selection depuis la position courante du
/// curseur, `false` deplace simplement le curseur - voir
/// `TextArea::move_left` pour la regle exacte quand une selection existe
/// deja.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyInput {
    Char(char),
    /// Tab (`true` avec Maj : en arriere).
    Tab(bool),
    Escape,
    Up(bool),
    Down(bool),
    Backspace,
    /// "Suppr" - retire le caractere APRES le curseur, sans le deplacer
    /// (a distinguer de `Backspace`, qui retire celui d'avant).
    Delete,
    Enter,
    MoveLeft(bool),
    MoveRight(bool),
    Home(bool),
    End(bool),
    SelectAll,
    /// Touche morte (`^`, `¨`...) : se compose avec la lettre suivante
    /// (voir `keymap::compose`).
    Dead(char),
    Copy,
    Cut,
    Paste,
    Undo,
    /// Ctrl+B / I / U / E (code), Ctrl+Maj+S (barre) : la lettre, pour le
    /// texte riche (voir `ui::models::rich::Mark::parse`).
    Format(char),
}

impl KeyInput {
    /// `false` pour les raccourcis Ctrl+... qui n'ont pas de sens repetes
    /// en boucle si la touche reste maintenue (Selectionner tout / Copier /
    /// Couper / Coller n'ont d'effet utile qu'une fois) - `true` pour tout
    /// le reste, y compris `Undo` (maintenir Ctrl+Z pour annuler plusieurs
    /// etapes d'affilee est un usage courant et attendu). Utilise par
    /// `AzureWindow::run` pour decider si une touche doit armer la
    /// repetition (voir `KEY_REPEAT_INITIAL_DELAY`).
    pub fn is_repeatable(&self) -> bool {
        !matches!(self, KeyInput::SelectAll | KeyInput::Dead(_) | KeyInput::Copy | KeyInput::Cut | KeyInput::Paste | KeyInput::Format(_))
    }
}

/// Les dispositions physiques qu'on sait traduire nous-memes - un sous-
/// ensemble volontairement restreint (pas les dizaines de variantes XKB
/// reelles), voir `detect_keyboard_layout` et `key_to_input`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardLayout {
    Qwerty,
    Azerty,
    Qwertz,
    /// La vraie carte du clavier (voir `Keymap::leak`) : toutes les
    /// touches, Maj, AltGr, touches mortes, comme dans les autres apps.
    Xkb(&'static Keymap),
}

impl KeyboardLayout {
    /// La touche `evdev` donne-t-elle AltGr ?
    pub fn is_level3(&self, evdev: u32) -> bool {
        matches!(self, KeyboardLayout::Xkb(k) if k.is_level3(evdev))
    }
}

/// Demande a l'OS quelle disposition clavier est configuree - via
/// `localectl status` (systemd), qui lit exactement ce que GNOME/KDE
/// utilisent eux-memes pour leurs propres reglages clavier, plutot que de
/// deviner ou de figer une disposition en dur dans le code. Retombe sur
/// `Qwerty` si la commande est absente, echoue, ou rapporte une
/// disposition qu'on ne sait pas encore traduire - mieux vaut un clavier
/// americain par defaut qu'un crash.
///
/// Ce n'est qu'un point de depart : une fenetre passe a
/// `KeyboardLayout::Xkb` des que le compositeur envoie la vraie carte du
/// clavier (voir `AzureWindow::run`), ce qui arrive a l'ouverture.
pub fn detect_keyboard_layout() -> KeyboardLayout {
    let output = std::process::Command::new("localectl").arg("status").output();
    let text = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        _ => return KeyboardLayout::Qwerty,
    };

    // Ligne du type "   X11 Layout: fr" - on ne regarde que le code
    // pays/langue, pas les variantes entre parentheses (ex: "fr(bepo)").
    let code = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("X11 Layout:"))
        .map(|v| v.trim());

    match code {
        Some(c) if c.starts_with("fr") || c.starts_with("be") => KeyboardLayout::Azerty,
        Some(c) if c.starts_with("de") || c.starts_with("ch") || c.starts_with("at") => KeyboardLayout::Qwertz,
        _ => KeyboardLayout::Qwerty,
    }
}

/// Traduction clavier -> `KeyInput` avec Maj et Ctrl seulement : les
/// touches de navigation/edition d'abord (independantes de la
/// disposition), puis le caractere de la touche. Ctrl+A/C/X/V/Z sont
/// reconnus par la lettre que la disposition met sur la touche. Voir
/// `key_to_input_with` pour AltGr, les verrous et les touches mortes.
pub fn key_to_input(evdev_code: u32, layout: KeyboardLayout, shift: bool, ctrl: bool) -> Option<KeyInput> {
    key_to_input_with(evdev_code, layout, Modifiers { shift, ctrl, ..Modifiers::default() })
}

/// Comme `key_to_input`, avec tous les modificateurs (AltGr, Verr. Maj,
/// Verr. Num, disposition active) - ceux que lit la vraie carte.
pub fn key_to_input_with(evdev_code: u32, layout: KeyboardLayout, mods: Modifiers) -> Option<KeyInput> {
    let (shift, ctrl) = (mods.shift, mods.ctrl);
    match evdev_code {
        14 => return Some(KeyInput::Backspace),
        15 => return Some(KeyInput::Tab(shift)), // KEY_TAB
        1 => return Some(KeyInput::Escape), // KEY_ESC
        103 => return Some(KeyInput::Up(shift)), // KEY_UP
        108 => return Some(KeyInput::Down(shift)), // KEY_DOWN
        111 => return Some(KeyInput::Delete), // KEY_DELETE
        28 | 96 => return Some(KeyInput::Enter), // KEY_ENTER, KEY_KPENTER
        105 => return Some(KeyInput::MoveLeft(shift)), // KEY_LEFT
        106 => return Some(KeyInput::MoveRight(shift)), // KEY_RIGHT
        102 => return Some(KeyInput::Home(shift)), // KEY_HOME
        107 => return Some(KeyInput::End(shift)), // KEY_END
        _ => {}
    }

    if let KeyboardLayout::Xkb(keymap) = layout {
        return xkb_input(keymap, evdev_code, mods);
    }

    let base = base_char(evdev_code, layout)?;

    if ctrl {
        return match base {
            'a' => Some(KeyInput::SelectAll),
            'c' => Some(KeyInput::Copy),
            'x' => Some(KeyInput::Cut),
            'v' => Some(KeyInput::Paste),
            'z' => Some(KeyInput::Undo),
            'b' | 'i' | 'u' | 'e' => Some(KeyInput::Format(base)),
            's' if shift => Some(KeyInput::Format('s')),
            _ => None,
        };
    }

    Some(KeyInput::Char(if shift { base.to_ascii_uppercase() } else { base }))
}

// Avec la vraie carte : Ctrl+lettre d'apres la lettre de base de la
// touche, sinon le caractere (ou la touche morte) du bon niveau.
fn xkb_input(keymap: &Keymap, evdev_code: u32, mods: Modifiers) -> Option<KeyInput> {
    if mods.ctrl {
        let base = keymap.base_keysym(evdev_code, mods.group).and_then(keymap::keysym_char)?;
        return match base.to_lowercase().next()? {
            'a' => Some(KeyInput::SelectAll),
            'c' => Some(KeyInput::Copy),
            'x' => Some(KeyInput::Cut),
            'v' => Some(KeyInput::Paste),
            'z' => Some(KeyInput::Undo),
            c @ ('b' | 'i' | 'u' | 'e') => Some(KeyInput::Format(c)),
            's' if mods.shift => Some(KeyInput::Format('s')),
            _ => None,
        };
    }
    let sym = keymap.keysym(evdev_code, mods)?;
    if let Some(accent) = keymap::dead_accent(sym) {
        return Some(KeyInput::Dead(accent));
    }
    keymap::keysym_char(sym).filter(|c| !c.is_control()).map(KeyInput::Char)
}

// La table brute position-physique -> caractere, par disposition -
// aucune notion de modificateur ici (voir `key_to_input`).
fn base_char(evdev_code: u32, layout: KeyboardLayout) -> Option<char> {
    let c = match (layout, evdev_code) {
        // Rangee des chiffres et touches communes : identiques dans les
        // trois dispositions qu'on sait traduire (voir le commentaire
        // ci-dessus sur les chiffres AZERTY).
        (_, 2) => '1', (_, 3) => '2', (_, 4) => '3', (_, 5) => '4', (_, 6) => '5',
        (_, 7) => '6', (_, 8) => '7', (_, 9) => '8', (_, 10) => '9', (_, 11) => '0',
        (_, 57) => ' ',

        // QWERTY (US) - disposition par defaut.
        (KeyboardLayout::Qwerty, 16) => 'q', (KeyboardLayout::Qwerty, 17) => 'w', (KeyboardLayout::Qwerty, 18) => 'e',
        (KeyboardLayout::Qwerty, 19) => 'r', (KeyboardLayout::Qwerty, 20) => 't', (KeyboardLayout::Qwerty, 21) => 'y',
        (KeyboardLayout::Qwerty, 22) => 'u', (KeyboardLayout::Qwerty, 23) => 'i', (KeyboardLayout::Qwerty, 24) => 'o',
        (KeyboardLayout::Qwerty, 25) => 'p',
        (KeyboardLayout::Qwerty, 30) => 'a', (KeyboardLayout::Qwerty, 31) => 's', (KeyboardLayout::Qwerty, 32) => 'd',
        (KeyboardLayout::Qwerty, 33) => 'f', (KeyboardLayout::Qwerty, 34) => 'g', (KeyboardLayout::Qwerty, 35) => 'h',
        (KeyboardLayout::Qwerty, 36) => 'j', (KeyboardLayout::Qwerty, 37) => 'k', (KeyboardLayout::Qwerty, 38) => 'l',
        (KeyboardLayout::Qwerty, 39) => ';',
        (KeyboardLayout::Qwerty, 44) => 'z', (KeyboardLayout::Qwerty, 45) => 'x', (KeyboardLayout::Qwerty, 46) => 'c',
        (KeyboardLayout::Qwerty, 47) => 'v', (KeyboardLayout::Qwerty, 48) => 'b', (KeyboardLayout::Qwerty, 49) => 'n',
        (KeyboardLayout::Qwerty, 50) => 'm',
        (KeyboardLayout::Qwerty, 51) => ',', (KeyboardLayout::Qwerty, 52) => '.', (KeyboardLayout::Qwerty, 53) => '/',

        // AZERTY (France/Belgique) - rangees decalees par rapport a QWERTY :
        // a/z, q/a, m/virgule notamment changent de position physique.
        (KeyboardLayout::Azerty, 16) => 'a', (KeyboardLayout::Azerty, 17) => 'z', (KeyboardLayout::Azerty, 18) => 'e',
        (KeyboardLayout::Azerty, 19) => 'r', (KeyboardLayout::Azerty, 20) => 't', (KeyboardLayout::Azerty, 21) => 'y',
        (KeyboardLayout::Azerty, 22) => 'u', (KeyboardLayout::Azerty, 23) => 'i', (KeyboardLayout::Azerty, 24) => 'o',
        (KeyboardLayout::Azerty, 25) => 'p',
        (KeyboardLayout::Azerty, 30) => 'q', (KeyboardLayout::Azerty, 31) => 's', (KeyboardLayout::Azerty, 32) => 'd',
        (KeyboardLayout::Azerty, 33) => 'f', (KeyboardLayout::Azerty, 34) => 'g', (KeyboardLayout::Azerty, 35) => 'h',
        (KeyboardLayout::Azerty, 36) => 'j', (KeyboardLayout::Azerty, 37) => 'k', (KeyboardLayout::Azerty, 38) => 'l',
        (KeyboardLayout::Azerty, 39) => 'm',
        (KeyboardLayout::Azerty, 44) => 'w', (KeyboardLayout::Azerty, 45) => 'x', (KeyboardLayout::Azerty, 46) => 'c',
        (KeyboardLayout::Azerty, 47) => 'v', (KeyboardLayout::Azerty, 48) => 'b', (KeyboardLayout::Azerty, 49) => 'n',
        (KeyboardLayout::Azerty, 50) => ',',

        // QWERTZ (Allemagne/Suisse/Autriche) - seul y/z est echange par
        // rapport a QWERTY (le point-virgule devient 'o' tremat, simplifie
        // ici en 'o' plutot que d'introduire un caractere non-ASCII de plus).
        (KeyboardLayout::Qwertz, 16) => 'q', (KeyboardLayout::Qwertz, 17) => 'w', (KeyboardLayout::Qwertz, 18) => 'e',
        (KeyboardLayout::Qwertz, 19) => 'r', (KeyboardLayout::Qwertz, 20) => 't', (KeyboardLayout::Qwertz, 21) => 'z',
        (KeyboardLayout::Qwertz, 22) => 'u', (KeyboardLayout::Qwertz, 23) => 'i', (KeyboardLayout::Qwertz, 24) => 'o',
        (KeyboardLayout::Qwertz, 25) => 'p',
        (KeyboardLayout::Qwertz, 30) => 'a', (KeyboardLayout::Qwertz, 31) => 's', (KeyboardLayout::Qwertz, 32) => 'd',
        (KeyboardLayout::Qwertz, 33) => 'f', (KeyboardLayout::Qwertz, 34) => 'g', (KeyboardLayout::Qwertz, 35) => 'h',
        (KeyboardLayout::Qwertz, 36) => 'j', (KeyboardLayout::Qwertz, 37) => 'k', (KeyboardLayout::Qwertz, 38) => 'l',
        (KeyboardLayout::Qwertz, 39) => 'o',
        (KeyboardLayout::Qwertz, 44) => 'y', (KeyboardLayout::Qwertz, 45) => 'x', (KeyboardLayout::Qwertz, 46) => 'c',
        (KeyboardLayout::Qwertz, 47) => 'v', (KeyboardLayout::Qwertz, 48) => 'b', (KeyboardLayout::Qwertz, 49) => 'n',
        (KeyboardLayout::Qwertz, 50) => 'm',

        _ => return None,
    };
    Some(c)
}
