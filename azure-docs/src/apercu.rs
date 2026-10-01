// Le rendu en direct des exemples rsC. Sous un exemple, un bloc ```apercu
// donne une petite page rsH ; elle est construite par le vrai moteur avec :
//
//   1. ui/apercu.rsc  les aides communes (.ap-zone, .ap-boite...) ;
//   2. l'exemple rsC  tel qu'il est ecrit dans la page ;
//   3. les styles propres a l'apercu (apres `---`).
//
// Le resultat remplace le cadre vide `#apercu-<n>` du gabarit : ce sont de
// vrais widgets (le survol, le focus et le defilement y marchent).
use crate::contenu::{Apercu, Page};
use azure_foundation::compiler::components::{with_default_styles, Library};
use azure_foundation::compiler::rsc::mangers::parser::parse as parse_rsc;
use azure_foundation::compiler::rsc::services::lexer::tokenize as tokenize_rsc;
use azure_foundation::compiler::rsh::mangers::parser::parse as parse_rsh;
use azure_foundation::compiler::rsh::services::lexer::tokenize as tokenize_rsh;
use azure_foundation::compiler::services::codegen::StyleSource;
use azure_foundation::compiler::services::condition::Context;
use azure_foundation::compiler::services::interpreter::build_ui_with_context;
use azure_foundation::ui::models::ui_node::UiNode;
use std::path::Path;
use std::sync::Arc;

/// Les widgets de l'apercu `a` ; `base` : le contenu de ui/apercu.rsc.
pub fn rendre(a: &Apercu, base: &str) -> Result<Vec<UiNode>, String> {
    // Seulement les composants d'Azure : un apercu n'a pas de dossier.
    let library = Arc::new(Library::for_page(Path::new("/")));
    let rsc = with_default_styles(&format!("{base}\n{}\n{}", a.rsc, a.rsc_en_plus), Some(&library));
    let sheet = parse_rsc(tokenize_rsc(&rsc)).map_err(|e| format!("rsC invalide : {e:?}"))?;
    let ast = parse_rsh(tokenize_rsh(&a.rsh)).map_err(|e| format!("rsH invalide : {e}"))?;
    Ok(build_ui_with_context(&ast, &StyleSource::Rsc(&sheet), &Context::new().with_library(library)))
}

/// Met le rendu de chaque apercu de `page` dans son cadre `#apercu-<n>`.
pub fn inserer(nodes: &mut [UiNode], page: &Page, base: &str) {
    for a in page.apercus() {
        let rendu = match rendre(a, base) {
            Ok(rendu) => rendu,
            Err(e) => {
                eprintln!("azure-docs : aperçu {} de {} : {e}", a.n, page.chemin());
                continue;
            }
        };
        remplir(nodes, &format!("apercu-{}", a.n), rendu);
    }
}

// Donne `enfants` au conteneur `#id` ; `Some` rend `enfants` s'il est introuvable.
fn remplir(nodes: &mut [UiNode], id: &str, enfants: Vec<UiNode>) -> Option<Vec<UiNode>> {
    let mut enfants = Some(enfants);
    for node in nodes {
        if let UiNode::Container(c) = node {
            if c.decoration.anchor == id {
                c.children = enfants.take()?;
                return None;
            }
            enfants = remplir(&mut c.children, id, enfants.take()?);
            enfants.as_ref()?;
        }
    }
    enfants
}
