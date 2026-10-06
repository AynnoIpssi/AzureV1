// Service `code` : colorer un code, en tirer les fonctions (voir
// `back::code`).
use super::{Methode, Service, Valeur};
use crate::back::code::{coloration, js, rust, Langage};

pub static SERVICE: Service = Service {
    nom: "code",
    description: "Lire du code source",
    methodes: &[
        Methode { nom: "colorer", description: "Le code en morceaux à colorer", arguments: "langage (rsh, rsc, rss, rust, toml, sh), code", reponse: "[[{ genre, texte }]] (une liste par ligne)", appeler: colorer },
        Methode { nom: "fonctions", description: "Les fonctions d'un fichier et leurs appels", arguments: "chemin (son extension dit le langage : .rs, .js, .ts…), source", reponse: "{ langage, lignes, test, types, fonctions: [{ nom, proprietaire, ligne, fin, appels: [{ nom, chemin, methode, ligne }] }] }", appeler: fonctions },
    ],
};

fn colorer(a: &Valeur) -> Result<Valeur, String> {
    let lignes = coloration::colorer(a.texte("langage")?, a.texte("code")?);
    Ok(Valeur::liste(lignes.into_iter().map(|l| Valeur::liste(l.into_iter().map(|(genre, texte)| Valeur::table([("genre", genre.into()), ("texte", texte.into())]))))))
}

fn fonctions(a: &Valeur) -> Result<Valeur, String> {
    let (chemin, source) = (a.texte("chemin")?, a.texte("source")?);
    let extension = chemin.rsplit_once('.').map_or("", |(_, e)| e);
    let langage = Langage::depuis_extension(extension).ok_or_else(|| format!("langage non lu : « {chemin} » (.rs, .js, .jsx, .ts, .tsx…)"))?;
    let f = match langage {
        Langage::Rust => rust::lire(chemin, source),
        Langage::Js => js::lire(chemin, source),
    };
    let fonctions = f.fonctions.iter().map(|g| {
        let appels = g.appels.iter().map(|c| Valeur::table([("nom", c.nom.clone().into()), ("chemin", c.chemin.clone().into()), ("methode", c.methode.into()), ("ligne", c.ligne.into())]));
        Valeur::table([("nom", g.nom.clone().into()), ("proprietaire", g.proprietaire.clone().into()), ("ligne", g.ligne.into()), ("fin", g.fin.into()), ("appels", Valeur::liste(appels))])
    });
    Ok(Valeur::table([("langage", langage.nom().into()), ("lignes", f.lignes.into()), ("test", f.test.into()), ("types", f.types.clone().into()), ("fonctions", Valeur::liste(fonctions))]))
}
