// Lire un fichier Rust : les fonctions (avec leur type quand elles sont dans
// un `impl` ou un `trait`), les appels de chacune, les `use`, les `mod x;`
// et les types definis. Pas un vrai compilateur : un decoupage en jetons
// (commentaires, textes, caracteres et lifetimes compris) puis une lecture
// des accolades. Les modules `#[cfg(test)]` sont sautes (ils appellent
// tout et brouilleraient la carte).
use super::{mot_cle, Appel, Fichier, Fonction, Import, Langage};

#[derive(Clone, Debug, PartialEq)]
pub enum Jeton {
    Mot(String),
    Symbole(char),
    /// Texte, caractere, nombre : seulement leur place compte.
    Litteral,
}

/// Les jetons et leur ligne.
pub fn jetons(source: &str) -> Vec<(Jeton, usize)> {
    let c: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut ligne = 1;
    let compter = |de: usize, a: usize, ligne: &mut usize| *ligne += c[de..a.min(c.len())].iter().filter(|x| **x == '\n').count();
    while i < c.len() {
        let x = c[i];
        if x == '\n' {
            ligne += 1;
            i += 1;
        } else if x.is_whitespace() {
            i += 1;
        } else if x == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if x == '/' && c.get(i + 1) == Some(&'*') {
            let debut = i;
            let mut prof = 0;
            while i < c.len() {
                if c[i] == '/' && c.get(i + 1) == Some(&'*') {
                    prof += 1;
                    i += 2;
                } else if c[i] == '*' && c.get(i + 1) == Some(&'/') {
                    prof -= 1;
                    i += 2;
                    if prof == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            compter(debut, i, &mut ligne);
        } else if (x == 'r' || x == 'b') && brut(&c, i).is_some() {
            // r"..", r#".."#, br"..".
            let (debut_texte, dieses) = brut(&c, i).unwrap();
            let debut = i;
            i = debut_texte;
            loop {
                if i >= c.len() {
                    break;
                }
                if c[i] == '"' && (0..dieses).all(|k| c.get(i + 1 + k) == Some(&'#')) {
                    i += 1 + dieses;
                    break;
                }
                i += 1;
            }
            compter(debut, i, &mut ligne);
            out.push((Jeton::Litteral, ligne));
        } else if x == '"' || (x == 'b' && c.get(i + 1) == Some(&'"')) {
            let debut = i;
            let lg = ligne;
            i += if x == 'b' { 2 } else { 1 };
            while i < c.len() && c[i] != '"' {
                if c[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            compter(debut, i, &mut ligne);
            out.push((Jeton::Litteral, lg));
        } else if x == '\'' || (x == 'b' && c.get(i + 1) == Some(&'\'')) {
            let j = if x == 'b' { i + 1 } else { i };
            // Caractere ('a', '\n', '\u{..}') ou lifetime ('a).
            if c.get(j + 1) == Some(&'\\') {
                i = j + 2;
                while i < c.len() && c[i] != '\'' && c[i] != '\n' {
                    i += 1;
                }
                i += 1;
                out.push((Jeton::Litteral, ligne));
            } else if c.get(j + 2) == Some(&'\'') {
                i = j + 3;
                out.push((Jeton::Litteral, ligne));
            } else {
                i = j + 1;
                while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                    i += 1;
                }
            }
        } else if x.is_ascii_digit() {
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || (c[i] == '.' && c.get(i + 1).is_some_and(|d| d.is_ascii_digit()))) {
                i += 1;
            }
            out.push((Jeton::Litteral, ligne));
        } else if x.is_alphabetic() || x == '_' {
            let debut = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            let mut mot: String = c[debut..i].iter().collect();
            // r#mot
            if mot == "r" && c.get(i) == Some(&'#') && c.get(i + 1).is_some_and(|y| y.is_alphabetic()) {
                let d = i + 1;
                i = d;
                while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                    i += 1;
                }
                mot = c[d..i].iter().collect();
            }
            out.push((Jeton::Mot(mot), ligne));
        } else {
            out.push((Jeton::Symbole(x), ligne));
            i += 1;
        }
    }
    out
}

// Un texte brut commence en `i` : (debut du texte, nombre de #).
fn brut(c: &[char], i: usize) -> Option<(usize, usize)> {
    let mut j = i;
    if c[j] == 'b' {
        j += 1;
    }
    if c.get(j) != Some(&'r') {
        return None;
    }
    if i > 0 && (c[i - 1].is_alphanumeric() || c[i - 1] == '_') {
        return None;
    }
    j += 1;
    let mut dieses = 0;
    while c.get(j) == Some(&'#') {
        dieses += 1;
        j += 1;
    }
    (c.get(j) == Some(&'"')).then_some((j + 1, dieses))
}

#[derive(Clone, Debug)]
enum Bloc {
    /// impl / trait : le type proprietaire.
    Type(String),
    Fonction(usize),
    Autre,
}

struct Lecteur {
    j: Vec<(Jeton, usize)>,
    i: usize,
}

impl Lecteur {
    fn mot_a(&self, k: usize) -> Option<&str> {
        match self.j.get(k) {
            Some((Jeton::Mot(m), _)) => Some(m),
            _ => None,
        }
    }

    fn symbole_a(&self, k: usize, s: char) -> bool {
        matches!(self.j.get(k), Some((Jeton::Symbole(x), _)) if *x == s)
    }

    fn ligne(&self, k: usize) -> usize {
        self.j.get(k).map_or(0, |x| x.1)
    }

    /// L'indice du jeton qui ferme le `{` / `(` / `[` en `k`.
    fn fermant(&self, k: usize) -> usize {
        let mut prof = 0i32;
        let mut i = k;
        while i < self.j.len() {
            if let (Jeton::Symbole(s), _) = &self.j[i] {
                match s {
                    '{' | '(' | '[' => prof += 1,
                    '}' | ')' | ']' => {
                        prof -= 1;
                        if prof == 0 {
                            return i;
                        }
                    }
                    _ => {}
                }
            }
            i += 1;
        }
        self.j.len().saturating_sub(1)
    }
}

/// Le type d'un `impl ... {` : celui apres `for` s'il y en a un, sinon le
/// dernier nom hors des `<>`.
fn type_impl(j: &[(Jeton, usize)]) -> String {
    let mut prof = 0;
    let mut morceau: Vec<&str> = Vec::new();
    for (t, _) in j {
        match t {
            Jeton::Symbole('<') => prof += 1,
            Jeton::Symbole('>') => prof -= 1,
            Jeton::Mot(m) if prof == 0 && m == "where" => break,
            Jeton::Mot(m) if prof == 0 && m == "for" => morceau.clear(),
            Jeton::Mot(m) if prof == 0 && !["mut", "dyn", "unsafe", "const"].contains(&m.as_str()) => morceau.push(m),
            _ => {}
        }
    }
    morceau.last().map(|s| s.to_string()).unwrap_or_default()
}

/// Lit un `use` a partir du jeton apres `use` ; rend l'indice apres le `;`.
fn lire_use(l: &Lecteur, mut i: usize, prefixe: Vec<String>, out: &mut Vec<Import>) -> usize {
    let mut chemin = prefixe;
    loop {
        match l.j.get(i) {
            Some((Jeton::Mot(m), _)) => {
                if m == "as" {
                    if let Some(local) = l.mot_a(i + 1) {
                        out.push(Import::Rust { chemin: chemin.clone(), local: local.to_string() });
                    }
                    return i + 2;
                }
                chemin.push(m.clone());
                i += 1;
            }
            Some((Jeton::Symbole(':'), _)) => i += 1,
            Some((Jeton::Symbole('*'), _)) => {
                out.push(Import::Rust { chemin: chemin.clone(), local: String::new() });
                return i + 1;
            }
            Some((Jeton::Symbole('{'), _)) => {
                // Liste : chaque element avec le meme prefixe.
                let fin = l.fermant(i);
                let mut k = i + 1;
                while k < fin {
                    k = lire_use(l, k, chemin.clone(), out);
                    while k < fin && !l.symbole_a(k, ',') {
                        k += 1;
                    }
                    k += 1;
                }
                return fin + 1;
            }
            _ => {
                // Fin : `;`, `,` ou `}`.
                if let Some(dernier) = chemin.last().cloned() {
                    let local = if dernier == "self" { chemin.get(chemin.len().wrapping_sub(2)).cloned().unwrap_or_default() } else { dernier };
                    let chemin = if chemin.last().is_some_and(|d| d == "self") { chemin[..chemin.len() - 1].to_vec() } else { chemin };
                    out.push(Import::Rust { chemin, local });
                }
                return i;
            }
        }
    }
}

/// Lit le fichier `chemin` (relatif) de contenu `source`.
pub fn lire(chemin: &str, source: &str) -> Fichier {
    let l = Lecteur { j: jetons(source), i: 0 };
    let mut f = Fichier { chemin: chemin.to_string(), langage: Langage::Rust, lignes: source.lines().count(), fonctions: Vec::new(), imports: Vec::new(), modules: Vec::new(), types: Vec::new(), test: super::est_test(chemin) };
    let mut pile: Vec<Bloc> = Vec::new();
    // Ce qu'ouvrira la prochaine `{` (impl, trait, fn).
    let mut attendu: Option<Bloc> = None;
    let mut i = l.i;
    let n = l.j.len();
    let fonction_courante = |pile: &[Bloc]| pile.iter().rev().find_map(|b| if let Bloc::Fonction(k) = b { Some(*k) } else { None });
    let proprietaire = |pile: &[Bloc]| pile.iter().rev().find_map(|b| if let Bloc::Type(t) = b { Some(t.clone()) } else { None }).unwrap_or_default();
    while i < n {
        match &l.j[i].0 {
            Jeton::Symbole('#') => {
                // #[cfg(test)] : saute l'element suivant (mod tests { ... }).
                if l.symbole_a(i + 1, '[') {
                    let fin = l.fermant(i + 1);
                    let attribut: Vec<&str> = (i + 2..fin).filter_map(|k| l.mot_a(k)).collect();
                    i = fin + 1;
                    if attribut == ["cfg", "test"] {
                        while i < n && !l.symbole_a(i, '{') && !l.symbole_a(i, ';') {
                            i += 1;
                        }
                        if l.symbole_a(i, '{') {
                            i = l.fermant(i);
                        }
                        i += 1;
                    }
                    continue;
                }
                i += 1;
            }
            Jeton::Symbole('{') => {
                pile.push(attendu.take().unwrap_or(Bloc::Autre));
                i += 1;
            }
            Jeton::Symbole('}') => {
                if let Some(Bloc::Fonction(k)) = pile.pop() {
                    f.fonctions[k].fin = l.ligne(i);
                }
                i += 1;
            }
            Jeton::Symbole(';') => {
                // `fn f();` (trait) : pas de corps.
                if let Some(Bloc::Fonction(k)) = attendu.take() {
                    f.fonctions.remove(k);
                }
                i += 1;
            }
            Jeton::Mot(m) => {
                let m = m.as_str();
                match m {
                    "macro_rules" if l.symbole_a(i + 1, '!') => {
                        // Le corps d'une macro : pas du code a lire.
                        let mut k = i + 2;
                        while k < n && !l.symbole_a(k, '{') && !l.symbole_a(k, '(') {
                            k += 1;
                        }
                        i = l.fermant(k) + 1;
                    }
                    "use" => {
                        i = lire_use(&l, i + 1, Vec::new(), &mut f.imports);
                    }
                    "mod" if l.mot_a(i + 1).is_some() && l.symbole_a(i + 2, ';') => {
                        f.modules.push(l.mot_a(i + 1).unwrap().to_string());
                        i += 3;
                    }
                    // Un bloc impl (pas `-> impl Trait` ni `x: impl Trait`).
                    "impl" if i == 0 || ['}', ';', '{', ']'].iter().any(|c| l.symbole_a(i - 1, *c)) || matches!(l.mot_a(i - 1), Some("unsafe" | "default")) => {
                        // impl Type { / impl Trait for Type {
                        let mut k = i + 1;
                        while k < n && !l.symbole_a(k, '{') && !l.symbole_a(k, ';') {
                            k += 1;
                        }
                        if l.symbole_a(k, '{') {
                            attendu = Some(Bloc::Type(type_impl(&l.j[i + 1..k])));
                        }
                        i = k;
                    }
                    "trait" if l.mot_a(i + 1).is_some() => {
                        let nom = l.mot_a(i + 1).unwrap().to_string();
                        f.types.push(nom.clone());
                        let mut k = i + 2;
                        while k < n && !l.symbole_a(k, '{') && !l.symbole_a(k, ';') {
                            k += 1;
                        }
                        if l.symbole_a(k, '{') {
                            attendu = Some(Bloc::Type(nom));
                        }
                        i = k;
                    }
                    "struct" | "enum" | "union" | "type" if l.mot_a(i + 1).is_some() && !matches!(l.mot_a(i.wrapping_sub(1)), Some("impl")) => {
                        f.types.push(l.mot_a(i + 1).unwrap().to_string());
                        i += 2;
                    }
                    "fn" if l.mot_a(i + 1).is_some() => {
                        let nom = l.mot_a(i + 1).unwrap().to_string();
                        // Methode : directement dans un impl / trait (une fn dans
                        // une fn n'en est pas une).
                        let proprio = if matches!(pile.last(), Some(Bloc::Type(_))) { proprietaire(&pile) } else { String::new() };
                        f.fonctions.push(Fonction { nom, proprietaire: proprio, ligne: l.ligne(i), fin: l.ligne(i), appels: Vec::new(), defaut: false });
                        attendu = Some(Bloc::Fonction(f.fonctions.len() - 1));
                        // La signature : jusqu'au `{` ou `;` hors parentheses.
                        let mut k = i + 2;
                        while k < n {
                            if l.symbole_a(k, '(') || l.symbole_a(k, '[') {
                                k = l.fermant(k) + 1;
                                continue;
                            }
                            if l.symbole_a(k, '{') || l.symbole_a(k, ';') {
                                break;
                            }
                            k += 1;
                        }
                        i = k;
                    }
                    _ => {
                        if let Some(k) = fonction_courante(&pile) {
                            appel(&l, i, &mut f.fonctions[k].appels);
                        }
                        i += 1;
                    }
                }
            }
            _ => i += 1,
        }
    }
    f
}

// Le mot en `i` est-il un appel ? `f(`, `x.f(`, `A::f(`, `f::<T>(`, et
// les chemins `A::f` passes en argument (`.map(Self::f)`).
fn appel(l: &Lecteur, i: usize, out: &mut Vec<Appel>) {
    let Some(nom) = l.mot_a(i) else { return };
    if mot_cle(nom) {
        return;
    }
    // Pas le milieu d'un chemin (le dernier segment seulement).
    if l.symbole_a(i + 1, ':') && l.symbole_a(i + 2, ':') && !l.symbole_a(i + 3, '<') {
        return;
    }
    // Un appel de macro : `f!(`.
    if l.symbole_a(i + 1, '!') {
        return;
    }
    let mut k = i + 1;
    if l.symbole_a(k, ':') && l.symbole_a(k + 1, ':') && l.symbole_a(k + 2, '<') {
        // turbofish
        let mut prof = 0;
        k += 2;
        while k < l.j.len() {
            if l.symbole_a(k, '<') {
                prof += 1;
            } else if l.symbole_a(k, '>') {
                prof -= 1;
                if prof == 0 {
                    k += 1;
                    break;
                }
            }
            k += 1;
        }
    }
    let parenthese = l.symbole_a(k, '(');
    let methode = i > 0 && l.symbole_a(i - 1, '.');
    // Le chemin qui precede : A::B::f.
    let mut chemin = Vec::new();
    let mut p = i;
    while p >= 3 && l.symbole_a(p - 1, ':') && l.symbole_a(p - 2, ':') {
        // A::<T>::f : saute les generiques.
        let mut q = p - 3;
        if l.symbole_a(q, '>') {
            let mut prof = 0;
            while q > 0 {
                if l.symbole_a(q, '>') {
                    prof += 1;
                } else if l.symbole_a(q, '<') {
                    prof -= 1;
                    if prof == 0 {
                        break;
                    }
                }
                q -= 1;
            }
            if q < 3 || !l.symbole_a(q - 1, ':') {
                break;
            }
            q -= 3;
        }
        match l.mot_a(q) {
            Some(m) => {
                chemin.insert(0, m.to_string());
                p = q;
            }
            None => break,
        }
    }
    let commence_minuscule = nom.chars().next().is_some_and(|c| c.is_lowercase() || c == '_');
    if !commence_minuscule {
        return; // Type, variante : pas une fonction.
    }
    // Une reference sans parenthese : seulement un chemin (`Self::f`).
    if !parenthese && (chemin.is_empty() || methode) {
        return;
    }
    // `fn f(` deja vu ; `let f(` n'existe pas ; `.0(` non plus.
    out.push(Appel { nom: nom.to_string(), chemin, methode, ligne: l.ligne(i) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonctions_appels_et_use() {
        let source = r##"
use crate::ui::{models::toile::{Toile, Dessin as D}, services::*};
use super::x;
/// Un commentaire avec fn faux(
const C: &str = r#"fn pas_une_fonction() { "#;
struct Boite<'a> { t: &'a str }
impl<'a> Trait for Boite<'a> {
    fn methode(&self) -> char {
        let c = '{';
        aide(1);
        self.autre().fin();
        Toile::cadrer(x);
        crate::a::b::libre::<u8>(2);
        v.iter().map(Self::autre)
    }
}
fn aide(x: u8) { println!("{}", calcul(x)); fn interne() {} }
#[cfg(test)]
mod tests { fn test_cache() { aide(2); } }
trait Trait { fn requise(&self); fn par_defaut(&self) { requise(); } }
"##;
        let f = lire("src/a.rs", source);
        let noms: Vec<String> = f.fonctions.iter().map(|x| x.nom_complet()).collect();
        assert_eq!(noms, ["Boite::methode", "aide", "interne", "Trait::par_defaut"]);
        let appels: Vec<String> = f.fonctions[0].appels.iter().map(|a| format!("{}{}{}", a.chemin.join("::"), if a.methode { "." } else { ":" }, a.nom)).collect();
        assert_eq!(appels, [":aide", ".autre", ".fin", "Toile:cadrer", "crate::a::b:libre", ".iter", ".map", "Self:autre"]);
        assert_eq!(f.fonctions[1].appels.iter().map(|a| a.nom.as_str()).collect::<Vec<_>>(), ["calcul"]);
        assert_eq!(f.fonctions[0].ligne, 8);
        assert_eq!(f.fonctions[0].fin, 15);
        let imports: Vec<String> = f.imports.iter().map(|i| match i {
            Import::Rust { chemin, local } => format!("{}={}", chemin.join("::"), local),
            _ => String::new(),
        }).collect();
        assert_eq!(imports, ["crate::ui::models::toile::Toile=Toile", "crate::ui::models::toile::Dessin=D", "crate::ui::services=", "super::x=x"]);
        assert_eq!(f.types, ["Boite", "Trait"]);
    }
}
