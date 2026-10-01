use crate::compiler::rsc::models::token::{Position, Spanned, TokenRsC};

fn advance(line: &mut usize, column: &mut usize, c: char) {
    if c == '\n' {
        *line += 1;
        *column = 1;
    } else {
        *column += 1;
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

pub fn tokenize(input: &str) -> Vec<Spanned<TokenRsC>> {
    let mut chars = input.chars().peekable();
    let mut result = Vec::new();

    let mut line = 1usize;
    let mut column = 1usize;

    while let Some(&c) = chars.peek() {
        let pos = Position { line, column };

        // Un run d'espaces et/ou de commentaires est condense en un seul
        // token `Combinator` : c'est au parser de decider s'il est
        // significatif (combinateur descendant) ou juste separateur.
        if c.is_whitespace() || is_comment_start(&mut chars.clone()) {
            let mut saw_any = false;
            loop {
                let next_is_whitespace = matches!(chars.peek(), Some(&w) if w.is_whitespace());
                if next_is_whitespace {
                    let w = chars.next().unwrap();
                    saw_any = true;
                    advance(&mut line, &mut column, w);
                } else if is_comment_start(&mut chars.clone()) {
                    saw_any = true;
                    skip_comment(&mut chars, &mut line, &mut column);
                } else {
                    break;
                }
            }
            // Deux separateurs de suite (un `/` ignore entre deux espaces,
            // `center / cover`) n'en font qu'un.
            if saw_any && !matches!(result.last().map(|t: &Spanned<TokenRsC>| &t.value), Some(TokenRsC::Combinator)) {
                result.push(Spanned::new(TokenRsC::Combinator, pos));
            }
            continue;
        }

        chars.next();
        advance(&mut line, &mut column, c);

        match c {
            '.' => {
                // `.5` (nombre avec point decimal sans partie entiere) vs
                // `.card` (selecteur de classe) : seul le caractere qui
                // suit permet de trancher, comme en CSS reel.
                if matches!(chars.peek(), Some(d) if d.is_ascii_digit()) {
                    let (value, _) = read_number(&mut chars, &mut line, &mut column, "0.".to_string());
                    result.push(Spanned::new(finish_number(&mut chars, &mut line, &mut column, value), pos));
                } else {
                    let mut name = String::new();
                    while let Some(&next) = chars.peek() {
                        if is_ident_continue(next) {
                            name.push(next);
                            chars.next();
                            advance(&mut line, &mut column, next);
                        } else {
                            break;
                        }
                    }
                    result.push(Spanned::new(TokenRsC::ClassSel(name), pos));
                }
            }
            '#' => {
                let mut name = String::new();
                while let Some(&next) = chars.peek() {
                    if is_ident_continue(next) {
                        name.push(next);
                        chars.next();
                        advance(&mut line, &mut column, next);
                    } else {
                        break;
                    }
                }
                result.push(Spanned::new(TokenRsC::Hash(name), pos));
            }
            '*' => result.push(Spanned::new(TokenRsC::Star, pos)),
            ':' => result.push(Spanned::new(TokenRsC::Colon, pos)),
            ';' => result.push(Spanned::new(TokenRsC::SemiColon, pos)),
            ',' => result.push(Spanned::new(TokenRsC::Comma, pos)),
            '{' => result.push(Spanned::new(TokenRsC::OpenBrace, pos)),
            '}' => result.push(Spanned::new(TokenRsC::CloseBrace, pos)),
            '(' => result.push(Spanned::new(TokenRsC::OpenParen, pos)),
            ')' => result.push(Spanned::new(TokenRsC::CloseParen, pos)),
            '>' => result.push(Spanned::new(TokenRsC::GreaterThan, pos)),
            '+' => result.push(Spanned::new(TokenRsC::Plus, pos)),
            '~' => result.push(Spanned::new(TokenRsC::Tilde, pos)),
            '!' => result.push(Spanned::new(TokenRsC::Bang, pos)),
            '"' | '\'' => {
                let quote = c;
                let mut content = String::new();
                while let Some(&next) = chars.peek() {
                    if next == quote {
                        chars.next();
                        advance(&mut line, &mut column, next);
                        break;
                    }
                    if next == '\\' {
                        chars.next();
                        advance(&mut line, &mut column, next);
                        if let Some(&escaped) = chars.peek() {
                            content.push(escaped);
                            chars.next();
                            advance(&mut line, &mut column, escaped);
                        }
                        continue;
                    }
                    content.push(next);
                    chars.next();
                    advance(&mut line, &mut column, next);
                }
                result.push(Spanned::new(TokenRsC::StringLiteral(content), pos));
            }
            '-' if matches!(chars.peek(), Some(d) if d.is_ascii_digit() || *d == '.') => {
                let (value, _) = read_number(&mut chars, &mut line, &mut column, "-".to_string());
                result.push(Spanned::new(finish_number(&mut chars, &mut line, &mut column, value), pos));
            }
            d if d.is_ascii_digit() => {
                let (value, _) = read_number(&mut chars, &mut line, &mut column, d.to_string());
                result.push(Spanned::new(finish_number(&mut chars, &mut line, &mut column, value), pos));
            }
            start if is_ident_start(start) || start == '-' => {
                let mut name = String::new();
                name.push(start);
                while let Some(&next) = chars.peek() {
                    if is_ident_continue(next) {
                        name.push(next);
                        chars.next();
                        advance(&mut line, &mut column, next);
                    } else {
                        break;
                    }
                }
                // `url(images/fond.png)` sans guillemets : le chemin est lu tel
                // quel jusqu'a la parenthese fermante (il peut contenir `/`,
                // `.`, `:`...), comme en CSS.
                let unquoted_url = name.eq_ignore_ascii_case("url") && chars.peek() == Some(&'(') && {
                    let mut probe = chars.clone();
                    probe.next();
                    let first = probe.find(|c| !c.is_whitespace());
                    !matches!(first, Some('"') | Some('\'') | Some(')') | None)
                };
                result.push(Spanned::new(TokenRsC::Ident(name), pos));
                if unquoted_url {
                    chars.next();
                    advance(&mut line, &mut column, '(');
                    result.push(Spanned::new(TokenRsC::OpenParen, pos));
                    let mut raw = String::new();
                    for c in chars.by_ref() {
                        advance(&mut line, &mut column, c);
                        if c == ')' {
                            break;
                        }
                        raw.push(c);
                    }
                    result.push(Spanned::new(TokenRsC::StringLiteral(raw.trim().to_string()), pos));
                    result.push(Spanned::new(TokenRsC::CloseParen, pos));
                }
            }
            // Caractere non reconnu (ex: '/' isole hors commentaire) : on
            // l'ignore silencieusement plutot que de bloquer tout le lexer,
            // le parser produira une erreur lisible si sa presence rend la
            // grammaire invalide.
            _ => {}
        }
    }

    result
}

fn is_comment_start(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    if chars.next() != Some('/') {
        return false;
    }
    // Ne consomme pas le deuxieme caractere (juste un `peek`) : cette
    // fonction ne fait que detecter un debut de commentaire, c'est
    // `skip_comment` qui consomme reellement le flux.
    matches!(chars.peek(), Some('*') | Some('/'))
}

// Avance `chars`/`line`/`column` au-dela d'un commentaire `/* ... */` ou
// `// ...`. Le style `//` n'existe pas en CSS standard mais est tolere ici
// par confort (c'etait deja le format du mini-langage `.style` existant).
fn skip_comment(chars: &mut std::iter::Peekable<std::str::Chars>, line: &mut usize, column: &mut usize) {
    let slash = chars.next().unwrap();
    advance(line, column, slash);
    match chars.next() {
        Some('*') => {
            advance(line, column, '*');
            let mut prev_star = false;
            for c in chars.by_ref() {
                advance(line, column, c);
                if prev_star && c == '/' {
                    break;
                }
                prev_star = c == '*';
            }
        }
        Some('/') => {
            advance(line, column, '/');
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    break;
                }
                chars.next();
                advance(line, column, c);
            }
        }
        _ => {}
    }
}

// Lit les chiffres/point decimal restants d'un nombre dont le prefixe
// (`prefix`) a deja ete consomme (signe et/ou premier chiffre/point).
fn read_number(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    line: &mut usize,
    column: &mut usize,
    prefix: String,
) -> (f32, String) {
    let mut raw = prefix;
    let mut seen_dot = raw.contains('.');
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            raw.push(c);
            chars.next();
            advance(line, column, c);
        } else if c == '.' && !seen_dot {
            seen_dot = true;
            raw.push(c);
            chars.next();
            advance(line, column, c);
        } else {
            break;
        }
    }
    let value = raw.parse::<f32>().unwrap_or(0.0);
    (value, raw)
}

// Une fois les chiffres lus, regarde si un '%' ou une unite alphabetique
// suit immediatement (sans espace) pour produire Percentage/Dimension au
// lieu d'un Number brut.
fn finish_number(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    line: &mut usize,
    column: &mut usize,
    value: f32,
) -> TokenRsC {
    if chars.peek() == Some(&'%') {
        chars.next();
        advance(line, column, '%');
        return TokenRsC::Percentage(value);
    }

    if matches!(chars.peek(), Some(&c) if c.is_alphabetic()) {
        let mut unit = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_alphabetic() {
                unit.push(c);
                chars.next();
                advance(line, column, c);
            } else {
                break;
            }
        }
        return TokenRsC::Dimension(value, unit);
    }

    TokenRsC::Number(value)
}
