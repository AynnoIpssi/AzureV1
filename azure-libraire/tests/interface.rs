// Le catalogue des modules d'interface et les themes.
use azure_libraire::interface::theme::{self, Theme};
use azure_libraire::interface::{categories, module, modules, styles};

#[test]
fn le_catalogue_est_coherent() {
    let mut noms: Vec<&str> = modules().iter().map(|m| m.nom).collect();
    assert!(noms.len() >= 97, "{} modules", noms.len());
    noms.sort();
    let avant = noms.len();
    noms.dedup();
    assert_eq!(noms.len(), avant, "un nom de module en double");
    for m in modules() {
        assert!(!m.rsh.trim().is_empty() && !m.resume.is_empty(), "<{}>", m.nom);
    }
    assert_eq!(categories(), ["application", "mise_en_page", "contenu", "messages", "navigation", "formulaire", "graphes", "couches"]);
    assert_eq!(module("card").unwrap().categorie, "contenu");
    assert!(module("inconnu").is_none());
}

#[test]
fn chaque_theme_definit_tous_les_jetons() {
    let reference: Vec<String> = Theme::integre(theme::DEFAUT).unwrap().jetons().map(|(k, _)| k.to_string()).collect();
    for nom in theme::integres() {
        let t = Theme::integre(nom).unwrap();
        assert_eq!(t.nom, nom);
        assert_eq!(t.jetons().map(|(k, _)| k.to_string()).collect::<Vec<_>>(), reference, "theme {nom}");
        // Plus aucun jeton dans la feuille une fois le theme applique.
        let feuille = t.appliquer(styles());
        let reste: Vec<&str> = feuille.lines().filter(|l| l.contains('$')).collect();
        assert!(reste.is_empty(), "theme {nom} : jetons inconnus\n{reste:#?}");
    }
    assert!(!Theme::integre("ivoire").unwrap().sombre);
}

#[test]
fn les_styles_des_modules_n_ecrivent_pas_de_couleur_en_dur() {
    // Seule exception : les couleurs de texte de la barre du texte riche,
    // qui sont celles qu'elle applique au texte.
    for m in modules().iter().filter(|m| m.nom != "richbar") {
        assert!(!m.rsc.contains('#') && !m.rsc.contains("rgb"), "<{}> : couleur en dur\n{}", m.nom, m.rsc);
    }
}

#[test]
fn appliquer_remplace_les_jetons() {
    let t = Theme::integre("sable").unwrap();
    assert_eq!(t.appliquer("a { color: $accent; border: 1px solid $trait/6; border-radius: $rayon-grand; }"), "a { color: #c9a878; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 12px; }");
    assert_eq!(t.appliquer("$accent/100 $accent/5,$surface-2;"), "rgba(201, 168, 120, 1.00) rgba(201, 168, 120, 0.05),#171615;");
    // Inconnu, ou pas une couleur : laisse tel quel.
    assert_eq!(t.appliquer("$inconnu $ $rayon/50"), "$inconnu $ 8px/50");
}

#[test]
fn un_theme_d_app_part_d_un_theme_fourni() {
    let t = Theme::lire("# Mon theme\nnom = menthe\nbase = ardoise\naccent = #7fd1b9\n").unwrap();
    assert_eq!((t.nom.as_str(), t.titre.as_str(), t.sombre), ("menthe", "menthe", true));
    assert_eq!(t.jeton("accent"), Some("#7fd1b9"));
    assert_eq!(t.jeton("surface"), Theme::integre("ardoise").unwrap().jeton("surface"));
    assert!(Theme::lire("base = rien").is_err());
    assert!(Theme::lire("Accent = #fff").is_err());
    assert!(Theme::lire("accent").is_err());
    assert_eq!(Theme::integre("sable").unwrap().avec("accent", "#fff").appliquer("$accent/50"), "rgba(255, 255, 255, 0.50)");
}

#[test]
fn changer_de_theme_change_la_version() {
    let avant = theme::version();
    theme::choisir("ivoire").unwrap();
    assert_eq!(theme::actif().nom, "ivoire");
    assert_eq!(theme::version(), avant + 1);
    theme::choisir("ivoire").unwrap();
    assert_eq!(theme::version(), avant + 1, "meme theme : rien a refaire");
    assert!(theme::choisir("rien").is_err());
    theme::definir(Theme::integre("sable").unwrap());
    assert_eq!(theme::actif().nom, "sable");
}
