# Back — les modules de logique d'Azure

`azure-libraire/src/back/` : la logique réutilisable qui ne dessine rien et
ne parle à aucun daemon. Tout est écrit à la main, sans dépendance, et se
teste seul : `cargo test -p azure-libraire`.

Pour s'en servir dans une app : `azure-libraire = { path = "/home/aynno/Dev/Azure/azure-libraire" }`
dans son `Cargo.toml` (et `[profile.dev.package.azure-libraire] opt-level = 3`,
posé par `azure new`), puis `use azure_libraire::back::…`.

## Index

| Module | Ce qu'il donne |
|---|---|
| `hachage::sha256` | `sha256(&[u8]) -> [u8; 32]`, `Sha256::new() / update / finish` |
| `hachage::hmac` | `hmac_sha256(cle, message)`, `pbkdf2_sha256(mdp, sel, tours, &mut out)`, `constant_time_eq(a, b)` |
| `hachage::sha1`, `hachage::md5` | `sha1(&[u8]) -> [u8; 20]`, `md5(&[u8]) -> [u8; 16]` — cassés pour la sécurité, pour les formats qui les imposent (Git, MySQL, PostgreSQL) |
| `hachage::crc32`, `hachage::adler32` | `crc32(&[u8]) -> u32` (+ `suite(crc, &[u8])`), `adler32(&[u8]) -> u32` |
| `chiffrement::aead` | ChaCha20-Poly1305 : `seal(cle, aad, clair)` / `open(cle, aad, scelle)` (nonce au hasard, rangé devant) ; `encrypt` / `decrypt` avec un nonce donné |
| `chiffrement::random` | `random_bytes(&mut [u8])` : octets du noyau (clés, nonces, sels) |
| `chiffrement::rsa` | `ClePublique::depuis_pem(texte)`, `.chiffrer_oaep(message, graine)` |
| `compression` | `compresser(&[u8]) -> Vec<u8>` et `decompresser(&[u8])` : DEFLATE brut |
| `compression::zlib` | `compress(&[u8])`, `decompress(&[u8])` (PNG, objets Git) |
| `compression::zip` | lire : `Zip::ouvrir(&octets)`, `.noms()`, `.fichier(nom)`, `.texte(nom)` ; écrire : `Archive::new()`, `.ajouter(nom, contenu)`, `.finir()` |
| `encodage::hexa` | `hexa(&[u8]) -> String`, `lire(&str) -> Option<Vec<u8>>` |
| `encodage::base64` | `base64(&[u8])`, `base64_lire(&str)` |
| `encodage::texte` | `decoder(&[u8]) -> String` (UTF-8, UTF-16, Windows-1252 devinés), `sans_accent(&str)` |
| `temps` | secondes depuis 1970, sans fuseau : `maintenant()`, `lire("2026-10-06 12:00")`, `date(s)`, `heure(s)`, `date_heure(s)`, `civil(jours) -> (a, m, j)`, `jours(a, m, j)` |
| `hasard` | `Alea::new(graine)` rejouable : `.u64()`, `.f64()`, `.sous(n)`, `.entre(min, max)`, `.parmi(&liste)` ; `Alea::graine_du_moment()` |
| `tableur` | `lire(chemin, feuille) -> Lu { lignes, feuilles }` (.csv, .tsv, .xlsx), `coller(texte)`, `rectangle`, `lettre(rang)`, `simplifier(nom)` ; `csv::lire`, `csv::separateur`, `xlsx::lire(&octets, feuille)` |
| `texte::diff` | `lignes(avant, apres) -> Vec<Ligne>` (genre, texte, numéros), `blocs(&lignes, 3)`, `unifie(nom_a, nom_b, avant, apres, 3)`, `mots(a, b)` (dans une ligne), `compte(&lignes)`, `operations(&[T], &[T])` |
| `code` | `analyse::analyser(dossier) -> Projet` (fichiers, fonctions, qui appelle qui), `rust::lire` / `js::lire` (un fichier), `rust::jetons` / `js::jetons`, `est_test(chemin)` |
| `code::coloration` | `colorer(langage, code) -> Vec<Vec<(genre, texte)>>` pour rsh, rsc, rss, rust, toml, sh |

## Exemples

```rust
use azure_libraire::back::texte::diff::{self, Genre};

for l in diff::lignes(&ancien, &nouveau) {
    let signe = match l.genre { Genre::Ajoute => '+', Genre::Retire => '-', Genre::Egal => ' ' };
    println!("{signe} {}", l.texte);
}
let correctif = diff::unifie("a/main.rs", "b/main.rs", &ancien, &nouveau, 3);
```

```rust
use azure_libraire::back::compression::zip::{Archive, Zip};

let mut archive = Archive::new();
archive.ajouter("notes/une.txt", b"bonjour")?;
let octets = archive.finir();
let relu = Zip::ouvrir(&octets)?.texte("notes/une.txt")?;
```

## Règles

- **Aucune dépendance**, même pas `libc` : ce qui vient du système se
  déclare à la main (voir `chiffrement/random.rs`).
- **Rien d'une app** : pas de fenêtre, pas de stockage, pas de modèle
  propre à une app. Ce qui dépend d'azure-foundation reste dans l'app.
- **Un module = un dossier** dans `back/`, déclaré dans `back/mod.rs`,
  avec ses tests à côté du code et une ligne dans l'index ci-dessus.
- **Vérifié contre un autre outil** quand un format ou une norme existe :
  `tests/back.rs` fait relire ce qu'on écrit par python3 (zlib, zipfile,
  hashlib) et par `patch` / `git apply`.
- **Qui s'en sert** : azure-core (`crypto` = `hachage` + `chiffrement`),
  azure-engine (`codec::{deflate, zlib}` = `compression`), Azure Data
  (tableur, dates, hasard, authentification des bases), Azure Map
  (`code`), Azure Docs (`code::coloration`). Les anciens chemins
  (`azure_core::crypto::…`, `azure_data::tableur::…`) restent valables.

## Limites connues

- `compression::compresser` : codes de Huffman fixes, donc un peu plus
  gros que zlib ; lu par tous les décompresseurs.
- `compression::zip` : pas de zip64, pas de chiffrement.
- `texte::diff` : au-delà de 3000 modifications entre deux textes, le
  milieu est donné comme entièrement retiré puis ajouté.
- La crypto n'est pas auditée.

## Restés dans les apps

- Le lexique Rust du Testeur (`langage/rust/lexique.rs`) : il cherche les
  tests, pas les appels ; à rapprocher de `code::rust` un jour.
- La coloration de Note (`code.rs`) : elle sort du texte riche de Note.
- Les clients MySQL / PostgreSQL / SQLite de Data : liés à son modèle
  `Source`.
- Le moteur de formules de Note, Merise et la lecture du SQL de Map.
