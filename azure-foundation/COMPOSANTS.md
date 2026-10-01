# Composants rsH

Toute balise qui n'est pas un élément de base (`container`, `text`, `title*`, `button`, `image`, `video`, `textarea`) est un **composant**. Une balise peut se fermer elle-même : `<divider/>`. Les attributs s'écrivent `nom="valeur"` et acceptent `{{variables}}`. Les classes aussi : `<badge.{{statut}}>`.

Styles : chaque composant a sa classe `az-…` (feuille par défaut : `src/compiler/components/builtin/components.rsc`). Une règle de l'app sur le même sélecteur l'emporte. Les variantes sont des classes en plus : `<btn.primary>`, `<alert.danger>`, `<badge.succes>`.

## Champs (valeurs lues par l'app : `ctx.value(id)`, `ctx.checked(id)`, `ctx.number(id)`)

| Balise | Attributs | Valeur |
|---|---|---|
| `<input#id>` | `placeholder`, `value`, `type="password\|number"`, `focus="true\|tout"` (prêt à taper ; `tout` : texte sélectionné) | texte |
| `<password#id>` / `<number#id>` / `<email#id>` / `<search#id>` | idem | texte (masqué / chiffres seulement) |
| `<textarea#id>` | — | texte multiligne |
| `<richtext#id valeur="…">` | `placeholder` | texte riche (voir plus bas) |
| `<checkbox#id>Texte<!checkbox>` | `checked`, `disabled`, `label` | `true` / `false` |
| `<radio#id name="groupe" value="M">M<!radio>` | `checked` | `ctx.value("groupe")` = le radio coché |
| `<switch#id label="…"/>` (`toggle`) | `checked` | `true` / `false` |
| `<slider#id/>` (`range`) | `value`, `min`, `max`, `step` | nombre |
| `<progress value="35"/>` | `max` | nombre (affichage) |
| `<rating#id value="4"/>` (`stars`) | `max` (5), `label` | nombre |
| `<select#id>` + `<option value="fr">France<!option>` (`dropdown`) | `options="a, b"`, `value` | valeur choisie |
| `<segmented#id options="Jour, Semaine"/>` | `value` | valeur choisie |

Couleur des cases, curseurs et barres : `accent-color` en rsC (`slider { accent-color: #f59e0b; }`). `background-color` : piste / case. `color` : texte.

## Texte riche (WYSIWYG)
`<richtext#corps valeur="{{contenu}}"><!richtext>` : zone de saisie où le texte est affiché avec ses styles. On peut y mettre du gras, de l'italique, du souligné, du barré, du `code`, de la couleur et des liens. Sa barre d'outils est `<richbar pour="corps"><!richbar>` : G, I, S, B, code, puis 7 couleurs. Sans `pour`, une seule barre sert toutes les zones de la page : elle agit sur celle qu'on était en train d'éditer.

- **Au clavier** : Ctrl+B (gras), Ctrl+I (italique), Ctrl+U (souligné), Ctrl+E (code), Ctrl+Maj+S (barré). Une touche marque la sélection, ou le texte tapé ensuite s'il n'y a pas de sélection. Ce style en attente est oublié dès que le curseur bouge.
- **Frappe** : taper continue le style du caractère d'avant, sauf un lien.
- **Attributs pour un éditeur par blocs** :
  - `commandes="true"` : taper `/` en début de mot appelle `on_click` avec `ctx.clicked == Some("slash-<id>")`, pour un menu de commandes.
  - `commandes="code|Nom|description|mots-clés;..."` : la fondation fait le menu elle-même, sous le curseur. Ce qui suit le `/` filtre la liste (sans accents ni majuscules ; nom, puis un de ses mots, puis un mot-clé), la fin du nom choisi est proposée en gris (`/tit` → `re 1`). ↑/↓ choisit, Entrée ou Tab valide, Échap ferme ; un espace après une recherche sans résultat ferme aussi. Valider retire `/tit` du texte et appelle `on_click` avec `commande-<id>@<code>@<position du />`.
  - `entree="true"` : Entrée (sans Maj) appelle `on_click` avec `entree-<id>@<curseur>` au lieu d'aller à la ligne, et Maj+Entrée va à la ligne.
  - `entree="true"` fait aussi appeler `on_click` par Retour arrière tout au début de la zone, sans sélection : `retour-<id>`, pour supprimer ou fusionner un bloc.
  - `focus="true"` : la zone a le focus dès la construction de la page, curseur à la fin ; `focus="12"` met le curseur au caractère 12.
- **Clic droit** sur du texte sélectionné (toute zone riche) : un panneau de mise en forme s'ouvre sous la souris, avec gras, italique, souligné, barré, code et 7 couleurs. Un clic l'applique à la sélection, qui est gardée ; un clic ailleurs ou Échap ferme le panneau.
- **Ctrl+Z** annule aussi les changements de style.
- **Boutons de l'app** : tout bouton `#rt-<id>-<marque>` agit sur la zone `#<id>`, qui garde sa sélection. Les marques sont `gras`, `italique`, `souligne`, `barre`, `code`, `couleur-e06c75`, `couleur-` (couleur par défaut) et `lien-<cible>`.
- **Valeur** (`valeur="…"` en entrée, `ctx.value("corps")` en sortie) : un texte sans aucun style reste tel quel. Sinon, ce sont des segments séparés par U+001E, chacun `marques U+001F #couleur U+001F lien U+001F texte`, avec les marques `g i s b c`. `azure_foundation::ui::models::rich` le lit (`parse`) et l'écrit (`serialize`).
- **Mise en page** : chaque morceau est mesuré dans sa police : graisse 700, oblique synthétique pour l'italique, police à chasse fixe pour le code. Le retour à la ligne, le clic, le curseur et la sélection utilisent ces mêmes positions.

Style rsC : sélecteur `richtext` (interligne 1,5).

## Mise en page
`row`, `column`, `stack`, `grid cols="2|3|4"`, `center`, `spacer`, `divider`, `section titre description`, `page titre description`, `navbar titre`, `footer`.

## Contenu
`card titre description pied`, `stat label valeur detail tendance="hausse|baisse"`, `feature titre description`, `price nom prix periode items="a, b"`, `hero titre description`, `list` + `list-item titre description`, `table` + `tr` + `th` / `td`, `info terme valeur`, `timeline` + `timeline-item titre date`, `accordion titre ouvert="true" id`, `quote auteur`, `code`, `kbd`, `avatar nom` (initiales), `empty titre description`, `skeleton`.

## Étiquettes et messages
`badge` (`.succes .danger .attention .neutre`), `tag`, `chip id` (bouton de retrait `#id`), `alert titre` (`.succes .attention .danger`), `toast titre` (`.succes .danger`), `meter label texte value max`.

## Navigation
`btn` (`.primary .danger .ghost .small`), `link`, `ancre vers="id"` (fait défiler jusqu'à l'élément `#id`), `menu` + `menu-item actif="true"`, `tabs id items actif` (boutons `#id-0`, `#id-1`…), `breadcrumb items`, `steps items courant="2"`, `pagination id pages="6" page="2"` (boutons `#id-1`…).

## Couches (par-dessus la page)
- `modal titre ouvert="true" id` : fond assombri, fenêtre centrée. Le bouton `#id-fermer` (x) est aussi activé par Échap. La page dessous ne reçoit plus les clics, et Tab reste dans la modale.
- `drawer titre ouvert="true" id` : panneau latéral droit, même principe.
- `toasts` : pile de `toast` fixée en bas à droite.
- `banner id` : bandeau (avec `#id-fermer`).
- `tooltip texte="…"` : infobulle sur ce qu'elle entoure, affichée après 0,5 s de survol immobile.

L'app garde l'état ouvert/fermé et redessine la page (`ctx.goto(...)`) quand `ctx.clicked == Some("m-fermer")`.

En rsC, on peut en faire d'autres : `position: fixed; top/right/bottom/left; z-index`. L'élément sort du flux, se place par rapport à la fenêtre, est dessiné par-dessus et reçoit les clics en premier.

## Glisser-déposer
- `draggable id="…"` : ce qu'elle entoure se saisit à la souris. Après 5 px, une copie translucide suit le pointeur (curseur main).
- `dropzone id="…"` : ce qu'elle entoure reçoit ce qu'on y lâche. Au survol, elle est entourée et un trait montre l'emplacement.

```
<for p in cartes>
  <draggable id="carte-{{p.id}}"><container.carte>{{p.titre}}<!container><!draggable>
<!for>
```

Lâcher sur une zone appelle `AzureWindow::on_drop(|ctx, d| ...)` avec `d.source` (id du draggable), `d.target` (id de la zone) et `d.position`. `position` est le rang parmi les draggables de la zone, sans compter celui qu'on lâche. Pour ce calcul, un élément large est coupé à mi-hauteur (liste), sinon à mi-largeur (rangée, galerie).

- Un simple clic sur l'élément, ou sur un bouton dedans, reste un clic : il est rendu au relâchement.
- Échap annule, et un lâcher hors zone ne fait rien.
- Une zone à l'intérieur de l'élément tenu ne reçoit rien.

## Clavier
- **Tab / Maj+Tab** : champ ou bouton suivant / précédent, avec un contour de focus. Les listes qui défilent suivent le focus.
- **Entrée / Espace** : active un bouton, coche une case, ouvre une liste.
- **Flèches** : radio voisin du groupe, curseur (± `step`), note, choix segmenté, option de liste. Début / Fin : curseur au min / max.
- **Haut / Bas** dans une zone de texte : ligne affichée précédente / suivante, à la même abscisse (avec Maj : étend la sélection).
- **Échap** : ferme la liste ouverte, sinon active `#…-fermer` de la couche du dessus, sinon retire le focus.

Une touche qui change un champ appelle `on_click` comme un clic (`ctx.clicked` = l'id).

## Composants de l'app
Un fichier `components/fiche.rsh` à côté de la page crée `<fiche>`, et un `components/fiche.rsc` éventuel donne ses styles. Un composant de l'app remplace celui d'Azure du même nom. Dans le modèle, on dispose de :
- `{{attribut}}`, `{{attribut_list}}` (découpé aux virgules) et `{{attribut_range}}` (1..n) ;
- `{{class}}`, `{{id}}` ;
- `{{contenu}}` (le texte entre les balises), `a_contenu` ;
- `{{initiales}}` (depuis `nom`) ;
- `<slot/>`, qui affiche le contenu passé, avec les variables de la page.

`<include src="parts/menu.rsh"/>` insère un autre fichier. Boucles : `<for.x in liste>` donne `x`, `x_index` (0…) et `x_rang` (1…).

## Styles rsC pris en charge
En plus de la boîte (tailles, marges, padding, bordures), du flux, de flex et de grid :
- `border-style` (`solid`, `dashed`, `dotted`, `double`, `none`), aussi dans `border: 1px dashed #fff` ;
- `box-shadow` (avec `inset`), `opacity`, `border-radius` (le contenu est découpé dans l'arrondi) ;
- `background-image: url(...)`, `background-size` (`cover`, `contain`, px, %), `background-position` ;
- `font-style: italic`, `letter-spacing`, `text-decoration` (`underline`, `line-through`, `overline`) ;
- `visibility: hidden` (la place reste prise, rien n'est dessiné ni cliquable) ;
- `cursor` : `pointer`, `text`, `not-allowed`, `crosshair`, `move`, `wait`, `grab`, `help`, `ew-resize`, `ns-resize`, `default` ;
- `transition: background-color 0.2s ease` (couleurs de survol et de focus des boutons et champs) ;
- `position: fixed | relative | absolute` + `top/right/bottom/left` + `z-index` ;
- `overflow-y` / `overflow-x` (défilement, barre saisissable), `scrollbar-color` (couleur de la poignée, héritée), `align-content`.

Une déclaration illisible est ignorée et signalée (`rsc::warnings`), le reste de la feuille est gardé.

Ce qui est vérifié par les tests : `tests/ui_components.rs` (galerie `tests/components/galerie.rsh`, capture `target/tmp/galerie.ppm`) et `tests/ui_interaction.rs`.
