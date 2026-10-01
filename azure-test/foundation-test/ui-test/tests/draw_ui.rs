#[cfg(test)]
mod tests {
    
    
    
    use azure_foundation::ui::models::ui_node::UiNode;
    
    
    
    
    
    use azure_engine::rendering::models::canvas::Canvas;
    use azure_engine::rendering::models::color::Color;
    
    use azure_foundation::ui::services::draw_ui::*;
    use azure_foundation::layout::models::layout_props::LayoutProps;
    use azure_foundation::ui::models::image::Image;

    // Chemin relatif au repertoire de travail de `cargo test` (la racine
    // du crate, `azure-foundation/`) - meme convention que `FONT_PATH`.
    const RGBA_FIXTURE: &str = "tests/fixtures/rgba_3x2.png";

    fn full_box() -> LayoutProps {
        LayoutProps::new(0.0, 0.0, 100.0, 100.0, 0.0, 0.0)
    }

    #[test]
    fn image_without_src_draws_the_placeholder_color() {
        let mut canvas = Canvas::new(4, 4);
        let image = Image::new(full_box(), String::new());
        draw_image(&image, (0, 0, 4, 4), &mut canvas);
        assert_eq!(&canvas.buffer[0..4], &[IMAGE_PLACEHOLDER_COLOR.b, IMAGE_PLACEHOLDER_COLOR.g, IMAGE_PLACEHOLDER_COLOR.r, IMAGE_PLACEHOLDER_COLOR.a]);
    }

    #[test]
    fn image_with_an_unreadable_src_falls_back_to_the_placeholder_color() {
        let mut canvas = Canvas::new(4, 4);
        let image = Image::new(full_box(), "does/not/exist.png".to_string());
        draw_image(&image, (0, 0, 4, 4), &mut canvas);
        assert_eq!(&canvas.buffer[0..4], &[IMAGE_PLACEHOLDER_COLOR.b, IMAGE_PLACEHOLDER_COLOR.g, IMAGE_PLACEHOLDER_COLOR.r, IMAGE_PLACEHOLDER_COLOR.a]);
    }

    #[test]
    fn image_with_a_valid_src_paints_the_real_decoded_pixels() {
        let mut canvas = Canvas::new(4, 4);
        let image = Image::new(full_box(), RGBA_FIXTURE.to_string());
        draw_image(&image, (0, 0, 4, 4), &mut canvas);

        // Premier pixel de la fixture (voir `codec::png` chez azure-engine,
        // ou son script de generation) : rouge opaque -> BGRA (0,0,255,255).
        assert_eq!(&canvas.buffer[0..4], &[0, 0, 255, 255]);
        // Ce n'est en tout cas plus le placeholder.
        assert_ne!(&canvas.buffer[0..4], &[IMAGE_PLACEHOLDER_COLOR.b, IMAGE_PLACEHOLDER_COLOR.g, IMAGE_PLACEHOLDER_COLOR.r, IMAGE_PLACEHOLDER_COLOR.a]);
    }

    #[test]
    fn image_is_clipped_to_its_own_box_not_drawn_beyond_it() {
        // Boite de 2x2 dans un canvas de 4x4 : l'image (3x2) ne doit rien
        // peindre au-dela de x=2 meme si elle est plus large que sa boite.
        let mut canvas = Canvas::new(4, 4);
        let image = Image::new(full_box(), RGBA_FIXTURE.to_string());
        draw_image(&image, (0, 0, 2, 2), &mut canvas);

        let (x, y) = (2, 0);
        let idx = ((y * 4 + x) * 4) as usize;
        assert_eq!(&canvas.buffer[idx..idx + 4], &[0, 0, 0, 0], "rien ne doit deborder de la boite 2x2");
    }

    #[test]
    fn container_clips_a_child_that_overflows_its_content_box() {
        // Conteneur de 2x4 (padding nul) contenant un enfant en mode Block
        // volontairement plus large que lui (4x4, comme un bug de style) :
        // rien ne doit peindre au-dela de x=2 - c'est le meme genre de
        // garde-fou que celui deja verifie plus haut pour une `Image`, mais
        // applique au niveau du `Container` lui-meme (correctif d'audit).
        use azure_foundation::ui::models::container::Container;
        use azure_foundation::ui::models::image::Image;

        let mut canvas = Canvas::new(4, 4);
        let overflowing_child = UiNode::Image(Image::new(LayoutProps::new(0.0, 0.0, 200.0, 100.0, 0.0, 0.0), RGBA_FIXTURE.to_string()));
        let container = Container::new(full_box(), Color::new(0, 0, 0, 0), vec![overflowing_child]);

        draw_container(&container, (0, 0, 2, 4), &mut canvas, -1, -1, false);

        let (x, y) = (2, 0);
        let idx = ((y * 4 + x) * 4) as usize;
        assert_eq!(&canvas.buffer[idx..idx + 4], &[0, 0, 0, 0], "l'enfant ne doit rien peindre au-dela de la boite du conteneur");
    }
}
