use std::iter::Peekable;
use std::str::Chars;

use azure_engine::rendering::models::color::Color;

use crate::style::models::style::Style;
use crate::style::models::stylesheet::Stylesheet;

/// Parse une feuille de style : une suite de blocs `.nom { propriete: valeur; ... }`.
/// C'est le format texte du systeme de composants reutilisables - chaque
/// bloc definit un `Style` nomme que n'importe quelle balise .rsh peut
/// appliquer via `.nom`.
///
/// Proprietes reconnues: `x`, `y`, `width`, `height`, `margin`, `padding`,
/// `radius` (valeurs numeriques), `background`, `color` (`#rrggbb` ou
/// `#rrggbbaa`), `font-size`, `font-weight` (valeurs numeriques). C'EST TOUT :
/// pas de selecteurs (balise/id/imbrication/combinateurs), pas de
/// pseudo-classes (`:hover`/`:focus`), et surtout pas de flex/grid (`display`,
/// `flex-*`, `grid-template-*`...) - une regle qui en utilise une echoue au
/// parsing ("propriete de style inconnue"), elle ne les ignore pas
/// silencieusement, mais rien ici ne permet de les exprimer non plus.
///
/// LEGACY : `compiler::rsc` (voir `style::mod` pour la comparaison complete)
/// couvre tout ce sous-ensemble ET le reste (selecteurs, pseudo-classes,
/// flex/grid), en alimentant le meme `Stylesheet`/`Style` en sortie - c'est
/// la DSL a utiliser pour tout nouveau code. Cette fonction reste ici pour
/// le code existant qui en depend deja (voir `examples/run_demo.rs`), pas
/// pour en gagner de nouveau.
#[deprecated(note = "utilisez compiler::rsc a la place - cette DSL n'a ni selecteurs, ni pseudo-classes, ni flex/grid, voir la doc de `parse`")]
pub fn parse(input: &str) -> Result<Stylesheet, String> {
    let mut sheet = Stylesheet::new();
    let mut chars = input.chars().peekable();

    loop {
        skip_whitespace_and_comments(&mut chars);
        if chars.peek().is_none() {
            break;
        }

        match chars.next() {
            Some('.') => {}
            Some(other) => {
                return Err(format!(
                    "caractere inattendu '{other}', un selecteur '.nom' etait attendu"
                ));
            }
            None => break,
        }

        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                name.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err("nom de classe vide apres '.'".to_string());
        }

        skip_whitespace_and_comments(&mut chars);
        match chars.next() {
            Some('{') => {}
            other => return Err(format!("'{{' attendu apres '.{name}', trouve {other:?}")),
        }

        let mut style = Style::new();
        loop {
            skip_whitespace_and_comments(&mut chars);
            match chars.peek() {
                Some('}') => {
                    chars.next();
                    break;
                }
                Some(_) => {
                    let (key, value) = parse_declaration(&mut chars)?;
                    apply_declaration(&mut style, &key, &value)?;
                }
                None => return Err(format!("bloc '.{name}' jamais referme")),
            }
        }

        sheet.define(&name, style);
    }

    Ok(sheet)
}

fn skip_whitespace_and_comments(chars: &mut Peekable<Chars>) {
    loop {
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        if chars.peek() == Some(&'/') {
            let mut lookahead = chars.clone();
            lookahead.next();
            if lookahead.peek() == Some(&'/') {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
        }

        break;
    }
}

fn parse_declaration(chars: &mut Peekable<Chars>) -> Result<(String, String), String> {
    let mut key = String::new();
    while let Some(&c) = chars.peek() {
        if c == ':' || c == '}' {
            break;
        }
        key.push(c);
        chars.next();
    }
    match chars.next() {
        Some(':') => {}
        other => return Err(format!("':' attendu apres '{}', trouve {other:?}", key.trim())),
    }

    let mut value = String::new();
    while let Some(&c) = chars.peek() {
        if c == ';' || c == '}' {
            break;
        }
        value.push(c);
        chars.next();
    }
    if chars.peek() == Some(&';') {
        chars.next();
    }

    Ok((key.trim().to_string(), value.trim().to_string()))
}

fn apply_declaration(style: &mut Style, key: &str, value: &str) -> Result<(), String> {
    match key {
        "x" => style.x = Some(parse_number(value)?),
        "y" => style.y = Some(parse_number(value)?),
        "width" => style.width = Some(parse_number(value)?),
        "height" => style.height = Some(parse_number(value)?),
        "margin" => style.margin = Some(parse_number(value)?),
        "padding" => style.padding = Some(parse_number(value)?),
        "radius" => style.radius = Some(parse_number(value)?),
        "background" => style.background = Some(parse_color(value)?),
        "color" => style.color = Some(parse_color(value)?),
        "font-size" => style.font_size = Some(parse_number(value)?),
        "font-weight" => style.font_weight = Some(parse_number(value)?),
        other => return Err(format!("propriete de style inconnue: '{other}'")),
    }
    Ok(())
}

fn parse_number(value: &str) -> Result<f32, String> {
    value
        .parse::<f32>()
        .map_err(|_| format!("valeur numerique invalide: '{value}'"))
}

fn parse_color(value: &str) -> Result<Color, String> {
    let hex = value
        .strip_prefix('#')
        .ok_or_else(|| format!("couleur invalide (attendu '#rrggbb' ou '#rrggbbaa'): '{value}'"))?;

    let component = |slice: &str| -> Result<u8, String> {
        u8::from_str_radix(slice, 16).map_err(|_| format!("composant de couleur invalide dans '{value}'"))
    };

    match hex.len() {
        6 => Ok(Color::new(
            component(&hex[0..2])?,
            component(&hex[2..4])?,
            component(&hex[4..6])?,
            255,
        )),
        8 => Ok(Color::new(
            component(&hex[0..2])?,
            component(&hex[2..4])?,
            component(&hex[4..6])?,
            component(&hex[6..8])?,
        )),
        _ => Err(format!(
            "couleur invalide (attendu 6 ou 8 chiffres hexa): '{value}'"
        )),
    }
}

// Teste volontairement du code legacy (voir la doc de `parse` ci-dessus) -
// pas un avertissement a corriger ici, on continue a le tester tant qu'il
// existe.
