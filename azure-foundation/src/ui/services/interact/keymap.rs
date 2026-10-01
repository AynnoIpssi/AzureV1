// La carte du clavier de l'utilisateur, telle que le compositeur l'envoie
// (`wl_keyboard::keymap`, texte `xkb_keymap { ... }`) : pour chaque touche
// physique, ce qu'elle produit seule, avec Maj, avec AltGr et avec
// Maj+AltGr, pour chaque disposition active (groupe). C'est elle qui fait
// qu'un clavier AZERTY tape `&é"'(` sur la rangee du haut, `€` avec
// AltGr+E, et `ê` avec `^` puis `e`.
//
// On n'en lit que ce qui sert a taper du texte :
// - `xkb_keycodes` : nom de touche (`<AE01>`) -> code ;
// - `xkb_symbols`  : les symboles de chaque touche, groupe par groupe.
// Les types de touches (`xkb_types`) ne sont pas interpretes : les niveaux
// suivent la regle commune Maj = 2, AltGr = 3, Maj+AltGr = 4, et Verr. Maj
// agit comme Maj sur les lettres (voir `Keymap::keysym`).
use super::keysyms::{CHARS, NAMES};
use std::collections::HashMap;

/// Une carte lue (voir `Keymap::parse`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Keymap {
    /// Code evdev -> symboles, groupe par groupe, niveau par niveau.
    keys: HashMap<u32, Key>,
    /// Touches qui donnent le niveau 3 (AltGr), en codes evdev.
    level3: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Key {
    groups: Vec<Vec<u32>>,
    /// `type= "ONE_LEVEL"` : un seul niveau, Maj n'y change rien.
    one_level: bool,
}

/// L'etat des modificateurs au moment d'une frappe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    /// AltGr (niveau 3).
    pub altgr: bool,
    pub caps_lock: bool,
    pub num_lock: bool,
    /// Disposition active (0 : la premiere).
    pub group: u32,
}

const ISO_LEVEL3_SHIFT: u32 = 0xfe03;
/// Les codes XKB sont les codes evdev decales de 8.
const EVDEV_OFFSET: u32 = 8;

impl Keymap {
    /// Lit le texte d'une carte XKB (format `xkb_v1`).
    pub fn parse(text: &str) -> Result<Keymap, String> {
        let codes = keycodes(section(text, "xkb_keycodes").ok_or("carte sans xkb_keycodes")?);
        let symbols = section(text, "xkb_symbols").ok_or("carte sans xkb_symbols")?;
        let mut keymap = Keymap::default();
        let mut rest = symbols;
        while let Some(at) = find_word(rest, "key") {
            rest = &rest[at + 3..];
            let Some(open) = rest.find('<') else { break };
            let Some(close) = rest[open..].find('>') else { break };
            let name = &rest[open + 1..open + close];
            let Some(body_start) = rest.find('{') else { break };
            let Some(body_len) = matching_brace(&rest[body_start..]) else { break };
            let body = &rest[body_start + 1..body_start + body_len];
            rest = &rest[body_start + body_len..];
            let Some(&code) = codes.get(name) else { continue };
            let key = parse_key(body);
            let evdev = code.saturating_sub(EVDEV_OFFSET);
            if key.groups.first().and_then(|g| g.first()) == Some(&ISO_LEVEL3_SHIFT) {
                keymap.level3.push(evdev);
            }
            keymap.keys.insert(evdev, key);
        }
        if keymap.keys.is_empty() {
            return Err("carte sans touches".to_string());
        }
        Ok(keymap)
    }

    /// La carte pour `KeyboardLayout::Xkb`. Elle vit jusqu'a la fin du
    /// programme : le compositeur n'en envoie qu'a l'ouverture d'une
    /// fenetre et quand l'utilisateur change de disposition.
    pub fn leak(self) -> &'static Keymap {
        Box::leak(Box::new(self))
    }

    /// La touche `evdev` donne-t-elle AltGr ?
    pub fn is_level3(&self, evdev: u32) -> bool {
        self.level3.contains(&evdev)
    }

    /// Le symbole (keysym) produit par `evdev` avec ces modificateurs, `None`
    /// si la touche n'en produit pas.
    pub fn keysym(&self, evdev: u32, mods: Modifiers) -> Option<u32> {
        let key = self.keys.get(&evdev)?;
        if key.groups.is_empty() {
            return None;
        }
        // Un groupe absent sur cette touche : on revient au debut, comme XKB.
        let syms = &key.groups[mods.group as usize % key.groups.len()];
        let first = *syms.first()?;
        if key.one_level {
            return Some(first).filter(|&s| s != 0);
        }
        let level = if is_keypad(first) && syms.len() >= 2 {
            // Pave numerique : Verr. Num donne les chiffres.
            usize::from(mods.num_lock != mods.shift)
        } else {
            // Verr. Maj agit comme Maj sur les lettres seulement.
            let letter = matches!((keysym_char(first), syms.get(1).copied().and_then(keysym_char)), (Some(l), Some(u)) if l.is_lowercase() && l.to_uppercase().eq(std::iter::once(u)));
            let shift = mods.shift != (mods.caps_lock && letter);
            usize::from(shift) + if mods.altgr { 2 } else { 0 }
        };
        let sym = match syms.get(level) {
            Some(&s) => s,
            // Touche a un seul symbole (l'espace) : le meme avec Maj.
            None if level == 1 => first,
            None => return None,
        };
        Some(sym).filter(|&s| s != 0)
    }

    /// Le symbole de base de la touche dans la disposition active : c'est
    /// par lui que Ctrl+C, Ctrl+V... sont reconnus.
    pub fn base_keysym(&self, evdev: u32, group: u32) -> Option<u32> {
        self.keysym(evdev, Modifiers { group, ..Modifiers::default() })
    }
}

fn is_keypad(sym: u32) -> bool {
    (0xff80..=0xffbd).contains(&sym)
}

/// Le caractere d'un symbole, `None` pour une touche de commande (Retour,
/// fleches...) ou une touche morte.
pub fn keysym_char(sym: u32) -> Option<char> {
    match sym {
        0x20..=0x7e | 0xa0..=0xff => char::from_u32(sym),
        0x0100_0100..=0x0110_ffff => char::from_u32(sym - 0x0100_0000),
        // Pave numerique.
        0xffb0..=0xffb9 => char::from_u32(sym - 0xffb0 + '0' as u32),
        0xffaa => Some('*'),
        0xffab => Some('+'),
        0xffac => Some(','),
        0xffad => Some('-'),
        0xffae => Some('.'),
        0xffaf => Some('/'),
        0xff80 => Some(' '),
        _ => CHARS.binary_search_by_key(&sym, |&(v, _)| v).ok().and_then(|i| char::from_u32(CHARS[i].1)),
    }
}

/// L'accent d'une touche morte (`dead_circumflex` -> `^`), `None` si `sym`
/// n'en est pas une ou si on ne sait pas la composer.
pub fn dead_accent(sym: u32) -> Option<char> {
    Some(match sym {
        0xfe50 => '`',
        0xfe51 => '´',
        0xfe52 => '^',
        0xfe53 => '~',
        0xfe54 => '¯',
        0xfe57 => '¨',
        0xfe58 => '°',
        0xfe5b => '¸',
        _ => return None,
    })
}

/// `accent` (voir `dead_accent`) puis `c` : le caractere compose (`^` puis
/// `e` -> `ê`), l'accent seul apres une espace, `None` si rien ne se compose.
pub fn compose(accent: char, c: char) -> Option<char> {
    if c == ' ' {
        return Some(accent);
    }
    let (from, to) = match accent {
        '`' => ("aeiouAEIOU", "àèìòùÀÈÌÒÙ"),
        '´' => ("aeiouyAEIOUYc", "áéíóúýÁÉÍÓÚÝć"),
        '^' => ("aeiouAEIOU", "âêîôûÂÊÎÔÛ"),
        '~' => ("aonAON", "ãõñÃÕÑ"),
        '¨' => ("aeiouyAEIOUY", "äëïöüÿÄËÏÖÜŸ"),
        '°' => ("aA", "åÅ"),
        '¸' => ("cC", "çÇ"),
        '¯' => ("aeiouAEIOU", "āēīōūĀĒĪŌŪ"),
        _ => return None,
    };
    from.chars().position(|x| x == c).and_then(|i| to.chars().nth(i))
}

// Le contenu `{ ... }` de la section `name` (`xkb_symbols "..." { ... };`).
fn section<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let at = find_word(text, name)?;
    let open = at + text[at..].find('{')?;
    let len = matching_brace(&text[open..])?;
    Some(&text[open + 1..open + len])
}

// Position de `word` entier (pas au milieu d'un autre mot) dans `text`.
fn find_word(text: &str, word: &str) -> Option<usize> {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut from = 0;
    while let Some(i) = text[from..].find(word) {
        let at = from + i;
        let before = text[..at].chars().next_back().is_none_or(|c| !ident(c));
        let after = text[at + word.len()..].chars().next().is_none_or(|c| !ident(c));
        if before && after {
            return Some(at);
        }
        from = at + word.len();
    }
    None
}

// `text` commence par `{` : position de la `}` qui la ferme.
fn matching_brace(text: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in text.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

// `<AE01> = 10;` et `alias <ALGR> = <RALT>;`.
fn keycodes(section: &str) -> HashMap<String, u32> {
    let mut codes = HashMap::new();
    let mut aliases = Vec::new();
    for statement in section.split(';') {
        let s = statement.trim();
        let Some((left, right)) = s.split_once('=') else { continue };
        let name = |t: &str| t.trim().trim_start_matches('<').trim_end_matches('>').to_string();
        if let Some(alias) = left.trim().strip_prefix("alias") {
            aliases.push((name(alias), name(right)));
        } else if left.trim().starts_with('<')
            && let Ok(code) = right.trim().parse()
        {
            codes.insert(name(left), code);
        }
    }
    for (alias, target) in aliases {
        if let Some(&code) = codes.get(&target) {
            codes.entry(alias).or_insert(code);
        }
    }
    codes
}

// Le corps d'une touche : `[ a, A ], [ q, Q ]`, `symbols[1]= [ ... ]`,
// `type= "ONE_LEVEL"` ; les `actions[...]= [ ... ]` sont ignorees.
fn parse_key(body: &str) -> Key {
    let mut key = Key::default();
    let mut word = String::new();
    let mut index: Option<usize> = None;
    let mut rest = body;
    while let Some(c) = rest.chars().next() {
        if c == '[' {
            let Some(end) = rest.find(']') else { break };
            let inner = rest[1..end].trim();
            rest = &rest[end + 1..];
            // `[1]`, `[Group2]` : l'indice du groupe qui suit.
            if let Ok(n) = inner.trim_start_matches("Group").parse::<usize>()
                && rest.trim_start().starts_with('=')
            {
                index = Some(n.saturating_sub(1));
                continue;
            }
            if word != "actions" {
                let syms: Vec<u32> = inner.split(',').map(|t| keysym_value(t.trim())).collect();
                let at = index.take().unwrap_or(key.groups.len());
                if key.groups.len() <= at {
                    key.groups.resize(at + 1, Vec::new());
                }
                key.groups[at] = syms;
            }
            index = None;
            word.clear();
            continue;
        }
        if c.is_ascii_alphanumeric() || c == '_' {
            word.push(c);
        } else if c == ',' || c == '{' || c == ';' {
            word.clear();
        } else if c == '"' {
            // `type= "ONE_LEVEL"`
            let Some(end) = rest[1..].find('"') else { break };
            if word == "type" && &rest[1..1 + end] == "ONE_LEVEL" {
                key.one_level = true;
            }
            rest = &rest[end + 2..];
            continue;
        } else if !(c == '=' || c.is_whitespace()) {
            word.clear();
        }
        rest = &rest[c.len_utf8()..];
    }
    key
}

// `0xe9`, `eacute`, `U20AC`, `1` (la touche 1), `NoSymbol`.
fn keysym_value(token: &str) -> u32 {
    if let Some(hex) = token.strip_prefix("0x") {
        return u32::from_str_radix(hex, 16).unwrap_or(0);
    }
    if token.len() == 1 && token.as_bytes()[0].is_ascii_digit() {
        return token.as_bytes()[0] as u32;
    }
    if let Some(hex) = token.strip_prefix('U')
        && hex.len() >= 4
        && let Ok(u) = u32::from_str_radix(hex, 16)
    {
        return 0x0100_0000 + u;
    }
    NAMES.binary_search_by_key(&token, |&(n, _)| n).map(|i| NAMES[i].1).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeurs_et_noms() {
        assert_eq!(keysym_value("0xe9"), 0xe9);
        assert_eq!(keysym_value("eacute"), 0xe9);
        assert_eq!(keysym_value("1"), 0x31);
        assert_eq!(keysym_value("U20AC"), 0x010020ac);
        assert_eq!(keysym_value("dead_circumflex"), 0xfe52);
        assert_eq!(keysym_char(0x20ac), Some('€'));
        assert_eq!(keysym_char(0x13bd), Some('œ'));
        assert_eq!(keysym_char(0xfe52), None, "une touche morte n'est pas un caractere");
    }

    #[test]
    fn corps_de_touche() {
        let k = parse_key(" symbols[1]= [ 0x26, 0x31 ], symbols[2]= [ 0x61, 0x41 ] ");
        assert_eq!(k.groups, vec![vec![0x26, 0x31], vec![0x61, 0x41]]);
        let k = parse_key("\t[ a, A ], [ q, Q ] ");
        assert_eq!(k.groups, vec![vec![0x61, 0x41], vec![0x71, 0x51]]);
        let k = parse_key(" type= \"ONE_LEVEL\", symbols[Group1]= [ ISO_Level3_Shift ], actions[Group1]= [ SetMods(modifiers=LevelThree) ] ");
        assert_eq!(k.groups, vec![vec![ISO_LEVEL3_SHIFT]]);
        assert!(k.one_level);
    }
}
