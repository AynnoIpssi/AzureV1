#[derive(Debug)]
pub enum WindowEvent {
    WindowClose,
    WindowResize(i32, i32),
    WindowKeyPress(u32, bool),
    WindowMouseMove(i32, i32),
    WindowMouseButton(u32, bool),
    /// Molette verticale (`wl_pointer::axis`, axe vertical uniquement -
    /// l'axe horizontal n'est pas remonte). La valeur est la distance de
    /// defilement brute rapportee par le compositeur (meme unite/echelle
    /// que les deplacements de pointeur, deja divisee par 256 - voir
    /// `wl_pointer::motion`), positive vers le bas (molette tournee vers
    /// l'utilisateur), pas un nombre de "crans" ni de lignes deja calcule.
    WindowScroll(f64),
    /// Defilement horizontal (molette inclinee, pave tactile), positif vers
    /// la droite, en pixels comme `WindowScroll`.
    WindowScrollH(f64),
}