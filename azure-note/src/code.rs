// Coloration des blocs de code. Le bloc garde le code brut ; a l'affichage,
// il devient un texte riche (police a chasse fixe partout) dont les mots
// cles, textes, nombres et commentaires sont colores. A l'enregistrement,
// on ne garde que le texte (voir `riche::brut`).
use crate::riche::{ecrire, Segment, Style};

const MOTS_CLES: &[&str] = &[
    // Rust
    "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl", "trait", "match", "if", "else", "for", "while", "loop", "return", "self", "Self", "crate", "super", "where", "as", "in", "move", "ref", "const", "static", "type", "async", "await", "dyn", "true", "false", "break", "continue",
    // Python / JS / autres
    "def", "class", "import", "from", "function", "var", "new", "this", "null", "None", "True", "False", "elif", "try", "except", "catch", "finally", "throw", "raise", "lambda", "yield", "with", "not", "and", "or", "export", "default", "switch", "case",
    // SQL / RsS
    "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "JOIN", "ON", "ORDER", "BY", "GROUP", "LIMIT", "AND", "OR", "NOT",
];

const MOT_CLE: &str = "#c678dd";
const TEXTE: &str = "#98c379";
const NOMBRE: &str = "#d19a66";
const COMMENTAIRE: &str = "#7f848e";
const APPEL: &str = "#e5c07b";

fn seg(texte: String, couleur: &str) -> Segment {
    Segment { texte, style: Style { code: true, couleur: couleur.to_string(), ..Style::default() } }
}

/// Le code `src` en texte riche colore.
pub fn colorer(src: &str) -> String {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let debut = i;
        // Commentaires `//`, `#` (en debut de mot), `--`.
        let commentaire = (c[i] == '/' && c.get(i + 1) == Some(&'/')) || (c[i] == '-' && c.get(i + 1) == Some(&'-')) || (c[i] == '#' && (i == 0 || c[i - 1].is_whitespace()));
        if commentaire {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            out.push(seg(c[debut..i].iter().collect(), COMMENTAIRE));
        } else if c[i] == '"' || c[i] == '\'' {
            let q = c[i];
            i += 1;
            while i < c.len() && c[i] != q && c[i] != '\n' {
                if c[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(c.len());
            out.push(seg(c[debut..i].iter().collect(), TEXTE));
        } else if c[i].is_ascii_digit() {
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '.' || c[i] == '_') {
                i += 1;
            }
            out.push(seg(c[debut..i].iter().collect(), NOMBRE));
        } else if c[i].is_alphabetic() || c[i] == '_' {
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            let mot: String = c[debut..i].iter().collect();
            let couleur = if MOTS_CLES.contains(&mot.as_str()) {
                MOT_CLE
            } else if c.get(i) == Some(&'(') || c.get(i) == Some(&'!') {
                APPEL
            } else {
                ""
            };
            out.push(seg(mot, couleur));
        } else {
            i += 1;
            out.push(seg(c[debut..i].iter().collect(), ""));
        }
    }
    let s = ecrire(&out);
    // Un code vide reste vide (et pas un segment sans texte).
    if src.is_empty() { String::new() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riche::{brut, lire};

    #[test]
    fn colore_et_se_relit() {
        let src = "fn main() {\n    let x = 42; // reponse\n    println!(\"salut\");\n}";
        let riche = colorer(src);
        assert_eq!(brut(&riche), src);
        let segs = lire(&riche);
        let couleur = |t: &str| segs.iter().find(|s| s.texte == t).map(|s| s.style.couleur.clone()).unwrap();
        assert_eq!(couleur("fn"), MOT_CLE);
        assert_eq!(couleur("42"), NOMBRE);
        assert_eq!(couleur("// reponse"), COMMENTAIRE);
        assert_eq!(couleur("\"salut\""), TEXTE);
        assert_eq!(couleur("println"), APPEL);
        assert!(segs.iter().all(|s| s.style.code));
        assert_eq!(colorer(""), "");
    }
}
