# Service — les services de la librairie d'Azure

`azure-libraire/src/service/` : des méthodes prêtes à être servies aux
autres apps par azure-service. Un service par module du back (voir
BACK.md) : la même logique, appelable par n'importe quelle app sans la
lier, et sans rien ouvrir.

## S'en servir depuis une app

L'app **Azure Services** (`services`, `~/Bureau/Application/azure-services`)
les sert tous. Azure lance sa tâche de fond au premier appel.

```
# app.azure de l'app qui appelle
[use diff-unifie@services]
```

```rust
use azure_foundation::flux::Value;

let diff = app.call("services", "diff-unifie", Value::map([
    ("avant", Value::from(ancien)),
    ("apres", Value::from(nouveau)),
]))?;
```

La fenêtre d'Azure Services (`azure run services`) liste tout le
catalogue avec la ligne `[use …]` de chaque méthode.

## Index

Le nom complet d'une méthode est `<service>-<méthode>`. `?` : argument
facultatif. Les octets voyagent en base64.

| Méthode | Arguments | Réponse |
|---|---|---|
| `diff-lignes` | avant, apres | `[{ genre, texte, ancien, nouveau }]` (genre : egal, retire, ajoute) |
| `diff-blocs` | avant, apres, contexte? | `{ ajoutees, retirees, blocs: [{ entete, lignes }] }` |
| `diff-unifie` | avant, apres, nom_avant?, nom_apres?, contexte? | texte (`diff -u`) |
| `diff-mots` | avant, apres | `[{ genre, texte }]` |
| `code-colorer` | langage (rsh, rsc, rss, rust, toml, sh), code | `[[{ genre, texte }]]`, une liste par ligne |
| `code-fonctions` | chemin (l'extension dit le langage), source | `{ langage, lignes, test, types, fonctions: [{ nom, proprietaire, ligne, fin, appels }] }` |
| `tableur-csv` | texte | `[[cellule]]` |
| `tableur-xlsx` | base64, feuille? | `{ lignes, feuilles }` |
| `archive-lister` | base64 | `[nom]` |
| `archive-extraire` | base64, nom | `{ base64 }` ou rien |
| `archive-creer` | fichiers: `[{ nom, texte ou base64 }]` | `{ base64 }` |
| `empreinte-calculer` | texte ou base64, algo? (sha256, sha1, md5, crc32) | texte hexadécimal |
| `encodage-base64` | texte | texte |
| `encodage-texte` | base64 | texte (UTF-8, UTF-16, Windows-1252 devinés) |
| `encodage-sans-accent` | texte | texte |
| `temps-maintenant` | aucun | `{ secondes, date, heure }` |
| `temps-lire` | texte | `{ secondes, date, heure }` |
| `temps-ecrire` | secondes | `{ secondes, date, heure }` |

## Servir un service depuis sa propre app

```rust
use azure_libraire::service;

let _servies = app.servir(service::service("diff").unwrap())?;
```

Chaque méthode doit être déclarée `[provide …]` dans le manifeste ;
`service::manifeste(&[...], tache, acces)` écrit ces sections
(`cargo run -p azure-libraire --example manifeste_services` pour tout).

Sans azure-service (essais, outils) : `service::appeler("diff-lignes", &arguments)`.

## Ajouter un service

1. Un fichier `src/service/<nom>.rs` : un `pub static SERVICE: Service`
   et une fonction `fn(&Valeur) -> Result<Valeur, String>` par méthode.
2. Le déclarer dans `src/service/mod.rs` (`pub mod` + la liste `SERVICES`).
3. Une ligne dans l'index ci-dessus, un test dans `tests/service.rs`.
4. `cargo test` dans azure-services dit quelles sections `[provide]`
   ajouter à son manifeste ; puis `azure build services --installer`.

## Règles

- **Une méthode ne fait que calculer sur ce qu'on lui donne.** Elle
  n'ouvre aucun fichier et ne parle à personne : un service tourne hors du
  bac à sable de l'app qui l'appelle, lire un chemin reçu lui ferait lire
  à sa place ce qu'elle n'a pas le droit de lire.
- **Aucune dépendance** : `Valeur` est le type de la librairie ;
  azure-service le convertit (`Value::from(valeur)`, `Valeur::from(&value)`).
- **Les erreurs sont des phrases** pour la personne qui appelle
  (« argument « apres » attendu (un texte) »).

## Pas servis

- `chiffrement` : des clés ne voyagent pas entre apps.
- `hasard` : rien à gagner à le demander à un autre processus.
- `code::analyse` (un projet entier) : demande de lire un dossier.
