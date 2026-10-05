// Le dialogue entre un essai et l'app qu'il pilote : une ligne par commande
// (`clic\tvalider`), une ligne par reponse (`ok`, `ok\t<donnees>` ou
// `erreur\t<message>`). Champs separes par des tabulations ; `\`, tabulation
// et retour a la ligne echappes.

pub fn echapper(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
}

pub fn desechapper(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut c = s.chars();
    while let Some(x) = c.next() {
        if x != '\\' {
            out.push(x);
            continue;
        }
        match c.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some(y) => out.push(y),
            None => out.push('\\'),
        }
    }
    out
}

pub fn ligne(champs: &[&str]) -> String {
    champs.iter().map(|c| echapper(c)).collect::<Vec<_>>().join("\t")
}

pub fn champs(ligne: &str) -> Vec<String> {
    ligne.trim_end_matches(['\n', '\r']).split('\t').map(desechapper).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour() {
        let c = ["remplir", "nom", "a\tb\nc\\d"];
        assert_eq!(champs(&ligne(&c)), c);
    }
}
