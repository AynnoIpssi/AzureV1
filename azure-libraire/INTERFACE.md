# azure-libraire — interface

La librairie d'Azure range tous les modules réutilisables, un dossier par domaine. `src/interface/` contient les **modules du front** : les balises qu'une page rsH appelle (`<card>`, `<sidebar>`, `<confirm>`…) et les **thèmes** qui leur donnent leurs couleurs.

La crate ne dépend de rien. azure-foundation la charge : une app n'a rien à ajouter, toute balise du catalogue est utilisable dans ses pages.

## Organisation

```
src/interface/
  catalogue.rs            la liste des modules (nom, catégorie, résumé)
  theme.rs                lecture des thèmes, remplacement des jetons, thème en cours
  themes/*.theme          sable (défaut), ivoire (clair), ardoise
  modules/<catégorie>/    <nom>.rsh (modèle) + <nom>.rsc (styles)
  modules/base/champs.rsc champs dessinés par la fondation (input, select…)
exemples/vitrine.rsh      une app complète faite de modules
```

## Ajouter un module

1. Écrire `modules/<catégorie>/<nom>.rsh` et `<nom>.rsc`.
2. L'ajouter dans `catalogue.rs` : `module!("<catégorie>", "<nom>", "attributs ; ce qu'il fait")`.

Règles :
- classes préfixées `az-<nom>` ; les variantes sont des classes en plus (`<btn.primary>`) ;
- **aucune couleur en dur** dans le `.rsc` (un test le vérifie) : uniquement des jetons ;
- le modèle dispose de `{{attribut}}`, `{{attribut_list}}`, `{{class}}`, `{{id}}`, `{{contenu}}`, `<slot/>` (voir `azure-foundation/COMPOSANTS.md`).

## Thèmes

Un thème est un fichier de jetons `nom = valeur`. Dans une feuille rsC :

| Écriture | Donne |
|---|---|
| `$surface` | la valeur du jeton |
| `$accent/14` | la couleur du jeton à 14 % d'opacité |

Les jetons sont remplacés dans les styles des modules **et dans la feuille de l'app** : une app qui écrit `background-color: $fond;` suit le thème.

| Famille | Jetons |
|---|---|
| Fonds | `fond` (page), `fond-creux`, `fond-barre`, `surface`, `surface-2` … `surface-6` (de plus en plus relevé) |
| Textes | `texte-fort`, `texte`, `texte-doux`, `texte-attenue`, `texte-discret`, `texte-faible`, `texte-eteint` |
| Accent | `accent`, `accent-vif` (survol), `accent-sombre`, `accent-ombre`, `accent-nuit`, `accent-texte`, `accent-texte-vif`, `sur-accent` (texte posé sur l'accent), `sur-couleur` |
| Traits | `trait`, `ombre`, `voile` — toujours avec une opacité (`$trait/8`) |
| États | `succes`, `attention`, `danger` et leur version texte (`succes-texte`…), `code-texte` |
| Graphes | `graphe-2` … `graphe-6` : couleurs des séries après la première, qui prend `accent` |
| Arrondis | `rayon-petit`, `rayon`, `rayon-moyen`, `rayon-grand`, `rayon-large`, `rayon-rond` |

Choisir le thème, depuis une app :

```rust
use azure_foundation::theme::{self, Theme};

theme::choisir("ivoire")?;                          // un thème fourni
theme::definir(Theme::fichier("ui/app.theme")?);    // le thème de l'app
theme::definir(theme::actif().as_ref().clone().avec("accent", "#7fd1b9"));
```

Après un changement en cours de route, `ctx.refresh()` redessine la page dans le nouveau thème. Pour essayer sans toucher au code : `AZURE_THEME=ivoire azure run docs`.

Un thème d'app part d'un thème fourni et n'écrit que ce qu'il change :

```
nom = menthe
base = ardoise
accent = #7fd1b9
```

Un **nouveau thème fourni** : un fichier dans `themes/`, une ligne dans `INTEGRES` (`theme.rs`). Un test vérifie qu'il définit tous les jetons.

Les champs dessinés par la fondation (case, interrupteur, curseur, choix segmenté…) prennent `accent` quand la page ne fixe pas `accent-color`.

## Modules (97)

**application** — la coque d'une app
`app` (`.colonne`), `sidebar titre`, `sidebar-group titre`, `sidebar-item id actif`, `main`, `toolbar titre`, `statusbar`.

**mise_en_page**
`row`, `column`, `stack`, `grid cols`, `center`, `spacer`, `divider`, `section titre description`, `page titre description`, `navbar titre`, `footer`, `header titre description` (actions à droite), `panel titre` (`.plat`), `split` + `pane` (`.etroit .large .creux`), `scroll`, `box` (`.creux .borde .serre`).

**contenu**
`card`, `stat`, `feature`, `price`, `hero`, `list` + `list-item`, `table` + `tr` + `th` / `td`, `info`, `timeline` + `timeline-item`, `accordion`, `quote`, `code`, `kbd`, `avatar`, `empty`, `skeleton`, `media nom titre description`, `shortcut label touches="Ctrl, K"`, `caption`, `lead`, `muted`.

**messages**
`badge`, `tag`, `chip`, `alert`, `toast`, `meter`, `banner`, `status` (`.succes .attention .danger .neutre`), `count` (`.accent .danger`).

**navigation**
`btn`, `link`, `ancre`, `menu` + `menu-item`, `tabs`, `breadcrumb`, `steps`, `pagination`, `btn-group`, `icon-btn` (`.small .primary`), `tree-item id niveau ouvert actif`.

**formulaire**
`form`, `field label aide erreur`, `richbar`, `label`, `help` (`.erreur`), `fieldset titre description`, `setting titre description` (champ à droite), `search-bar id placeholder valeur bouton` (champ `#id`, bouton `#id-ok`), `actions` (`.gauche`).

**graphes** — voir « Graphes » plus bas
`chart` (carte nue pour son propre `<graphe>`), `chart-line`, `chart-area`, `chart-bars`, `chart-stack`, `chart-points`, `chart-hbars`, `chart-donut`, `chart-pie`, `chart-gauge`, `chart-radar`, `spark`, `stat-spark`.

**couches**
`modal`, `drawer`, `toasts`, `confirm id titre ouvert oui non` (boutons `#id-oui` et `#id-fermer`, ce dernier aussi par Échap ; `.danger`).

Le résumé de chaque module (attributs, variantes) est dans `catalogue.rs` ; `azure_libraire::interface::modules()` le donne au code (complétion, documentation).

Une variante s'écrit en classe, pas en attribut : `<confirm.danger …>`.

## Graphes

Le dessin est fait par la fondation (balise `<graphe type="…">`, antialiasé) ; les modules `chart-*` l'habillent d'une carte : `titre`, `detail` (sous le titre), `valeur` (en grand à droite), `aide` (sous le graphe). Variantes : `.petit`, `.grand`, `.nu` (sans carte).

| Module | `type` de `<graphe>` | Montre |
|---|---|---|
| `chart-line` | `ligne` | une ou plusieurs courbes |
| `chart-area` | `aire` | courbe remplie dessous |
| `chart-bars` | `barres` | barres verticales, côte à côte par série |
| `chart-stack` | `empile` | barres empilées |
| `chart-points` | `points` | nuage de points |
| `chart-hbars` | `barres-h` | classement : étiquette, barre, valeur |
| `chart-donut` | `anneau` | parts, total (ou `centre`) au milieu, légende à droite |
| `chart-pie` | `secteurs` | parts en disque |
| `chart-gauge` | `jauge` | une valeur entre `min` et `max` (0 à 100) |
| `chart-radar` | `radar` | une branche par étiquette, un polygone par série |
| `spark`, `stat-spark` | `spark` | mini-courbe sans axes (`.succes .attention .danger`, ou `ton` pour `stat-spark`) |

Données :

| Attribut | Rôle |
|---|---|
| `valeurs="1, 4, 2"` | une série |
| `series="CPU: 1 4 2; RAM: 3 3 5"` | plusieurs séries nommées |
| `etiquettes="Lun, Mar, Mer"` | axe du bas, noms des parts ou des branches ; une étiquette vide garde sa place (`"0 s, , , 3 s"`) |
| `min`, `max` | bornes de l'échelle (sinon celles des données, à partir de 0, arrondies) |
| `unite=" W"` | ajoutée aux valeurs affichées |
| `legende="true"` | noms des séries au-dessus |
| `grille="false"`, `lisse="false"` | sans échelle ; sans arrondi des courbes |
| `centre="35 s"` | texte au milieu d'un anneau |
| `couleurs="#4ade80, #f87171"` | couleurs imposées (sinon `accent-color` / l'accent du thème, puis `graphe-2`…) |

Les nombres s'écrivent avec un point. En rsC, sur `graphe` ou sa classe : `color` (textes), `background-color` (grille et pistes), `accent-color` (première série), `width`, `height`.

```
<chart-area titre="Puissance" detail="2 dernières minutes" valeur="{{p.valeur}}" unite=" W" valeurs="{{p.valeurs}}"/>

<for.g in graphes>
    <chart titre="{{g.titre}}" valeur="{{g.valeur}}">
        <graphe.az-chart-plot type="{{g.genre}}" valeurs="{{g.valeurs}}" unite="{{g.unite}}"/>
    <!chart>
<!for>
```

`exemples/graphes.rsh` les montre tous (`cargo test -p azure-foundation --test libraire_graphes` → `target/tmp/graphes-<thème>.ppm`). Azure Benchmark s'en sert : puissance et processeur (Machine), répartition par app (Apps), quatre graphes par app, énergie et processeur par banc (Tests).

Pas encore fait : la valeur sous la souris au survol.

## Exemple

```
<app>
    <sidebar titre="Mon app">
        <sidebar-item id="nav-accueil" actif="true">Accueil<!sidebar-item>
    <!sidebar>
    <main>
        <toolbar titre="Accueil"><spacer/><btn.primary id="nouveau">Nouveau<!btn><!toolbar>
        <page>
            <header titre="Bonjour" description="Vos projets"/>
            <panel titre="RÉGLAGES">
                <setting titre="Notifications"><switch#notifs/><!setting>
            <!panel>
        <!page>
    <!main>
<!app>
```

`exemples/vitrine.rsh` va plus loin ; `cargo test -p azure-foundation --test libraire_themes` la dessine dans chaque thème (`target/tmp/vitrine-<thème>.ppm`).
