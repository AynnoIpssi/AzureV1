use std::collections::HashMap;
use std::rc::Rc;

// Chaque `draw_text` relisait le fichier de police sur le disque (plusieurs
// centaines de Ko, a chaque texte de chaque image) : il est maintenant lu
// une seule fois par chemin et par thread.
thread_local! {
    static FONTS: std::cell::RefCell<HashMap<String, Rc<Vec<u8>>>> = std::cell::RefCell::new(HashMap::new());
}

pub fn load_font(path: &str) -> Result<Rc<Vec<u8>>, String> {
    if let Some(data) = FONTS.with(|fonts| fonts.borrow().get(path).cloned()) {
        return Ok(data);
    }
    let data = Rc::new(std::fs::read(path).map_err(|e| e.to_string())?);
    FONTS.with(|fonts| fonts.borrow_mut().insert(path.to_string(), data.clone()));
    Ok(data)
}
