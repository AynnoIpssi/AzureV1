// Chemins dans l'etat d'un flux : `"panier.items.0.nom"`. Un segment
// numerique designe une case de liste. `""` = la racine.
//
// Un motif d'ecoute peut contenir `*` pour un segment quelconque :
// `"panier.items.*.prix"`.

pub const MAX_SEGMENTS: usize = 32;
pub const MAX_SEGMENT_LEN: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    segments: Vec<String>,
}

impl Path {
    pub fn root() -> Path {
        Path { segments: Vec::new() }
    }

    pub fn parse(text: &str) -> Result<Path, String> {
        Path::parse_with(text, false)
    }

    /// Comme `parse`, `*` accepte comme segment.
    pub fn pattern(text: &str) -> Result<Path, String> {
        Path::parse_with(text, true)
    }

    fn parse_with(text: &str, wildcard: bool) -> Result<Path, String> {
        if text.is_empty() {
            return Ok(Path::root());
        }
        let segments: Vec<String> = text.split('.').map(str::to_string).collect();
        if segments.len() > MAX_SEGMENTS {
            return Err(format!("Chemin '{text}' trop long (plus de {MAX_SEGMENTS} niveaux)"));
        }
        for segment in &segments {
            if segment.is_empty() {
                return Err(format!("Chemin '{text}' : segment vide"));
            }
            if segment.len() > MAX_SEGMENT_LEN || segment.chars().any(char::is_control) {
                return Err(format!("Chemin '{text}' : segment invalide"));
            }
            if segment == "*" && !wildcard {
                return Err(format!("Chemin '{text}' : '*' reserve aux motifs d'ecoute"));
            }
        }
        Ok(Path { segments })
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Un motif et un chemin sont lies si l'un prolonge l'autre : une
    /// modification de `panier` touche `panier.total`, et une de
    /// `panier.total` touche `panier`.
    pub fn related(&self, path: &Path) -> bool {
        self.segments.iter().zip(&path.segments).all(|(pattern, segment)| pattern == "*" || pattern == segment)
    }

    pub fn as_string(&self) -> String {
        self.segments.join(".")
    }
}
