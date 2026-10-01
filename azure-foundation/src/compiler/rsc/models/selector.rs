/// Un selecteur simple : l'equivalent d'un "compound selector" CSS
/// (`container.card#main`) - au plus un selecteur de type (nom de balise ou
/// `*`), zero ou plusieurs classes, au plus un id.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SimpleSelector {
    pub universal: bool,
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    /// Pseudo-classes reconnues syntaxiquement (`:hover`, `:focus`,
    /// `:first-child`, `:nth-child(2)`...). Seules `:hover` et `:focus`
    /// sont reellement evaluees (voir `PseudoState`, comparee par
    /// `matches`) - les seules a avoir un sens pour les elements
    /// interactifs actuels (`Button`/`TextArea`). Toute autre pseudo-classe
    /// est conservee (pour ne pas faire echouer le parsing d'une feuille
    /// qui l'utilise) mais ne correspond jamais a un element, quel que
    /// soit son etat - leur evaluation reelle est prevue pour plus tard.
    pub pseudo_classes: Vec<String>,
}

/// Quels pseudo-etats sont actuellement actifs pour UN element donne au
/// moment de la cascade (voir `services::link::resolve_element`) - c'est ce
/// qu'un selecteur comme `button:hover` compare a ses `pseudo_classes` (voir
/// `SimpleSelector::matches`). `PseudoState::default()` (rien d'actif)
/// retrouve exactement le comportement d'avant leur evaluation : un
/// selecteur avec une pseudo-classe ne correspond jamais.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PseudoState {
    pub hover: bool,
    pub focus: bool,
    /// Pendant l'appui (`button:active`). Un bouton appuye est aussi
    /// survole : on le resout avec `hover` ET `active`.
    pub active: bool,
}

impl PseudoState {
    fn is_active(&self, name: &str) -> bool {
        match name {
            "hover" => self.hover,
            "focus" => self.focus,
            "active" => self.active,
            _ => false,
        }
    }
}

/// Les combinateurs CSS entre deux selecteurs simples d'un selecteur
/// complexe - tous evalues par `services::link::chain_matches`.
/// `AdjacentSibling` (`+`) matche le frere immediatement precedent,
/// `GeneralSibling` (`~`) n'importe quel frere precedent (avec
/// backtracking : plusieurs candidats sont essayes si le premier ne mene a
/// aucune suite valide plus a gauche dans la chaine). Limite connue : au
/// dela d'un `Child`/`Descendant` dans la chaine, l'info des freres du
/// nouveau sujet (l'ancetre) n'est pas suivie - un combinateur frere encore
/// plus a gauche (`a + b > c`, lu de droite a gauche) ne trouve donc jamais
/// de candidat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Combinator {
    Descendant,
    Child,
    AdjacentSibling,
    GeneralSibling,
}

/// Un selecteur complexe : une chaine de selecteurs simples relies par des
/// combinateurs, dans l'ordre racine -> feuille. `parts[i].1` est le
/// combinateur entre `parts[i-1]` et `parts[i]` (donc toujours `None` pour
/// `parts[0]`). Exemple : `container .card > button` devient
/// `[(container, None), (.card, Some(Descendant)), (button, Some(Child))]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexSelector {
    pub parts: Vec<(SimpleSelector, Option<Combinator>)>,
}

/// Specificite CSS `(id, classes+pseudo-classes, types)`, comparee
/// lexicographiquement comme le veut la specification : un seul id l'emporte
/// toujours sur n'importe quel nombre de classes ou de types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl SimpleSelector {
    pub fn specificity(&self) -> Specificity {
        let id = u32::from(self.id.is_some());
        let classes = (self.classes.len() + self.pseudo_classes.len()) as u32;
        let types = u32::from(self.tag.is_some());
        Specificity(id, classes, types)
    }

    /// `pseudo` decrit l'etat de CET element (survole ? focalise ?) au
    /// moment du test - toutes ses `pseudo_classes` doivent y etre actives
    /// pour matcher (voir `PseudoState`) ; une pseudo-classe non reconnue
    /// (`PseudoState::is_active` la sait toujours inactive) ne correspond
    /// donc jamais, quel que soit `pseudo`. Le selecteur universel (`*`)
    /// sans autre contrainte correspond a n'importe quel element.
    pub fn matches(&self, tag: &str, classes: &[String], id: &str, pseudo: PseudoState) -> bool {
        if !self.pseudo_classes.iter().all(|p| pseudo.is_active(p)) {
            return false;
        }
        if let Some(want) = &self.tag
            && !want.eq_ignore_ascii_case(tag) {
                return false;
            }
        if let Some(want) = &self.id
            && id != want {
                return false;
            }
        self.classes.iter().all(|c| classes.iter().any(|has| has == c))
    }
}

impl ComplexSelector {
    /// La specificite d'un selecteur complexe est la somme de celles de
    /// chacun de ses maillons.
    pub fn specificity(&self) -> Specificity {
        self.parts.iter().fold(Specificity::default(), |acc, (simple, _)| {
            let sp = simple.specificity();
            Specificity(acc.0 + sp.0, acc.1 + sp.1, acc.2 + sp.2)
        })
    }
}
