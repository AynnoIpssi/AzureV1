// Ce qu'Azure Docs sert aux autres apps (`[provide ...]` dans app.azure).
// Azure Note s'en sert pour chercher dans la doc et l'afficher dans sa
// propre petite fenetre.
//
//   chercher  { q }                   -> [{ genre, id, titre, lieu, extrait, chemin }]
//   page      { chemin } ou { exemple } -> { chemin, titre, resume, section, blocs, exemple }
//
// Les blocs ont la forme de ceux du gabarit (voir `ecrans::bloc`) ; les
// apercus et les demonstrations, propres a l'app, n'y sont pas.
use crate::contenu::{Bloc, Docs};
use crate::ecrans::{bloc, cle};
use crate::index::{chercher as chercher_dans, Genre};
use azure_foundation::flux::{from_rsh, Value};

/// Au plus autant de resultats par recherche.
pub const MAX_RESULTATS: usize = 30;

fn texte(v: &Value, champ: &str) -> String {
    v.get(champ).and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn chercher(docs: &Docs, args: &Value) -> Result<Value, String> {
    let q = texte(args, "q");
    Ok(Value::list(chercher_dans(docs, &q).into_iter().take(MAX_RESULTATS).map(|r| {
        Value::map([
            ("genre", Value::from(if r.genre == Genre::Exemple { "exemple" } else { "page" })),
            ("id", r.id.into()),
            ("titre", r.titre.into()),
            ("lieu", r.lieu.into()),
            ("extrait", r.extrait.into()),
            ("chemin", r.chemin.into()),
        ])
    })))
}

pub fn page(docs: &Docs, args: &Value) -> Result<Value, String> {
    let (chemin, exemple) = (texte(args, "chemin"), texte(args, "exemple"));
    let p = if exemple.is_empty() {
        let (s, p) = chemin.strip_prefix("/doc/").and_then(|c| c.split_once('/')).ok_or_else(|| format!("chemin invalide : « {chemin} »"))?;
        docs.page(s, p).ok_or_else(|| format!("page introuvable : {chemin}"))?
    } else {
        docs.exemple(&exemple).map(|(p, _)| p).ok_or_else(|| format!("exemple introuvable : {exemple}"))?
    };
    let section = docs.section(&p.section).map(|s| s.titre.clone()).unwrap_or_default();
    let blocs = p.blocs.iter().filter(|b| !matches!(b, Bloc::Apercu(_) | Bloc::Demo { .. })).map(|b| from_rsh(&bloc(b)));
    Ok(Value::map([
        ("chemin", p.chemin().into()),
        ("titre", p.titre.clone().into()),
        ("resume", p.resume.clone().into()),
        ("section", section.into()),
        ("blocs", Value::list(blocs)),
        // L'ancre de l'exemple demande (vide : le haut de la page).
        ("exemple", if exemple.is_empty() { Value::from("") } else { cle(&exemple).into() }),
    ]))
}
