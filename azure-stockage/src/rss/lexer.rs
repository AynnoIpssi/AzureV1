// Decoupe un texte RsS en jetons. Mots-cles et noms sans guillemets sont
// insensibles a la casse (mis en minuscules) ; "Nom" garde sa casse.

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// Mot (mot-cle ou nom). `quoted` = ecrit entre guillemets doubles :
    /// jamais un mot-cle.
    Word { text: String, quoted: bool },
    Int(i64),
    Float(f64),
    Str(String),
    /// Parametre `?`, remplace par une valeur fournie a part.
    Param,
    Sym(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
    pub column: usize,
}

const SYMBOLS: [&str; 20] = ["<>", "!=", "<=", ">=", "||", "(", ")", ",", ";", "*", "=", "<", ">", "+", "-", "/", "%", ".", "@", "?"];

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let (mut i, mut line, mut column) = (0usize, 1usize, 1usize);
    let advance = |i: &mut usize, line: &mut usize, column: &mut usize, n: usize, chars: &[char]| {
        for c in &chars[*i..*i + n] {
            if *c == '\n' {
                *line += 1;
                *column = 1;
            } else {
                *column += 1;
            }
        }
        *i += n;
    };
    while i < chars.len() {
        let c = chars[i];
        let (start_line, start_column) = (line, column);
        let err = |msg: &str| format!("RsS ligne {start_line}, colonne {start_column} : {msg}");
        if c.is_whitespace() {
            advance(&mut i, &mut line, &mut column, 1, &chars);
            continue;
        }
        // Commentaires `-- ...` et `/* ... */`.
        if c == '-' && chars.get(i + 1) == Some(&'-') {
            let len = chars[i..].iter().position(|&c| c == '\n').unwrap_or(chars.len() - i);
            advance(&mut i, &mut line, &mut column, len, &chars);
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            let end = (i + 2..chars.len().saturating_sub(1)).find(|&j| chars[j] == '*' && chars[j + 1] == '/').ok_or_else(|| err("commentaire /* jamais ferme"))?;
            let len = end + 2 - i;
            advance(&mut i, &mut line, &mut column, len, &chars);
            continue;
        }
        let tok = if c.is_alphabetic() || c == '_' {
            let len = chars[i..].iter().position(|&c| !(c.is_alphanumeric() || c == '_')).unwrap_or(chars.len() - i);
            let text: String = chars[i..i + len].iter().collect::<String>().to_lowercase();
            advance(&mut i, &mut line, &mut column, len, &chars);
            Tok::Word { text, quoted: false }
        } else if c.is_ascii_digit() {
            // `1.5` est un nombre, mais dans `@1.notes` le point separe
            // l'app de la table : il faut un chiffre apres le point.
            let mut len = chars[i..].iter().position(|c| !c.is_ascii_digit()).unwrap_or(chars.len() - i);
            if chars.get(i + len) == Some(&'.') && chars.get(i + len + 1).is_some_and(char::is_ascii_digit) {
                len += 1 + chars[i + len + 1..].iter().position(|c| !c.is_ascii_digit()).unwrap_or(chars.len() - i - len - 1);
            }
            let text: String = chars[i..i + len].iter().collect();
            advance(&mut i, &mut line, &mut column, len, &chars);
            if text.contains('.') {
                Tok::Float(text.parse().map_err(|_| err(&format!("nombre invalide '{text}'")))?)
            } else {
                Tok::Int(text.parse().map_err(|_| err(&format!("nombre trop grand '{text}'")))?)
            }
        } else if c == '\'' || c == '"' {
            // Texte 'it''s' ou nom "Ma Colonne" : le guillemet double se
            // double pour s'ecrire lui-meme.
            let mut text = String::new();
            let mut j = i + 1;
            loop {
                match chars.get(j) {
                    None => return Err(err("texte jamais ferme")),
                    Some(&q) if q == c && chars.get(j + 1) == Some(&c) => {
                        text.push(c);
                        j += 2;
                    }
                    Some(&q) if q == c => break,
                    Some(&other) => {
                        text.push(other);
                        j += 1;
                    }
                }
            }
            let len = j + 1 - i;
            advance(&mut i, &mut line, &mut column, len, &chars);
            if c == '\'' { Tok::Str(text) } else { Tok::Word { text, quoted: true } }
        } else {
            let rest: String = chars[i..(i + 2).min(chars.len())].iter().collect();
            let sym = SYMBOLS.iter().find(|s| rest.starts_with(**s)).ok_or_else(|| err(&format!("caractere inattendu '{c}'")))?;
            advance(&mut i, &mut line, &mut column, sym.chars().count(), &chars);
            if *sym == "?" { Tok::Param } else { Tok::Sym(sym) }
        };
        tokens.push(Token { tok, line: start_line, column: start_column });
    }
    Ok(tokens)
}
