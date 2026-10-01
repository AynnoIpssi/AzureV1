// Une fenetre envoyee d'une app a d'autres apps via azure-rooter (voir
// `navigation_manager::send_window`) : sa spec + son titre + sa source rsH et
// rsC. L'app qui la recoit n'a besoin de rien connaitre de l'app emettrice :
// elle interprete la source elle-meme (voir `to_window`) et ouvre la fenetre.
//
// Sur le fil, c'est un message texte normal du routeur (meme canal que les
// `Route`), reconnu par son prefixe `MARKER` (voir `decode`).
use crate::compiler::rsc::mangers::parser::parse as parse_rsc;
use crate::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use crate::compiler::rsh::mangers::parser::parse as parse_rsh;
use crate::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use crate::compiler::services::codegen::StyleSource;
use crate::compiler::services::interpreter::build_ui;
use crate::window::models::window::AzureWindow;
use azure_core::models::window_model::{WindowKind, WindowScope, WindowSize, WindowSpec, WindowState};

// "Record Separator" ASCII : un message qui commence par lui n'est jamais une
// `Route` (un chemin commence par un caractere imprimable).
const MARKER: &str = "\u{1E}azure-window";
// Meme separateur que `navigation::models::route::Route`.
const SEPARATOR: char = '\u{1F}';
const FIELDS: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct SharedWindow {
    pub spec: WindowSpec,
    pub title: String,
    pub rsh: String,
    pub rsc: String,
}

impl SharedWindow {
    pub fn new(spec: WindowSpec, title: &str, rsh: &str, rsc: &str) -> SharedWindow {
        SharedWindow { spec, title: title.to_string(), rsh: rsh.to_string(), rsc: rsc.to_string() }
    }

    pub fn encode(&self) -> Result<String, String> {
        for (name, text) in [("titre", &self.title), ("rsH", &self.rsh), ("rsC", &self.rsc)] {
            if text.contains(SEPARATOR) {
                return Err(format!("Le {name} de la fenetre contient le caractere separateur U+001F"));
            }
        }
        let spec = &self.spec;
        let fields = [
            MARKER.to_string(),
            spec.owner_app_id().to_string(),
            spec.size().width().to_string(),
            spec.size().height().to_string(),
            spec.state().code().to_string(),
            spec.scope().code().to_string(),
            spec.kind().code().to_string(),
            self.title.clone(),
            self.rsh.clone(),
            self.rsc.clone(),
        ];
        Ok(fields.join(&SEPARATOR.to_string()))
    }

    /// `true` si `raw` est une fenetre encodee (a decoder avec `decode`)
    /// plutot qu'une `Route`.
    pub fn is_shared_window(raw: &str) -> bool {
        raw.starts_with(MARKER)
    }

    /// L'inverse d'`encode`. La spec est reconstruite avec `WindowSpec::new` :
    /// une fenetre recue qui viole les regles du core est refusee.
    pub fn decode(raw: &str) -> Result<SharedWindow, String> {
        let fields: Vec<&str> = raw.splitn(FIELDS, SEPARATOR).collect();
        if fields.len() != FIELDS || fields[0] != MARKER {
            return Err("Message recu : pas une fenetre encodee".to_string());
        }
        let number = |i: usize| fields[i].parse::<u32>().map_err(|_| format!("Champ {i} invalide : '{}'", fields[i]));
        let size = WindowSize::new(number(2)?, number(3)?)?;
        let state = WindowState::from_code(number(4)?).ok_or("Etat de fenetre inconnu")?;
        let scope = WindowScope::from_code(number(5)?).ok_or("Scope de fenetre inconnu")?;
        let kind = WindowKind::from_code(number(6)?).ok_or("Type de fenetre inconnu")?;
        let spec = WindowSpec::new(number(1)?, size, state, scope, kind)?;
        Ok(SharedWindow::new(spec, fields[7], fields[8], fields[9]))
    }

    /// Interprete la source rsH/rsC et construit la fenetre, prete a `run`.
    pub fn to_window(&self) -> Result<AzureWindow, String> {
        let sheet = parse_rsc(tokenize_rsc(&self.rsc)).map_err(|e| format!("rsC invalide : {e:?}"))?;
        let ast = parse_rsh(tokenize_rsh(&self.rsh)).map_err(|e| format!("rsH invalide : {e:?}"))?;
        let nodes = build_ui(&ast, &StyleSource::Rsc(&sheet));
        Ok(AzureWindow::new(&self.title).spec(self.spec).ui(nodes))
    }
}
