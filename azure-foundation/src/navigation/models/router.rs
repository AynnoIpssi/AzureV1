// Table de routes facon Laravel, commune a `RouteTable` (un chemin -> un
// ecran, voir `route_table.rs`) et `WindowTable` (un chemin -> une fenetre,
// voir `window::models::window_table`). Une route tient en une ligne, et se
// deplie en "accordeon" si besoin :
//
//     RouteTable::new()
//         .route("/accueil", |_| accueil())
//         .route("/user/{id}", |r| profil(r.param("id").unwrap_or("")))
//             .name("user.show")
//             .where_number("id")
//         .view("/aide", "ui/aide.rsh", "ui/app.rsc")
//         .redirect("/", "/accueil")
//         .group("/admin", |g| g
//             .route("/stats", |_| stats())
//             .route("/logs/{jour?}", |r| logs(r.param("jour"))))
//         .fallback(|r| page_404(&r.path))
//
// `.name(...)` et les `.where_*(...)` s'appliquent a la DERNIERE route
// ajoutee, comme `->name()` / `->whereNumber()` en Laravel. Purement locale :
// ne parle jamais au routeur azure-rooter.

/// Ecran de repli quand aucune route ne correspond.
type Fallback<T> = Box<dyn Fn(&Request) -> T + Send>;

/// Ce que recoit le handler d'une route : le chemin demande, la charge utile
/// libre (le `payload` de `goto` / `navigate_to` / `open_window`) et les
/// parametres `{nom}` captures dans le chemin.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub path: String,
    pub payload: String,
    params: Vec<(String, String)>,
}

impl Request {
    pub fn new(path: &str, payload: &str) -> Request {
        Request { path: path.to_string(), payload: payload.to_string(), params: Vec::new() }
    }

    /// Le parametre `{name}` du chemin, `None` s'il n'existe pas (ou si
    /// c'est un `{name?}` absent de ce chemin).
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    pub fn params(&self) -> &[(String, String)] {
        &self.params
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Segment {
    Static(String),
    Param(String),
    Optional(String),
}

enum Constraint {
    Number,
    Alpha,
    AlphaNumeric,
    In(Vec<String>),
}

impl Constraint {
    fn accepts(&self, value: &str) -> bool {
        match self {
            Constraint::Number => value.chars().all(|c| c.is_ascii_digit()),
            Constraint::Alpha => value.chars().all(char::is_alphabetic),
            Constraint::AlphaNumeric => value.chars().all(char::is_alphanumeric),
            Constraint::In(values) => values.iter().any(|v| v == value),
        }
    }
}

enum Action<T> {
    Handler(Box<dyn Fn(&Request) -> T + Send>),
    Redirect(String),
}

struct Entry<T> {
    pattern: String,
    segments: Vec<Segment>,
    name: Option<String>,
    constraints: Vec<(String, Constraint)>,
    action: Action<T>,
}

// Chemin decoupe sur '/', segments vides ignores : "/a/", "a" et "/a" sont
// la meme route.
fn split(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter(|part| !part.is_empty())
}

fn parse_pattern(pattern: &str) -> Vec<Segment> {
    split(pattern)
        .map(|part| match part.strip_prefix('{').and_then(|p| p.strip_suffix('}')) {
            Some(name) => match name.strip_suffix('?') {
                Some(name) => Segment::Optional(name.to_string()),
                None => Segment::Param(name.to_string()),
            },
            None => Segment::Static(part.to_string()),
        })
        .collect()
}

fn join(prefix: &str, path: &str) -> String {
    let joined: Vec<&str> = split(prefix).chain(split(path)).collect();
    format!("/{}", joined.join("/"))
}

impl<T> Entry<T> {
    // Les parametres captures si `path` correspond a cette route (motif ET
    // contraintes), `None` sinon.
    fn matches(&self, path: &str) -> Option<Vec<(String, String)>> {
        let parts: Vec<&str> = split(path).collect();
        let mut params = Vec::new();
        for (i, segment) in self.segments.iter().enumerate() {
            match (segment, parts.get(i)) {
                (Segment::Static(expected), Some(part)) if expected == part => {}
                (Segment::Param(name) | Segment::Optional(name), Some(part)) => params.push((name.clone(), part.to_string())),
                (Segment::Optional(_), None) => {}
                _ => return None,
            }
        }
        if parts.len() > self.segments.len() {
            return None;
        }
        let respected = self.constraints.iter().all(|(name, constraint)| {
            params.iter().find(|(key, _)| key == name).is_none_or(|(_, value)| constraint.accepts(value))
        });
        respected.then_some(params)
    }
}

// Route NOMMEE envoyee a une autre vue / une autre app, qui seule connait
// ses noms : le chemin transporte le nom et les parametres, et c'est le
// `dispatch` du receveur qui les transforme en vrai chemin (voir `url`).
// Format : NAMED + nom, puis NAMED + cle=valeur par parametre. Le caractere
// de controle "Group Separator", jamais dans un vrai chemin, et different
// du separateur path/payload ('\u{1F}') et du marqueur de fenetre partagee
// ('\u{1E}').
const NAMED: char = '\u{1D}';

/// Le chemin a envoyer (avec `goto`/`navigate_to`) pour viser la route
/// nommee `name` du RECEVEUR. Erreur si un nom, une cle ou une valeur
/// contient un caractere de controle du transport, ou une cle un '='.
pub fn named_path(name: &str, params: &[(&str, &str)]) -> Result<String, String> {
    let forbidden = |text: &str| text.contains([NAMED, '\u{1E}', '\u{1F}']);
    if name.is_empty() || forbidden(name) {
        return Err(format!("Nom de route invalide : {name:?}"));
    }
    let mut path = format!("{NAMED}{name}");
    for (key, value) in params {
        if key.is_empty() || key.contains('=') || forbidden(key) || forbidden(value) {
            return Err(format!("Parametre de route invalide : {key:?} = {value:?}"));
        }
        path.push_str(&format!("{NAMED}{key}={value}"));
    }
    Ok(path)
}

// L'inverse de `named_path` : `None` si `path` n'est pas une route nommee.
fn parse_named(path: &str) -> Option<(&str, Vec<(&str, &str)>)> {
    let mut parts = path.strip_prefix(NAMED)?.split(NAMED);
    let name = parts.next()?;
    Some((name, parts.filter_map(|pair| pair.split_once('=')).collect()))
}

// Garde-fou contre `redirect("/a", "/b")` + `redirect("/b", "/a")`.
const MAX_REDIRECTS: usize = 16;

pub struct Router<T> {
    entries: Vec<Entry<T>>,
    last: Option<usize>,
    fallback: Option<Fallback<T>>,
}

impl<T> Router<T> {
    pub fn new() -> Router<T> {
        Router { entries: Vec::new(), last: None, fallback: None }
    }

    fn push(mut self, pattern: &str, action: Action<T>) -> Router<T> {
        let pattern = join("", pattern);
        let entry = Entry { segments: parse_pattern(&pattern), pattern, name: None, constraints: Vec::new(), action };
        // Meme chemin enregistre deux fois : le second remplace le premier
        // (sur place, pour garder l'ordre), comme l'ancien `on`.
        self.insert(entry);
        self
    }

    fn insert(&mut self, entry: Entry<T>) {
        match self.entries.iter().position(|e| e.pattern == entry.pattern) {
            Some(i) => {
                self.entries[i] = entry;
                self.last = Some(i);
            }
            None => {
                self.entries.push(entry);
                self.last = Some(self.entries.len() - 1);
            }
        }
    }

    // La derniere route ajoutee, celle que visent `.name()` / `.where_*()`.
    fn last_mut(&mut self) -> Option<&mut Entry<T>> {
        self.entries.get_mut(self.last?)
    }

    /// Une route : `path` peut contenir des parametres `{id}` (obligatoire)
    /// ou `{id?}` (facultatif, seulement en fin de chemin). A la resolution,
    /// la premiere route enregistree qui correspond gagne.
    pub fn route(self, path: &str, handler: impl Fn(&Request) -> T + Send + 'static) -> Router<T> {
        self.push(path, Action::Handler(Box::new(handler)))
    }

    /// Forme courte quand le handler n'a besoin que du payload.
    pub fn on(self, path: &str, handler: impl Fn(&str) -> T + Send + 'static) -> Router<T> {
        self.route(path, move |request| handler(&request.payload))
    }

    /// `from` resout comme `to` (le payload est garde).
    pub fn redirect(self, from: &str, to: &str) -> Router<T> {
        self.push(from, Action::Redirect(join("", to)))
    }

    /// Nomme la derniere route, pour la retrouver avec `url`.
    pub fn name(mut self, name: &str) -> Router<T> {
        if let Some(entry) = self.last_mut() {
            entry.name = Some(name.to_string());
        }
        self
    }

    fn constrain(mut self, param: &str, constraint: Constraint) -> Router<T> {
        if let Some(entry) = self.last_mut() {
            entry.constraints.push((param.to_string(), constraint));
        }
        self
    }

    /// Le parametre `param` de la derniere route n'accepte que des chiffres.
    pub fn where_number(self, param: &str) -> Router<T> {
        self.constrain(param, Constraint::Number)
    }

    /// Le parametre `param` de la derniere route n'accepte que des lettres.
    pub fn where_alpha(self, param: &str) -> Router<T> {
        self.constrain(param, Constraint::Alpha)
    }

    /// Le parametre `param` de la derniere route n'accepte que lettres et chiffres.
    pub fn where_alpha_numeric(self, param: &str) -> Router<T> {
        self.constrain(param, Constraint::AlphaNumeric)
    }

    /// Le parametre `param` de la derniere route n'accepte que ces valeurs.
    pub fn where_in(self, param: &str, values: &[&str]) -> Router<T> {
        self.constrain(param, Constraint::In(values.iter().map(|v| v.to_string()).collect()))
    }

    /// Toutes les routes declarees dans `routes` recoivent le prefixe
    /// `prefix` (redirections comprises). Les groupes s'imbriquent.
    pub fn group(mut self, prefix: &str, routes: impl FnOnce(Router<T>) -> Router<T>) -> Router<T> {
        for mut entry in routes(Router::new()).entries {
            entry.pattern = join(prefix, &entry.pattern);
            entry.segments = parse_pattern(&entry.pattern);
            if let Action::Redirect(target) = &mut entry.action {
                *target = join(prefix, target);
            }
            self.insert(entry);
        }
        self
    }

    /// Appele quand aucune route ne correspond (sinon `resolve` rend `None`).
    pub fn fallback(mut self, handler: impl Fn(&Request) -> T + Send + 'static) -> Router<T> {
        self.fallback = Some(Box::new(handler));
        self
    }

    /// Le chemin de la route nommee `name`, parametres remplis :
    /// `url("user.show", &[("id", "42")])` -> `Some("/user/42")`. `None` si
    /// le nom est inconnu ou si un parametre obligatoire manque.
    pub fn url(&self, name: &str, params: &[(&str, &str)]) -> Option<String> {
        let entry = self.entries.iter().find(|e| e.name.as_deref() == Some(name))?;
        let mut parts = Vec::new();
        for segment in &entry.segments {
            let value = |key: &str| params.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string());
            match segment {
                Segment::Static(part) => parts.push(part.clone()),
                Segment::Param(key) => parts.push(value(key)?),
                Segment::Optional(key) => parts.extend(value(key)),
            }
        }
        Some(format!("/{}", parts.join("/")))
    }

    /// Ce que rend la route qui correspond a `path` (en suivant les
    /// redirections), sinon le `fallback`, sinon `None`. `path` peut aussi
    /// venir de `named_path` : il est alors d'abord traduit par `url`. Un
    /// nom inconnu (ou un parametre manquant) va au `fallback`, avec le nom
    /// de la route comme `path`.
    pub fn dispatch(&self, path: &str, payload: &str) -> Option<T> {
        let mut path = match parse_named(path) {
            Some((name, params)) => match self.url(name, &params) {
                Some(url) => url,
                None => return self.fallback.as_ref().map(|handler| handler(&Request::new(name, payload))),
            },
            None => path.to_string(),
        };
        for _ in 0..=MAX_REDIRECTS {
            let found = self.entries.iter().find_map(|entry| entry.matches(&path).map(|params| (entry, params)));
            match found {
                Some((entry, params)) => match &entry.action {
                    Action::Handler(handler) => {
                        return Some(handler(&Request { path, payload: payload.to_string(), params }));
                    }
                    Action::Redirect(target) => path = target.clone(),
                },
                None => return self.fallback.as_ref().map(|handler| handler(&Request::new(&path, payload))),
            }
        }
        None
    }
}

impl<T> Default for Router<T> {
    fn default() -> Router<T> {
        Router::new()
    }
}
