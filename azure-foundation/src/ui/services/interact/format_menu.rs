// Panneau de mise en forme au clic droit : texte selectionne dans une zone
// de texte riche, clic droit, un petit panneau apparait sous la souris
// (gras, italique, souligne, barre, code, puis les couleurs, puis la
// taille de la police). Un clic sur
// un element l'applique a la selection ; un clic ailleurs ou Echap ferme.
use crate::layout::managers::layout_manager::{contains, Rect};
use azure_engine::rendering::models::color::Color;

/// Le panneau ouvert (voir `EventState::format_menu`).
#[derive(Debug, Clone, PartialEq)]
pub struct FormatMenu {
    /// L'`#id` de la zone de texte riche visee.
    pub area: String,
    /// Coin haut-gauche du panneau.
    pub x: i32,
    pub y: i32,
}

/// Un element du panneau : marque (voir `rich::Mark::parse`), texte, couleur
/// du texte (pour les couleurs).
pub struct FormatItem {
    pub mark: &'static str,
    pub label: &'static str,
    pub color: Option<Color>,
    pub rect: Rect,
}

const STYLES: [(&str, &str); 5] = [("gras", "G"), ("italique", "I"), ("souligne", "S"), ("barre", "B"), ("code", "</>")];
const COLORS: [(&str, Color); 7] = [
    ("couleur-", Color::new(214, 211, 204, 255)),
    ("couleur-e06c75", Color::new(0xe0, 0x6c, 0x75, 255)),
    ("couleur-d19a66", Color::new(0xd1, 0x9a, 0x66, 255)),
    ("couleur-e5c07b", Color::new(0xe5, 0xc0, 0x7b, 255)),
    ("couleur-98c379", Color::new(0x98, 0xc3, 0x79, 255)),
    ("couleur-c678dd", Color::new(0xc6, 0x78, 0xdd, 255)),
    ("couleur-8a8a8a", Color::new(0x8a, 0x8a, 0x8a, 255)),
];

// Tailles de police en px ; « Auto » : celle de la zone.
const SIZES: [(&str, &str); 7] = [("taille-", "Auto"), ("taille-12", "12"), ("taille-14", "14"), ("taille-16", "16"), ("taille-20", "20"), ("taille-24", "24"), ("taille-32", "32")];

pub const PANEL_W: u32 = 7 * CELL + 2 * PAD;
pub const PANEL_H: u32 = 3 * CELL + 2 * PAD + 2 * GAP;
const CELL: u32 = 32;
const PAD: u32 = 6;
const GAP: u32 = 4;

impl FormatMenu {
    /// Ouvre le panneau pour `area` sous la souris, garde dans `bounds`.
    pub fn at(area: &str, x: i32, y: i32, bounds: (u32, u32, u32, u32)) -> FormatMenu {
        let max_x = (bounds.0 + bounds.2).saturating_sub(PANEL_W + 4) as i32;
        let max_y = (bounds.1 + bounds.3).saturating_sub(PANEL_H + 4) as i32;
        FormatMenu { area: area.to_string(), x: (x + 4).min(max_x).max(bounds.0 as i32), y: (y + 8).min(max_y).max(bounds.1 as i32) }
    }

    pub fn rect(&self) -> Rect {
        (self.x, self.y, PANEL_W, PANEL_H)
    }

    /// Les elements et leur place : les styles sur la 1re ligne, les
    /// couleurs sur la 2e, les tailles sur la 3e.
    pub fn items(&self) -> Vec<FormatItem> {
        let (x0, y0) = (self.x + PAD as i32, self.y + PAD as i32);
        let style_w = (7 * CELL) / STYLES.len() as u32;
        let mut out: Vec<FormatItem> = STYLES
            .iter()
            .enumerate()
            .map(|(i, (mark, label))| FormatItem { mark, label, color: None, rect: (x0 + (i as u32 * style_w) as i32, y0, style_w, CELL) })
            .collect();
        let y1 = y0 + (CELL + GAP) as i32;
        out.extend(COLORS.iter().enumerate().map(|(i, (mark, color))| FormatItem { mark, label: "A", color: Some(*color), rect: (x0 + (i as u32 * CELL) as i32, y1, CELL, CELL) }));
        let y2 = y1 + (CELL + GAP) as i32;
        out.extend(SIZES.iter().enumerate().map(|(i, (mark, label))| FormatItem { mark, label, color: None, rect: (x0 + (i as u32 * CELL) as i32, y2, CELL, CELL) }));
        out
    }

    /// La marque sous `(x, y)`.
    pub fn mark_at(&self, x: i32, y: i32) -> Option<&'static str> {
        self.items().into_iter().find(|it| contains(it.rect, x, y)).map(|it| it.mark)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_et_elements() {
        let m = FormatMenu::at("x", 990, 700, (0, 0, 1000, 720));
        assert!(m.x + PANEL_W as i32 <= 1000 && m.y + PANEL_H as i32 <= 720);
        let items = m.items();
        assert_eq!(items.len(), 19);
        let g = items[0].rect;
        assert_eq!(m.mark_at(g.0 + 2, g.1 + 2), Some("gras"));
        let rouge = items[6].rect;
        assert_eq!(m.mark_at(rouge.0 + 5, rouge.1 + 5), Some("couleur-e06c75"));
        assert_eq!(m.mark_at(m.x - 5, m.y), None);
        let t20 = items[16].rect;
        assert_eq!(m.mark_at(t20.0 + 5, t20.1 + 5), Some("taille-20"));
    }
}
