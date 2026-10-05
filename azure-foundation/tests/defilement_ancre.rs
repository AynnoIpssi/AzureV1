// Garder ce qu'on lit quand la page est reconstruite (« scroll anchoring ») :
// des lignes apparaissent au-dessus de ce qu'on lit (une liste qui grandit
// par le haut), la ligne lue reste a la meme hauteur a l'ecran ; tout en
// haut, on continue de voir le haut.
use azure_foundation::compiler::components::with_default_styles;
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::interpreter::build_ui;
use azure_foundation::ui::models::ui_node::UiNode;
use azure_foundation::ui::services::interact;

const PAGE: (u32, u32, u32, u32) = (0, 0, 600, 300);

// Une zone qui defile avec un bloc de `haut` lignes au-dessus de la liste,
// et les lignes `#r-<n>` de `premiere` a 1 (les plus recentes en haut).
fn page(haut: usize, premiere: usize) -> Vec<UiNode> {
    let mut rsh = String::from("<container.zone>\n");
    for i in 0..haut {
        rsh += &format!("<text.l>en cours {i}<!text>\n");
    }
    rsh += "<text.l>TERMINES<!text>\n";
    for n in (1..=premiere).rev() {
        rsh += &format!("<button.l#r-{n}>test {n}<!button>\n");
    }
    rsh += "<!container>";
    let rsc = ".zone { display: flex; flex-direction: column; height: 300px; overflow-y: auto; } .l { height: 20px; }";
    let sheet = parse_rsc(tokenize_rsc(&with_default_styles(rsc, None))).unwrap();
    build_ui(&parse_rsh(tokenize_rsh(&rsh)).unwrap(), &StyleSource::Rsc(&sheet))
}

fn y_de(nodes: &[UiNode], id: &str) -> i32 {
    let mut y = None;
    interact::walk(nodes, PAGE, &mut |n, b| {
        if n.decoration().anchor == id {
            y = Some(b.1);
        }
    });
    y.unwrap_or_else(|| panic!("#{id} absent"))
}

fn defiler(nodes: &mut [UiNode], px: u32) {
    let UiNode::Container(c) = &mut nodes[0] else { panic!() };
    c.scroll_offset = px;
    c.scroll_target = px;
}

#[test]
fn la_ligne_lue_reste_en_place() {
    let mut vieux = page(3, 40);
    defiler(&mut vieux, 200);
    let lue = "r-31";
    let avant = y_de(&vieux, lue);
    // 5 nouvelles lignes en haut de la liste, un bloc « en cours » plus petit.
    let mut neuf = page(1, 45);
    interact::carry_scroll_anchored(&vieux, &mut neuf, PAGE);
    assert_eq!(y_de(&neuf, lue), avant);
    // Sans ancrage, la ligne aurait bouge (de 3 lignes de 20 px).
    let mut brut = page(1, 45);
    interact::carry_scroll(&vieux, &mut brut);
    assert_ne!(y_de(&brut, lue), avant);
}

#[test]
fn tout_en_haut_on_voit_les_nouvelles() {
    let vieux = page(2, 10);
    let mut neuf = page(2, 15);
    interact::carry_scroll_anchored(&vieux, &mut neuf, PAGE);
    let UiNode::Container(c) = &neuf[0] else { panic!() };
    assert_eq!((c.scroll_offset, c.scroll_target), (0, 0));
}
