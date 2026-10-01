# PROJECT_MAP — Azure

Azure : écosystème Rust maison (GUI/windowing) sans toolkit externe —
protocole Wayland implémenté à la main, moteur de rendu logiciel, DSL de
style/markup propriétaire (rsC/rsH).

## Modules (workspace)

| Crate | Rôle | Chemin |
|---|---|---|
| azure-core | Contrats, modèles, managers de base (config, logger, events, registry, uuid) — aucune logique métier | azure-core/ |
| azure-engine | Runtime bas niveau : protocole Wayland brut, rendu logiciel (formes, texte, polices), codec image (PNG/deflate/zlib) | azure-engine/ |
| azure-foundation | Framework UI applicatif : fenêtre, layout, composants UI, compilateur DSL (rsC=style, rsH=markup), event dispatch | azure-foundation/ |
| azure-rooter | Service/daemon de routage (routeur + client) | azure-rooter/ |
| azure-stockage | Daemon de stockage : espace privé chiffré par app (emplacement choisi par app) + partage (public / protégé par comptes avec rôle) + RsS (RustSql, le SQL d'Azure : tables, index, jointures, agrégats, transactions). Crypto maison (SHA-256, HMAC, PBKDF2, ChaCha20-Poly1305) | azure-stockage/ |
| azure-provider | Superviseur des processus d'arrière-plan : lance routeur + stockage (et les services déclarés dans ~/.config/azure/provider.conf ou enregistrés par une app), vérifie leur socket, relance avec délai croissant (abandon après 5 plantages/60 s). Lancé automatiquement par les apps (`Stockage::connect`, `navigation_manager::connect`) | azure-provider/ |
| azure-service | Tâches complexes réutilisables, servies par un daemon (`service_daemon`, surveillé par le provider). Les **appels** entre apps (une app sert une méthode, les apps autorisées l'appellent et attendent la réponse, avec délai max) et le **flux** — une app partage en temps réel les modifications d'un état en arbre (set/delete/push) + des événements libres ; les apps qu'elle choisit l'écoutent (état actuel puis chaque modification, filtré par chemins) | azure-service/ |
| azure-manager | Facilite toute la communication entre apps : manifeste `app.azure` (nom, fenêtre, pages, flux partagés/écoutés, tâches de fond, stockage), daemon `manager_daemon` (nom → id attribué, qui tourne, accès aux flux = manifeste + choix du tableau de bord, état publié en flux `etat`), ligne de commande `azure_manager` | azure-manager/ |
| azure-dashboard | Tableau de bord d'Azure (`azure_dashboard`, app rsH/rsC) : apps, qui parle à qui, services ; autoriser/retirer une app, public/privé, relancer un service ; page Terminal (commandes `azure install/uninstall/list/run/autostart`, exécutées par azure-manager, opcode AZURE_COMMAND, azure-manager/src/managers/terminal.rs) | azure-dashboard/ |
| azure-cli | Commande `azure` : `new <nom>` (crée une app vide azure-<nom> dans le projet Azure noté par setup dans ~/.local/share/azure/source, l'ajoute au workspace ; src/new.rs), `build <nom|dossier> [--installer]` (cargo build --release puis install ; src/build.rs), `setup` (installe les daemons dans ~/.local/share/azure/bin + lien ~/.local/bin/azure + tableau de bord), `install <dossier>` / `uninstall` / `list` / `run` (apps dans ~/.local/share/azure/apps/<nom>/ + lanceur .desktop), `autostart on|off|status` (service systemd utilisateur azure-provider) | azure-cli/ |
| azure-docs | La documentation d'Azure, en app Azure : contenu/ (une page par fichier, exemples de code à identifiant), ui/docs.rsh + docs.rsc, index de recherche réutilisable par l'IDE (src/index.rs) | azure-docs/ |
| azure-note | Azure Note v2, façon Notion : pages imbriquées, blocs (texte riche, titres, listes, tâches, code coloré, tableaux, sous-pages, vues de base), propriétés (8 genres dont formules calculées depuis les sous-pages), bases avec vues table / kanban (glisser-déposer) / liste / galerie, filtres et tri ; stockage RsS, migration des notes v1 ; fenêtre Doc reliée à Azure Docs. src/ : formule (moteur), modele (Espace), classeur (RsS), page (données de l'écran), clics (actions), proprietes (valeurs en pastilles/calendrier/pages liées + carte d'une propriété, sans formulaire), code (coloration), riche ; ui/note.rsh + components/ (champ, editeur-valeur, carte-prop, types-prop, menu-blocs) | azure-note/ |
| azure-testeur | Azure Testeur : relie des projets (+ l'environnement Azure lu dans ~/.local/share/azure/source avant le bac à sable), trouve leurs tests (lexique Rust maison), les lance (`cargo test` en thread, sortie lue en direct, arrêt par groupe de processus), et un Atelier qui écrit les tests DIRECTEMENT dans le code du projet (code générique + tests, éditeur = bloc de code d'Azure Note via `azure_note::code`). src/ : langage/ (trait `Langage` + `Operation` ; rust/{lexique,cargo}), execution, projet, ecran, clics ; ui/testeur.rsh + parts/{tests,atelier}.rsh | azure-testeur/ |
| azure-test | Workspace de test : apps d'exemple (app-a/b/c) + tests d'intégration engine/event/window | azure-test/ |

## Dépendances (qui dépend de qui)
azure-core ← azure-engine ← azure-foundation
azure-core ← azure-rooter
azure-core ← azure-stockage
azure-provider ← azure-foundation (libc seulement)
azure-core + azure-provider ← azure-service ← azure-foundation
azure-core + azure-provider + azure-service ← azure-manager ← azure-foundation ← azure-dashboard
azure-engine ← azure-test (+ azure-rooter via le bin AzureTest)

## Points d'entrée
- azure-rooter/src/bin/routeur_daemon.rs — binaire daemon
- azure-stockage/src/bin/stockage_daemon.rs — daemon de stockage (`cargo run -p azure-stockage --bin stockage_daemon -- [--root <dossier>] [--socket <chemin>]` ; sinon `AZURE_STOCKAGE_ROOT`, puis `~/.config/azure/stockage.conf` `root = ...`)
- azure-provider/src/bin/azure_provider.rs — superviseur (`azure_provider [--socket S] [--config F] [--logs D]` ; `azure_provider status|start|stop|restart <service>|shutdown`). Journaux : ~/.local/state/azure/provider/<service>.log
- azure-service/src/bin/service_daemon.rs — daemon azure-service (`--socket`, `--data` ; registre des apps dans ~/.local/share/azure/service/apps.txt)
- azure-manager/src/bin/manager_daemon.rs — daemon azure-manager ; azure-manager/src/bin/azure_manager.rs — CLI (`apps`, `liens`, `grant|revoke <app> <flux> <autre>`, `public`, `reset`, `forget`)
- azure-dashboard/src/main.rs — tableau de bord (`cargo run -p azure-dashboard`)
- azure-cli/src/main.rs — commande `azure` (installer et lancer Azure et ses apps)
- azure-test/src/main.rs — binaire AzureTest
- azure-test/app-test/app-{a,b,c}/src/main.rs — apps d'exemple
- azure-test/engine-test/event-test/src/main.rs — test d'intégration event
- azure-docs/src/main.rs — app Azure Docs (`cargo run -p azure-docs`) ; contenu : azure-docs/contenu/, règles : azure-docs/README.md
- azure-foundation/examples/*.rs — démos (run_demo, run_window_demo, show_image)

## Où chercher quoi

| Sujet | Chemin |
|---|---|
| Protocole Wayland (bas niveau) | azure-engine/src/platform/wayland/ |
| Rendu (formes, canvas, couleurs) | azure-engine/src/rendering/{models,managers}/ |
| Texte / polices / kerning | azure-engine/src/rendering/services/text/ |
| Codec image (PNG/zlib/deflate) | azure-engine/src/codec/ |
| Config / logger / registry / uuid | azure-core/src/managers/ |
| Contrats / cycle de vie / providers | azure-core/src/rules/ |
| Fenêtre applicative | azure-foundation/src/window/ |
| Layout | azure-foundation/src/layout/ (modèle web rsC : managers/web_layout.rs + models/css_box.rs ; ancien modèle en % : layout_manager.rs) |
| Composants UI (button, label, image…) | azure-foundation/src/ui/models/ |
| DSL style "rsC" (CSS-like) | azure-foundation/src/compiler/rsc/ |
| DSL markup "rsH" (HTML-like + if/while) | azure-foundation/src/compiler/rsh/ |
| Codegen / interprète du compilateur | azure-foundation/src/compiler/services/ |
| Dispatch d'événements applicatifs | azure-foundation/src/event/ |
| Feuilles de style runtime | azure-foundation/src/style/ |
| Stockage (objet AzureStockage, daemon, client, crypto) | azure-stockage/src/{managers,services,crypto}/ ; règles (clé, rôle, accès) : azure-core/src/models/storage_model.rs ; expressions côté app : azure-foundation/src/storage/ |
| RsS (SQL d'Azure) | azure-stockage/src/rss/ (lexer, parser, ast, table, store = fichiers + journal de commit, engine) ; côté app : azure-foundation/src/storage/models/db.rs (`store.db()`) |
| Supervision des daemons (provider) | azure-provider/src/managers/supervisor.rs (états, relances, sondes), models/{service,policy,config}.rs, services/client.rs (`Provider`, `Service`) ; côté app : azure-foundation/src/provider/mod.rs |
| Flux temps réel (partage/écoute entre apps) | azure-service/src/flux/ (value, path, change = modifs + filtre, hub = côté daemon, client = `Flux`/`Shared`/`Listener`) ; côté app : azure-foundation/src/flux/mod.rs, `AzureWindow::flux` |
| Format binaire commun / identité d'une app au bout d'un socket | azure-core/src/models/wire.rs, azure-core/src/managers/identity.rs |
| Point d'entrée unique d'une app (manifeste) | azure-foundation/src/app/mod.rs (`AzureApp::from_manifest` : send/navigate par nom, share/listen, stockage, window) ; manifeste : azure-manager/src/models/manifest.rs |
| Registre des apps, accès aux flux, état du tableau de bord | azure-manager/src/managers/manager.rs (cœur), daemon.rs (socket + synchro service/provider) |
| Interpolation rsH `{{...}}`, boucles avec variable | azure-foundation/src/compiler/services/condition.rs (`interpolate`, `Context::with_value`), interpreter.rs (`build_for`) ; données d'une page : `RouteTable::view_with` |
| Installation (dossiers, lanceur .desktop, systemd) | azure-cli/src/{paths,install,setup,autostart}.rs ; manifeste `[app] exec / icon / files` ; trouver son manifeste : `azure_app!()` (azure-foundation/src/app) |
| Appels entre apps (demande → réponse) | azure-service/src/call/ (broker = côté daemon, client = `Flux::serve` / `Flux::call`) ; manifeste `[provide m]` / `[use m@app]` ; droits : azure-manager (`Kind::Call`) ; côté app : `AzureApp::serve` / `call` |
| Glisser-déposer (`<draggable>`, `<dropzone>`, `AzureWindow::on_drop`) | azure-foundation/src/ui/services/interact/drag.rs, event/services/dispatch.rs, draw_ui::draw_drag |
| Texte riche WYSIWYG (`<richtext>`, `<richbar>`) | azure-foundation/src/ui/models/rich.rs (styles dont taille, format d'échange), ui/services/rich_layout.rs (positions par police, hauteur par ligne), textarea.rs (`rich`), draw_ui::draw_rich_textarea |
| Composants rsH (48 intégrés + ceux de l'app), champs de formulaire | azure-foundation/src/compiler/components/ (mod.rs + builtin/*.rsh + components.rsc), ui/models/control.rs, ui/services/{draw_control,form}.rs ; référence : azure-foundation/COMPOSANTS.md |
| Clavier (focus, Tab, touches) et couches `position: fixed` (modales, panneaux, notifications, infobulles) | azure-foundation/src/ui/services/interact/ (tree.rs: layer_paths ; focus.rs: focus_next, key_on_focused, escape), event/services/dispatch.rs, layout/managers/web_layout.rs (fixed_box) |
| Redessiner la même page sans perdre le défilement (`ctx.refresh`) | window_context.rs (`Effects::keep_scroll`), window.rs (pending_screen), interact/scroll.rs `carry_scroll` |
| Survol d'un parent (`.bloc:hover .outil { opacity }`) | compiler/rsc/services/link.rs (`is_hover_group`, `resolve_element_in`), codegen.rs `decoration_for`, draw_ui.rs (`HOVER_GROUPS`), interact/pointer.rs `hover_group_at` |
| Persistance : flux `persist = true` (azure-service/src/flux/hub.rs, `Hub::with_store`), messages en attente du routeur (azure-rooter/src/models/mailbox.rs) | voir ces fichiers |
| Observabilité : compteurs flux/appels (opcode STATS d'azure-service), événements des apps (`AzureApp::error`, plantages → manager REPORT), journaux (manager LOGS), pages Événements / Journaux du tableau de bord | azure-service/src/call/broker.rs, azure-manager/src/managers/{manager,daemon}.rs, azure-dashboard/ |
| Sécurité (bac à sable Landlock, identité par empreinte, daemons durcis, chiffrement au repos) | azure-core/src/security/ (sandbox, hardening, vault, registry), azure-core/src/crypto/, `[permissions]` du manifeste ; doc : SECURITE.md |
| Routage réseau | azure-rooter/src/managers/router.rs, services/client.rs |
| Décoration de fenêtre (native xdg-decoration + repli barre maison) | azure-engine/.../managers/decoration_manager.rs, azure-foundation/src/window/models/header_bar.rs |
| app_id (identité pour barre des tâches/dock) | azure-engine/.../managers/xdg_manager.rs (set_app_id), azure-foundation/src/window/models/window.rs (AzureWindow::app_id) |

## Docs existantes (ne pas dupliquer, pointer dessus)
- SECURITE.md — modèle de sécurité d'Azure (bac à sable, identité, chiffrement, limites)
- azure-core/README.md — doc technique détaillée (managers/models/rules)
- azure-engine/README.md (+ .html) — doc très détaillée (protocole, rendu, police, historique de bugs)

## Particularités structurelles
- Chaque crate a son propre dépôt .git imbriqué — pas de repo git à la racine du workspace.
- azure-test est lui-même un workspace Cargo avec ses propres membres (app-test/*, engine-test/event-test).
- azure-engine/src/lib.rs contient un test volumineux (`tests8`) servant de démo manuelle du moteur.
- azure-engine/tests/ contient plusieurs fichiers `debug_*` : expérimentations ponctuelles sur le rendu de texte, pas une suite stable.
- `set_app_id` déclare l'identifiant au compositeur mais ne fournit pas d'icône : l'icône barre des tâches/dock vient d'un fichier `.desktop` (absent du projet) dont `Icon=` correspond à cet id.
- Décoration serveur (xdg-decoration) non supportée par GNOME/Mutter : la barre d'en-tête maison (`header_bar`/`draw_header`) reste le chemin actif sur ces compositeurs.

_Dernière mise à jour : 2026-09-29 (Azure Testeur ; ctx.refresh) ; 2026-09-28 (Azure Note v2 ; glisser-déposer et texte riche dans azure-foundation ; sécurité ; persistance + observabilité ; composants rsH + champs ; appels entre apps ; azure-cli : installer/lancer ; azure-manager + AzureApp + tableau de bord + interpolation rsH ; azure-service + flux ; azure-provider ; azure-stockage + RsS + emplacement ; routes façon Laravel dans azure-foundation/src/navigation/models/router.rs)_
