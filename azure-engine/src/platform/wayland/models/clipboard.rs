use std::collections::HashMap;

/// Etat du presse-papiers d'une fenetre (protocole `wl_data_device`, voir
/// `managers::clipboard_manager`) : ce qu'on propose quand on a copie, et
/// ce que proposent les autres apps.
#[derive(Default)]
pub struct Clipboard {
    /// `wl_data_device` du siege : absent si le compositeur n'a pas de
    /// `wl_data_device_manager` (le presse-papiers reste alors interne).
    pub device_id: Option<u32>,
    pub manager_id: u32,
    /// Notre `wl_data_source` tant que la selection du systeme est a nous.
    pub source_id: Option<u32>,
    /// Le texte que notre source envoie a qui le demande.
    pub text: String,
    /// Offres recues (`wl_data_offer`) et leurs types MIME.
    pub offers: HashMap<u32, Vec<String>>,
    /// L'offre qui represente la selection actuelle du systeme.
    pub selection: Option<u32>,
    /// Offre d'un glisser-deposer en cours (ignore, detruite a la sortie).
    pub drag: Option<u32>,
}
