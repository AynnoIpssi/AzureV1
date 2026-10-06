// Le texte en colonnes : CSV (separateur `,` `;` ou `|`, devine), TSV
// (tabulations, ce que donne un bloc copie depuis Excel ou LibreOffice).
// Une cellule entre guillemets peut contenir le separateur, des retours
// a la ligne et des guillemets doubles (`""`).

pub use crate::back::encodage::texte::decoder as texte;

/// Le separateur du texte : celui qui revient autant de fois sur chacune
/// des premieres lignes (hors guillemets). A egalite : tabulation, `;`,
/// `,` puis `|` (« 1,5;2,5 » est du `;` avec des virgules decimales).
pub fn separateur(texte: &str) -> char {
    const CANDIDATS: [char; 4] = ['\t', ';', ',', '|'];
    let mut comptes: Vec<[usize; 4]> = Vec::new();
    let mut courant = [0usize; 4];
    let (mut dans, mut vide) = (false, true);
    for c in texte.chars() {
        match c {
            '"' => dans = !dans,
            '\n' if !dans => {
                if !vide {
                    comptes.push(courant);
                }
                courant = [0; 4];
                vide = true;
                if comptes.len() == 20 {
                    break;
                }
                continue;
            }
            _ if !dans => {
                if let Some(k) = CANDIDATS.iter().position(|s| *s == c) {
                    courant[k] += 1;
                }
            }
            _ => {}
        }
        if c != '\r' {
            vide = false;
        }
    }
    if !vide && comptes.len() < 20 {
        comptes.push(courant);
    }
    let present = |k: usize| !comptes.is_empty() && comptes.iter().all(|l| l[k] > 0);
    let regulier = |k: usize| present(k) && comptes.iter().all(|l| l[k] == comptes[0][k]);
    (0..4).find(|k| regulier(*k)).or_else(|| (0..4).find(|k| present(*k))).or_else(|| (0..4).find(|k| comptes.iter().any(|l| l[*k] > 0))).map_or(',', |k| CANDIDATS[k])
}

pub fn lire(texte: &str) -> Vec<Vec<String>> {
    lire_avec(texte, separateur(texte))
}

pub fn lire_avec(texte: &str, sep: char) -> Vec<Vec<String>> {
    let mut lignes = Vec::new();
    let mut ligne: Vec<String> = Vec::new();
    let mut cellule = String::new();
    // `cite` : la cellule a commence par un guillemet ; `dans` : on est
    // encore entre ses guillemets.
    let (mut dans, mut cite) = (false, false);
    let mut car = texte.chars().peekable();
    while let Some(c) = car.next() {
        if dans {
            match c {
                '"' if car.peek() == Some(&'"') => {
                    car.next();
                    cellule.push('"');
                }
                '"' => dans = false,
                _ => cellule.push(c),
            }
            continue;
        }
        match c {
            '"' if cellule.is_empty() && !cite => {
                dans = true;
                cite = true;
            }
            c if c == sep => {
                ligne.push(std::mem::take(&mut cellule));
                cite = false;
            }
            '\r' if car.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                ligne.push(std::mem::take(&mut cellule));
                lignes.push(std::mem::take(&mut ligne));
                cite = false;
            }
            _ => cellule.push(c),
        }
    }
    if !cellule.is_empty() || cite || !ligne.is_empty() {
        ligne.push(cellule);
        lignes.push(ligne);
    }
    lignes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_separateur_se_devine() {
        assert_eq!(separateur("a,b,c\n1,2,3\n"), ',');
        assert_eq!(separateur("nom;prix\nStylo;1,5\nCahier;3\n"), ';');
        assert_eq!(separateur("nom\tprix\nStylo\t1,5\n"), '\t');
        // Des virgules partout, mais c'est le point-virgule qui separe.
        assert_eq!(separateur("a;b\n1,5;2,5\n"), ';');
        // Un separateur entre guillemets ne compte pas.
        assert_eq!(separateur("\"a;b\",c\n\"d;e;f\",g\n"), ',');
        assert_eq!(separateur("une seule colonne\nautre\n"), ',');
    }

    #[test]
    fn les_guillemets_protegent_separateur_retour_et_guillemet() {
        let lignes = lire("nom,note\n\"Dupont, Jean\",\"dit \"\"JD\"\"\"\n\"deux\nlignes\",\n,\"\"\n");
        assert_eq!(lignes, [vec!["nom", "note"], vec!["Dupont, Jean", "dit \"JD\""], vec!["deux\nlignes", ""], vec!["", ""]]);
    }

    #[test]
    fn un_bloc_copie_d_un_tableur_se_lit() {
        assert_eq!(lire("Stylo\t1,5\tvrai\r\nCahier\t3\t\r\n"), [["Stylo", "1,5", "vrai"], ["Cahier", "3", ""]]);
        // Sans retour final, et une seule cellule.
        assert_eq!(lire("a;b\nc;d"), [["a", "b"], ["c", "d"]]);
        assert_eq!(lire("seul"), [["seul"]]);
        assert!(lire("").is_empty());
    }

    #[test]
    fn les_encodages_d_excel_se_lisent() {
        assert_eq!(texte(b"\xEF\xBB\xBFa;\xC3\xA9"), "a;é");
        // Windows-1252 : « é » = E9, « € » = 80.
        assert_eq!(texte(b"caf\xE9;5\x80"), "café;5€");
        assert_eq!(texte(&[0xFF, 0xFE, b'a', 0, 0xE9, 0]), "aé");
        assert_eq!(texte(&[0xFE, 0xFF, 0, b'a', 0, 0xE9]), "aé");
    }
}
