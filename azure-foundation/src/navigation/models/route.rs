// Separateur entre `path` et `payload` dans le `content` textuel envoye sur
// le socket du routeur (voir `Route::encode`/`decode`) - le caractere de
// controle ASCII "Unit Separator", jamais tape au clavier ni produit par du
// texte normal, contrairement a ':' ou '|' qui pourraient legitimement
// apparaitre dans un chemin ou une charge utile.
const SEPARATOR: char = '\u{1F}';
// Avant le jeton d'activation, a la fin (voir `Route::activation`) : le
// caractere "Record Separator", absent d'une navigation d'avant ce jeton.
const ACTIVATION_SEPARATOR: char = '\u{1E}';

/// Une navigation demandee vers une AUTRE app : `path` identifie l'ecran
/// vise (les cles enregistrees dans `RouteTable::on` de l'app cible),
/// `payload` transporte une donnee libre pour cet ecran (vide si aucune).
/// Independant de tout `app_id` - c'est `navigation_manager::navigate` qui
/// sait a qui l'envoyer.
pub struct Route {
    pub path: String,
    pub payload: String,
    /// Jeton pour mettre la fenetre qui recoit au premier plan (voir
    /// `WindowContext::activation_token`), vide sinon.
    pub activation: String,
}

impl Route {
    pub fn new(path: &str, payload: &str) -> Route {
        Route { path: path.to_string(), payload: payload.to_string(), activation: String::new() }
    }

    /// Avec un jeton d'activation : l'app qui recoit passe devant.
    pub fn with_activation(mut self, token: &str) -> Route {
        self.activation = token.to_string();
        self
    }

    /// Le `content` a passer a `navigation_manager::client::send` - a
    /// reconstruire avec `decode` cote receveur.
    pub fn encode(&self) -> String {
        let mut raw = format!("{}{}{}", self.path, SEPARATOR, self.payload);
        if !self.activation.is_empty() {
            raw.push(ACTIVATION_SEPARATOR);
            raw.push_str(&self.activation);
        }
        raw
    }

    /// L'inverse d'`encode`. `raw` sans separateur est traite comme un
    /// `path` sans `payload`, plutot que rejete - un message envoye par
    /// autre chose que `navigate` (une app qui publierait directement une
    /// chaine simple) reste donc exploitable.
    pub fn decode(raw: &str) -> Route {
        if let Some((rest, token)) = raw.rsplit_once(ACTIVATION_SEPARATOR) {
            return Route::decode(rest).with_activation(token);
        }
        match raw.split_once(SEPARATOR) {
            Some((path, payload)) => Route::new(path, payload),
            None => Route::new(raw, ""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_aller_retour() {
        let r = Route::decode(&Route::new("/doc/a", "x").with_activation("jeton-1").encode());
        assert_eq!((r.path.as_str(), r.payload.as_str(), r.activation.as_str()), ("/doc/a", "x", "jeton-1"));
        let r = Route::decode(&Route::new("/", "").encode());
        assert_eq!((r.path.as_str(), r.activation.as_str()), ("/", ""));
    }
}
