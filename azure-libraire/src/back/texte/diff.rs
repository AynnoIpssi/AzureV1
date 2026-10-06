// Comparer deux textes : quelles lignes ont ete retirees, ajoutees,
// gardees. L'algorithme est celui de Myers (le plus court chemin de
// modifications), celui de `diff` et de Git.
//
// - `lignes(avant, apres)` : chaque ligne avec son genre et ses numeros ;
// - `blocs(&lignes, contexte)` : les modifications regroupees, entourees
//   de quelques lignes inchangees (ce qu'un ecran de diff affiche) ;
// - `unifie(...)` : le format `diff -u`, relu par `patch` et `git apply` ;
// - `mots(avant, apres)` : la meme comparaison mot par mot, pour montrer
//   ce qui a change DANS une ligne ;
// - `operations(a, b)` : l'algorithme nu, sur n'importe quelles listes.

/// Au-dela de ce nombre de modifications, on ne cherche plus le plus court
/// chemin (la memoire grandit avec son carre) : tout le milieu est donne
/// comme retire puis ajoute.
const LIMITE: usize = 3000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genre {
    Egal,
    Retire,
    Ajoute,
}

/// Un pas de la comparaison : les rangs (a partir de 0) dans `a` et `b`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Garde { a: usize, b: usize },
    Retire { a: usize },
    Ajoute { b: usize },
}

/// Ce qu'il faut faire a `a` pour obtenir `b`, dans l'ordre.
pub fn operations<T: PartialEq>(a: &[T], b: &[T]) -> Vec<Operation> {
    // Le debut et la fin communs ne coutent rien a ecarter.
    let debut = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let fin = a[debut..].iter().rev().zip(b[debut..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (ma, mb) = (&a[debut..a.len() - fin], &b[debut..b.len() - fin]);
    let mut out: Vec<Operation> = (0..debut).map(|i| Operation::Garde { a: i, b: i }).collect();
    let milieu = myers(ma, mb).unwrap_or_else(|| (0..ma.len()).map(|a| Operation::Retire { a }).chain((0..mb.len()).map(|b| Operation::Ajoute { b })).collect());
    out.extend(milieu.into_iter().map(|o| match o {
        Operation::Garde { a, b } => Operation::Garde { a: a + debut, b: b + debut },
        Operation::Retire { a } => Operation::Retire { a: a + debut },
        Operation::Ajoute { b } => Operation::Ajoute { b: b + debut },
    }));
    out.extend((0..fin).map(|i| Operation::Garde { a: a.len() - fin + i, b: b.len() - fin + i }));
    out
}

// `None` si le chemin demande plus de LIMITE modifications.
fn myers<T: PartialEq>(a: &[T], b: &[T]) -> Option<Vec<Operation>> {
    let (n, m) = (a.len() as i32, b.len() as i32);
    if n == 0 || m == 0 {
        return Some((0..a.len()).map(|a| Operation::Retire { a }).chain((0..b.len()).map(|b| Operation::Ajoute { b })).collect());
    }
    let max = ((n + m) as usize).min(LIMITE) as i32;
    let decalage = max + 1;
    // v[k] : le plus loin qu'on arrive dans `a` sur la diagonale k (x - y).
    let mut v = vec![0i32; 2 * max as usize + 3];
    // L'etat de `v` avant chaque tour, diagonales -d..=d seulement.
    let mut traces: Vec<Vec<i32>> = Vec::new();
    let mut trouve = None;
    'tours: for d in 0..=max {
        traces.push(v[(decalage - d) as usize..=(decalage + d) as usize].to_vec());
        let mut k = -d;
        while k <= d {
            let i = (decalage + k) as usize;
            let mut x = if k == -d || (k != d && v[i - 1] < v[i + 1]) { v[i + 1] } else { v[i - 1] + 1 };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[i] = x;
            if x >= n && y >= m {
                trouve = Some(d);
                break 'tours;
            }
            k += 2;
        }
    }
    // On remonte le chemin depuis la fin.
    let mut out = Vec::new();
    let (mut x, mut y) = (n, m);
    for d in (0..=trouve?).rev() {
        if d == 0 {
            while x > 0 && y > 0 {
                out.push(Operation::Garde { a: x as usize - 1, b: y as usize - 1 });
                x -= 1;
                y -= 1;
            }
            break;
        }
        let trace = &traces[d as usize];
        let v = |k: i32| trace[(k + d) as usize];
        let k = x - y;
        let avant_k = if k == -d || (k != d && v(k - 1) < v(k + 1)) { k + 1 } else { k - 1 };
        let avant_x = v(avant_k);
        let avant_y = avant_x - avant_k;
        while x > avant_x && y > avant_y {
            out.push(Operation::Garde { a: x as usize - 1, b: y as usize - 1 });
            x -= 1;
            y -= 1;
        }
        if avant_k == k + 1 {
            out.push(Operation::Ajoute { b: y as usize - 1 });
        } else {
            out.push(Operation::Retire { a: x as usize - 1 });
        }
        (x, y) = (avant_x, avant_y);
    }
    out.reverse();
    Some(out)
}

/// Une ligne d'un diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ligne<'a> {
    pub genre: Genre,
    /// Sans son retour a la ligne.
    pub texte: &'a str,
    /// Son numero (a partir de 1) dans l'ancien texte, sauf si ajoutee.
    pub ancien: Option<usize>,
    /// Son numero dans le nouveau texte, sauf si retiree.
    pub nouveau: Option<usize>,
    /// `false` pour une derniere ligne qui n'a pas de retour a la ligne.
    pub terminee: bool,
}

fn decouper(texte: &str) -> Vec<&str> {
    texte.split_inclusive('\n').collect()
}

/// Compare `avant` et `apres` ligne par ligne.
pub fn lignes<'a>(avant: &'a str, apres: &'a str) -> Vec<Ligne<'a>> {
    let (a, b) = (decouper(avant), decouper(apres));
    let ligne = |genre, brut: &'a str, ancien: Option<usize>, nouveau: Option<usize>| {
        let texte = brut.strip_suffix('\n').unwrap_or(brut);
        Ligne { genre, texte, ancien: ancien.map(|i| i + 1), nouveau: nouveau.map(|i| i + 1), terminee: brut.ends_with('\n') }
    };
    operations(&a, &b)
        .into_iter()
        .map(|o| match o {
            Operation::Garde { a: i, b: j } => ligne(Genre::Egal, a[i], Some(i), Some(j)),
            Operation::Retire { a: i } => ligne(Genre::Retire, a[i], Some(i), None),
            Operation::Ajoute { b: j } => ligne(Genre::Ajoute, b[j], None, Some(j)),
        })
        .collect()
}

/// (lignes ajoutees, lignes retirees).
pub fn compte(lignes: &[Ligne]) -> (usize, usize) {
    (lignes.iter().filter(|l| l.genre == Genre::Ajoute).count(), lignes.iter().filter(|l| l.genre == Genre::Retire).count())
}

/// Des modifications voisines et les lignes inchangees qui les entourent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bloc<'a> {
    /// Premiere ligne du bloc dans l'ancien texte (a partir de 1 ; la
    /// ligne d'avant si le bloc n'en contient aucune) et leur nombre.
    pub ancien: (usize, usize),
    pub nouveau: (usize, usize),
    pub lignes: Vec<Ligne<'a>>,
}

impl Bloc<'_> {
    /// `@@ -12,7 +12,9 @@`
    pub fn entete(&self) -> String {
        let plage = |(debut, nombre): (usize, usize)| if nombre == 1 { debut.to_string() } else { format!("{debut},{nombre}") };
        format!("@@ -{} +{} @@", plage(self.ancien), plage(self.nouveau))
    }
}

/// Regroupe les modifications en blocs, avec `contexte` lignes inchangees
/// autour (3 d'habitude). Deux modifications dont les contextes se
/// touchent partagent un bloc. Aucun bloc si rien n'a change.
pub fn blocs<'a>(lignes: &[Ligne<'a>], contexte: usize) -> Vec<Bloc<'a>> {
    let modifiees: Vec<usize> = (0..lignes.len()).filter(|i| lignes[*i].genre != Genre::Egal).collect();
    let mut plages: Vec<(usize, usize)> = Vec::new();
    for i in modifiees {
        let (debut, fin) = (i.saturating_sub(contexte), (i + contexte + 1).min(lignes.len()));
        match plages.last_mut() {
            Some(derniere) if debut <= derniere.1 => derniere.1 = fin,
            _ => plages.push((debut, fin)),
        }
    }
    plages
        .into_iter()
        .map(|(debut, fin)| {
            let morceau = &lignes[debut..fin];
            // Les lignes deja passees de chaque cote avant ce bloc.
            let passees_a = lignes[..debut].iter().filter(|l| l.genre != Genre::Ajoute).count();
            let passees_b = lignes[..debut].iter().filter(|l| l.genre != Genre::Retire).count();
            let nombre_a = morceau.iter().filter(|l| l.genre != Genre::Ajoute).count();
            let nombre_b = morceau.iter().filter(|l| l.genre != Genre::Retire).count();
            let depart = |passees: usize, nombre: usize| if nombre == 0 { passees } else { passees + 1 };
            Bloc { ancien: (depart(passees_a, nombre_a), nombre_a), nouveau: (depart(passees_b, nombre_b), nombre_b), lignes: morceau.to_vec() }
        })
        .collect()
}

/// Le diff au format unifie (`diff -u`), vide si les textes sont egaux.
/// `nom_avant` / `nom_apres` : ce qui suit `---` et `+++`.
pub fn unifie(nom_avant: &str, nom_apres: &str, avant: &str, apres: &str, contexte: usize) -> String {
    let lignes = lignes(avant, apres);
    let blocs = blocs(&lignes, contexte);
    if blocs.is_empty() {
        return String::new();
    }
    let mut out = format!("--- {nom_avant}\n+++ {nom_apres}\n");
    for bloc in blocs {
        out.push_str(&bloc.entete());
        out.push('\n');
        for l in &bloc.lignes {
            out.push(match l.genre {
                Genre::Egal => ' ',
                Genre::Retire => '-',
                Genre::Ajoute => '+',
            });
            out.push_str(l.texte);
            out.push('\n');
            if !l.terminee {
                out.push_str("\\ No newline at end of file\n");
            }
        }
    }
    out
}

/// Compare deux lignes mot par mot : les morceaux, dans l'ordre, avec leur
/// genre. Les morceaux `Egal` et `Retire` mis bout a bout redonnent
/// `avant` ; `Egal` et `Ajoute`, `apres`.
pub fn mots<'a>(avant: &'a str, apres: &'a str) -> Vec<(Genre, &'a str)> {
    let (a, b) = (jetons(avant), jetons(apres));
    let mut out: Vec<(Genre, &'a str)> = Vec::new();
    for o in operations(&a, &b) {
        let (genre, texte) = match o {
            Operation::Garde { a: i, .. } => (Genre::Egal, a[i]),
            Operation::Retire { a: i } => (Genre::Retire, a[i]),
            Operation::Ajoute { b: j } => (Genre::Ajoute, b[j]),
        };
        // Deux morceaux voisins du meme genre et qui se suivent dans le
        // meme texte n'en font qu'un.
        let source = if genre == Genre::Ajoute { apres } else { avant };
        match out.last_mut() {
            Some((g, dernier)) if *g == genre && dernier.as_ptr() as usize + dernier.len() == texte.as_ptr() as usize => {
                let debut = dernier.as_ptr() as usize - source.as_ptr() as usize;
                *dernier = &source[debut..debut + dernier.len() + texte.len()];
            }
            _ => out.push((genre, texte)),
        }
    }
    out
}

// Un mot (lettres, chiffres, `_`), une suite d'espaces, ou un seul autre
// caractere.
fn jetons(texte: &str) -> Vec<&str> {
    let classe = |c: char| if c.is_alphanumeric() || c == '_' { 1 } else if c.is_whitespace() { 2 } else { 0 };
    let mut out = Vec::new();
    let mut debut = 0;
    let mut precedente = 0;
    for (i, c) in texte.char_indices() {
        let k = classe(c);
        if i > debut && (k == 0 || k != precedente) {
            out.push(&texte[debut..i]);
            debut = i;
        }
        precedente = k;
    }
    if debut < texte.len() {
        out.push(&texte[debut..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::back::hasard::Alea;

    fn appliquer<T: Clone + PartialEq>(a: &[T], b: &[T]) -> (Vec<T>, usize) {
        let ops = operations(a, b);
        let mut out = Vec::new();
        let (mut ia, mut modifs) = (0, 0);
        for o in &ops {
            match *o {
                Operation::Garde { a: i, b: j } => {
                    assert_eq!((i, j), (ia, out.len()));
                    assert!(a[i] == b[j]);
                    out.push(a[i].clone());
                    ia += 1;
                }
                Operation::Retire { a: i } => {
                    assert_eq!(i, ia);
                    ia += 1;
                    modifs += 1;
                }
                Operation::Ajoute { b: j } => {
                    assert_eq!(j, out.len());
                    out.push(b[j].clone());
                    modifs += 1;
                }
            }
        }
        assert_eq!(ia, a.len());
        (out, modifs)
    }

    #[test]
    fn l_exemple_de_myers() {
        let (a, b): (Vec<char>, Vec<char>) = ("ABCABBA".chars().collect(), "CBABAC".chars().collect());
        let (obtenu, modifs) = appliquer(&a, &b);
        assert_eq!(obtenu, b);
        // Le plus court chemin de l'article : 5 modifications.
        assert_eq!(modifs, 5);
    }

    #[test]
    fn des_listes_au_hasard_se_reconstruisent() {
        let mut alea = Alea::new(3);
        for _ in 0..300 {
            let liste = |alea: &mut Alea| (0..alea.sous(30)).map(|_| alea.sous(4) as u8).collect::<Vec<u8>>();
            let (a, b) = (liste(&mut alea), liste(&mut alea));
            assert_eq!(appliquer(&a, &b).0, b);
        }
        let vide: [u8; 0] = [];
        assert_eq!(appliquer(&vide, &vide), (vec![], 0));
        assert_eq!(appliquer(&[1, 2], &vide).1, 2);
        assert_eq!(appliquer(&vide, &[1, 2]).1, 2);
        assert_eq!(appliquer(&[1, 2, 3], &[1, 2, 3]).1, 0);
    }

    #[test]
    fn deux_textes_sans_rapport_au_dela_de_la_limite() {
        let a: Vec<usize> = (0..LIMITE).collect();
        let b: Vec<usize> = (LIMITE..2 * LIMITE + 10).collect();
        assert_eq!(appliquer(&a, &b), (b.clone(), 2 * LIMITE + 10));
    }

    #[test]
    fn les_lignes_portent_leurs_numeros() {
        let l = lignes("un\ndeux\ntrois\n", "un\nDEUX\ntrois\nquatre");
        let vu: Vec<(Genre, &str, Option<usize>, Option<usize>)> = l.iter().map(|l| (l.genre, l.texte, l.ancien, l.nouveau)).collect();
        assert_eq!(
            vu,
            [
                (Genre::Egal, "un", Some(1), Some(1)),
                (Genre::Retire, "deux", Some(2), None),
                (Genre::Ajoute, "DEUX", None, Some(2)),
                (Genre::Egal, "trois", Some(3), Some(3)),
                (Genre::Ajoute, "quatre", None, Some(4)),
            ]
        );
        assert!(!l[4].terminee);
        assert_eq!(compte(&l), (2, 1));
    }

    #[test]
    fn les_blocs_gardent_leur_contexte() {
        let avant: String = (1..=30).map(|i| format!("ligne {i}\n")).collect();
        let apres = avant.replace("ligne 5\n", "cinq\n").replace("ligne 25\n", "");
        let l = lignes(&avant, &apres);
        let b = blocs(&l, 3);
        assert_eq!(b.iter().map(Bloc::entete).collect::<Vec<_>>(), ["@@ -2,7 +2,7 @@", "@@ -22,7 +22,6 @@"]);
        // Avec un grand contexte, les deux modifications se rejoignent.
        assert_eq!(blocs(&l, 10).len(), 1);
        assert!(blocs(&lignes(&avant, &avant), 3).is_empty());
    }

    #[test]
    fn le_format_unifie() {
        assert_eq!(unifie("a", "b", "x\n", "x\n", 3), "");
        assert_eq!(unifie("a/f", "b/f", "un\ndeux\n", "un\ntrois", 3), "--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n un\n-deux\n+trois\n\\ No newline at end of file\n");
        // Un fichier qui apparait : la plage de l'ancien est `0,0`.
        assert_eq!(unifie("a", "b", "", "neuf\n", 3), "--- a\n+++ b\n@@ -0,0 +1 @@\n+neuf\n");
    }

    #[test]
    fn mot_par_mot() {
        let m = mots("let total = prix * 2;", "let total = prix_ht * 2.5;");
        assert_eq!(m, [(Genre::Egal, "let total = "), (Genre::Retire, "prix"), (Genre::Ajoute, "prix_ht"), (Genre::Egal, " * 2"), (Genre::Ajoute, ".5"), (Genre::Egal, ";")]);
        let redonne = |garde: Genre| m.iter().filter(|(g, _)| *g == Genre::Egal || *g == garde).map(|(_, t)| *t).collect::<String>();
        assert_eq!(redonne(Genre::Retire), "let total = prix * 2;");
        assert_eq!(redonne(Genre::Ajoute), "let total = prix_ht * 2.5;");
    }
}
