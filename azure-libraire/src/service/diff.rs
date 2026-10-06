// Service `diff` : comparer deux textes (voir `back::texte::diff`).
use super::{Methode, Service, Valeur};
use crate::back::texte::diff::{self, Genre, Ligne};

pub static SERVICE: Service = Service {
    nom: "diff",
    description: "Comparer deux textes",
    methodes: &[
        Methode { nom: "lignes", description: "Chaque ligne avec son genre", arguments: "avant, apres", reponse: "[{ genre, texte, ancien, nouveau }]", appeler: lignes },
        Methode { nom: "blocs", description: "Les modifications et leur contexte", arguments: "avant, apres, contexte?", reponse: "{ ajoutees, retirees, blocs: [{ entete, lignes }] }", appeler: blocs },
        Methode { nom: "unifie", description: "Le diff au format unifié", arguments: "avant, apres, nom_avant?, nom_apres?, contexte?", reponse: "texte", appeler: unifie },
        Methode { nom: "mots", description: "Ce qui change dans une ligne", arguments: "avant, apres", reponse: "[{ genre, texte }]", appeler: mots },
    ],
};

fn genre(g: Genre) -> Valeur {
    Valeur::from(match g {
        Genre::Egal => "egal",
        Genre::Retire => "retire",
        Genre::Ajoute => "ajoute",
    })
}

fn ligne(l: &Ligne) -> Valeur {
    Valeur::table([("genre", genre(l.genre)), ("texte", l.texte.into()), ("ancien", l.ancien.into()), ("nouveau", l.nouveau.into())])
}

fn lignes(a: &Valeur) -> Result<Valeur, String> {
    Ok(Valeur::liste(diff::lignes(a.texte("avant")?, a.texte("apres")?).iter().map(ligne)))
}

fn blocs(a: &Valeur) -> Result<Valeur, String> {
    let lignes = diff::lignes(a.texte("avant")?, a.texte("apres")?);
    let (ajoutees, retirees) = diff::compte(&lignes);
    let blocs = diff::blocs(&lignes, a.entier_ou("contexte", 3).clamp(0, 1000) as usize);
    Ok(Valeur::table([
        ("ajoutees", ajoutees.into()),
        ("retirees", retirees.into()),
        ("blocs", Valeur::liste(blocs.iter().map(|b| Valeur::table([("entete", b.entete().into()), ("lignes", Valeur::liste(b.lignes.iter().map(ligne)))])))),
    ]))
}

fn unifie(a: &Valeur) -> Result<Valeur, String> {
    let contexte = a.entier_ou("contexte", 3).clamp(0, 1000) as usize;
    Ok(diff::unifie(a.texte_ou("nom_avant", "avant"), a.texte_ou("nom_apres", "apres"), a.texte("avant")?, a.texte("apres")?, contexte).into())
}

fn mots(a: &Valeur) -> Result<Valeur, String> {
    Ok(Valeur::liste(diff::mots(a.texte("avant")?, a.texte("apres")?).into_iter().map(|(g, t)| Valeur::table([("genre", genre(g)), ("texte", t.into())]))))
}
