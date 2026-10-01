// Choix du fichier de police pour une `font-family` rsC. Une seule police
// est livree avec le moteur (Sora) ; pour `monospace` (le code) et `serif`
// (titrage, texte editorial), on prend la premiere police de ce genre
// presente sur le systeme, sinon Sora.
use crate::ui::services::draw_ui::FONT_PATH;
use std::sync::OnceLock;

const MONOSPACE_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/adwaita-mono-fonts/AdwaitaMono-Regular.ttf",
    "/usr/share/fonts/liberation-mono-fonts/LiberationMono-Regular.ttf",
    "/usr/share/fonts/dejavu-sans-mono-fonts/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
];

const SERIF_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/google-noto-vf/NotoSerif[wght].ttf",
    "/usr/share/fonts/noto/NotoSerif-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf",
    "/usr/share/fonts/liberation-serif-fonts/LiberationSerif-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
    "/usr/share/fonts/dejavu-serif-fonts/DejaVuSerif.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    "/usr/share/fonts/TTF/DejaVuSerif.ttf",
];

fn first_present(candidates: &[&'static str]) -> &'static str {
    candidates.iter().copied().find(|p| std::path::Path::new(p).exists()).unwrap_or(FONT_PATH)
}

fn monospace() -> &'static str {
    static FOUND: OnceLock<&'static str> = OnceLock::new();
    FOUND.get_or_init(|| first_present(MONOSPACE_CANDIDATES))
}

fn serif() -> &'static str {
    static FOUND: OnceLock<&'static str> = OnceLock::new();
    FOUND.get_or_init(|| first_present(SERIF_CANDIDATES))
}

/// Fichier de police pour la famille `family` (deja en minuscules).
pub fn font_for(family: Option<&str>) -> &'static str {
    match family {
        Some(f) if f.contains("mono") || f == "code" || f.contains("courier") || f.contains("consolas") => monospace(),
        // `sans-serif` reste Sora.
        Some(f) if (f.contains("serif") && !f.contains("sans")) || f.contains("georgia") || f.contains("times") => serif(),
        _ => FONT_PATH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families() {
        assert_eq!(font_for(None), FONT_PATH);
        assert_eq!(font_for(Some("sans-serif")), FONT_PATH);
        assert_eq!(font_for(Some("sora")), FONT_PATH);
        assert_eq!(font_for(Some("serif")), serif());
        assert_eq!(font_for(Some("georgia")), serif());
        assert_eq!(font_for(Some("monospace")), monospace());
        assert!(std::path::Path::new(serif()).exists());
    }
}
