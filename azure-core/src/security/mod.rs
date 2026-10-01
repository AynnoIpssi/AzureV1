// La securite d'Azure. Toutes les apps tournent sous le meme utilisateur
// Unix : le systeme ne les separe pas entre elles. Azure le fait donc
// lui-meme :
// - `sandbox` : chaque app est enfermee (Landlock) : elle ne voit que son
//   dossier, les dossiers systeme et ce que son manifeste declare ; le
//   reseau est coupe sauf permission.
// - `hardening` : les daemons ne se laissent pas lire la memoire, et leurs
//   dossiers sont prives.
// - `vault` : chiffrement au repos des fichiers des daemons.
// - `isolation` : espaces de noms poses par le lanceur (dossier de session
//   masque, processus et reseau a part), pour les noyaux ou Landlock ne
//   couvre pas tout.
// - `termination` : `kill` arrete proprement une app, meme isolee.
pub mod hardening;
pub mod isolation;
pub mod limits;
pub mod registry;
pub mod sandbox;
pub mod termination;
pub mod vault;
