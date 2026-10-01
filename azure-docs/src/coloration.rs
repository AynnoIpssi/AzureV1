// Coloration du code des exemples. Chaque ligne devient une suite de
// morceaux `(genre, texte)` ; le gabarit pose un `<text.tk.<genre>>` par
// morceau, et docs.rsc donne sa couleur a chaque genre.
//
// Genres : mot (mots-cles), balise, classe (.classe, #id, types), attr
// (attributs, proprietes rsC), chaine, nombre, commentaire, interp ({{ }},
// parametres ?), fonction, ponct, tx (le reste).
//
// Ce n'est pas un parseur : un decoupage ligne par ligne suffit pour des
// exemples courts. Les commentaires sur plusieurs lignes (`/* */` en rsC et
// Rust, `<!-- -->` en rsH) sont suivis d'une ligne a l'autre.

pub type Morceau = (&'static str, String);

pub fn colorer(langage: &str, code: &str) -> Vec<Vec<Morceau>> {
    let mut etat = Etat::default();
    code.lines()
        .map(|ligne| {
            let ligne = ligne.replace('\t', "    ");
            let mut m = Morceaux::default();
            match langage {
                "rsh" => rsh(&ligne, &mut m, &mut etat),
                "rsc" => rsc(&ligne, &mut m, &mut etat),
                "rss" => rss(&ligne, &mut m),
                "rust" => rust(&ligne, &mut m, &mut etat),
                "toml" => toml(&ligne, &mut m),
                "sh" => sh(&ligne, &mut m),
                _ => m.pousser("tx", &ligne),
            }
            m.finir()
        })
        .collect()
}

#[derive(Default)]
struct Etat {
    /// Dans un commentaire ouvert sur une ligne precedente.
    commentaire: bool,
    /// rsC : profondeur d'accolades (0 = selecteurs).
    accolades: usize,
}

#[derive(Default)]
struct Morceaux(Vec<Morceau>);

impl Morceaux {
    /// Les espaces rejoignent le morceau precedent (moins de textes a
    /// dessiner) ; deux morceaux voisins du meme genre fusionnent.
    fn pousser(&mut self, genre: &'static str, texte: &str) {
        if texte.is_empty() {
            return;
        }
        let blanc = texte.chars().all(char::is_whitespace);
        match self.0.last_mut() {
            Some(dernier) if dernier.0 == genre || blanc => dernier.1.push_str(texte),
            _ => self.0.push((if blanc { "tx" } else { genre }, texte.to_string())),
        }
    }

    fn finir(mut self) -> Vec<Morceau> {
        if self.0.is_empty() {
            // Une ligne vide garde sa hauteur.
            self.0.push(("tx", " ".to_string()));
        }
        self.0
    }
}

fn est_mot(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Longueur (en octets) du prefixe de `s` fait de caracteres `f`.
fn prendre(s: &str, f: impl Fn(char) -> bool) -> usize {
    s.char_indices().find(|&(_, c)| !f(c)).map_or(s.len(), |(i, _)| i)
}

/// Une chaine qui commence par `s[0]` (guillemet), jusqu'au guillemet
/// fermant (ou la fin de ligne).
fn chaine(s: &str) -> usize {
    let q = s.chars().next().unwrap();
    let mut echappe = false;
    for (i, c) in s.char_indices().skip(1) {
        if echappe {
            echappe = false;
        } else if c == '\\' {
            echappe = true;
        } else if c == q {
            return i + c.len_utf8();
        }
    }
    s.len()
}

fn nombre(s: &str) -> usize {
    prendre(s, |c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '%')
}

/// Premier caractere de `s` (au moins un octet valide).
fn un(s: &str) -> usize {
    s.chars().next().map_or(0, char::len_utf8)
}

// ---------------------------------------------------------------- rsH

const RSH_MOTS: &[&str] = &["if", "else", "for", "match", "arm", "in"];

fn rsh(ligne: &str, m: &mut Morceaux, etat: &mut Etat) {
    let mut s = ligne;
    while !s.is_empty() {
        if etat.commentaire {
            let n = s.find("-->").map_or(s.len(), |i| i + 3);
            etat.commentaire = !s[..n].ends_with("-->");
            m.pousser("commentaire", &s[..n]);
            s = &s[n..];
        } else if s.starts_with("<!--") {
            etat.commentaire = true;
            m.pousser("commentaire", "<!--");
            s = &s[4..];
        } else if s.starts_with("{{") {
            let n = s.find("}}").map_or(s.len(), |i| i + 2);
            m.pousser("interp", &s[..n]);
            s = &s[n..];
        } else if s.starts_with('<') {
            s = rsh_balise(s, m);
        } else {
            let n = s.find('<').into_iter().chain(s.find("{{")).min().unwrap_or(s.len());
            m.pousser("tx", &s[..n]);
            s = &s[n..];
        }
    }
}

/// `<nom.classe#id attr="v">`, `<!nom>`, `<if.x == "a">`, `<champ/>`.
/// Rend ce qui suit la balise.
fn rsh_balise<'a>(mut s: &'a str, m: &mut Morceaux) -> &'a str {
    let ouvre = if s.starts_with("<!") { 2 } else { 1 };
    m.pousser("ponct", &s[..ouvre]);
    s = &s[ouvre..];
    let n = prendre(s, |c| est_mot(c) || c == '-');
    let nom = &s[..n];
    m.pousser(if RSH_MOTS.contains(&nom) { "mot" } else { "balise" }, nom);
    s = &s[n..];
    let controle = RSH_MOTS.contains(&nom);
    while !s.is_empty() {
        let c = s.chars().next().unwrap();
        if c == '>' || s.starts_with("/>") {
            let n = if c == '>' { 1 } else { 2 };
            m.pousser("ponct", &s[..n]);
            return &s[n..];
        } else if s.starts_with("{{") {
            let n = s.find("}}").map_or(s.len(), |i| i + 2);
            m.pousser("interp", &s[..n]);
            s = &s[n..];
        } else if c == '"' || c == '\'' {
            let n = chaine(s);
            m.pousser("chaine", &s[..n]);
            s = &s[n..];
        } else if (c == '.' || c == '#') && !controle {
            let n = 1 + prendre(&s[1..], |c| est_mot(c) || c == '-');
            m.pousser("classe", &s[..n]);
            s = &s[n..];
        } else if c.is_ascii_digit() {
            let n = nombre(s);
            m.pousser("nombre", &s[..n]);
            s = &s[n..];
        } else if est_mot(c) {
            let n = prendre(s, |c| est_mot(c) || c == '-' || (controle && c == '.'));
            let mot = &s[..n];
            let genre = if controle && RSH_MOTS.contains(&mot) {
                "mot"
            } else if controle || mot == "true" || mot == "false" {
                "tx"
            } else {
                "attr"
            };
            m.pousser(genre, mot);
            s = &s[n..];
        } else {
            let n = un(s);
            m.pousser(if c.is_whitespace() { "tx" } else { "ponct" }, &s[..n]);
            s = &s[n..];
        }
    }
    s
}

// ---------------------------------------------------------------- rsC

fn rsc(ligne: &str, m: &mut Morceaux, etat: &mut Etat) {
    let mut s = ligne;
    // Dans un bloc : `propriete: valeur;`. Vrai apres `:` jusqu'a `;`.
    let mut valeur = false;
    while !s.is_empty() {
        let c = s.chars().next().unwrap();
        if etat.commentaire {
            let n = s.find("*/").map_or(s.len(), |i| i + 2);
            etat.commentaire = !s[..n].ends_with("*/");
            m.pousser("commentaire", &s[..n]);
            s = &s[n..];
        } else if s.starts_with("/*") {
            etat.commentaire = true;
            m.pousser("commentaire", "/*");
            s = &s[2..];
        } else if c == '{' || c == '}' {
            if c == '{' {
                etat.accolades += 1;
            } else {
                etat.accolades = etat.accolades.saturating_sub(1);
            }
            valeur = false;
            m.pousser("ponct", &s[..1]);
            s = &s[1..];
        } else if c == '"' || c == '\'' {
            let n = chaine(s);
            m.pousser("chaine", &s[..n]);
            s = &s[n..];
        } else if etat.accolades == 0 || (!valeur && rsc_selecteur_imbrique(s)) {
            // Selecteurs : `.classe`, `#id`, `:hover`, `@media`, balises.
            let n = if matches!(c, '.' | '#' | ':' | '@') { 1 + prendre(&s[1..], |c| est_mot(c) || c == '-') } else { prendre(s, |c| est_mot(c) || c == '-').max(un(s)) };
            let genre = match c {
                '.' | '#' => "classe",
                ':' | '@' => "mot",
                c if est_mot(c) => "balise",
                c if c.is_whitespace() => "tx",
                _ => "ponct",
            };
            m.pousser(genre, &s[..n]);
            s = &s[n..];
        } else if !valeur && (est_mot(c) || c == '-') {
            let n = prendre(s, |c| est_mot(c) || c == '-');
            m.pousser("attr", &s[..n]);
            s = &s[n..];
        } else if c == ':' || c == ';' {
            valeur = c == ':';
            m.pousser("ponct", &s[..1]);
            s = &s[1..];
        } else if c == '#' || c.is_ascii_digit() || (c == '-' && s[1..].starts_with(|c: char| c.is_ascii_digit())) {
            let n = un(s) + nombre(&s[un(s)..]);
            m.pousser("nombre", &s[..n]);
            s = &s[n..];
        } else if est_mot(c) || c == '-' {
            let n = prendre(s, |c| est_mot(c) || c == '-');
            let genre = if s[n..].starts_with('(') { "fonction" } else { "tx" };
            m.pousser(genre, &s[..n]);
            s = &s[n..];
        } else {
            let n = un(s);
            m.pousser(if c.is_whitespace() { "tx" } else { "ponct" }, &s[..n]);
            s = &s[n..];
        }
    }
}

/// Dans un bloc, une ligne qui ouvre un bloc imbrique (`@media`) : ses
/// mots sont des selecteurs, pas des proprietes.
fn rsc_selecteur_imbrique(s: &str) -> bool {
    let fin = s.find([';', '}']).unwrap_or(s.len());
    s[..fin].contains('{')
}

// ---------------------------------------------------------------- rsS

const RSS_MOTS: &[&str] = &[
    "select", "from", "where", "insert", "into", "values", "update", "set", "delete", "create", "table", "drop", "index", "unique", "on", "join", "left", "inner", "as", "and", "or", "not", "null", "is", "in", "like", "between",
    "order", "by", "asc", "desc", "limit", "offset", "group", "having", "distinct", "primary", "key", "default", "if", "exists", "begin", "commit", "rollback", "show", "tables", "describe", "share", "unshare",
    "with", "public", "protected", "read", "write", "to", "autoincrement", "true", "false", "all", "transaction",
];
const RSS_TYPES: &[&str] = &["int", "integer", "text", "real", "float", "bool", "boolean", "blob"];

fn rss(ligne: &str, m: &mut Morceaux) {
    let mut s = ligne;
    while !s.is_empty() {
        let c = s.chars().next().unwrap();
        if s.starts_with("--") {
            m.pousser("commentaire", s);
            return;
        } else if c == '\'' || c == '"' {
            let n = chaine(s);
            m.pousser("chaine", &s[..n]);
            s = &s[n..];
        } else if c == '?' {
            m.pousser("interp", "?");
            s = &s[1..];
        } else if c == '@' {
            // `@3.notes` : la table d'une autre app.
            let n = 1 + prendre(&s[1..], |c| c.is_ascii_digit());
            m.pousser("interp", &s[..n]);
            s = &s[n..];
        } else if c.is_ascii_digit() {
            let n = nombre(s);
            m.pousser("nombre", &s[..n]);
            s = &s[n..];
        } else if est_mot(c) {
            let n = prendre(s, est_mot);
            let mot = s[..n].to_lowercase();
            let genre = if RSS_TYPES.contains(&mot.as_str()) {
                "classe"
            } else if RSS_MOTS.contains(&mot.as_str()) {
                "mot"
            } else if s[n..].starts_with('(') {
                "fonction"
            } else {
                "tx"
            };
            m.pousser(genre, &s[..n]);
            s = &s[n..];
        } else {
            let n = un(s);
            m.pousser(if c.is_whitespace() { "tx" } else { "ponct" }, &s[..n]);
            s = &s[n..];
        }
    }
}

// ---------------------------------------------------------------- Rust

const RUST_MOTS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self",
    "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
];

fn rust(ligne: &str, m: &mut Morceaux, etat: &mut Etat) {
    let mut s = ligne;
    while !s.is_empty() {
        let c = s.chars().next().unwrap();
        if etat.commentaire {
            let n = s.find("*/").map_or(s.len(), |i| i + 2);
            etat.commentaire = !s[..n].ends_with("*/");
            m.pousser("commentaire", &s[..n]);
            s = &s[n..];
        } else if s.starts_with("//") {
            m.pousser("commentaire", s);
            return;
        } else if s.starts_with("/*") {
            etat.commentaire = true;
            m.pousser("commentaire", "/*");
            s = &s[2..];
        } else if c == '"' {
            let n = chaine(s);
            m.pousser("chaine", &s[..n]);
            s = &s[n..];
        } else if c == '\'' {
            // 'a' (caractere) ou 'a (duree de vie)
            let fin = s[1..].char_indices().nth(1).filter(|&(_, c)| c == '\'').map(|(i, _)| i + 2);
            let n = match fin {
                Some(n) => n,
                None if s[1..].starts_with('\\') => chaine(s),
                None => 1 + prendre(&s[1..], est_mot),
            };
            m.pousser(if fin.is_some() || s[1..].starts_with('\\') { "chaine" } else { "mot" }, &s[..n]);
            s = &s[n..];
        } else if c.is_ascii_digit() {
            let n = nombre(s);
            m.pousser("nombre", &s[..n]);
            s = &s[n..];
        } else if est_mot(c) {
            let n = prendre(s, est_mot);
            let mot = &s[..n];
            let reste = &s[n..];
            let genre = if RUST_MOTS.contains(&mot) {
                "mot"
            } else if reste.starts_with('!') || reste.starts_with('(') {
                "fonction"
            } else if c.is_uppercase() {
                "classe"
            } else {
                "tx"
            };
            let n = if reste.starts_with('!') { n + 1 } else { n };
            m.pousser(genre, &s[..n]);
            s = &s[n..];
        } else if c == '#' && s[1..].starts_with('[') {
            let n = s.find(']').map_or(s.len(), |i| i + 1);
            m.pousser("commentaire", &s[..n]);
            s = &s[n..];
        } else {
            let n = un(s);
            m.pousser(if c.is_whitespace() { "tx" } else { "ponct" }, &s[..n]);
            s = &s[n..];
        }
    }
}

// ---------------------------------------------------------------- toml, sh

fn toml(ligne: &str, m: &mut Morceaux) {
    let t = ligne.trim_start();
    if t.starts_with('#') {
        m.pousser("commentaire", ligne);
    } else if t.starts_with('[') {
        m.pousser("balise", ligne);
    } else if let Some(i) = ligne.find('=') {
        m.pousser("attr", &ligne[..i]);
        m.pousser("ponct", "=");
        valeurs(&ligne[i + 1..], m);
    } else {
        valeurs(ligne, m);
    }
}

/// Chaines, nombres et le reste : valeurs toml, arguments sh.
fn valeurs(mut s: &str, m: &mut Morceaux) {
    while !s.is_empty() {
        let c = s.chars().next().unwrap();
        let (genre, n) = if c == '"' || c == '\'' {
            ("chaine", chaine(s))
        } else if c == '#' && m.0.last().is_none_or(|d| d.1.ends_with(' ')) {
            ("commentaire", s.len())
        } else if c.is_ascii_digit() {
            ("nombre", nombre(s))
        } else if c == '$' {
            ("interp", 1 + prendre(&s[1..], |c| est_mot(c) || c == '{' || c == '}'))
        } else if c == '-' && m.0.last().is_none_or(|d| d.1.ends_with(' ')) {
            ("attr", prendre(s, |c| !c.is_whitespace()))
        } else if c == '|' || c == '&' || c == ';' || c == '>' || c == '=' || c == '[' || c == ']' || c == ',' {
            ("ponct", un(s))
        } else {
            ("tx", prendre(s, |c| !matches!(c, '"' | '\'' | '|' | '&' | ';' | '>' | '=' | '$' | '[' | ']' | ',')).max(un(s)))
        };
        m.pousser(genre, &s[..n]);
        s = &s[n..];
    }
}

fn sh(ligne: &str, m: &mut Morceaux) {
    let t = ligne.trim_start();
    if t.starts_with('#') {
        m.pousser("commentaire", ligne);
        return;
    }
    // Le premier mot de la ligne est la commande.
    let blanc = ligne.len() - t.len();
    m.pousser("tx", &ligne[..blanc]);
    let n = prendre(t, |c| !c.is_whitespace());
    m.pousser("fonction", &t[..n]);
    valeurs(&t[n..], m);
}
