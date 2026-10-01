// Les notes, rangees dans le stockage prive de l'app :
//
//   notes           les ids, "3,1,2"
//   note.<id>.titre
//   note.<id>.texte
//   note.<id>.modifie   secondes depuis 1970
//
// Toutes sont gardees en memoire ; chaque changement est ecrit aussitot.
// Sans stockage (tests, daemon absent), le carnet vit en memoire seulement.
use azure_foundation::storage::models::stockage::Stockage;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub id: u64,
    pub titre: String,
    pub texte: String,
    pub modifie: u64,
}

impl Note {
    /// Le titre, ou « Sans titre ».
    pub fn nom(&self) -> &str {
        let t = self.titre.trim();
        if t.is_empty() { "Sans titre" } else { t }
    }

    /// La premiere ligne non vide du texte, raccourcie.
    pub fn apercu(&self) -> String {
        let ligne = self.texte.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
        let mut s: String = ligne.chars().take(60).collect();
        if ligne.chars().count() > 60 {
            s.push('…');
        }
        s
    }
}

pub fn maintenant() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// « à l'instant », « il y a 5 min »...
pub fn depuis(modifie: u64, maintenant: u64) -> String {
    let s = maintenant.saturating_sub(modifie);
    match s {
        0..60 => "à l'instant".to_string(),
        60..3600 => format!("il y a {} min", s / 60),
        3600..86400 => format!("il y a {} h", s / 3600),
        _ => format!("il y a {} j", s / 86400),
    }
}

#[derive(Clone, Default)]
pub struct Carnet {
    notes: Arc<Mutex<Vec<Note>>>,
    store: Option<Stockage>,
}

impl Carnet {
    /// Un carnet sans stockage.
    pub fn en_memoire() -> Carnet {
        Carnet::default()
    }

    /// Lit les notes du stockage de l'app.
    pub fn charger(store: Stockage) -> Result<Carnet, String> {
        let ids: String = store.get("notes")?.unwrap_or_default();
        let mut notes = Vec::new();
        for id in ids.split(',').filter_map(|i| i.trim().parse::<u64>().ok()) {
            let cle = |champ: &str| format!("note.{id}.{champ}");
            notes.push(Note { id, titre: store.get(&cle("titre"))?.unwrap_or_default(), texte: store.get(&cle("texte"))?.unwrap_or_default(), modifie: store.get(&cle("modifie"))?.unwrap_or(0) });
        }
        Ok(Carnet { notes: Arc::new(Mutex::new(notes)), store: Some(store) })
    }

    fn notes_mut(&self) -> MutexGuard<'_, Vec<Note>> {
        self.notes.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Les notes, la plus recemment modifiee d'abord.
    pub fn notes(&self) -> Vec<Note> {
        let mut notes = self.notes_mut().clone();
        notes.sort_by(|a, b| b.modifie.cmp(&a.modifie).then(b.id.cmp(&a.id)));
        notes
    }

    pub fn note(&self, id: u64) -> Option<Note> {
        self.notes_mut().iter().find(|n| n.id == id).cloned()
    }

    fn ecrire_ids(&self, notes: &[Note]) -> Result<(), String> {
        let Some(store) = &self.store else { return Ok(()) };
        store.set("notes", notes.iter().map(|n| n.id.to_string()).collect::<Vec<_>>().join(","))
    }

    fn ecrire_note(&self, n: &Note) -> Result<(), String> {
        let Some(store) = &self.store else { return Ok(()) };
        store.set(&format!("note.{}.titre", n.id), n.titre.as_str())?;
        store.set(&format!("note.{}.texte", n.id), n.texte.as_str())?;
        store.set(&format!("note.{}.modifie", n.id), n.modifie)
    }

    /// Une nouvelle note vide ; rend son id.
    pub fn nouvelle(&self) -> Result<u64, String> {
        let mut notes = self.notes_mut();
        let id = notes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
        let note = Note { id, titre: String::new(), texte: String::new(), modifie: maintenant() };
        self.ecrire_note(&note)?;
        notes.push(note);
        self.ecrire_ids(&notes)?;
        Ok(id)
    }

    /// Remplace le titre et le texte de la note `id` ; `true` si quelque
    /// chose a change (alors seulement elle est ecrite et datee).
    pub fn modifier(&self, id: u64, titre: &str, texte: &str) -> Result<bool, String> {
        let mut notes = self.notes_mut();
        let Some(note) = notes.iter_mut().find(|n| n.id == id) else { return Ok(false) };
        if note.titre == titre && note.texte == texte {
            return Ok(false);
        }
        note.titre = titre.to_string();
        note.texte = texte.to_string();
        note.modifie = maintenant();
        let note = note.clone();
        self.ecrire_note(&note)?;
        Ok(true)
    }

    /// Supprime la note `id` ; `true` si elle existait.
    pub fn supprimer(&self, id: u64) -> Result<bool, String> {
        let mut notes = self.notes_mut();
        let avant = notes.len();
        notes.retain(|n| n.id != id);
        if notes.len() == avant {
            return Ok(false);
        }
        self.ecrire_ids(&notes)?;
        if let Some(store) = &self.store {
            for champ in ["titre", "texte", "modifie"] {
                store.forget(&format!("note.{id}.{champ}"))?;
            }
        }
        Ok(true)
    }
}
