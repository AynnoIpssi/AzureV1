// Lecture d'un fichier Rust, juste assez pour trouver ses tests : les mots
// et les signes, sans les commentaires, textes et caracteres (une accolade
// dans un texte ne compte pas). Puis les `mod x { }` et les fonctions
// marquees `#[test]` (ou `#[tokio::test]`...), avec leurs positions exactes
// pour que l'atelier puisse les reecrire.

#[derive(Clone, Debug, PartialEq)]
pub enum Genre {
    Mot(String),
    Signe(char),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Jeton {
    pub genre: Genre,
    /// Octets dans le texte.
    pub debut: usize,
    pub fin: usize,
}

impl Jeton {
    fn mot(&self) -> Option<&str> {
        match &self.genre {
            Genre::Mot(m) => Some(m),
            Genre::Signe(_) => None,
        }
    }

    fn signe(&self, c: char) -> bool {
        self.genre == Genre::Signe(c)
    }
}

/// Les jetons de `src`.
pub fn jetons(src: &str) -> Vec<Jeton> {
    let c: Vec<(usize, char)> = src.char_indices().collect();
    let n = c.len();
    let at = |i: usize| c.get(i).map(|x| x.1).unwrap_or('\0');
    let off = |i: usize| c.get(i).map(|x| x.0).unwrap_or(src.len());
    let mut out = Vec::new();
    let mut i = 0;
    // Fin d'un texte "..." qui commence en `i`.
    let chaine = |mut i: usize| {
        i += 1;
        while i < n {
            match at(i) {
                '\\' => i += 2,
                '"' => return i + 1,
                _ => i += 1,
            }
        }
        n
    };
    // Fin d'un caractere '.' qui commence en `i`, ou `None` (duree de vie).
    let caractere = |i: usize| {
        if at(i + 1) == '\\' {
            let mut j = i + 3;
            while j < n && at(j) != '\'' && at(j) != '\n' {
                j += 1;
            }
            Some(j + 1)
        } else if at(i + 2) == '\'' {
            Some(i + 3)
        } else {
            None
        }
    };
    while i < n {
        let ch = at(i);
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '/' && at(i + 1) == '/' {
            while i < n && at(i) != '\n' {
                i += 1;
            }
        } else if ch == '/' && at(i + 1) == '*' {
            let mut prof = 1;
            i += 2;
            while i < n && prof > 0 {
                if at(i) == '/' && at(i + 1) == '*' {
                    prof += 1;
                    i += 2;
                } else if at(i) == '*' && at(i + 1) == '/' {
                    prof -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if ch == '"' {
            i = chaine(i);
        } else if ch == '\'' {
            match caractere(i) {
                Some(j) => i = j,
                None => {
                    // Duree de vie : 'a
                    i += 1;
                    while i < n && (at(i).is_alphanumeric() || at(i) == '_') {
                        i += 1;
                    }
                }
            }
        } else if ch.is_alphabetic() || ch == '_' {
            let mut j = i;
            while j < n && (at(j).is_alphanumeric() || at(j) == '_') {
                j += 1;
            }
            let mot: String = c[i..j].iter().map(|x| x.1).collect();
            // Textes bruts r"..." r#"..."#, octets b"..." b'.'.
            if matches!(mot.as_str(), "r" | "br" | "cr") && (at(j) == '"' || at(j) == '#') {
                let mut k = j;
                let mut dieses = 0;
                while at(k) == '#' {
                    dieses += 1;
                    k += 1;
                }
                if at(k) == '"' {
                    k += 1;
                    'fin: while k < n {
                        if at(k) == '"' && (1..=dieses).all(|d| at(k + d) == '#') {
                            k += 1 + dieses;
                            break 'fin;
                        }
                        k += 1;
                    }
                    i = k;
                    continue;
                }
            }
            if matches!(mot.as_str(), "b" | "c") && at(j) == '"' {
                i = chaine(j);
                continue;
            }
            if mot == "b" && at(j) == '\'' {
                i = caractere(j).unwrap_or(j + 1);
                continue;
            }
            out.push(Jeton { genre: Genre::Mot(mot), debut: off(i), fin: off(j) });
            i = j;
        } else if ch.is_ascii_digit() {
            while i < n && (at(i).is_alphanumeric() || at(i) == '_' || (at(i) == '.' && at(i + 1).is_ascii_digit())) {
                i += 1;
            }
        } else {
            out.push(Jeton { genre: Genre::Signe(ch), debut: off(i), fin: off(i + 1) });
            i += 1;
        }
    }
    out
}

/// Un attribut `#[...]` : son chemin (`test`, `tokio::test`) et son texte.
#[derive(Clone, Debug, PartialEq)]
pub struct Attribut {
    pub chemin: String,
    pub texte: String,
    pub debut: usize,
    pub fin: usize,
}

/// Une fonction de test.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Trouve {
    pub nom: String,
    /// Modules ecrits dans le fichier (`mod tests { }`), du plus exterieur.
    pub modules: Vec<String>,
    pub ligne: usize,
    pub attributs: Vec<Attribut>,
    /// Debut du premier attribut.
    pub debut: usize,
    /// La signature : de la fin des attributs a l'accolade ouvrante.
    pub signature: (usize, usize),
    /// L'interieur des accolades du corps.
    pub corps: (usize, usize),
    /// Juste apres l'accolade fermante.
    pub fin: usize,
    /// Debut de l'interieur du module qui la contient (0 : le fichier).
    pub zone: usize,
}

impl Trouve {
    pub fn a(&self, chemin: &str) -> bool {
        self.attributs.iter().any(|a| a.chemin == chemin)
    }
}

/// Un `mod x { }` ecrit dans le fichier.
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub nom: String,
    /// Marque `#[cfg(test)]`.
    pub de_test: bool,
    /// Debut de la ligne du `mod`.
    pub ligne_debut: usize,
    /// L'interieur des accolades.
    pub corps: (usize, usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Carte {
    pub tests: Vec<Trouve>,
    pub modules: Vec<Module>,
}

impl Carte {
    /// Le module ou ranger les tests unitaires : `#[cfg(test)]`, sinon `tests`.
    pub fn module_de_tests(&self) -> Option<&Module> {
        self.modules.iter().find(|m| m.de_test).or_else(|| self.modules.iter().find(|m| m.nom == "tests"))
    }
}

/// Un attribut est-il un attribut de test ?
fn est_test(chemin: &str) -> bool {
    chemin == "test" || chemin.ends_with("::test") || chemin == "rstest" || chemin == "test_case"
}

/// Numero de ligne (1...) de l'octet `o`.
pub fn ligne_de(src: &str, o: usize) -> usize {
    src[..o.min(src.len())].matches('\n').count() + 1
}

/// Les tests et modules de `src`.
pub fn carte(src: &str) -> Carte {
    let j = jetons(src);
    let mut carte = Carte::default();
    let mut prof = 0usize;
    // (nom, profondeur de son interieur, indice dans carte.modules)
    let mut pile: Vec<(String, usize, usize)> = Vec::new();
    let mut attributs: Vec<Attribut> = Vec::new();
    let mut attente: Option<Trouve> = None;
    let mut corps_ouvert: Option<(usize, usize)> = None;
    let mut k = 0;
    while k < j.len() {
        let t = &j[k];
        if t.signe('#') {
            let interieur = j.get(k + 1).is_some_and(|x| x.signe('!'));
            let ouvre = k + if interieur { 2 } else { 1 };
            if j.get(ouvre).is_some_and(|x| x.signe('[')) {
                let mut niveau = 0;
                let mut m = ouvre;
                while m < j.len() {
                    if j[m].signe('[') {
                        niveau += 1;
                    } else if j[m].signe(']') {
                        niveau -= 1;
                        if niveau == 0 {
                            break;
                        }
                    }
                    m += 1;
                }
                if !interieur {
                    let mut chemin = String::new();
                    let mut p = ouvre + 1;
                    while p < j.len() {
                        match &j[p].genre {
                            Genre::Mot(w) => chemin.push_str(w),
                            Genre::Signe(':') => chemin.push(':'),
                            _ => break,
                        }
                        p += 1;
                    }
                    let fin = j.get(m).map(|x| x.fin).unwrap_or(src.len());
                    attributs.push(Attribut { chemin, texte: src[t.debut..fin].to_string(), debut: t.debut, fin });
                }
                k = m + 1;
                continue;
            }
        }
        match &t.genre {
            Genre::Mot(w) if w == "mod" => {
                if let (Some(nom), Some(ouvre)) = (j.get(k + 1).and_then(Jeton::mot), j.get(k + 2))
                    && ouvre.signe('{')
                {
                    let de_test = attributs.iter().any(|a| a.chemin == "cfg" && a.texte.contains("test"));
                    let debut = attributs.first().map(|a| a.debut).unwrap_or(t.debut);
                    let ligne_debut = src[..debut].rfind('\n').map(|p| p + 1).unwrap_or(0);
                    carte.modules.push(Module { nom: nom.to_string(), de_test, ligne_debut, corps: (ouvre.fin, src.len()) });
                    prof += 1;
                    pile.push((nom.to_string(), prof, carte.modules.len() - 1));
                    attributs.clear();
                    k += 3;
                    continue;
                }
                attributs.clear();
            }
            Genre::Mot(w) if w == "fn" => {
                if let Some(nom) = j.get(k + 1).and_then(Jeton::mot)
                    && attributs.iter().any(|a| est_test(&a.chemin))
                {
                    let debut = attributs[0].debut;
                    let fin_attributs = attributs.last().map(|a| a.fin).unwrap_or(t.debut);
                    attente = Some(Trouve {
                        nom: nom.to_string(),
                        modules: pile.iter().map(|p| p.0.clone()).collect(),
                        ligne: ligne_de(src, t.debut),
                        attributs: std::mem::take(&mut attributs),
                        debut,
                        signature: (fin_attributs, t.debut),
                        zone: pile.last().map(|p| carte.modules[p.2].corps.0).unwrap_or(0),
                        ..Trouve::default()
                    });
                    k += 2;
                    continue;
                }
                attributs.clear();
            }
            Genre::Signe('{') => {
                if let Some(mut trouve) = attente.take() {
                    trouve.signature.1 = t.debut;
                    trouve.corps.0 = t.fin;
                    carte.tests.push(trouve);
                    corps_ouvert = Some((carte.tests.len() - 1, prof));
                }
                prof += 1;
                attributs.clear();
            }
            Genre::Signe('}') => {
                prof = prof.saturating_sub(1);
                if let Some((i, p)) = corps_ouvert
                    && p == prof
                {
                    carte.tests[i].corps.1 = t.debut;
                    carte.tests[i].fin = t.fin;
                    corps_ouvert = None;
                }
                while pile.last().is_some_and(|p| p.1 > prof) {
                    if let Some((_, _, m)) = pile.pop() {
                        carte.modules[m].corps.1 = t.debut;
                    }
                }
                attributs.clear();
            }
            Genre::Signe(';') => {
                attente = None;
                attributs.clear();
            }
            _ => {}
        }
        k += 1;
    }
    // Corps non ferme (fichier en cours d'ecriture) : jusqu'au bout.
    for t in carte.tests.iter_mut().filter(|t| t.fin == 0) {
        t.corps.1 = src.len();
        t.fin = src.len();
    }
    carte
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trouve_les_tests_dans_les_modules() {
        let src = "fn a() { let s = \"}\"; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn somme() {\n        assert_eq!(1 + 1, 2); // }\n    }\n\n    #[test]\n    #[ignore]\n    fn lent() { let c = '{'; }\n}\n";
        let c = carte(src);
        assert_eq!(c.tests.iter().map(|t| t.nom.as_str()).collect::<Vec<_>>(), ["somme", "lent"]);
        assert_eq!(c.tests[0].modules, ["tests"]);
        assert_eq!(c.tests[0].ligne, 7);
        assert!(c.tests[1].a("ignore"));
        assert_eq!(&src[c.tests[0].corps.0..c.tests[0].corps.1].trim(), &"assert_eq!(1 + 1, 2); // }");
        let m = c.module_de_tests().unwrap();
        assert!(m.de_test);
        assert!(src[m.corps.0..m.corps.1].contains("fn lent"));
    }

    #[test]
    fn textes_bruts_et_durees_de_vie() {
        let src = "fn f<'a>(x: &'a str) -> &'a str { let r = r#\"{ \"# ; x }\n#[tokio::test]\nasync fn reseau() {}\n";
        let c = carte(src);
        assert_eq!(c.tests.len(), 1);
        assert_eq!(c.tests[0].nom, "reseau");
        assert!(c.tests[0].modules.is_empty());
    }
}
