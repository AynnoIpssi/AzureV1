use azure_engine::rendering::models::color::Color;

/// Unite d'une longueur CSS. Volontairement plat : la resolution en pixels
/// reels depend du contexte (taille de police courante pour `em`, taille du
/// viewport pour `vw`/`vh`, ...) et se fera au moment du layout, pas ici -
/// voir le TODO dans `services::link::to_legacy_style`.
#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Px,
    Percent,
    Em,
    Rem,
    Vw,
    Vh,
    /// Unite `fr` (CSS Grid) - part de l'espace RESTANT d'un axe de grille,
    /// une fois les pistes de taille fixe retirees (voir
    /// `layout::managers::layout_manager::track_offsets`), au prorata de son
    /// poids. N'a de sens que pour `grid-template-columns`/`grid-template-rows`.
    Fr,
    /// Unite syntaxiquement valide mais pas encore geree (`cm`, `deg`, `s`,
    /// `ms`...) : conservee telle quelle plutot que perdue.
    Unknown(String),
}

impl Unit {
    pub fn parse(raw: &str) -> Unit {
        match raw.to_ascii_lowercase().as_str() {
            "px" => Unit::Px,
            "em" => Unit::Em,
            "rem" => Unit::Rem,
            "vw" => Unit::Vw,
            "vh" => Unit::Vh,
            "fr" => Unit::Fr,
            other => Unit::Unknown(other.to_string()),
        }
    }
}

/// Une valeur CSS deja typee. Une liste (`Value::List`) sert aussi bien a
/// une liste separee par des virgules (`font-family: "A", sans-serif`) qu'a
/// une liste separee par des espaces (`margin: 10px 20px`) : la distinction
/// structurelle entre les deux n'est pas encore necessaire tant que les
/// proprietes raccourcies (shorthand) ne sont pas eclatees par cote -
/// prochaine etape annoncee pour plus tard.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Length(f32, Unit),
    Number(f32),
    Color(Color),
    Keyword(String),
    Str(String),
    List(Vec<Value>),
    /// Appel de fonction autre que `rgb`/`rgba` (`linear-gradient(...)`,
    /// `radial-gradient(...)`...) : son nom en minuscules et ses arguments
    /// (separes par des virgules ; un argument fait de plusieurs mots, comme
    /// `#fff 50%`, est une `Value::List`). C'est la propriete qui l'utilise
    /// qui l'interprete (voir `services::link`).
    Function(String, Vec<Value>),
}

impl Value {
    /// Longueur en pixels si la valeur est directement exploitable comme
    /// telle (`Length(_, Px)` ou un nombre nu, tolere comme un pixel par
    /// commodite). `None` pour les pourcentages, unites non pixel, mots-cles
    /// (`auto`) ou listes - la resolution complete ne fait pas encore
    /// partie de ce module.
    pub fn as_length_px(&self) -> Option<f32> {
        match self {
            Value::Length(n, Unit::Px) => Some(*n),
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// La grandeur numerique brute d'une longueur, quelle que soit son
    /// unite (`%`, `px`, `em`...) - sans la resoudre en pixels reels.
    /// `LayoutProps` (le pont vers le moteur de rendu) n'a pas encore de
    /// systeme d'unites : ses champs x/y/largeur/hauteur sont des
    /// pourcentages de fenetre, ses autres champs des nombres bruts. Cette
    /// methode expose donc juste "le nombre", laissant l'appelant decider
    /// quoi en faire - voir `services::link::to_legacy_style`.
    pub fn as_bare_number(&self) -> Option<f32> {
        match self {
            Value::Length(n, _) => Some(*n),
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_keyword(&self) -> Option<&str> {
        match self {
            Value::Keyword(k) => Some(k.as_str()),
            _ => None,
        }
    }

    pub fn as_color(&self) -> Option<Color> {
        match self {
            Value::Color(c) => Some(*c),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f32> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
}

/// Parse une couleur hexadecimale CSS : `#rgb`, `#rrggbb`, `#rgba` ou
/// `#rrggbbaa`.
pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let component = |s: &str| u8::from_str_radix(s, 16).ok();
    let expand = |c: char| component(&format!("{c}{c}"));

    let chars: Vec<char> = hex.chars().collect();
    match chars.len() {
        3 => Some(Color::new(expand(chars[0])?, expand(chars[1])?, expand(chars[2])?, 255)),
        4 => Some(Color::new(expand(chars[0])?, expand(chars[1])?, expand(chars[2])?, expand(chars[3])?)),
        6 => Some(Color::new(
            component(&hex[0..2])?,
            component(&hex[2..4])?,
            component(&hex[4..6])?,
            255,
        )),
        8 => Some(Color::new(
            component(&hex[0..2])?,
            component(&hex[2..4])?,
            component(&hex[4..6])?,
            component(&hex[6..8])?,
        )),
        _ => None,
    }
}

/// Table des couleurs nommees CSS les plus courantes. Pas les ~148 de la
/// specification, mais l'essentiel + celles deja utilisees dans les
/// exemples du projet (`beige`) - a completer a la demande.
pub fn named_color(name: &str) -> Option<Color> {
    let c = |r, g, b| Some(Color::new(r, g, b, 255));
    match name.to_ascii_lowercase().as_str() {
        "transparent" => Some(Color::new(0, 0, 0, 0)),
        "currentcolor" | "inherit" => None,
        "black" => c(0, 0, 0),
        "white" => c(255, 255, 255),
        "red" => c(255, 0, 0),
        "green" => c(0, 128, 0),
        "lime" => c(0, 255, 0),
        "blue" => c(0, 0, 255),
        "yellow" => c(255, 255, 0),
        "orange" => c(255, 165, 0),
        "purple" => c(128, 0, 128),
        "pink" => c(255, 192, 203),
        "gray" | "grey" => c(128, 128, 128),
        "darkgray" | "darkgrey" => c(169, 169, 169),
        "lightgray" | "lightgrey" => c(211, 211, 211),
        "silver" => c(192, 192, 192),
        "brown" => c(165, 42, 42),
        "beige" => c(245, 245, 220),
        "cyan" | "aqua" => c(0, 255, 255),
        "magenta" | "fuchsia" => c(255, 0, 255),
        "navy" => c(0, 0, 128),
        "teal" => c(0, 128, 128),
        "olive" => c(128, 128, 0),
        "maroon" => c(128, 0, 0),
        "indigo" => c(75, 0, 130),
        "gold" => c(255, 215, 0),
        "coral" => c(255, 127, 80),
        "salmon" => c(250, 128, 114),
        "khaki" => c(240, 230, 140),
        "crimson" => c(220, 20, 60),
        "chocolate" => c(210, 105, 30),
        "tomato" => c(255, 99, 71),
        "violet" => c(238, 130, 238),
        "skyblue" => c(135, 206, 235),
        "steelblue" => c(70, 130, 180),
        "slategray" | "slategrey" => c(112, 128, 144),
        "ivory" => c(255, 255, 240),
        "lavender" => c(230, 230, 250),
        "plum" => c(221, 160, 221),
        "orchid" => c(218, 112, 214),
        "turquoise" => c(64, 224, 208),
        "seagreen" => c(46, 139, 87),
        "forestgreen" => c(34, 139, 34),
        "firebrick" => c(178, 34, 34),
        "darkred" => c(139, 0, 0),
        "darkblue" => c(0, 0, 139),
        "darkgreen" => c(0, 100, 0),
        "lightblue" => c(173, 216, 230),
        "lightgreen" => c(144, 238, 144),
        "lightyellow" => c(255, 255, 224),
        "lightpink" => c(255, 182, 193),
        _ => None,
    }
}
