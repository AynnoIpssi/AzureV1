// Texte riche d'une zone de saisie (`<richtext>`) : un style par caractere
// (gras, italique, souligne, barre, code, couleur, lien), garde a cote du
// texte de la `TextArea` (voir `TextArea::rich`).
//
// Format d'echange (valeur initiale, `ctx.value(id)`) : le texte seul s'il
// n'a aucun style, sinon des segments separes par U+001E, chacun
//
//   <marques> U+001F <couleur> U+001F <lien> U+001F <texte>
//
// marques = lettres g (gras) i (italique) s (souligne) b (barre) c (code),
// puis t<taille> pour une taille de police en px (`gt18`) ;
// couleur = "#rrggbb" ou vide ; lien = texte libre (id de page...) ou vide.
use azure_engine::rendering::models::color::Color;

pub const FIELD: char = '\u{1f}';
pub const SPAN: char = '\u{1e}';

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RichStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub code: bool,
    pub color: Option<Color>,
    pub link: String,
    /// Taille de police en px ; `None` : celle de la zone.
    pub size: Option<f32>,
}

/// Un morceau de texte d'un seul style.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RichSpan {
    pub text: String,
    pub style: RichStyle,
}

/// Ce qu'on applique a une selection (voir `TextArea::toggle_mark`).
#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    Bold,
    Italic,
    Underline,
    Strike,
    Code,
    /// `None` : couleur du texte par defaut.
    Color(Option<Color>),
    /// Vide : retire le lien.
    Link(String),
    /// `None` : taille de la zone.
    Size(Option<f32>),
}

impl Mark {
    /// `b`, `i`, `u`, `s`, `e` (raccourcis Ctrl+...), ou le nom d'un bouton
    /// de barre d'outils : `gras`, `couleur-#e06c75`, `lien-12`...
    pub fn parse(name: &str) -> Option<Mark> {
        Some(match name {
            "b" | "gras" | "bold" => Mark::Bold,
            "i" | "italique" | "italic" => Mark::Italic,
            "u" | "souligne" | "underline" => Mark::Underline,
            "s" | "barre" | "strike" => Mark::Strike,
            "e" | "code" => Mark::Code,
            _ => {
                if let Some(c) = name.strip_prefix("couleur-").or_else(|| name.strip_prefix("color-")) {
                    Mark::Color(parse_color(c))
                } else if let Some(t) = name.strip_prefix("taille-").or_else(|| name.strip_prefix("size-")) {
                    Mark::Size(t.parse::<f32>().ok().filter(|t| *t > 0.0))
                } else if let Some(l) = name.strip_prefix("lien-").or_else(|| name.strip_prefix("link-")) {
                    Mark::Link(l.to_string())
                } else {
                    return None;
                }
            }
        })
    }

    pub fn on(&self, style: &RichStyle) -> bool {
        match self {
            Mark::Bold => style.bold,
            Mark::Italic => style.italic,
            Mark::Underline => style.underline,
            Mark::Strike => style.strike,
            Mark::Code => style.code,
            Mark::Color(c) => style.color == *c,
            Mark::Link(l) => style.link == *l,
            Mark::Size(t) => style.size == *t,
        }
    }

    /// Met la marque (`true`) ou l'enleve.
    pub fn apply(&self, style: &mut RichStyle, on: bool) {
        match self {
            Mark::Bold => style.bold = on,
            Mark::Italic => style.italic = on,
            Mark::Underline => style.underline = on,
            Mark::Strike => style.strike = on,
            Mark::Code => style.code = on,
            Mark::Color(c) => style.color = if on { *c } else { None },
            Mark::Link(l) => style.link = if on { l.clone() } else { String::new() },
            Mark::Size(t) => style.size = if on { *t } else { None },
        }
    }

    /// Une couleur, un lien, une taille se posent toujours ; les autres
    /// basculent.
    pub fn toggles(&self) -> bool {
        !matches!(self, Mark::Color(_) | Mark::Link(_) | Mark::Size(_))
    }
}

/// `#rrggbb` (ou `rrggbb`) ; vide ou invalide : `None`.
pub fn parse_color(s: &str) -> Option<Color> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some(Color::new(p(0)?, p(2)?, p(4)?, 255))
}

pub fn color_hex(c: &Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// Lit le format d'echange.
pub fn parse(s: &str) -> Vec<RichSpan> {
    if !s.contains(FIELD) {
        return if s.is_empty() { Vec::new() } else { vec![RichSpan { text: s.to_string(), style: RichStyle::default() }] };
    }
    s.split(SPAN)
        .filter_map(|span| {
            let mut f = span.splitn(4, FIELD);
            let (marks, color, link, text) = (f.next()?, f.next()?, f.next()?, f.next()?);
            let (letters, size) = marks.split_once('t').unwrap_or((marks, ""));
            let has = |c: char| letters.contains(c);
            let size = size.parse::<f32>().ok().filter(|t| *t > 0.0);
            let style = RichStyle { bold: has('g'), italic: has('i'), underline: has('s'), strike: has('b'), code: has('c'), color: parse_color(color), link: link.to_string(), size };
            Some(RichSpan { text: text.to_string(), style })
        })
        .collect()
}

/// Ecrit le format d'echange.
pub fn serialize(spans: &[RichSpan]) -> String {
    let spans = merge(spans);
    if spans.len() <= 1 && spans.iter().all(|s| s.style == RichStyle::default()) {
        return spans.first().map(|s| s.text.clone()).unwrap_or_default();
    }
    spans
        .iter()
        .map(|span| {
            let s = &span.style;
            let mut marks: String = [(s.bold, 'g'), (s.italic, 'i'), (s.underline, 's'), (s.strike, 'b'), (s.code, 'c')].iter().filter(|m| m.0).map(|m| m.1).collect();
            if let Some(t) = s.size {
                marks.push_str(&format!("t{t}"));
            }
            let color = s.color.as_ref().map(color_hex).unwrap_or_default();
            [marks.as_str(), &color, &s.link, &span.text].join(&FIELD.to_string())
        })
        .collect::<Vec<_>>()
        .join(&SPAN.to_string())
}

/// Colle les voisins de meme style, retire les vides.
pub fn merge(spans: &[RichSpan]) -> Vec<RichSpan> {
    let mut out: Vec<RichSpan> = Vec::new();
    for s in spans.iter().filter(|s| !s.text.is_empty()) {
        match out.last_mut() {
            Some(last) if last.style == s.style => last.text.push_str(&s.text),
            _ => out.push(s.clone()),
        }
    }
    out
}

/// Texte + un style par caractere.
pub fn flatten(spans: &[RichSpan]) -> (String, Vec<RichStyle>) {
    let mut text = String::new();
    let mut styles = Vec::new();
    for s in spans {
        text.push_str(&s.text);
        styles.extend(std::iter::repeat_n(s.style.clone(), s.text.chars().count()));
    }
    (text, styles)
}

/// L'inverse de `flatten`.
pub fn spans_of(text: &str, styles: &[RichStyle]) -> Vec<RichSpan> {
    let mut out: Vec<RichSpan> = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let style = styles.get(i).cloned().unwrap_or_default();
        match out.last_mut() {
            Some(last) if last.style == style => last.text.push(c),
            _ => out.push(RichSpan { text: c.to_string(), style }),
        }
    }
    out
}

/// L'etat riche d'une `TextArea`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RichText {
    /// Un style par caractere du texte.
    pub styles: Vec<RichStyle>,
    /// Style du prochain caractere tape (Ctrl+B sans selection...), oublie
    /// des que le curseur bouge.
    pub pending: Option<RichStyle>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour() {
        let spans = vec![
            RichSpan { text: "Un ".into(), style: RichStyle::default() },
            RichSpan { text: "mot".into(), style: RichStyle { bold: true, color: parse_color("#e06c75"), ..Default::default() } },
            RichSpan { text: " lié".into(), style: RichStyle { underline: true, link: "4".into(), ..Default::default() } },
            RichSpan { text: " grand".into(), style: RichStyle { bold: true, size: Some(20.0), ..Default::default() } },
        ];
        let s = serialize(&spans);
        assert_eq!(parse(&s), spans);
        let (text, styles) = flatten(&spans);
        assert_eq!(text, "Un mot lié grand");
        assert_eq!(spans_of(&text, &styles), spans);
        assert_eq!(serialize(&parse("simple")), "simple");
    }

    #[test]
    fn marques() {
        assert_eq!(Mark::parse("couleur-#112233"), Some(Mark::Color(Some(Color::new(0x11, 0x22, 0x33, 255)))));
        assert_eq!(Mark::parse("couleur-"), Some(Mark::Color(None)));
        assert_eq!(Mark::parse("b"), Some(Mark::Bold));
        assert_eq!(Mark::parse("x"), None);
        assert_eq!(Mark::parse("taille-18"), Some(Mark::Size(Some(18.0))));
        assert_eq!(Mark::parse("taille-"), Some(Mark::Size(None)));
    }
}
