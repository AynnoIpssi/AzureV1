// Separateur entre `path` et `payload` dans le `message` textuel envoye via
// `IntraRouter` (voir `encode`/`decode`) - meme choix que le pendant
// inter-app (`azure_foundation::navigation::models::route::Route`) : le
// caractere de controle ASCII "Unit Separator", jamais tape au clavier ni
// produit par du texte normal.
const SEPARATOR: char = '\u{1F}';

/// Une navigation demandee vers une AUTRE vue du MEME process : `path`
/// identifie l'ecran vise (les cles enregistrees dans le `RouteTable` local
/// de la vue ciblee), `payload` transporte une donnee libre pour cet ecran
/// (vide si aucune). Independant de tout id - c'est
/// `managers::intra_router::IntraRouter::navigate` qui sait a qui l'envoyer.
///
/// Copie volontairement independante de l'equivalent inter-app
/// (`azure_foundation::navigation::models::route::Route`) : azure-rooter ne
/// depend pas d'azure-foundation, et ce fichier ne doit jamais devenir un
/// pont vers le routeur par socket (`managers::router`) - seul
/// `managers::intra_router` lui donne un sens.
pub struct IntraRoute {
    pub path: String,
    pub payload: String,
}

impl IntraRoute {
    pub fn new(path: &str, payload: &str) -> IntraRoute {
        IntraRoute { path: path.to_string(), payload: payload.to_string() }
    }

    /// Le `message` a passer a `IntraRouter::send` - a reconstruire avec
    /// `decode` cote receveur.
    pub fn encode(&self) -> String {
        format!("{}{}{}", self.path, SEPARATOR, self.payload)
    }

    /// L'inverse d'`encode`. `raw` sans separateur est traite comme un
    /// `path` sans `payload`, plutot que rejete - un message envoye par
    /// autre chose que `navigate` (une vue qui publierait directement une
    /// chaine simple) reste donc exploitable.
    pub fn decode(raw: &str) -> IntraRoute {
        match raw.split_once(SEPARATOR) {
            Some((path, payload)) => IntraRoute::new(path, payload),
            None => IntraRoute::new(raw, ""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_round_trips() {
        let route = IntraRoute::new("/settings", "tab=audio");
        let decoded = IntraRoute::decode(&route.encode());
        assert_eq!(decoded.path, "/settings");
        assert_eq!(decoded.payload, "tab=audio");
    }

    #[test]
    fn decoding_a_raw_string_without_payload_keeps_it_as_the_path() {
        let decoded = IntraRoute::decode("/home");
        assert_eq!(decoded.path, "/home");
        assert_eq!(decoded.payload, "");
    }
}
