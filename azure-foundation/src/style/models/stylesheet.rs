use std::collections::HashMap;

use crate::style::models::style::Style;

/// La table des composants de style nommes, chacun identifie par le nom de
/// classe utilise cote .rsh (`<container.card>` -> `stylesheet.get("card")`).
#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    rules: HashMap<String, Style>,
}

impl Stylesheet {
    pub fn new() -> Stylesheet {
        Stylesheet { rules: HashMap::new() }
    }

    pub fn define(&mut self, class: &str, style: Style) {
        self.rules.insert(class.to_string(), style);
    }

    pub fn get(&self, class: &str) -> Option<&Style> {
        self.rules.get(class)
    }

    /// Resout le style final d'un element : le style par defaut de base,
    /// puis le style nomme (s'il existe) applique par-dessus. Un element
    /// sans classe, ou dont la classe est inconnue, retombe simplement sur
    /// `base`.
    /// `class` peut porter plusieurs classes separees par des espaces : elles
    /// s'appliquent dans l'ordre, la derniere l'emportant.
    pub fn resolve(&self, class: &str, base: &Style) -> Style {
        class.split_whitespace().fold(base.clone(), |style, name| match self.get(name) {
            Some(named) => style.merge(named),
            None => style,
        })
    }
}
