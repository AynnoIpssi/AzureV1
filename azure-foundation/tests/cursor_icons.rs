// Verifie les icones de curseur generees (voir `cursor::services::cursor_icon`)
// et ecrit un apercu agrandi x8 sur fond clair puis sombre :
// target/tmp/cursor_preview.ppm (converti en PNG pour le regarder).
use azure_foundation::cursor::models::cursor_kind::CursorKind;
use azure_foundation::cursor::services::cursor_icon::{generate, CURSOR_SIZE};
use azure_foundation::ui::services::interact::HoverKind;

const ZOOM: u32 = 8;

#[test]
fn icons_are_well_formed() {
    for kind in CursorKind::ALL {
        let img = generate(kind);
        assert_eq!(img.pixels.len(), (CURSOR_SIZE * CURSOR_SIZE * 4) as usize, "{kind:?}");
        // Premultiplie : aucune composante ne depasse l'alpha.
        for px in img.pixels.chunks(4) {
            assert!(px[0] <= px[3] && px[1] <= px[3] && px[2] <= px[3], "{kind:?}: pixel non premultiplie {px:?}");
        }
        // Le point chaud tombe sur (ou tout contre) la forme dessinee.
        let (hx, hy) = (img.hotspot_x, img.hotspot_y);
        let alpha_at = |x: u32, y: u32| img.pixels[((y * CURSOR_SIZE + x) * 4 + 3) as usize];
        let near = (hx.saturating_sub(1)..=hx + 1).any(|x| (hy.saturating_sub(1)..=hy + 1).any(|y| alpha_at(x, y) > 200));
        assert!(near, "{kind:?}: point chaud ({hx},{hy}) hors de la forme");
    }
}

#[test]
fn text_cursor_only_for_editable_text() {
    assert_eq!(CursorKind::from_hover(HoverKind::TextArea), CursorKind::Text);
    assert_eq!(CursorKind::from_hover(HoverKind::Button), CursorKind::Pointer);
    // Un Label (texte affiche) ne produit aucun survol : fleche.
    assert_eq!(CursorKind::from_hover(HoverKind::None), CursorKind::Default);
}

#[test]
fn write_preview() {
    let icons: Vec<_> = CursorKind::ALL.iter().map(|k| generate(*k)).collect();
    let cell = CURSOR_SIZE * ZOOM;
    let (w, h) = (cell * icons.len() as u32, cell * 2);
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for y in 0..h {
        let bg: [f32; 3] = if y < cell { [0.93, 0.93, 0.95] } else { [0.12, 0.12, 0.18] };
        for x in 0..w {
            let img = &icons[(x / cell) as usize];
            let (sx, sy) = ((x % cell) / ZOOM, (y % cell) / ZOOM);
            let i = ((sy * CURSOR_SIZE + sx) * 4) as usize;
            let a = img.pixels[i + 3] as f32 / 255.0;
            let src = [img.pixels[i + 2], img.pixels[i + 1], img.pixels[i]];
            let hotspot = sx == img.hotspot_x && sy == img.hotspot_y && (x % ZOOM == ZOOM / 2) && (y % ZOOM == ZOOM / 2);
            for c in 0..3 {
                let v = if hotspot { [1.0, 0.0, 0.0][c] } else { src[c] as f32 / 255.0 + bg[c] * (1.0 - a) };
                out.push((v.clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
    }
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("cursor_preview.ppm");
    std::fs::write(&path, out).unwrap();
    println!("apercu: {}", path.display());
}
