# Refactorisation

État au 27 septembre 2026. Aucun de ces points n'est un bug : ils rendent le code plus simple à lire et à faire évoluer. Après chaque étape, on relance les vérifications d'`AUDIT.md` (`cargo test --workspace`, `azure-test`, `scripts/verification-e2e.sh`, `cargo clippy`).

Règle suivie jusqu'ici : ne toucher ni au format des messages, ni aux fichiers enregistrés, ni à la crypto. Une refactorisation ne doit rien changer vu de l'extérieur.

## Fait

| Point | Résultat |
|---|---|
| Démarrage des daemons copié 5 fois | `azure_core::daemon::{bind, serve}` : ouverture du socket, plafond de connexions, un thread par client |
| `read_frame`, `write_frame`, `error`, `check_status` en 3 copies | `azure_core::models::frame`, plus `response(result)` ; chaque daemon garde sa taille maximale |
| `registry.rs` lisait ses trames à la main (une réponse trop grande était coupée) | passe par `frame::read_frame` |
| Routeur : une vingtaine de `read_exact`, opcodes `0`..`8` en dur, 4 `Arc<Mutex>` passés partout | `models/request.rs` (opcodes nommés, format documenté), état `Router` unique, une méthode par opcode ; le client construit ses requêtes avec `Writer` |
| `interact.rs` (1537 lignes) | dossier `ui/services/interact/` : `tree`, `pointer`, `scroll`, `text`, `focus`, `keyboard` |

## Reste à faire

### 1. Un vrai type `Rect` (azure-foundation)
- Le tuple `(u32, u32, u32, u32)` apparaît environ 62 fois, surtout dans `ui/services/interact/`, à côté de `layout_manager::Rect = (i32, i32, u32, u32)`.
- Les deux se convertissent à la main un peu partout.
- Proposition : une seule struct `Rect { x, y, w, h }` avec `contains`, `inset` et `offset`, utilisée par la mise en page, le dessin et l'interaction.
- C'est le chantier le plus large : il touche aussi `layout_manager`, `web_layout` et `draw_ui`.

### 2. Découper `compiler/services/codegen.rs` (1074 lignes)
Séparer l'émission des nœuds, du style, des parties dynamiques (déjà en partie dans `codegen_runtime.rs`) et des littéraux. Même méthode que pour `interact` : un dossier, et `mod.rs` qui ré-exporte.

### 3. Découper `window/models/window.rs` (880 lignes)
Séparer la boucle d'événements, la gestion du clavier (répétition des touches), les flux et le rendu.

### 4. Autres gros fichiers à regarder
`web_layout.rs` (873 lignes), `rsc/services/link.rs` (964 lignes), `draw_ui.rs` (772 lignes), `azure-stockage/src/rss/engine.rs` (848 lignes), `azure-manager/src/managers/manager.rs` (753 lignes).

### 5. Petites corrections
- Dossiers `compiler/rsh/mangers` et `compiler/rsc/mangers` : faute de frappe, à renommer `managers` (mettre à jour les `mod` et les `use`).
- `azure-engine/src/rendering/services/buffer.rs` : indentation cassée.

### 6. En attente de décision
- **`cargo fmt` :** 2158 endroits seraient reformatés, parce que le code utilise volontairement des lignes longues. Si on le fait, ce serait avec un `rustfmt.toml` qui garde des lignes longues (par exemple `max_width = 160`).
- **`azure-docs/archive` :** 959 lignes de vieux code. À garder ou à supprimer ?
