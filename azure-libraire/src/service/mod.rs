// Les services de la librairie : des methodes pretes a etre servies aux
// autres apps par azure-service (`[provide ...]` / `[use ...]`).
//
// - Un service = un fichier ici : un nom, une description et ses methodes.
//   Une methode recoit une `Valeur` (ses arguments, une table de champs)
//   et renvoie une `Valeur` ou un message d'erreur.
// - Le nom complet d'une methode, celui du manifeste, est
//   `<service>-<methode>` : `diff-lignes`, `code-colorer`.
// - Une methode ne fait que calculer sur ce qu'on lui donne : elle n'ouvre
//   aucun fichier (un service tourne hors du bac a sable de l'app qui
//   l'appelle ; lire un chemin recu lui ferait lire a sa place).
//
// La librairie ne sert rien elle-meme : `AzureApp::servir` (fondation)
// branche un service sur azure-service. Reference : SERVICE.md.
pub mod archive;
pub mod code;
pub mod diff;
pub mod empreinte;
pub mod encodage;
pub mod tableur;
pub mod temps;
pub mod valeur;

pub use valeur::Valeur;

pub struct Methode {
    pub nom: &'static str,
    pub description: &'static str,
    /// Les champs attendus, pour la documentation : `avant, apres, contexte?`.
    pub arguments: &'static str,
    /// Ce qui revient, pour la documentation.
    pub reponse: &'static str,
    pub appeler: fn(&Valeur) -> Result<Valeur, String>,
}

pub struct Service {
    pub nom: &'static str,
    pub description: &'static str,
    pub methodes: &'static [Methode],
}

impl Service {
    /// `<service>-<methode>` : le nom a declarer dans le manifeste.
    pub fn nom_complet(&self, methode: &Methode) -> String {
        format!("{}-{}", self.nom, methode.nom)
    }

    pub fn methode(&self, nom: &str) -> Option<&'static Methode> {
        self.methodes.iter().find(|m| m.nom == nom)
    }
}

static SERVICES: &[&Service] = &[&diff::SERVICE, &code::SERVICE, &tableur::SERVICE, &archive::SERVICE, &empreinte::SERVICE, &encodage::SERVICE, &temps::SERVICE];

/// Tous les services de la librairie.
pub fn services() -> &'static [&'static Service] {
    SERVICES
}

pub fn service(nom: &str) -> Option<&'static Service> {
    SERVICES.iter().copied().find(|s| s.nom == nom)
}

/// La methode de nom complet `<service>-<methode>`.
pub fn methode(nom_complet: &str) -> Option<&'static Methode> {
    SERVICES.iter().find_map(|s| nom_complet.strip_prefix(s.nom)?.strip_prefix('-').and_then(|m| s.methode(m)))
}

/// Appelle une methode par son nom complet, sans passer par azure-service
/// (essais, ligne de commande).
pub fn appeler(nom_complet: &str, arguments: &Valeur) -> Result<Valeur, String> {
    let m = methode(nom_complet).ok_or_else(|| format!("méthode inconnue : {nom_complet}"))?;
    (m.appeler)(arguments)
}

/// Les sections `[provide ...]` a mettre dans le manifeste d'une app qui
/// sert ces services. `tache` : la tache de fond (`[service <tache>]`) qui
/// les sert, pour qu'Azure la lance au premier appel ; vide : servies
/// seulement quand l'app tourne. `acces` : `public = true` ou `to = a, b`.
pub fn manifeste(services: &[&Service], tache: &str, acces: &str) -> String {
    let mut out = String::new();
    for s in services {
        for m in s.methodes {
            out.push_str(&format!("[provide {}]\n{acces}\n", s.nom_complet(m)));
            if !tache.is_empty() {
                out.push_str(&format!("service = {tache}\n"));
            }
            out.push_str(&format!("description = {} ({} -> {})\n\n", m.description, m.arguments, m.reponse));
        }
    }
    out
}
