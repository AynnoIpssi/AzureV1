// Les formules des proprietes calculees :
//
//   moyenne(enfants.avancement)
//   somme(enfants.heures) / 8
//   compte(enfants, statut = "Fait") / compte(enfants) * 100
//   si(fini, "Oui", "Non")
//   max(enfants, echeance)            (2 arguments : l'expression est
//                                      evaluee sur chaque enfant)
//
// Nombres, "textes", vrai/faux, + - * /, = != < <= > >=, et/ou/non,
// parentheses. Un nom est une propriete de la page (majuscules, accents
// d'espaces : `Temps passé` s'ecrit temps_passé). `enfants.x` est la liste
// des x des sous-pages. Fonctions : moyenne somme min max compte si arrondi
// abs concat longueur.
//
// Le moteur ne connait pas les pages : il lit par le trait `Contexte`.
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Valeur {
    Vide,
    Nombre(f64),
    Texte(String),
    Booleen(bool),
    Liste(Vec<Valeur>),
}

impl Valeur {
    pub fn nombre(&self) -> Option<f64> {
        match self {
            Valeur::Nombre(n) => Some(*n),
            Valeur::Booleen(b) => Some(if *b { 1.0 } else { 0.0 }),
            Valeur::Texte(t) => t.trim().replace(',', ".").trim_end_matches('%').trim().parse().ok(),
            _ => None,
        }
    }

    pub fn vrai(&self) -> bool {
        match self {
            Valeur::Vide => false,
            Valeur::Nombre(n) => *n != 0.0,
            Valeur::Texte(t) => !t.is_empty(),
            Valeur::Booleen(b) => *b,
            Valeur::Liste(l) => !l.is_empty(),
        }
    }
}

impl fmt::Display for Valeur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Valeur::Vide => Ok(()),
            Valeur::Nombre(n) => {
                // 2 decimales au plus, sans zeros inutiles.
                let s = format!("{:.2}", n);
                let s = s.trim_end_matches('0').trim_end_matches('.');
                write!(f, "{}", if s == "-0" { "0" } else { s })
            }
            Valeur::Texte(t) => write!(f, "{t}"),
            Valeur::Booleen(b) => write!(f, "{}", if *b { "vrai" } else { "faux" }),
            Valeur::Liste(l) => {
                let v: Vec<String> = l.iter().map(|x| x.to_string()).collect();
                write!(f, "{}", v.join(", "))
            }
        }
    }
}

/// Ce qu'une formule peut lire.
pub trait Contexte {
    /// La propriete `nom` (deja normalise) de la page ; None si elle n'existe pas.
    fn propriete(&self, nom: &str) -> Result<Option<Valeur>, String>;
    /// Les sous-pages.
    fn enfants(&self) -> Vec<Box<dyn Contexte + '_>>;
}

/// `Temps passé` -> `temps_passé`.
pub fn normaliser(nom: &str) -> String {
    nom.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("_")
}

// ---- analyse ------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Jeton {
    Nombre(f64),
    Texte(String),
    Nom(String),
    Op(&'static str),
    Ouvre,
    Ferme,
    Virgule,
    Point,
}

fn decouper(src: &str) -> Result<Vec<Jeton>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() {
            let debut = i;
            while i < c.len() && (c[i].is_ascii_digit() || (c[i] == '.' && i + 1 < c.len() && c[i + 1].is_ascii_digit())) {
                i += 1;
            }
            let s: String = c[debut..i].iter().collect();
            out.push(Jeton::Nombre(s.parse().map_err(|_| format!("nombre invalide : {s}"))?));
        } else if ch == '"' {
            i += 1;
            let mut s = String::new();
            while i < c.len() && c[i] != '"' {
                s.push(c[i]);
                i += 1;
            }
            if i == c.len() {
                return Err("texte non fermé (\")".into());
            }
            i += 1;
            out.push(Jeton::Texte(s));
        } else if ch.is_alphabetic() || ch == '_' {
            let debut = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Jeton::Nom(c[debut..i].iter().collect::<String>().to_lowercase()));
        } else {
            let deux: String = c[i..(i + 2).min(c.len())].iter().collect();
            let op = match deux.as_str() {
                "!=" => Some("!="),
                "<=" => Some("<="),
                ">=" => Some(">="),
                "==" => Some("="),
                _ => None,
            };
            if let Some(op) = op {
                out.push(Jeton::Op(op));
                i += 2;
                continue;
            }
            out.push(match ch {
                '+' => Jeton::Op("+"),
                '-' => Jeton::Op("-"),
                '*' => Jeton::Op("*"),
                '/' => Jeton::Op("/"),
                '=' => Jeton::Op("="),
                '<' => Jeton::Op("<"),
                '>' => Jeton::Op(">"),
                '(' => Jeton::Ouvre,
                ')' => Jeton::Ferme,
                ',' | ';' => Jeton::Virgule,
                '.' => Jeton::Point,
                _ => return Err(format!("caractère inattendu : {ch}")),
            });
            i += 1;
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Valeur(Valeur),
    Prop(String),
    /// `enfants.x`
    Enfants(Option<String>),
    Non(Box<Expr>),
    Moins(Box<Expr>),
    Binaire(&'static str, Box<Expr>, Box<Expr>),
    Appel(String, Vec<Expr>),
}

/// Une formule analysee, prete a etre evaluee autant de fois qu'il faut.
#[derive(Debug, Clone, PartialEq)]
pub struct Formule {
    racine: Expr,
}

struct Analyse {
    j: Vec<Jeton>,
    i: usize,
}

impl Analyse {
    fn voir(&self) -> Option<&Jeton> {
        self.j.get(self.i)
    }

    fn avancer(&mut self) -> Option<Jeton> {
        let j = self.j.get(self.i).cloned();
        self.i += 1;
        j
    }

    fn mot(&self, m: &str) -> bool {
        matches!(self.voir(), Some(Jeton::Nom(n)) if n == m)
    }

    fn op(&self, ops: &[&'static str]) -> Option<&'static str> {
        match self.voir() {
            Some(Jeton::Op(o)) => ops.iter().find(|x| *x == o).copied(),
            _ => None,
        }
    }

    fn ou(&mut self) -> Result<Expr, String> {
        let mut g = self.et()?;
        while self.mot("ou") {
            self.i += 1;
            g = Expr::Binaire("ou", Box::new(g), Box::new(self.et()?));
        }
        Ok(g)
    }

    fn et(&mut self) -> Result<Expr, String> {
        let mut g = self.non()?;
        while self.mot("et") {
            self.i += 1;
            g = Expr::Binaire("et", Box::new(g), Box::new(self.non()?));
        }
        Ok(g)
    }

    fn non(&mut self) -> Result<Expr, String> {
        if self.mot("non") {
            self.i += 1;
            return Ok(Expr::Non(Box::new(self.non()?)));
        }
        self.comparaison()
    }

    fn comparaison(&mut self) -> Result<Expr, String> {
        let g = self.addition()?;
        if let Some(op) = self.op(&["=", "!=", "<", "<=", ">", ">="]) {
            self.i += 1;
            return Ok(Expr::Binaire(op, Box::new(g), Box::new(self.addition()?)));
        }
        Ok(g)
    }

    fn addition(&mut self) -> Result<Expr, String> {
        let mut g = self.terme()?;
        while let Some(op) = self.op(&["+", "-"]) {
            self.i += 1;
            g = Expr::Binaire(op, Box::new(g), Box::new(self.terme()?));
        }
        Ok(g)
    }

    fn terme(&mut self) -> Result<Expr, String> {
        let mut g = self.unaire()?;
        while let Some(op) = self.op(&["*", "/"]) {
            self.i += 1;
            g = Expr::Binaire(op, Box::new(g), Box::new(self.unaire()?));
        }
        Ok(g)
    }

    fn unaire(&mut self) -> Result<Expr, String> {
        if self.op(&["-"]).is_some() {
            self.i += 1;
            return Ok(Expr::Moins(Box::new(self.unaire()?)));
        }
        self.primaire()
    }

    fn primaire(&mut self) -> Result<Expr, String> {
        match self.avancer() {
            Some(Jeton::Nombre(n)) => Ok(Expr::Valeur(Valeur::Nombre(n))),
            Some(Jeton::Texte(t)) => Ok(Expr::Valeur(Valeur::Texte(t))),
            Some(Jeton::Ouvre) => {
                let e = self.ou()?;
                match self.avancer() {
                    Some(Jeton::Ferme) => Ok(e),
                    _ => Err("parenthèse « ) » attendue".into()),
                }
            }
            Some(Jeton::Nom(n)) => {
                if self.voir() == Some(&Jeton::Ouvre) {
                    self.i += 1;
                    let mut args = Vec::new();
                    if self.voir() != Some(&Jeton::Ferme) {
                        loop {
                            args.push(self.ou()?);
                            match self.avancer() {
                                Some(Jeton::Virgule) => continue,
                                Some(Jeton::Ferme) => break,
                                _ => return Err(format!("« , » ou « ) » attendu dans {n}(...)")),
                            }
                        }
                    } else {
                        self.i += 1;
                    }
                    return Ok(Expr::Appel(n, args));
                }
                match n.as_str() {
                    "vrai" => return Ok(Expr::Valeur(Valeur::Booleen(true))),
                    "faux" => return Ok(Expr::Valeur(Valeur::Booleen(false))),
                    "vide" => return Ok(Expr::Valeur(Valeur::Vide)),
                    _ => {}
                }
                if n == "enfants" {
                    if self.voir() == Some(&Jeton::Point) {
                        self.i += 1;
                        return match self.avancer() {
                            Some(Jeton::Nom(p)) => Ok(Expr::Enfants(Some(p))),
                            _ => Err("nom de propriété attendu après « enfants. »".into()),
                        };
                    }
                    return Ok(Expr::Enfants(None));
                }
                Ok(Expr::Prop(n))
            }
            Some(j) => Err(format!("inattendu : {}", decrire(&j))),
            None => Err("formule incomplète".into()),
        }
    }
}

fn decrire(j: &Jeton) -> String {
    match j {
        Jeton::Nombre(n) => n.to_string(),
        Jeton::Texte(t) => format!("\"{t}\""),
        Jeton::Nom(n) => n.clone(),
        Jeton::Op(o) => o.to_string(),
        Jeton::Ouvre => "(".into(),
        Jeton::Ferme => ")".into(),
        Jeton::Virgule => ",".into(),
        Jeton::Point => ".".into(),
    }
}

const FONCTIONS: &[&str] = &["moyenne", "somme", "min", "max", "compte", "si", "arrondi", "abs", "concat", "longueur"];

impl Formule {
    pub fn analyser(src: &str) -> Result<Formule, String> {
        let j = decouper(src)?;
        if j.is_empty() {
            return Err("formule vide".into());
        }
        let mut a = Analyse { j, i: 0 };
        let racine = a.ou()?;
        if let Some(j) = a.voir() {
            return Err(format!("inattendu : {}", decrire(j)));
        }
        verifier(&racine)?;
        Ok(Formule { racine })
    }

    /// Les proprietes de la page que la formule lit (pour detecter les cycles).
    pub fn proprietes(&self) -> Vec<String> {
        let mut v = Vec::new();
        noms(&self.racine, &mut v, false);
        v
    }

    /// Lit-elle les sous-pages ?
    pub fn lit_enfants(&self) -> bool {
        fn cherche(e: &Expr) -> bool {
            match e {
                Expr::Enfants(_) => true,
                Expr::Non(x) | Expr::Moins(x) => cherche(x),
                Expr::Binaire(_, a, b) => cherche(a) || cherche(b),
                Expr::Appel(_, args) => args.iter().any(cherche),
                _ => false,
            }
        }
        cherche(&self.racine)
    }

    pub fn evaluer(&self, ctx: &dyn Contexte) -> Result<Valeur, String> {
        eval(&self.racine, ctx)
    }
}

fn verifier(e: &Expr) -> Result<(), String> {
    match e {
        Expr::Appel(f, args) => {
            if !FONCTIONS.contains(&f.as_str()) {
                return Err(format!("fonction inconnue : {f} (connues : {})", FONCTIONS.join(", ")));
            }
            let n = args.len();
            let ok = match f.as_str() {
                "moyenne" | "somme" | "min" | "max" | "compte" => (1..=2).contains(&n),
                "si" => n == 2 || n == 3,
                "arrondi" => (1..=2).contains(&n),
                "abs" | "longueur" => n == 1,
                _ => true,
            };
            if !ok {
                return Err(format!("{f} : mauvais nombre d'arguments ({n})"));
            }
            args.iter().try_for_each(verifier)
        }
        Expr::Enfants(None) => Ok(()),
        Expr::Non(x) | Expr::Moins(x) => verifier(x),
        Expr::Binaire(_, a, b) => verifier(a).and_then(|_| verifier(b)),
        _ => Ok(()),
    }
}

/// Les proprietes lues sur la page elle-meme ; `dans_enfant` : on est dans
/// le 2e argument d'une agregation, les noms y designent l'enfant.
fn noms(e: &Expr, v: &mut Vec<String>, dans_enfant: bool) {
    match e {
        Expr::Prop(p) if !dans_enfant => {
            if !v.contains(p) {
                v.push(p.clone())
            }
        }
        Expr::Non(x) | Expr::Moins(x) => noms(x, v, dans_enfant),
        Expr::Binaire(_, a, b) => {
            noms(a, v, dans_enfant);
            noms(b, v, dans_enfant);
        }
        Expr::Appel(f, args) => {
            let agregat = matches!(f.as_str(), "moyenne" | "somme" | "min" | "max" | "compte");
            for (i, a) in args.iter().enumerate() {
                noms(a, v, dans_enfant || (agregat && i == 1));
            }
        }
        _ => {}
    }
}

fn prop(ctx: &dyn Contexte, nom: &str) -> Result<Valeur, String> {
    ctx.propriete(nom)?.ok_or_else(|| format!("propriété inconnue : {nom}"))
}

/// Pour une sous-page, une propriete absente vaut Vide (les enfants n'ont
/// pas tous le meme schema).
fn prop_enfant(ctx: &dyn Contexte, nom: &str) -> Result<Valeur, String> {
    Ok(ctx.propriete(nom)?.unwrap_or(Valeur::Vide))
}

fn eval(e: &Expr, ctx: &dyn Contexte) -> Result<Valeur, String> {
    Ok(match e {
        Expr::Valeur(v) => v.clone(),
        Expr::Prop(p) => prop(ctx, p)?,
        Expr::Enfants(None) => Valeur::Liste(ctx.enfants().iter().map(|_| Valeur::Booleen(true)).collect()),
        Expr::Enfants(Some(p)) => {
            let mut l = Vec::new();
            for enfant in ctx.enfants() {
                l.push(prop_enfant(enfant.as_ref(), p)?);
            }
            Valeur::Liste(l)
        }
        Expr::Non(x) => Valeur::Booleen(!eval(x, ctx)?.vrai()),
        Expr::Moins(x) => match eval(x, ctx)?.nombre() {
            Some(n) => Valeur::Nombre(-n),
            None => Valeur::Vide,
        },
        Expr::Binaire(op, a, b) => binaire(op, a, b, ctx)?,
        Expr::Appel(f, args) => appel(f, args, ctx)?,
    })
}

/// Evalue `e` sur la sous-page : ses noms sont ceux de l'enfant.
fn eval_enfant(e: &Expr, enfant: &dyn Contexte) -> Result<Valeur, String> {
    struct Tolerant<'a>(&'a dyn Contexte);
    impl Contexte for Tolerant<'_> {
        fn propriete(&self, nom: &str) -> Result<Option<Valeur>, String> {
            Ok(Some(self.0.propriete(nom)?.unwrap_or(Valeur::Vide)))
        }
        fn enfants(&self) -> Vec<Box<dyn Contexte + '_>> {
            self.0.enfants()
        }
    }
    eval(e, &Tolerant(enfant))
}

fn binaire(op: &str, a: &Expr, b: &Expr, ctx: &dyn Contexte) -> Result<Valeur, String> {
    if op == "et" {
        return Ok(Valeur::Booleen(eval(a, ctx)?.vrai() && eval(b, ctx)?.vrai()));
    }
    if op == "ou" {
        return Ok(Valeur::Booleen(eval(a, ctx)?.vrai() || eval(b, ctx)?.vrai()));
    }
    let (x, y) = (eval(a, ctx)?, eval(b, ctx)?);
    if matches!(op, "=" | "!=" | "<" | "<=" | ">" | ">=") {
        use std::cmp::Ordering;
        let ord = match (x.nombre(), y.nombre(), &x, &y) {
            (_, _, Valeur::Vide, Valeur::Vide) => Some(Ordering::Equal),
            (_, _, Valeur::Vide, _) | (_, _, _, Valeur::Vide) => None,
            (Some(p), Some(q), _, _) => p.partial_cmp(&q),
            _ => Some(x.to_string().to_lowercase().cmp(&y.to_string().to_lowercase())),
        };
        let r = match op {
            "=" => ord == Some(Ordering::Equal),
            "!=" => ord != Some(Ordering::Equal),
            "<" => ord == Some(Ordering::Less),
            "<=" => matches!(ord, Some(Ordering::Less | Ordering::Equal)),
            ">" => ord == Some(Ordering::Greater),
            _ => matches!(ord, Some(Ordering::Greater | Ordering::Equal)),
        };
        return Ok(Valeur::Booleen(r));
    }
    // `"a" + "b"` concatene ; sinon arithmetique, Vide si un cote manque.
    if op == "+" && (matches!(x, Valeur::Texte(_)) && x.nombre().is_none() || matches!(y, Valeur::Texte(_)) && y.nombre().is_none()) {
        return Ok(Valeur::Texte(format!("{x}{y}")));
    }
    let (Some(p), Some(q)) = (x.nombre(), y.nombre()) else { return Ok(Valeur::Vide) };
    Ok(match op {
        "+" => Valeur::Nombre(p + q),
        "-" => Valeur::Nombre(p - q),
        "*" => Valeur::Nombre(p * q),
        _ if q == 0.0 => Valeur::Vide,
        _ => Valeur::Nombre(p / q),
    })
}

fn appel(f: &str, args: &[Expr], ctx: &dyn Contexte) -> Result<Valeur, String> {
    match f {
        "moyenne" | "somme" | "min" | "max" | "compte" => {
            // Les valeurs a agreger : la liste du 1er argument, ou le 2e
            // argument evalue sur chaque enfant.
            let valeurs: Vec<Valeur> = if args.len() == 2 {
                if !matches!(args[0], Expr::Enfants(None)) {
                    return Err(format!("{f}(enfants, ...) : le 1er argument doit être « enfants »"));
                }
                let mut v = Vec::new();
                for enfant in ctx.enfants() {
                    v.push(eval_enfant(&args[1], enfant.as_ref())?);
                }
                v
            } else {
                match eval(&args[0], ctx)? {
                    Valeur::Liste(l) => l,
                    Valeur::Vide => Vec::new(),
                    autre => vec![autre],
                }
            };
            if f == "compte" {
                let n = if args.len() == 2 { valeurs.iter().filter(|v| v.vrai()).count() } else { valeurs.iter().filter(|v| **v != Valeur::Vide).count() };
                return Ok(Valeur::Nombre(n as f64));
            }
            let nombres: Vec<f64> = valeurs.iter().filter_map(Valeur::nombre).collect();
            if nombres.is_empty() {
                return Ok(if f == "somme" { Valeur::Nombre(0.0) } else { Valeur::Vide });
            }
            let somme: f64 = nombres.iter().sum();
            Ok(Valeur::Nombre(match f {
                "somme" => somme,
                "moyenne" => somme / nombres.len() as f64,
                "min" => nombres.iter().cloned().fold(f64::INFINITY, f64::min),
                _ => nombres.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            }))
        }
        "si" => {
            if eval(&args[0], ctx)?.vrai() {
                eval(&args[1], ctx)
            } else if let Some(sinon) = args.get(2) {
                eval(sinon, ctx)
            } else {
                Ok(Valeur::Vide)
            }
        }
        "arrondi" => {
            let Some(x) = eval(&args[0], ctx)?.nombre() else { return Ok(Valeur::Vide) };
            let d = match args.get(1) {
                Some(a) => eval(a, ctx)?.nombre().unwrap_or(0.0).clamp(0.0, 10.0) as i32,
                None => 0,
            };
            let m = 10f64.powi(d);
            Ok(Valeur::Nombre((x * m).round() / m))
        }
        "abs" => Ok(eval(&args[0], ctx)?.nombre().map(|x| Valeur::Nombre(x.abs())).unwrap_or(Valeur::Vide)),
        "longueur" => Ok(Valeur::Nombre(match eval(&args[0], ctx)? {
            Valeur::Liste(l) => l.len(),
            Valeur::Vide => 0,
            v => v.to_string().chars().count(),
        } as f64)),
        "concat" => {
            let mut s = String::new();
            for a in args {
                s.push_str(&eval(a, ctx)?.to_string());
            }
            Ok(Valeur::Texte(s))
        }
        _ => Err(format!("fonction inconnue : {f}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct P {
        props: HashMap<String, Valeur>,
        enfants: Vec<P>,
    }

    fn p(props: &[(&str, Valeur)], enfants: Vec<P>) -> P {
        P { props: props.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(), enfants }
    }

    impl Contexte for P {
        fn propriete(&self, nom: &str) -> Result<Option<Valeur>, String> {
            Ok(self.props.get(nom).cloned())
        }
        fn enfants(&self) -> Vec<Box<dyn Contexte + '_>> {
            self.enfants.iter().map(|e| Box::new(Ref(e)) as Box<dyn Contexte + '_>).collect()
        }
    }

    struct Ref<'a>(&'a P);

    impl Contexte for Ref<'_> {
        fn propriete(&self, nom: &str) -> Result<Option<Valeur>, String> {
            self.0.propriete(nom)
        }
        fn enfants(&self) -> Vec<Box<dyn Contexte + '_>> {
            self.0.enfants()
        }
    }

    fn n(x: f64) -> Valeur {
        Valeur::Nombre(x)
    }
    fn t(s: &str) -> Valeur {
        Valeur::Texte(s.into())
    }

    fn projet() -> P {
        p(&[("titre", t("Projet")), ("budget", n(1000.0))], vec![
            p(&[("avancement", n(100.0)), ("statut", t("Fait")), ("heures", n(3.0))], vec![]),
            p(&[("avancement", n(50.0)), ("statut", t("En cours")), ("heures", n(5.0))], vec![]),
            p(&[("avancement", n(0.0)), ("statut", t("À faire"))], vec![]),
        ])
    }

    fn calc(src: &str) -> Valeur {
        Formule::analyser(src).unwrap().evaluer(&projet()).unwrap()
    }

    #[test]
    fn agregats_sur_les_enfants() {
        assert_eq!(calc("moyenne(enfants.avancement)"), n(50.0));
        assert_eq!(calc("somme(enfants.heures)"), n(8.0));
        assert_eq!(calc("min(enfants.avancement)"), n(0.0));
        assert_eq!(calc("max(enfants.avancement)"), n(100.0));
        assert_eq!(calc("compte(enfants)"), n(3.0));
        assert_eq!(calc("compte(enfants, statut = \"fait\")"), n(1.0));
        assert_eq!(calc("compte(enfants, statut = \"Fait\") / compte(enfants) * 100").to_string(), "33.33");
        assert_eq!(calc("somme(enfants, heures * 2)"), n(16.0));
    }

    #[test]
    fn arithmetique_et_priorites() {
        assert_eq!(calc("1 + 2 * 3"), n(7.0));
        assert_eq!(calc("(1 + 2) * 3"), n(9.0));
        assert_eq!(calc("-budget / 4"), n(-250.0));
        assert_eq!(calc("1 / 0"), Valeur::Vide);
        assert_eq!(calc("arrondi(10 / 3, 2)"), n(3.33));
    }

    #[test]
    fn logique_et_textes() {
        assert_eq!(calc("si(budget > 500, \"gros\", \"petit\")"), t("gros"));
        assert_eq!(calc("budget >= 1000 et non faux"), Valeur::Booleen(true));
        assert_eq!(calc("titre + \" !\""), t("Projet !"));
        assert_eq!(calc("concat(titre, \" : \", compte(enfants))"), t("Projet : 3"));
        assert_eq!(calc("longueur(titre)"), n(6.0));
    }

    #[test]
    fn erreurs_claires() {
        assert!(Formule::analyser("moyenne(").is_err());
        assert!(Formule::analyser("truc(1)").unwrap_err().contains("fonction inconnue"));
        assert!(Formule::analyser("1 +").is_err());
        assert!(Formule::analyser("\"ouvert").is_err());
        let f = Formule::analyser("inconnu * 2").unwrap();
        assert!(f.evaluer(&projet()).unwrap_err().contains("propriété inconnue"));
    }

    #[test]
    fn dependances() {
        let f = Formule::analyser("a + moyenne(enfants.x) + somme(enfants, b) + c").unwrap();
        assert_eq!(f.proprietes(), vec!["a".to_string(), "c".to_string()]);
        assert!(f.lit_enfants());
        assert!(!Formule::analyser("a * 2").unwrap().lit_enfants());
    }

    #[test]
    fn affichage() {
        assert_eq!(n(2.5).to_string(), "2.5");
        assert_eq!(n(3.0).to_string(), "3");
        assert_eq!(Valeur::Nombre(100.0 / 3.0).to_string(), "33.33");
        assert_eq!(normaliser("  Temps   Passé "), "temps_passé");
    }
}
