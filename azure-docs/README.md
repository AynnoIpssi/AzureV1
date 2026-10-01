# Azure Docs

La documentation d'Azure, écrite comme une app Azure : `cargo run -p azure-docs`, ou `azure install azure-docs` puis `azure run docs`.

- `contenu/` : le texte, un dossier par section et un fichier par page ;
- `ui/docs.rsh` + `ui/docs.rsc` : l'interface (un seul gabarit, quatre vues : accueil, section, page, recherche) ;
- `src/contenu.rs` : lecture des pages ;
- `src/index.rs` : la recherche ;
- `src/ecrans.rs` : les données des vues et les clics ;
- `src/service.rs` : ce que Docs sert aux autres apps.

## Servi aux autres apps
Tant qu'elle est ouverte, Docs sert deux méthodes (`[provide]` dans `app.azure`, réservées à Azure Note) :

- `chercher` `{ q }` : les résultats de la recherche (`genre`, `id`, `titre`, `lieu`, `extrait`, `chemin`) ;
- `page` `{ chemin }` ou `{ exemple }` : une page, ses blocs sous la même forme que dans le gabarit (sans aperçus ni démonstrations).

Azure Note les affiche dans sa fenêtre Doc (voir `azure-note/src/doc.rs`). Pour ouvrir ces méthodes à une autre app, ajoutez-la à `to =`.

## Ajouter une page
Il suffit de créer `contenu/<nn>-<section>/<nn>-<page>.page`. Les numéros fixent l'ordre et ne font pas partie des identifiants. Une nouvelle section demande aussi un fichier `_section`, avec `titre:` et `resume:`.

````text
titre: Flexbox
resume: Aligner des éléments sur une ligne ou une colonne.

## Une partie
Un paragraphe ; les lignes qui se suivent sont réunies.
- un élément de liste
> note: une remarque            (aussi astuce: et attention:)
> demo routeur: ce qu'elle montre   (un bouton « Ouvrir » ; voir plus bas)
| Colonne | Colonne |
|---|---|
| a | b |
```rsc rsc.flex.1 "Titre de l'exemple"
.barre { display: flex; }
```
````

Pas besoin de relancer l'app : la page apparaît dès qu'on navigue.

Chaque bloc de code a un bouton **Copier** : il met le code exact dans le presse-papiers du système (`ctx.copy`) et affiche « Copié » un instant (`ctx.flash`).

## Aperçus rsC
Sous un exemple rsC, un bloc `apercu` affiche le rendu en direct, calculé par le moteur d'Azure : ce sont de vrais widgets (survol, focus, défilement). Il contient une petite page rsH, puis, après une ligne `---`, des styles propres à l'aperçu si besoin :

````text
```apercu "Légende facultative"
<container.barre.ap-zone>
    <button.ap-boite>Accueil<!button>
<!container>
---
.barre { height: 48px; }
```
````

La page est construite avec, dans l'ordre : `ui/apercu.rsc` (des aides `ap-zone`, `ap-boite`, `ap-case`, `ap-ligne`, `ap-grand`, `ap-ecran` pour rendre les boîtes visibles), l'exemple rsC qui précède, puis les styles après `---`. Les tests construisent chaque aperçu et écrivent son rendu dans `target/tmp/azure-docs/apercu-<exemple>.ppm`.

## Démonstrations
Un bloc `> demo <nom>: texte` affiche un bouton « Ouvrir » qui ouvre de vraies fenêtres (`ctx.open_window`). Les démonstrations sont dans `src/demos.rs`, leurs interfaces dans `ui/demos/` :

| Nom | Fenêtres |
|---|---|
| fenetre | une fenêtre avec son propre on_click (un compteur) |
| tailles | trois fenêtres de tailles différentes |
| partagee | une note construite depuis sa seule source rsH + rsC |
| routeur | Émetteur et Récepteur : chaque clic de l'un redessine l'autre |

Un nom inconnu, ou une démonstration citée par aucune page, fait échouer les tests.

## Identifiants des exemples
Chaque bloc de code porte un identifiant `<section>.<page>.<n>`, numéroté 1, 2, 3… dans l'ordre de la page. Il est affiché au-dessus du code et sert à retrouver un exemple :
- dans l'app, depuis la recherche (taper l'identifiant) ;
- depuis un autre outil (l'IDE), en Rust :

```rust
let docs = azure_docs::Docs::charger(&azure_docs::contenu::dossier_par_defaut())?;
let resultats = azure_docs::chercher(&docs, "flex wrap");   // pages et exemples, les plus pertinents d'abord
let (page, exemple) = docs.exemple("rsc.flex.1").unwrap();   // un exemple par son identifiant
```

Langages reconnus : `rsh`, `rsc`, `rust`, `toml`, `sh`, `texte`, `rss`.

## Ce que vérifient les tests (`cargo test -p azure-docs`)
- Chaque exemple rsH, rsC et RsS est lu par les vrais parseurs, et une déclaration rsC ignorée compte comme une erreur.
- Chaque identifiant est unique, bien formé et numéroté dans l'ordre de sa page ; chaque exemple a un titre.
- La recherche retrouve chaque exemple par son identifiant.
- Chaque vue s'affiche avec les identifiants et le code de ses exemples (captures dans `target/tmp/azure-docs/`), les cartes sont cliquables et le code trop large défile.
