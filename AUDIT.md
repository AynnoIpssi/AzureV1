# Audit d'Azure

Date : 27 septembre 2026. Périmètre : tout le workspace (core, engine, foundation, rooter, stockage, provider, service, manager, cli, dashboard, docs) et azure-test. La partie serveur, qui n'existe pas encore, en est exclue.

Deux passes :
1. relecture complète, et correction des failles et des connexions cassées ;
2. correction de tout ce que la première passe avait laissé ouvert.

## 1. État final

| Vérification | Résultat |
|---|---|
| `cargo test --workspace` | **301 réussis, 0 échec** (9 ignorés : des sous-tests lancés à part dans un processus neuf, pour Landlock, et un test qui demande une vraie session) |
| `azure-test` (workspace séparé) | **295 réussis, 0 échec** |
| `scripts/verification-e2e.sh` : vrais binaires, vrais daemons, 2 apps enfermées et isolées, HOME et sockets temporaires | **47 / 47** |
| `cargo clippy --workspace --all-targets` | **0 avertissement** (119 au départ) ; azure-test : 0 avertissement |
| Tableau de bord dans une vraie fenêtre Wayland | S'ouvre, lance le provider et les 4 daemons (sockets dans `/run/user/<uid>/azure`, en 700), s'enregistre, est enfermé |
| Tableau de bord au clic | Chaque bouton de chaque page est cliqué à sa vraie position (défilement compris) : c'est bien lui qui est touché, et il déclenche la bonne action |

## 2. Sécurité

### Failles trouvées et corrigées (passe 1)

| # | Problème | Correction |
|---|---|---|
| S1 | Routeur sans vérification d'identité : un processus pouvait recevoir les messages d'une autre app. | Vérification auprès d'azure-manager ; sinon, le premier exécutable arrivé garde l'id. |
| S2 | Une connexion du routeur pouvait agir au nom d'une autre app. | Chaque connexion est liée à son id. |
| S3 | Une app enfermée pouvait faire lancer n'importe quelle commande par le provider. | Un processus `NoNewPrivs` n'a droit qu'à l'état et à `ENSURE`. |
| S4 | Le bus D-Bus de session était joignable depuis l'enclos. | Landlock ABI 9 : seuls les sockets d'Azure et de Wayland restent joignables. |
| S5 | Une app enfermée pouvait tuer les daemons. | Landlock ABI 6 : pas de signal hors de l'enclos. |
| S6–S9 | Routeur fragile : tailles non bornées, contenu des messages dans les journaux, connexions mortes gardées, destinataire lent qui bloquait tout. | Plafond de 16 Mo, journaux nettoyés, fermeture et nettoyage des connexions, délai d'écriture. |

### Ce qui restait, corrigé (passe 2)

| Problème | Correction | Test |
|---|---|---|
| Sockets à nom fixe dans `/tmp` : un autre compte pouvait se faire passer pour un daemon. | Dossier privé : `$XDG_RUNTIME_DIR/azure/` (sinon `/tmp/azure-<uid>/`). Créé en 700, refusé s'il appartient à un autre compte ou si d'autres peuvent y écrire. | `azure-core/tests/paths.rs`, e2e |
| X11 joignable depuis l'enclos. | Le bac à sable n'autorise plus que le dossier d'Azure et Wayland. `/run` est en lecture seule. | `security.rs` |
| Noyaux anciens : bus, X11 et signaux non protégés. | Espaces de noms posés par le lanceur (`azure run`, ou relance automatique d'une app installée) :<br>• dossier de session masqué, sauf Wayland et Azure ;<br>• l'app est le processus 1 de son espace (aucun autre processus visible) ;<br>• pas de réseau sans permission. | `security.rs` (sans Landlock), e2e |
| Pas de plafond de connexions. | 64 connexions simultanées par processus et 1024 en tout, par daemon. | `azure-core/tests/limits.rs` |
| Verrous empoisonnés : un plantage bloquait le daemon. | Verrou récupéré au lieu de refuser toutes les requêtes suivantes. | relecture |
| Faux positif du provider (outil d'Azure lancé avec `NoNewPrivs`). | `azure` et azure-manager, installés à côté du provider, gardent la main. | relecture |
| Crypto vérifiée seulement par les vecteurs officiels. | Comparée à une implémentation indépendante aux cas limites. Comparaison en temps constant protégée contre le compilateur, aléa par `getrandom`. | `azure-stockage/tests/crypto_oracle.rs` |

## 3. Fonctionnel

### Passe 1

- `call_wait` : un appel peut attendre que la méthode soit servie.
- Le programme d'azure-test qui ne compilait plus est réparé.
- `IntraRouter` est ré-exporté par azure-foundation.
- Le README d'azure-core est réécrit.

### Passe 2

| Problème | Correction | Test |
|---|---|---|
| Événements et compteurs du manager perdus au redémarrage. | Enregistrés chiffrés (`activite.bin`). Les compteurs restent cumulés quand azure-service redémarre. | `azure-manager/tests/activite.rs` |
| Codegen : composants et `{{...}}` non générés. | Les parties dynamiques sont construites par l'interpréteur au lancement. Le reste reste du Rust généré. `build_ui_with(ctx)`. | `tests/codegen_dynamic.rs` (identique au pixel près) |
| Codegen : `position: fixed`, `inset`, `z-index`, infobulle absents. | Émis, comme toutes les nouvelles propriétés. | `tests/codegen_web.rs` (arbres identiques) |
| Propriétés rsC sans effet (PROBLEMES #13). | Toutes appliquées. Détail dans `azure-foundation/COMPOSANTS.md`. | `tests/css_complet.rs`, `azure-engine/tests/{box_styles,text_options}.rs` |
| Une valeur rsC illisible (`url(a/b.png)`, `center / cover`, couleur fausse) faisait échouer toute la feuille. | La déclaration est sautée et signalée, comme dans un navigateur. `url()` sans guillemets est lu. | `css_complet.rs`, `rsc_parser.rs` |
| Défilement (#20). | Défilement horizontal, barre saisissable à la souris, molette et pavé tactile distingués. | `css_complet.rs` |
| Coins arrondis (#26). | Le contenu est découpé dans l'arrondi, ombre `inset`. | `css_complet.rs` |
| Liens d'ancre (#35). | `#ancre-<id>`, composant `<ancre vers>`, `ctx.scroll_to`. | `css_complet.rs` |
| Tableau de bord jamais testé au clic. | Tous les boutons cliqués à leur position, plus un parcours complet. | `azure-dashboard/tests/clics.rs` |
| Outils `debug_*` du moteur (#21). | Écrivent dans `target/tmp`, et tournent avec les autres tests. | `cargo test` |
| Tests qui laissaient leurs sockets dans `/tmp`. | Sockets dans `target/tmp` ; le scénario e2e a son propre dossier de sockets. | — |

## 4. Ce qui reste

Plus rien d'ouvert dans le code (`azure-docs/PROBLEMES.md` : « Plus rien d'ouvert »). Restent des limites qui ne se règlent pas par du code :

- **La crypto est maison.** Elle est vérifiée par les vecteurs officiels et par comparaison avec une implémentation indépendante, mais seul un audit extérieur peut la certifier.
- **Un programme lancé hors d'Azure n'est pas enfermé**, comme tout programme de l'utilisateur.
- **Pas de signature par une autorité externe :** l'identité est l'empreinte enregistrée par l'installateur local.
- **Système qui interdit à la fois Landlock récent et les espaces de noms sans privilège :** la protection se réduit à ce que Landlock y offre, et l'app le signale.
- **Écarts CSS assumés** (documentés dans `web_layout.rs`) :
  - `position: absolute` se place dans son conteneur direct ;
  - pas de fusion des marges ni de texte « inline » ;
  - `border-radius` n'a qu'un rayon ;
  - les transitions ne portent que sur les couleurs de survol et de focus.
- **Point 7 (git, CI, documentation générale) :** laissé de côté à ta demande.

## Pour relancer les vérifications

```sh
cargo test --workspace
(cd azure-test && cargo test --workspace)
bash scripts/verification-e2e.sh
cargo clippy --workspace --all-targets
```
