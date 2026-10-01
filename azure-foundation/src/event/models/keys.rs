// Codes evdev (linux/input-event-codes.h) que le systeme d'evenements a
// besoin de reconnaitre specifiquement - pas une table exhaustive, juste
// ce dont `services::dispatch` se sert (bouton de clic, touches
// modificatrices).

/// Clic gauche, tel que porte par `WindowEvent::WindowMouseButton`
/// (`wl_pointer::button` cote Wayland) - pas un numero arbitraire.
pub const BTN_LEFT: u32 = 272;
/// Clic droit.
pub const BTN_RIGHT: u32 = 273;

pub const KEY_LEFTSHIFT: u32 = 42;
pub const KEY_RIGHTSHIFT: u32 = 54;
pub const KEY_LEFTCTRL: u32 = 29;
pub const KEY_RIGHTCTRL: u32 = 97;

/// `true` si `code` est l'une des deux touches Shift (gauche ou droite).
pub fn is_shift_key(code: u32) -> bool {
    code == KEY_LEFTSHIFT || code == KEY_RIGHTSHIFT
}

/// `true` si `code` est l'une des deux touches Ctrl (gauche ou droite).
pub fn is_ctrl_key(code: u32) -> bool {
    code == KEY_LEFTCTRL || code == KEY_RIGHTCTRL
}
