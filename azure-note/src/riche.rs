// Le texte riche d'un bloc : des segments, chacun avec son style.
//
// Range en texte : segments separes par SEP_LIGNE, champs par SEP :
//
//   <styles> SEP <couleur> SEP <lien> SEP <texte>
//
// styles = lettres g (gras) i (italique) s (souligne) b (barre) c (code),
// puis t<taille> pour une taille de police en px (`gt18`) ;
// couleur = "#rrggbb" ou vide ; lien = id de page ou vide. Un texte sans
// ces separateurs (notes v1, texte colle) est un seul segment sans style.
use crate::modele::{SEP, SEP_LIGNE};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    pub gras: bool,
    pub italique: bool,
    pub souligne: bool,
    pub barre: bool,
    pub code: bool,
    pub couleur: String,
    pub lien: Option<i64>,
    /// Taille de police en px (« 18 ») ; vide : celle du bloc.
    pub taille: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Segment {
    pub texte: String,
    pub style: Style,
}

impl Segment {
    pub fn simple(texte: &str) -> Segment {
        Segment { texte: texte.to_string(), style: Style::default() }
    }
}

pub fn lire(s: &str) -> Vec<Segment> {
    if !s.contains(SEP) {
        return if s.is_empty() { Vec::new() } else { vec![Segment::simple(s)] };
    }
    s.split(SEP_LIGNE)
        .filter_map(|seg| {
            let mut champs = seg.splitn(4, SEP);
            let (styles, couleur, lien, texte) = (champs.next()?, champs.next()?, champs.next()?, champs.next()?);
            let (lettres, taille) = styles.split_once('t').unwrap_or((styles, ""));
            let a = |c: char| lettres.contains(c);
            let style = Style { gras: a('g'), italique: a('i'), souligne: a('s'), barre: a('b'), code: a('c'), couleur: couleur.to_string(), lien: lien.parse().ok(), taille: taille.to_string() };
            Some(Segment { texte: texte.to_string(), style })
        })
        .collect()
}

pub fn ecrire(segments: &[Segment]) -> String {
    let segments = fusionner(segments);
    // Sans aucun style : le texte tel quel.
    if segments.len() <= 1 && segments.iter().all(|s| s.style == Style::default()) {
        return segments.first().map(|s| s.texte.clone()).unwrap_or_default();
    }
    segments
        .iter()
        .map(|seg| {
            let s = &seg.style;
            let mut styles: String = [(s.gras, 'g'), (s.italique, 'i'), (s.souligne, 's'), (s.barre, 'b'), (s.code, 'c')].iter().filter(|x| x.0).map(|x| x.1).collect();
            if !s.taille.is_empty() {
                styles.push('t');
                styles.push_str(&s.taille);
            }
            let lien = s.lien.map(|l| l.to_string()).unwrap_or_default();
            [styles.as_str(), &s.couleur, &lien, &seg.texte].join(&SEP.to_string())
        })
        .collect::<Vec<_>>()
        .join(&SEP_LIGNE.to_string())
}

/// Le texte sans style (recherche, apercu).
pub fn brut(s: &str) -> String {
    lire(s).iter().map(|x| x.texte.as_str()).collect()
}

/// Coupe le texte riche au caractere `i` (du texte brut) : avant, apres.
pub fn couper(s: &str, i: usize) -> (String, String) {
    let (mut avant, mut apres) = (Vec::new(), Vec::new());
    let mut n = 0;
    for seg in lire(s) {
        let len = seg.texte.chars().count();
        if n + len <= i {
            avant.push(seg);
        } else if n >= i {
            apres.push(seg);
        } else {
            let k = i - n;
            avant.push(Segment { texte: seg.texte.chars().take(k).collect(), style: seg.style.clone() });
            apres.push(Segment { texte: seg.texte.chars().skip(k).collect(), style: seg.style });
        }
        n += len;
    }
    (ecrire(&avant), ecrire(&apres))
}

/// Retire le dernier `/` (celui tape pour ouvrir le menu des blocs).
pub fn sans_slash(s: &str) -> String {
    let Some(i) = brut(s).chars().collect::<Vec<_>>().iter().rposition(|c| *c == '/') else { return s.to_string() };
    let (avant, apres) = couper(s, i);
    let (_, apres) = couper(&apres, 1);
    let mut segs = lire(&avant);
    segs.extend(lire(&apres));
    ecrire(&segs)
}

/// Colle les segments voisins de meme style ; retire les vides.
pub fn fusionner(segments: &[Segment]) -> Vec<Segment> {
    let mut v: Vec<Segment> = Vec::new();
    for s in segments.iter().filter(|s| !s.texte.is_empty()) {
        match v.last_mut() {
            Some(d) if d.style == s.style => d.texte.push_str(&s.texte),
            _ => v.push(s.clone()),
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour() {
        let segs = vec![
            Segment::simple("Un "),
            Segment { texte: "mot".into(), style: Style { gras: true, couleur: "#e06c75".into(), ..Style::default() } },
            Segment { texte: " lié".into(), style: Style { souligne: true, lien: Some(4), ..Style::default() } },
            Segment { texte: " grand".into(), style: Style { gras: true, taille: "20".into(), ..Style::default() } },
        ];
        let s = ecrire(&segs);
        assert_eq!(lire(&s), segs);
        // Le meme format que `<richtext>` de la fondation.
        let f = azure_foundation::ui::models::rich::parse(&s);
        assert_eq!((f[3].style.size, f[3].style.bold), (Some(20.0), true));
        assert_eq!(azure_foundation::ui::models::rich::serialize(&f), s);
        assert_eq!(brut(&s), "Un mot lié grand");
    }

    #[test]
    fn couper_et_slash() {
        let segs = vec![Segment::simple("Un "), Segment { texte: "mot".into(), style: Style { gras: true, ..Style::default() } }];
        let (a, b) = couper(&ecrire(&segs), 4);
        assert_eq!((brut(&a), brut(&b)), ("Un m".to_string(), "ot".to_string()));
        assert!(lire(&b)[0].style.gras && lire(&a)[1].style.gras);
        assert_eq!(couper("abc", 0), (String::new(), "abc".to_string()));
        assert_eq!(sans_slash("a/b /"), "a/b ");
        assert_eq!(sans_slash("/"), "");
        assert_eq!(sans_slash("rien"), "rien");
    }

    #[test]
    fn texte_simple_et_fusion() {
        assert_eq!(ecrire(&[Segment::simple("a"), Segment::simple("b")]), "ab");
        assert_eq!(lire("note v1\nligne 2"), vec![Segment::simple("note v1\nligne 2")]);
        assert!(lire("").is_empty());
        assert_eq!(ecrire(&[]), "");
    }
}
