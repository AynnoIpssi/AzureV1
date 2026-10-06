// Lire un fichier JavaScript / TypeScript (jsx, tsx compris) : les
// fonctions (`function f`, `const f = () =>`, methodes de classe), leurs
// appels (et les composants React utilises : `<Carte .../>`), les imports
// (`import ... from`, `require(...)`, `export ... from`).
use super::{mot_cle, Appel, Fichier, Fonction, Import, Langage};

#[derive(Clone, Debug, PartialEq)]
pub enum Jeton {
    Mot(String),
    Symbole(char),
    /// Le contenu d'un texte '...' ou "..." (pour les noms de modules).
    Texte(String),
    Litteral,
}

/// Les jetons et leur ligne.
pub fn jetons(source: &str) -> Vec<(Jeton, usize)> {
    let c: Vec<char> = source.chars().collect();
    let mut out: Vec<(Jeton, usize)> = Vec::new();
    let mut i = 0;
    let mut ligne = 1;
    // Profondeur d'accolades ou reprend chaque gabarit `...${ ... }...`.
    let mut gabarits: Vec<i32> = Vec::new();
    let mut prof = 0i32;
    // Lit un gabarit depuis `i` (apres ` ou }) ; rend true s'il s'arrete sur `${`.
    fn gabarit(c: &[char], i: &mut usize, ligne: &mut usize) -> bool {
        while *i < c.len() {
            match c[*i] {
                '\\' => *i += 2,
                '`' => {
                    *i += 1;
                    return false;
                }
                '$' if c.get(*i + 1) == Some(&'{') => {
                    *i += 2;
                    return true;
                }
                '\n' => {
                    *ligne += 1;
                    *i += 1;
                }
                _ => *i += 1,
            }
        }
        false
    }
    while i < c.len() {
        let x = c[i];
        if x == '\n' {
            ligne += 1;
            i += 1;
        } else if x.is_whitespace() {
            i += 1;
        } else if x == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if x == '/' && c.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < c.len() && !(c[i] == '*' && c[i + 1] == '/') {
                if c[i] == '\n' {
                    ligne += 1;
                }
                i += 1;
            }
            i += 2;
        } else if x == '\'' || x == '"' {
            // Un texte s'arrete a la fin de la ligne : une apostrophe dans du
            // texte JSX (« l'app ») ne mange pas la suite du fichier.
            let mut j = i + 1;
            let mut s = String::new();
            while j < c.len() && c[j] != x && c[j] != '\n' {
                if c[j] == '\\' {
                    j += 1;
                }
                if let Some(&y) = c.get(j) {
                    s.push(y);
                }
                j += 1;
            }
            if c.get(j) == Some(&x) {
                out.push((Jeton::Texte(s), ligne));
                i = j + 1;
            } else {
                i += 1;
            }
        } else if x == '`' {
            i += 1;
            out.push((Jeton::Litteral, ligne));
            if gabarit(&c, &mut i, &mut ligne) {
                gabarits.push(prof);
                prof += 1;
            }
        } else if x == '}' && gabarits.last() == Some(&(prof - 1)) {
            prof -= 1;
            gabarits.pop();
            i += 1;
            if gabarit(&c, &mut i, &mut ligne) {
                gabarits.push(prof);
                prof += 1;
            }
        } else if x == '/' && regex_possible(out.last().map(|t| &t.0)) {
            i += 1;
            let mut classe = false;
            while i < c.len() && c[i] != '\n' {
                match c[i] {
                    '\\' => i += 1,
                    '[' => classe = true,
                    ']' => classe = false,
                    '/' if !classe => break,
                    _ => {}
                }
                i += 1;
            }
            i += 1;
            while i < c.len() && c[i].is_alphabetic() {
                i += 1;
            }
            out.push((Jeton::Litteral, ligne));
        } else if x.is_ascii_digit() {
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '.' || c[i] == '_') {
                i += 1;
            }
            out.push((Jeton::Litteral, ligne));
        } else if x.is_alphabetic() || x == '_' || x == '$' {
            let debut = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '$') {
                i += 1;
            }
            out.push((Jeton::Mot(c[debut..i].iter().collect()), ligne));
        } else {
            if x == '{' {
                prof += 1;
            } else if x == '}' {
                prof -= 1;
            }
            out.push((Jeton::Symbole(x), ligne));
            i += 1;
        }
    }
    out
}

// Un `/` ici commence-t-il une expression reguliere (et pas une division) ?
fn regex_possible(avant: Option<&Jeton>) -> bool {
    match avant {
        None => true,
        Some(Jeton::Symbole(s)) => "(,=:[!&|?{};+-*%<>~^".contains(*s),
        Some(Jeton::Mot(m)) => matches!(m.as_str(), "return" | "typeof" | "case" | "do" | "else" | "in" | "of" | "new" | "delete" | "void" | "throw" | "yield" | "await"),
        _ => false,
    }
}

#[derive(Clone, Debug)]
enum Bloc {
    Classe(String),
    Fonction(usize),
    Autre,
}

struct Lecteur {
    j: Vec<(Jeton, usize)>,
}

impl Lecteur {
    fn mot_a(&self, k: usize) -> Option<&str> {
        match self.j.get(k) {
            Some((Jeton::Mot(m), _)) => Some(m),
            _ => None,
        }
    }

    fn texte_a(&self, k: usize) -> Option<&str> {
        match self.j.get(k) {
            Some((Jeton::Texte(m), _)) => Some(m),
            _ => None,
        }
    }

    fn symbole_a(&self, k: usize, s: char) -> bool {
        matches!(self.j.get(k), Some((Jeton::Symbole(x), _)) if *x == s)
    }

    fn ligne(&self, k: usize) -> usize {
        self.j.get(k).map_or(0, |x| x.1)
    }

    fn fermant(&self, k: usize) -> usize {
        let mut prof = 0i32;
        for i in k..self.j.len() {
            if let (Jeton::Symbole(s), _) = &self.j[i] {
                match s {
                    '{' | '(' | '[' => prof += 1,
                    '}' | ')' | ']' => {
                        prof -= 1;
                        if prof == 0 {
                            return i;
                        }
                    }
                    _ => {}
                }
            }
        }
        self.j.len().saturating_sub(1)
    }

    /// Apres `(params)` en `k` : `=>` (fleche) ? Rend l'indice apres `=>`.
    fn fleche_apres(&self, k: usize) -> Option<usize> {
        let mut k = k;
        // TS : `(): Type =>`
        if self.symbole_a(k, ':') {
            while k < self.j.len() && !(self.symbole_a(k, '=') && self.symbole_a(k + 1, '>')) && !self.symbole_a(k, ';') && !self.symbole_a(k, '{') {
                k += 1;
            }
        }
        (self.symbole_a(k, '=') && self.symbole_a(k + 1, '>')).then_some(k + 2)
    }

    /// La fin d'un corps de fleche sans accolades : `;`, `,` ou fermante au
    /// niveau 0, ou un nouveau `const`/`function`/`export`.
    fn fin_expression(&self, k: usize) -> usize {
        let mut prof = 0;
        let mut i = k;
        while i < self.j.len() {
            match &self.j[i].0 {
                Jeton::Symbole('(' | '[' | '{') => prof += 1,
                Jeton::Symbole(')' | ']' | '}') => {
                    if prof == 0 {
                        return i;
                    }
                    prof -= 1;
                }
                Jeton::Symbole(';' | ',') if prof == 0 => return i,
                Jeton::Mot(m) if prof == 0 && i > k && matches!(m.as_str(), "const" | "let" | "var" | "function" | "export" | "class" | "import") => return i,
                _ => {}
            }
            i += 1;
        }
        i
    }
}

/// Lit le fichier `chemin` (relatif) de contenu `source`.
pub fn lire(chemin: &str, source: &str) -> Fichier {
    let l = Lecteur { j: jetons(source) };
    let n = l.j.len();
    let mut f = Fichier { chemin: chemin.to_string(), langage: Langage::Js, lignes: source.lines().count(), fonctions: Vec::new(), imports: Vec::new(), modules: Vec::new(), types: Vec::new(), test: super::est_test(chemin) };
    let mut pile: Vec<Bloc> = Vec::new();
    let mut attendu: Option<Bloc> = None;
    let courante = |pile: &[Bloc]| pile.iter().rev().find_map(|b| if let Bloc::Fonction(k) = b { Some(*k) } else { None });
    let stem = chemin.rsplit('/').next().unwrap_or(chemin).split('.').next().unwrap_or("").to_string();
    let mut i = 0;
    while i < n {
        match &l.j[i].0 {
            Jeton::Symbole('{') => {
                pile.push(attendu.take().unwrap_or(Bloc::Autre));
                i += 1;
            }
            Jeton::Symbole('}') => {
                if let Some(Bloc::Fonction(k)) = pile.pop() {
                    f.fonctions[k].fin = l.ligne(i);
                }
                i += 1;
            }
            // `a < B` est une comparaison ; apres `return`, `(`, `>`... c'est du JSX.
            Jeton::Symbole('<') if l.mot_a(i + 1).is_some_and(|m| m.starts_with(|c: char| c.is_uppercase())) && match l.j.get(i.wrapping_sub(1)).map(|t| &t.0) {
                Some(Jeton::Mot(m)) => matches!(m.as_str(), "return" | "yield" | "default" | "case" | "await"),
                Some(Jeton::Litteral | Jeton::Texte(_)) | Some(Jeton::Symbole(')' | ']')) => false,
                _ => true,
            } => {
                // JSX : <Composant ...> utilise le composant.
                if let Some(k) = courante(&pile) {
                    f.fonctions[k].appels.push(Appel { nom: l.mot_a(i + 1).unwrap().to_string(), chemin: Vec::new(), methode: false, ligne: l.ligne(i) });
                }
                i += 2;
            }
            Jeton::Mot(m) => {
                let m = m.as_str();
                let dans_fonction = courante(&pile).is_some();
                match m {
                    "import" if !l.symbole_a(i + 1, '(') && !l.symbole_a(i.wrapping_sub(1), '.') => {
                        i = lire_import(&l, i + 1, &mut f.imports);
                    }
                    "export" if l.symbole_a(i + 1, '*') || l.symbole_a(i + 1, '{') => {
                        // export * from 'm' / export { a } from 'm'
                        let mut k = i + 1;
                        while k < n && !l.symbole_a(k, ';') && l.mot_a(k) != Some("from") && !(l.symbole_a(k, '}') && l.mot_a(k + 1) != Some("from")) {
                            k += 1;
                        }
                        if l.mot_a(k) == Some("from") {
                            i = lire_import(&l, i + 1, &mut f.imports);
                        } else {
                            i = k + 1;
                        }
                    }
                    "require" | "import" if l.symbole_a(i + 1, '(') && l.texte_a(i + 2).is_some() => {
                        let module = l.texte_a(i + 2).unwrap().to_string();
                        // const { a, b } = require('m') / const x = require('m')
                        let mut noms = Vec::new();
                        if i >= 2 && l.symbole_a(i - 1, '=') {
                            if l.symbole_a(i - 2, '}') {
                                let mut k = i - 3;
                                while k > 0 && !l.symbole_a(k, '{') {
                                    if let Some(nm) = l.mot_a(k) {
                                        noms.push((nm.to_string(), nm.to_string()));
                                    }
                                    k -= 1;
                                }
                            } else if let Some(nm) = l.mot_a(i - 2) {
                                noms.push(("*".to_string(), nm.to_string()));
                            }
                        }
                        if noms.is_empty() {
                            noms.push((String::new(), String::new()));
                        }
                        for (nom, local) in noms {
                            f.imports.push(Import::Js { module: module.clone(), nom, local });
                        }
                        i += 3;
                    }
                    "class" if l.mot_a(i + 1).is_some() => {
                        let nom = l.mot_a(i + 1).unwrap().to_string();
                        f.types.push(nom.clone());
                        let mut k = i + 2;
                        while k < n && !l.symbole_a(k, '{') {
                            k += 1;
                        }
                        attendu = Some(Bloc::Classe(nom));
                        i = k;
                    }
                    "function" if !dans_fonction => {
                        let mut k = i + 1;
                        if l.symbole_a(k, '*') {
                            k += 1;
                        }
                        let defaut = l.mot_a(i.wrapping_sub(1)) == Some("default") || (l.mot_a(i.wrapping_sub(1)) == Some("async") && l.mot_a(i.wrapping_sub(2)) == Some("default"));
                        let nom = match l.mot_a(k) {
                            Some(nm) => {
                                k += 1;
                                nm.to_string()
                            }
                            None if defaut => stem.clone(),
                            None => {
                                i += 1;
                                continue;
                            }
                        };
                        if !l.symbole_a(k, '(') && !l.symbole_a(k, '<') {
                            i += 1;
                            continue;
                        }
                        while k < n && !l.symbole_a(k, '(') {
                            k += 1;
                        }
                        let apres = l.fermant(k) + 1;
                        let mut b = apres;
                        while b < n && !l.symbole_a(b, '{') && !l.symbole_a(b, ';') {
                            b += 1;
                        }
                        f.fonctions.push(Fonction { nom, proprietaire: String::new(), ligne: l.ligne(i), fin: l.ligne(i), appels: Vec::new(), defaut });
                        if l.symbole_a(b, '{') {
                            attendu = Some(Bloc::Fonction(f.fonctions.len() - 1));
                        } else {
                            f.fonctions.pop(); // declaration TS sans corps
                        }
                        i = b;
                    }
                    "const" | "let" | "var" if !dans_fonction && l.mot_a(i + 1).is_some() && (l.symbole_a(i + 2, '=') || l.symbole_a(i + 2, ':')) => {
                        let nom = l.mot_a(i + 1).unwrap().to_string();
                        let mut k = i + 2;
                        // TS : const f: Type = ...
                        if l.symbole_a(k, ':') {
                            while k < n && !(l.symbole_a(k, '=') && !l.symbole_a(k + 1, '>')) && !l.symbole_a(k, ';') {
                                k += 1;
                            }
                        }
                        if !l.symbole_a(k, '=') {
                            i += 1;
                            continue;
                        }
                        k += 1;
                        if l.mot_a(k) == Some("async") {
                            k += 1;
                        }
                        let defaut = false;
                        // = function (...) { ... }
                        if l.mot_a(k) == Some("function") {
                            let mut p = k + 1;
                            while p < n && !l.symbole_a(p, '(') {
                                p += 1;
                            }
                            let mut b = l.fermant(p) + 1;
                            while b < n && !l.symbole_a(b, '{') {
                                b += 1;
                            }
                            f.fonctions.push(Fonction { nom, proprietaire: String::new(), ligne: l.ligne(i), fin: l.ligne(i), appels: Vec::new(), defaut });
                            attendu = Some(Bloc::Fonction(f.fonctions.len() - 1));
                            i = b;
                            continue;
                        }
                        // = (...) => / = x =>
                        let apres_params = if l.symbole_a(k, '(') {
                            Some(l.fermant(k) + 1)
                        } else if l.mot_a(k).is_some() && l.symbole_a(k + 1, '=') && l.symbole_a(k + 2, '>') {
                            Some(k + 1)
                        } else if l.symbole_a(k, '<') {
                            // TS generique : <T>(x: T) =>
                            let mut p = k;
                            while p < n && !l.symbole_a(p, '(') {
                                p += 1;
                            }
                            Some(l.fermant(p) + 1)
                        } else {
                            None
                        };
                        let Some(corps) = apres_params.and_then(|p| l.fleche_apres(p)) else {
                            i += 1;
                            continue;
                        };
                        f.fonctions.push(Fonction { nom, proprietaire: String::new(), ligne: l.ligne(i), fin: l.ligne(i), appels: Vec::new(), defaut });
                        let idx = f.fonctions.len() - 1;
                        if l.symbole_a(corps, '{') {
                            attendu = Some(Bloc::Fonction(idx));
                            i = corps;
                        } else {
                            let fin = l.fin_expression(corps);
                            for p in corps..fin {
                                appel(&l, p, &mut f.fonctions[idx].appels);
                            }
                            f.fonctions[idx].fin = l.ligne(fin.saturating_sub(1));
                            i = fin;
                        }
                    }
                    _ if matches!(pile.last(), Some(Bloc::Classe(_))) && l.symbole_a(i + 1, '(') && !mot_cle(m) => {
                        // Methode de classe : nom(...) { ... }
                        let proprio = if let Some(Bloc::Classe(c)) = pile.last() { c.clone() } else { String::new() };
                        let mut b = l.fermant(i + 1) + 1;
                        while b < n && !l.symbole_a(b, '{') && !l.symbole_a(b, ';') {
                            b += 1;
                        }
                        f.fonctions.push(Fonction { nom: m.to_string(), proprietaire: proprio, ligne: l.ligne(i), fin: l.ligne(i), appels: Vec::new(), defaut: false });
                        if l.symbole_a(b, '{') {
                            attendu = Some(Bloc::Fonction(f.fonctions.len() - 1));
                        } else {
                            f.fonctions.pop();
                        }
                        i = b;
                    }
                    _ if matches!(pile.last(), Some(Bloc::Classe(_))) && l.symbole_a(i + 1, '=') && !l.symbole_a(i + 2, '=') => {
                        // Champ fleche : nom = (...) => { ... }
                        let proprio = if let Some(Bloc::Classe(c)) = pile.last() { c.clone() } else { String::new() };
                        let mut k = i + 2;
                        if l.mot_a(k) == Some("async") {
                            k += 1;
                        }
                        let corps = if l.symbole_a(k, '(') { l.fleche_apres(l.fermant(k) + 1) } else { None };
                        match corps {
                            Some(c) if l.symbole_a(c, '{') => {
                                f.fonctions.push(Fonction { nom: m.to_string(), proprietaire: proprio, ligne: l.ligne(i), fin: l.ligne(i), appels: Vec::new(), defaut: false });
                                attendu = Some(Bloc::Fonction(f.fonctions.len() - 1));
                                i = c;
                            }
                            _ => i += 1,
                        }
                    }
                    _ => {
                        if let Some(k) = courante(&pile) {
                            appel(&l, i, &mut f.fonctions[k].appels);
                        }
                        i += 1;
                    }
                }
            }
            _ => i += 1,
        }
    }
    // export default Nom; : la fonction Nom est l'export par defaut.
    for k in 0..n {
        if l.mot_a(k) == Some("export") && l.mot_a(k + 1) == Some("default")
            && let Some(nom) = l.mot_a(k + 2)
            && let Some(fct) = f.fonctions.iter_mut().find(|x| x.nom == nom && x.proprietaire.is_empty())
        {
            fct.defaut = true;
        }
    }
    f
}

// `import ...` a partir du jeton apres `import` (ou `export`) ; rend l'indice suivant.
fn lire_import(l: &Lecteur, debut: usize, out: &mut Vec<Import>) -> usize {
    let n = l.j.len();
    // import 'm';
    if let Some(m) = l.texte_a(debut) {
        out.push(Import::Js { module: m.to_string(), nom: String::new(), local: String::new() });
        return debut + 1;
    }
    let mut k = debut;
    let mut noms: Vec<(String, String)> = Vec::new();
    if l.mot_a(k) == Some("type") {
        k += 1;
    }
    while k < n && l.mot_a(k) != Some("from") && !l.symbole_a(k, ';') {
        if l.symbole_a(k, '*') {
            // * as ns  (export * : pas de nom local)
            if l.mot_a(k + 1) == Some("as") {
                noms.push(("*".into(), l.mot_a(k + 2).unwrap_or("").to_string()));
                k += 3;
            } else {
                noms.push(("*".into(), String::new()));
                k += 1;
            }
        } else if l.symbole_a(k, '{') {
            let fin = l.fermant(k);
            let mut p = k + 1;
            while p < fin {
                if l.mot_a(p) == Some("type") && l.mot_a(p + 1).is_some() && l.mot_a(p + 1) != Some("as") {
                    p += 1;
                }
                if let Some(nom) = l.mot_a(p) {
                    let local = if l.mot_a(p + 1) == Some("as") { l.mot_a(p + 2).unwrap_or(nom).to_string() } else { nom.to_string() };
                    noms.push((nom.to_string(), local));
                    p += if l.mot_a(p + 1) == Some("as") { 3 } else { 1 };
                } else {
                    p += 1;
                }
            }
            k = fin + 1;
        } else if let Some(nom) = l.mot_a(k) {
            noms.push(("default".into(), nom.to_string()));
            k += 1;
        } else {
            k += 1;
        }
    }
    if l.mot_a(k) == Some("from")
        && let Some(m) = l.texte_a(k + 1)
    {
        if noms.is_empty() {
            noms.push((String::new(), String::new()));
        }
        for (nom, local) in noms {
            out.push(Import::Js { module: m.to_string(), nom, local });
        }
        return k + 2;
    }
    k + 1
}

// Le mot en `i` est-il un appel ? `f(`, `x.f(`, `ns.f(`.
fn appel(l: &Lecteur, i: usize, out: &mut Vec<Appel>) {
    let Some(nom) = l.mot_a(i) else { return };
    if mot_cle(nom) || !l.symbole_a(i + 1, '(') {
        return;
    }
    if l.mot_a(i.wrapping_sub(1)) == Some("function") || l.mot_a(i.wrapping_sub(1)) == Some("new") {
        return;
    }
    let methode = i > 0 && l.symbole_a(i - 1, '.');
    let chemin = if methode && i >= 2 { l.mot_a(i - 2).filter(|m| *m != "this").map(|m| vec![m.to_string()]).unwrap_or_default() } else { Vec::new() };
    let this = methode && i >= 2 && l.mot_a(i - 2) == Some("this");
    out.push(Appel { nom: nom.to_string(), chemin: if this { vec!["this".into()] } else { chemin }, methode: methode && !this, ligne: l.ligne(i) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonctions_appels_et_imports() {
        let source = r#"
import React, { useState as etat } from 'react';
import * as outils from './outils';
import Carte from "../composants/Carte";
const { formater } = require('./format');
export { aide } from './aide';
// function cachee() {}
const re = /a\/b{2}/g;
export default function App() {
  const [n, setN] = etat(0);
  const clic = () => setN(n + 1);
  const t = `total ${formater(n)} € l'app`;
  return <div onClick={clic}><Carte titre="l'accueil" />{outils.somme(1, 2)}</div>;
}
const double = (x) => calcul(x) * 2;
async function charger(url: string): Promise<void> { await fetch(url); }
class Panier {
  ajouter(p) { this.vider(); total(p); }
  vider = () => { this.items = []; }
}
"#;
        let f = lire("src/App.jsx", source);
        let noms: Vec<String> = f.fonctions.iter().map(|x| x.nom_complet()).collect();
        assert_eq!(noms, ["App", "double", "charger", "Panier::ajouter", "Panier::vider"]);
        assert!(f.fonctions[0].defaut);
        let appels: Vec<String> = f.fonctions[0].appels.iter().map(|a| format!("{}{}{}", a.chemin.join("."), if a.methode { "." } else { ":" }, a.nom)).collect();
        assert_eq!(appels, [":etat", ":setN", ":formater", ":Carte", "outils.somme"]);
        assert_eq!(f.fonctions[1].appels[0].nom, "calcul");
        assert_eq!(f.fonctions[3].appels.iter().map(|a| format!("{}:{}", a.chemin.join("."), a.nom)).collect::<Vec<_>>(), ["this:vider", ":total"]);
        let imports: Vec<String> = f.imports.iter().map(|i| match i {
            Import::Js { module, nom, local } => format!("{module}:{nom}={local}"),
            _ => String::new(),
        }).collect();
        assert_eq!(imports, ["react:default=React", "react:useState=etat", "./outils:*=outils", "../composants/Carte:default=Carte", "./format:formater=formater", "./aide:aide=aide"]);
    }
}
