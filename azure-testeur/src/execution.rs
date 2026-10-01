// Lance les etapes (`Langage::commande`) dans un thread a part : la sortie
// est lue ligne par ligne, le lecteur du langage en tire les resultats, la
// fenetre redessine quand `version` change. Une seule execution a la fois ;
// `arreter` tue tout le groupe de processus (cargo et les tests).
use crate::langage::{Etape, Evenement, Langage, Statut};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

/// Lignes de sortie gardees.
pub const LIGNES: usize = 4000;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resultat {
    pub statut: Statut,
    /// Sortie de l'echec.
    pub sortie: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Execution {
    pub en_cours: bool,
    /// Ce qui tourne (ou a tourne en dernier).
    pub titre: String,
    pub etape: usize,
    pub nb_etapes: usize,
    pub lignes: Vec<String>,
    /// Par cle de test (voir `Test::cle`).
    pub resultats: HashMap<String, Resultat>,
    /// Resume de la derniere execution.
    pub bilan: String,
    pub echec_commande: bool,
    /// Change a chaque nouveaute, sortie comprise.
    pub version: u64,
    /// Change quand ce que montre l'ecran hors console change : un
    /// resultat, le debut ou la fin (voir `ecran::Rafraichir`).
    pub version_resultats: u64,
    /// Groupe de processus de la commande en cours.
    groupe: Option<i32>,
    arret: bool,
    debut: Option<Instant>,
}

impl Execution {
    pub fn statut(&self, cle: &str) -> Statut {
        self.resultats.get(cle).map(|r| r.statut).unwrap_or_default()
    }

    fn noter(&mut self, ligne: String) {
        self.lignes.push(ligne);
        // Par paquets : pas de decalage de tout le tableau a chaque ligne.
        if self.lignes.len() > LIGNES + 500 {
            let trop = self.lignes.len() - LIGNES;
            self.lignes.drain(..trop);
        }
        self.version += 1;
    }
}

#[derive(Clone, Default)]
pub struct Lanceur {
    pub etat: Arc<Mutex<Execution>>,
}

impl Lanceur {
    pub fn lire(&self) -> MutexGuard<'_, Execution> {
        self.etat.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn version(&self) -> u64 {
        self.lire().version
    }

    /// Lance `etapes` l'une apres l'autre ; `attendus` (cles) passent « en
    /// cours ». Erreur si une execution tourne deja.
    pub fn lancer(&self, projet: PathBuf, langage: Arc<dyn Langage>, titre: &str, etapes: Vec<Etape>, attendus: Vec<String>) -> Result<(), String> {
        {
            let mut e = self.lire();
            if e.en_cours {
                return Err(format!("« {} » tourne encore", e.titre));
            }
            e.en_cours = true;
            e.arret = false;
            e.titre = titre.to_string();
            e.etape = 0;
            e.nb_etapes = etapes.len();
            e.echec_commande = false;
            e.bilan.clear();
            e.lignes.clear();
            e.debut = Some(Instant::now());
            for cle in &attendus {
                e.resultats.insert(cle.clone(), Resultat { statut: Statut::EnCours, sortie: Vec::new() });
            }
            e.version += 1;
            e.version_resultats += 1;
        }
        let lanceur = self.clone();
        std::thread::spawn(move || {
            for (i, etape) in etapes.iter().enumerate() {
                {
                    let mut e = lanceur.lire();
                    if e.arret {
                        break;
                    }
                    e.etape = i + 1;
                    let titre = etape.titre.clone();
                    e.noter(format!("$ {titre}"));
                }
                if let Err(err) = lanceur.executer(&projet, langage.as_ref(), etape) {
                    let mut e = lanceur.lire();
                    e.echec_commande = true;
                    e.noter(format!("erreur : {err}"));
                }
            }
            let mut e = lanceur.lire();
            // Ce qui n'a pas donne de resultat n'a pas tourne.
            let mut compte: HashMap<Statut, usize> = HashMap::new();
            for cle in &attendus {
                if let Some(r) = e.resultats.get_mut(cle) {
                    if r.statut == Statut::EnCours {
                        r.statut = Statut::Erreur;
                    }
                    *compte.entry(r.statut).or_default() += 1;
                }
            }
            let duree = e.debut.map(|d| d.elapsed().as_secs_f32()).unwrap_or(0.0);
            let n = |s| compte.get(&s).copied().unwrap_or(0);
            let mut bilan = format!("{} réussi(s), {} échoué(s)", n(Statut::Reussi), n(Statut::Echoue));
            if n(Statut::Ignore) > 0 {
                bilan.push_str(&format!(", {} ignoré(s)", n(Statut::Ignore)));
            }
            if n(Statut::Erreur) > 0 {
                bilan.push_str(&format!(", {} non exécuté(s)", n(Statut::Erreur)));
            }
            bilan.push_str(&format!(" · {duree:.1} s"));
            if e.arret {
                bilan = format!("Arrêté · {bilan}");
            }
            e.bilan = bilan;
            e.en_cours = false;
            e.groupe = None;
            e.version += 1;
            e.version_resultats += 1;
        });
        Ok(())
    }

    fn executer(&self, projet: &std::path::Path, langage: &dyn Langage, etape: &Etape) -> Result<(), String> {
        let mut cmd = langage.commande(projet, etape)?;
        // Sortie et erreurs dans le meme tuyau, dans l'ordre (cargo ecrit
        // « Running » sur l'une et les resultats sur l'autre).
        let (lecture, ecriture) = std::io::pipe().map_err(|e| e.to_string())?;
        let copie = ecriture.try_clone().map_err(|e| e.to_string())?;
        cmd.stdout(ecriture).stderr(copie).stdin(std::process::Stdio::null()).process_group(0);
        // Priorite basse : cargo compile sur tous les coeurs, la fenetre (et
        // le reste du bureau) doit rester fluide.
        // SAFETY : un seul appel systeme, entre fork et exec.
        unsafe {
            cmd.pre_exec(|| {
                libc::nice(10);
                Ok(())
            });
        }
        let mut enfant = cmd.spawn().map_err(|e| format!("lancement impossible : {e}"))?;
        drop(cmd);
        self.lire().groupe = Some(enfant.id() as i32);
        let mut lecteur = langage.lecteur(projet, etape);
        for ligne in BufReader::new(lecture).lines() {
            let Ok(ligne) = ligne else { break };
            let evenements = lecteur.ligne(&ligne);
            let mut e = self.lire();
            appliquer(&mut e, evenements);
            e.noter(ligne);
        }
        let statut = enfant.wait().map_err(|e| e.to_string())?;
        let mut e = self.lire();
        appliquer(&mut e, lecteur.fin());
        e.groupe = None;
        if !statut.success() && !e.arret {
            e.echec_commande = true;
            let code = statut.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".to_string());
            e.noter(format!("(code de sortie {code})"));
        }
        Ok(())
    }

    /// Arrete l'execution en cours (et les etapes suivantes).
    pub fn arreter(&self) {
        let mut e = self.lire();
        e.arret = true;
        if let Some(g) = e.groupe {
            // SAFETY : un simple signal au groupe cree par `process_group(0)`.
            unsafe {
                libc::kill(-g, libc::SIGTERM);
            }
        }
        e.version += 1;
        e.version_resultats += 1;
    }
}

fn appliquer(e: &mut Execution, evenements: Vec<Evenement>) {
    for ev in evenements {
        match ev {
            Evenement::Statut { cle, statut } => {
                let r = e.resultats.entry(cle).or_default();
                r.statut = statut;
                if statut != Statut::Echoue {
                    r.sortie.clear();
                }
            }
            Evenement::Sortie { cle, lignes } => e.resultats.entry(cle).or_default().sortie = lignes,
        }
        e.version += 1;
        e.version_resultats += 1;
    }
}
