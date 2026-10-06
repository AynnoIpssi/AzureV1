# Sécurité d'Azure

Toutes les apps tournent sous le même utilisateur Unix : le système ne les sépare pas entre elles. C'est donc Azure qui le fait.

## 1. Les apps sont enfermées (Landlock)
`AzureApp` enferme l'app dès son démarrage (`azure-core/src/security/sandbox.rs`). Le noyau applique la règle de façon définitive, y compris aux processus que l'app lance.

- **Permis par défaut :** lecture des dossiers système (`/usr`, `/etc`, `/run`…), de son propre dossier et de la police. Lecture et écriture de `/tmp`, `/var/tmp` et `/dev`. Connexion à deux sortes de sockets seulement : ceux des daemons d'Azure (leur dossier privé, voir §3) et celui de Wayland.
- **Interdit :**
  - le reste du dossier personnel, donc les données et les clés d'Azure (`~/.local/share/azure`), les autres apps et la configuration (`~/.config/azure`, systemd) ;
  - le réseau TCP ;
  - l'inspection des autres processus (`ptrace`) ;
  - les signaux vers les processus hors de l'enclos : une app ne peut pas tuer un daemon (noyau 6.12+, Landlock ABI 6) ;
  - tout autre socket unix nommé : le bus D-Bus de session (par lequel l'app ferait lancer n'importe quoi par `systemd --user`), le bus système, les applications X11 (`/tmp/.X11-unix`)… (Landlock ABI 9).
- **Déclaré dans le manifeste :**
  ```ini
  [permissions]
  lecture = ~/Documents
  ecriture = ~/Documents/Boutique
  reseau = true
  stockage = false
  ```
  Une permission qui ouvrirait les dossiers d'Azure, ou l'un de leurs parents (`~`, `~/.local`), est refusée dès la lecture du manifeste.
- **Noyaux plus anciens (espaces de noms) :** ce que Landlock ne sait pas faire sur un vieux noyau, le lanceur le fait (`azure-core/src/security/isolation.rs`). `azure run` lance chaque app installée dans ses propres espaces de noms :
  - le dossier de session (`$XDG_RUNTIME_DIR`) est remplacé par un dossier vide où ne reviennent que le socket Wayland et le dossier des daemons ; X11 et le bus système sont masqués ;
  - l'app est le processus 1 de son espace : elle ne voit ni ne peut viser aucun autre processus, donc aucun signal ;
  - sans permission réseau, elle a son propre réseau vide (ni TCP, ni sockets abstraits).

  Une app installée lancée sans `azure run` sur un noyau sans Landlock ABI 9 se relance elle-même ainsi. Si le système interdit les espaces de noms sans privilège, l'app garde Landlock seul et démarre quand même.
- **Désactiver (`bac_a_sable = false`) :** seulement en développement. Une app installée est toujours enfermée.
- **Lancement des daemons :** une app enfermée ne peut pas lancer azure-provider, sinon les daemons seraient enfermés avec elle. Les daemons sont donc démarrés avant : par `AzureApp` avant de s'enfermer, par `azure run`, ou au login avec `azure autostart on`.
- **Pilotage du provider :** une app enfermée peut seulement lire l'état et demander qu'un service déjà déclaré tourne (`ENSURE`). Enregistrer une commande, arrêter, relancer ou éteindre lui sont refusés : sinon le provider, qui n'est pas enfermé, lancerait pour elle la commande de son choix. Le provider reconnaît une app enfermée à `NoNewPrivs` (tout thread du processus). Les outils d'Azure installés à côté de lui (`azure`, azure-manager) gardent la main même s'ils tournent avec `NoNewPrivs` (service systemd durci).

## 2. Identité des apps
- **À l'installation :** `azure install` enregistre auprès d'azure-manager l'exécutable, son empreinte SHA-256 et le manifeste installé.
  - Une app installée est reconnue à son empreinte : elle peut être déplacée, pas remplacée.
  - Elle garde son id, et donc ses données, d'une installation à l'autre.
  - Le manifeste qui fait foi est celui de l'installation, pas celui que l'app annonce.
- **Vérification par les daemons :** azure-stockage, azure-service et le routeur demandent au manager qui est l'app qui se présente (opcode `IDENTIFY`). Un imposteur est refusé, et une app sans permission `stockage` n'accède pas au stockage.
- **Apps de développement :** elles sont reconnues à leur chemin ou à leur empreinte.
- **Ids écrits à la main (< 1000) :** premier exécutable arrivé.
- **Routeur :** une connexion n'agit qu'au nom de l'app sous laquelle elle s'est enregistrée (abonnement, suivi, désinscription, fenêtres). Un message de plus de 16 Mo coupe la connexion. Un destinataire qui ne lit plus est abandonné après 2 s, et ses messages vont dans sa boîte aux lettres. Le contenu des messages n'est plus écrit dans les journaux.

## 2 bis. L'admin des données (Azure Data)
L'espace privé d'une app n'est lisible que par elle, à une exception voulue : **Azure Data**, l'outil qui montre et modifie les données de toutes les apps.
- **Qui :** l'app installée sous le nom réservé `data` (`azure_stockage::managers::daemon::ADMIN_APP`). Le daemon de stockage ne la reconnaît que si azure-manager la dit **installée** (`azure install`, donc par l'utilisateur) et que son empreinte correspond. Une app de développement qui se nomme `data` n'est pas admin.
- **Quoi :** les requêtes `ADMIN_*` du daemon (lister les apps, leur schéma, exécuter du RsS au nom d'une app, lire ses clés privées). Toute autre app reçoit « Reserve a Azure Data ».
- **Essais et développement :** `stockage_daemon --admin <exécutable>` ou `AZURE_STOCKAGE_ADMIN=a:b` désignent d'autres exécutables admin. Rien de tel n'est posé par l'installation.
- **Conséquence :** qui peut installer une app nommée `data` peut tout lire ; c'est l'utilisateur lui-même (`azure install` est réservé aux outils d'Azure). Azure Data garde aussi les mots de passe de ses connexions externes dans son espace chiffré.
- **Prévu plus tard (choix de Yoann, 2026-10-06) :** remplacer cet accès global par des identifiants que chaque app donne.
- **Bases externes :** les clients MySQL et PostgreSQL d'Azure Data ne chiffrent pas la connexion (pas de TLS). Le mot de passe ne circule jamais en clair (défi-réponse, ou RSA pour `caching_sha2_password`), mais les données si : à garder pour un réseau de confiance.

## 3. Daemons durcis
- **Sockets dans un dossier privé :** `$XDG_RUNTIME_DIR/azure/` (sinon `/tmp/azure-<uid>/`, ou `$AZURE_RUNTIME_DIR`). Chaque daemon le crée en 700 et refuse de démarrer s'il appartient à un autre compte ou si d'autres peuvent y écrire : un autre compte ne peut pas se faire passer pour un daemon. Une app enfermée ne peut pas y écrire.
- **Plafond de connexions :** 64 connexions simultanées par processus client et 1024 en tout, par daemon (au-delà, la connexion est fermée aussitôt).
- **Robustesse :** un thread qui plante en tenant un verrou ne bloque plus les autres (verrou récupéré au lieu de refuser toute requête suivante).
- azure-stockage, azure-service, le routeur et le provider se rendent non inspectables (`PR_SET_DUMPABLE`) : aucun autre processus ne peut lire leur mémoire, donc pas la clé maître.
- Leurs dossiers sont en 700 et leurs fichiers en 600.
- Le manager reste inspectable, parce qu'azure-service vérifie son exécutable. Une app enfermée ne peut quand même pas l'inspecter.

## 4. Chiffrement au repos (ChaCha20-Poly1305)
- **Fichiers concernés :** stockage (déjà chiffré), flux persistants, messages en attente du routeur, registre du manager, événements et compteurs du manager.
- **Vérification :** en plus des vecteurs officiels (NIST, RFC 4231, 7914, 8439), chaque algorithme est comparé à une implémentation indépendante (python `cryptography`) aux cas limites : bords de remplissage de SHA-256, réductions modulaires de Poly1305 (entrées de la RFC 8439 A.3), toutes les longueurs de ChaCha20-Poly1305. Comparaison d'étiquette en temps constant (protégée contre l'optimisation du compilateur), aléa par `getrandom`.
- **Clés :** une clé par daemon (`cle.bin`, droits 600), illisible par les apps enfermées.
- **Intégrité :** un fichier modifié ou échangé contre un autre est refusé.

## Limites qui ne dépendent pas du code
- La crypto est écrite à la main : elle est vérifiée (voir §4), mais seul un audit par des spécialistes extérieurs peut la certifier.
- Un programme qui n'utilise pas `AzureApp`, ou que l'utilisateur lance lui-même hors d'Azure, n'est pas enfermé. Il a tous les droits de l'utilisateur, comme tout programme.
- Il n'existe pas de signature par une autorité externe : l'identité est l'empreinte enregistrée par l'installateur local.
- Sur un système qui n'a ni Landlock récent ni espaces de noms sans privilège (certaines distributions les interdisent), la protection se réduit à ce que Landlock y offre ; l'app le signale au tableau de bord.

Tests : `azure-core/tests/security.rs` (fichiers, réseau, signaux, sockets, vrai bus, espaces de noms), `azure-core/tests/paths.rs`, `azure-core/tests/limits.rs`, `azure-stockage/tests/crypto_oracle.rs`, `azure-rooter/tests/securite.rs`, `azure-provider/tests/securite.rs`, `azure-foundation/tests/app_sandbox.rs`, `azure-manager/tests/identity.rs`, `azure-cli/tests/cli.rs`.
