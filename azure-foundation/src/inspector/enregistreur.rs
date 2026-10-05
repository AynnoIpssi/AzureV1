// L'enregistreur de l'inspecteur : ce que la personne fait dans la vraie
// fenetre devient un essai (voir `crate::essai`) - un clic sur `#id`, la
// valeur finale d'un champ (pas chaque touche), Entree/Echap/Tab/fleches,
// la molette, et les textes a verifier choisis dans l'inspecteur. A
// l'arret, le test Rust est ecrit, pret a coller dans `tests/` de l'app.
use crate::ui::models::ui_node::UiNode;
use crate::ui::services::interact;
use azure_core::rules::window_event::WindowEvent;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Etape {
    Clic(String),
    /// Un clic sur un element sans `#id` : pas rejouable, note en commentaire.
    ClicSansId(i32, i32),
    Remplir(String, String),
    Touche(&'static str),
    Molette(String, f64),
    Verifier(String),
}

#[derive(Default)]
pub struct Enregistrement {
    pub etapes: Vec<Etape>,
    /// Les champs a la derniere etape : ce qui a change depuis est tape.
    champs: BTreeMap<String, String>,
}

/// La valeur affichee de chaque champ a `#id` (texte brut, option choisie,
/// case, curseur) : ce que `AppPilotee::remplir` sait redonner.
pub(crate) fn champs(nodes: &[UiNode]) -> BTreeMap<String, String> {
    fn go(nodes: &[UiNode], out: &mut BTreeMap<String, String>) {
        for n in nodes {
            match n {
                UiNode::TextArea(t) if !t.id.is_empty() => {
                    out.insert(t.id.clone(), t.text.clone());
                }
                UiNode::Control(c) if !c.id.is_empty() => {
                    let v = if !c.options.is_empty() {
                        c.options.get(c.selected).map(|o| o.0.clone()).unwrap_or_default()
                    } else if c.label.is_empty() && c.value != 0.0 {
                        c.value.to_string()
                    } else {
                        c.checked.to_string()
                    };
                    out.insert(c.id.clone(), v);
                }
                UiNode::Container(c) => go(&c.children, out),
                _ => {}
            }
        }
    }
    let mut out = BTreeMap::new();
    go(nodes, &mut out);
    out
}

/// L'element a `#id` le plus profond sous `(x, y)`, s'il y en a un.
fn id_sous(nodes: &[UiNode], page: (u32, u32, u32, u32), x: i32, y: i32) -> Option<String> {
    let mut trouve = None;
    interact::walk_with_paths(nodes, page, &mut |n, own, clip, _| {
        let dans = |r: (i32, i32, u32, u32)| x >= r.0 && y >= r.1 && x < r.0 + r.2 as i32 && y < r.1 + r.3 as i32;
        if dans(own) && dans(clip) && n.decoration().visible
            && let Some(id) = crate::window::models::pilote::id_de(n)
        {
            trouve = Some(id.to_string());
        }
    });
    trouve
}

fn nom_touche(k: u32) -> Option<&'static str> {
    Some(match k {
        28 => "entree",
        1 => "echap",
        15 => "tab",
        103 => "haut",
        108 => "bas",
        105 => "gauche",
        106 => "droite",
        _ => return None,
    })
}

impl Enregistrement {
    pub fn commencer(nodes: &[UiNode]) -> Enregistrement {
        Enregistrement { etapes: Vec::new(), champs: champs(nodes) }
    }

    /// Les champs changes depuis la derniere etape deviennent des `Remplir`.
    pub fn noter_champs(&mut self, nodes: &[UiNode]) {
        let maintenant = champs(nodes);
        for (id, v) in &maintenant {
            if self.champs.get(id) != Some(v) {
                // Plusieurs changements d'affilee du meme champ : le dernier.
                if let Some(Etape::Remplir(dernier, valeur)) = self.etapes.last_mut()
                    && dernier == id
                {
                    *valeur = v.clone();
                    continue;
                }
                self.etapes.push(Etape::Remplir(id.clone(), v.clone()));
            }
        }
        self.champs = maintenant;
    }

    /// Un evenement que l'app va recevoir (avant qu'elle le traite : l'ecran
    /// est encore celui que la personne voit).
    pub fn noter(&mut self, event: &WindowEvent, nodes: &[UiNode], page: (u32, u32, u32, u32), souris: (i32, i32)) {
        match *event {
            WindowEvent::WindowMouseButton(272, true) => {
                self.noter_champs(nodes);
                self.etapes.push(match id_sous(nodes, page, souris.0, souris.1) {
                    Some(id) => Etape::Clic(id),
                    None => Etape::ClicSansId(souris.0 - page.0 as i32, souris.1 - page.1 as i32),
                });
            }
            WindowEvent::WindowKeyPress(k, true) => {
                if let Some(nom) = nom_touche(k) {
                    self.noter_champs(nodes);
                    self.etapes.push(Etape::Touche(nom));
                }
            }
            WindowEvent::WindowScroll(dy) => {
                let Some(id) = id_sous(nodes, page, souris.0, souris.1) else { return };
                self.noter_champs(nodes);
                // Les crans d'affilee au meme endroit : un seul coup de molette.
                if let Some(Etape::Molette(dernier, total)) = self.etapes.last_mut()
                    && *dernier == id
                {
                    *total += dy;
                    return;
                }
                self.etapes.push(Etape::Molette(id, dy));
            }
            _ => {}
        }
    }

    /// Le test Rust : `exe` est le nom du binaire de l'app (`azure_testeur`).
    pub fn code(&self, exe: &str) -> String {
        let s = |t: &str| format!("{t:?}");
        let mut corps = String::new();
        for e in &self.etapes {
            corps += &match e {
                Etape::Clic(id) => format!("    app.clic({});\n", s(id)),
                Etape::ClicSansId(x, y) => format!("    // Clic a ({x}, {y}) sur un element sans #id : donnez-lui un id dans le .rsh pour le rejouer.\n"),
                Etape::Remplir(id, v) => format!("    app.remplir({}, {});\n", s(id), s(v)),
                Etape::Touche(t) => format!("    app.touche({});\n", s(t)),
                Etape::Molette(id, dy) => format!("    app.molette({}, {dy:.0}.0);\n", s(id)),
                Etape::Verifier(t) => format!("    app.attendre_texte({});\n", s(t)),
            };
        }
        let nom = format!("scenario_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()));
        format!(
            "// Enregistre avec l'inspecteur d'Azure (bouton « Enregistrer »).\n#[test]\nfn {nom}() {{\n    let env = azure_foundation::essai::Environnement::demarrer({}).unwrap();\n    let mut app = env.lancer(env!(\"CARGO_BIN_EXE_{exe}\")).unwrap();\n{corps}}}\n",
            s(&nom)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_d_un_scenario() {
        let e = Enregistrement { etapes: vec![Etape::Clic("lier".into()), Etape::Remplir("nom".into(), "a \"b\"".into()), Etape::Touche("entree"), Etape::Molette("liste".into(), 240.0), Etape::Verifier("2 tests".into()), Etape::ClicSansId(10, 20)], ..Default::default() };
        let code = e.code("azure_testeur");
        assert!(code.contains("env!(\"CARGO_BIN_EXE_azure_testeur\")"), "{code}");
        assert!(code.contains("app.clic(\"lier\");\n    app.remplir(\"nom\", \"a \\\"b\\\"\");\n    app.touche(\"entree\");\n    app.molette(\"liste\", 240.0);\n    app.attendre_texte(\"2 tests\");"), "{code}");
        assert!(code.contains("// Clic a (10, 20)"), "{code}");
    }
}
