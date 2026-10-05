// Ce qui entoure la boucle d'une fenetre : le compositeur Wayland pour une
// vraie fenetre (`WaylandHote`), la memoire pour le pilote d'essai (voir
// `crate::essai`). La boucle (`window::sur_evenement` / `sur_tic`) ne
// connait que ce trait : l'essai fait tourner exactement le meme code que
// la vraie fenetre.
use crate::cursor::managers::cursor_manager::CursorSet;
use crate::cursor::models::cursor_kind::CursorKind;
use crate::event::models::app_state::EventState;
use crate::ui::services::interact::{KeyboardLayout, Keymap};
use crate::window::models::resize_edge::ResizeEdge;
use azure_engine::platform::wayland::managers::surface_manager::{commit, damage_buffer};
use azure_engine::platform::wayland::managers::xdg_manager::{attach, move_toplevel, resize_toplevel, set_fullscreen, set_minimized, unset_fullscreen};
use azure_engine::platform::wayland::models::window::Window as WaylandWindow;
use azure_engine::rendering::models::canvas::Canvas;
use std::cell::Cell;

pub(crate) trait Hote {
    /// Carte du clavier et verrous (Maj, Num) tels que le systeme les donne.
    fn clavier(&mut self, _event: &mut EventState, _layout: &Cell<KeyboardLayout>) {}
    /// Montre l'image de la fenetre.
    fn presenter(&mut self, _canvas: &Canvas) {}
    fn minimiser(&mut self) {}
    fn plein_ecran(&mut self, _actif: bool) {}
    /// Redimensionner a la souris depuis ce bord (le systeme prend la main).
    fn redimensionner_au_bord(&mut self, _bord: ResizeEdge) {}
    /// Deplacer la fenetre a la souris (le systeme prend la main).
    fn deplacer(&mut self) {}
    /// La fenetre a change de taille : nouvelle memoire d'image.
    fn nouvelle_taille(&mut self, _largeur: i32, _hauteur: i32) {}
    fn presse_papiers(&mut self) -> Option<String> {
        None
    }
    fn ecrire_presse_papiers(&mut self, _texte: &str) -> Result<(), String> {
        Ok(())
    }
    fn curseur(&mut self, _forme: CursorKind) {}
    /// Jeton pour faire passer une autre fenetre au premier plan.
    fn jeton_activation(&mut self) -> Option<String> {
        None
    }
    fn activer(&mut self, _jeton: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Une vraie fenetre, chez le compositeur.
pub(crate) struct WaylandHote<'a> {
    pub win: &'a mut WaylandWindow,
    pub toplevel: u32,
    pub surface: u32,
    pub curseurs: Option<&'a CursorSet>,
}

impl Hote for WaylandHote<'_> {
    fn clavier(&mut self, event: &mut EventState, layout: &Cell<KeyboardLayout>) {
        if let Some(text) = self.win.take_keymap() {
            match Keymap::parse(&text) {
                Ok(keymap) => layout.set(KeyboardLayout::Xkb(keymap.leak())),
                Err(err) => eprintln!("AzureWindow: carte du clavier illisible ({err}), disposition {:?} gardee", layout.get()),
            }
        }
        let (locked, group) = self.win.keyboard_modifiers();
        // Bits usuels des cartes XKB : Lock (Verr. Maj) = 2, Mod2 (Verr. Num) = 16.
        event.caps_lock = locked & 2 != 0;
        event.num_lock = locked & 16 != 0;
        event.group = group;
    }

    // Copie l'image dans la memoire partagee et republie la surface. Sans
    // synchronisation par `wl_surface.frame` (un seul buffer). `buffer_id`
    // est relu a chaque fois : apres `resize`, c'est un nouveau `wl_buffer`.
    fn presenter(&mut self, canvas: &Canvas) {
        let (width, height) = (canvas.width, canvas.height);
        unsafe {
            std::ptr::copy_nonoverlapping(canvas.buffer.as_ptr(), self.win.ptr(), canvas.buffer.len());
        }
        let buffer_id = self.win.buffer_id();
        attach(self.win.connection_mut(), self.surface, buffer_id).expect("Failed to attach");
        damage_buffer(self.win.connection_mut(), self.surface, 0, 0, width as i32, height as i32).expect("Failed to damage");
        commit(self.win.connection_mut(), self.surface).expect("Failed to commit");
    }

    fn minimiser(&mut self) {
        set_minimized(self.win.connection_mut(), self.toplevel).expect("Failed to request minimize");
    }

    fn plein_ecran(&mut self, actif: bool) {
        if actif {
            set_fullscreen(self.win.connection_mut(), self.toplevel).expect("Failed to request fullscreen");
        } else {
            unset_fullscreen(self.win.connection_mut(), self.toplevel).expect("Failed to unset fullscreen");
        }
    }

    // Le serial DOIT etre celui de l'appui qui demande (voir
    // `Window::last_pointer_serial`).
    fn redimensionner_au_bord(&mut self, bord: ResizeEdge) {
        let (seat, serial) = (self.win.seat_id(), self.win.last_pointer_serial());
        resize_toplevel(self.win.connection_mut(), self.toplevel, seat, serial, bord.to_wayland()).expect("Failed to request interactive resize");
    }

    fn deplacer(&mut self) {
        let (seat, serial) = (self.win.seat_id(), self.win.last_pointer_serial());
        move_toplevel(self.win.connection_mut(), self.toplevel, seat, serial).expect("Failed to request interactive move");
    }

    fn nouvelle_taille(&mut self, largeur: i32, hauteur: i32) {
        self.win.resize(largeur, hauteur).expect("Failed to resize window buffer");
    }

    fn presse_papiers(&mut self) -> Option<String> {
        self.win.clipboard_text()
    }

    fn ecrire_presse_papiers(&mut self, texte: &str) -> Result<(), String> {
        self.win.set_clipboard(texte).map_err(|e| e.to_string())
    }

    fn curseur(&mut self, forme: CursorKind) {
        if let Some(curseurs) = self.curseurs
            && let Err(err) = curseurs.show(self.win, forme)
        {
            eprintln!("AzureWindow: echec du changement de curseur: {err}");
        }
    }

    fn jeton_activation(&mut self) -> Option<String> {
        self.win.activation_token()
    }

    fn activer(&mut self, jeton: &str) -> Result<(), String> {
        self.win.activate(jeton).map_err(|e| e.to_string())
    }
}
