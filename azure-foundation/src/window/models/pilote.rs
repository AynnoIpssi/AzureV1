// Une fenetre sans ecran, pilotee par un essai (voir `crate::essai`) : l'app
// tourne pour de vrai (ses clics, ses tics, ses pages), sa fenetre n'est
// qu'une image en memoire. Les commandes arrivent par un socket
// (`AZURE_PILOTE`) et passent par la meme boucle que la vraie fenetre
// (`sur_evenement` / `sur_tic`).
use crate::essai::protocole::{champs, ligne};
use crate::event::models::keys::BTN_LEFT;
use crate::ui::models::ui_node::UiNode;
use crate::ui::services::interact::{self, KeyboardLayout};
use crate::window::models::header_bar::{ButtonLayout, HeaderBar};
use crate::window::models::hote::Hote;
use crate::window::models::window::{content_box, redraw, sur_evenement, sur_tic, terminer, AzureWindow, LoopState};
use azure_core::rules::window_event::WindowEvent;
use std::cell::Cell;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::time::Duration;

/// Le pilote n'a ni compositeur ni presse-papiers du systeme.
struct EnMemoire {
    presse_papiers: Option<String>,
}

impl Hote for EnMemoire {
    fn presse_papiers(&mut self) -> Option<String> {
        self.presse_papiers.clone()
    }

    fn ecrire_presse_papiers(&mut self, texte: &str) -> Result<(), String> {
        self.presse_papiers = Some(texte.to_string());
        Ok(())
    }
}

/// L'id d'un element (`#id` du rsH), s'il en a un.
pub(crate) fn id_de(n: &UiNode) -> Option<&str> {
    let id = match n {
        UiNode::Button(b) => b.id.as_str(),
        UiNode::TextArea(t) => t.id.as_str(),
        UiNode::Control(c) => c.id.as_str(),
        _ => "",
    };
    let id = if id.is_empty() { n.decoration().anchor.as_str() } else { id };
    (!id.is_empty()).then_some(id)
}

/// Le texte qu'un element montre.
pub(crate) fn texte_de(n: &UiNode) -> Option<String> {
    let t = match n {
        UiNode::Label(l) => l.text.clone(),
        UiNode::Button(b) => b.text.clone(),
        UiNode::TextArea(t) if !t.text.is_empty() => t.text.clone(),
        UiNode::TextArea(t) => t.placeholder.clone(),
        UiNode::Control(c) if !c.options.is_empty() => c.selected_label().to_string(),
        UiNode::Control(c) => c.label.clone(),
        _ => String::new(),
    };
    (!t.trim().is_empty()).then_some(t)
}

// Les elements visibles a l'ecran : (noeud, partie visible). Le parcours
// donne la boite de l'element et la zone ou ses parents le laissent se
// dessiner : la partie visible est leur intersection.
fn visibles(s: &LoopState, f: &mut dyn FnMut(&UiNode, crate::layout::managers::layout_manager::Rect)) {
    let page = content_box(s);
    interact::walk_with_paths(&s.event.ui_nodes, page, &mut |n, own, clip, _| {
        let (x0, y0) = (own.0.max(clip.0), own.1.max(clip.1));
        let x1 = (own.0 + own.2 as i32).min(clip.0 + clip.2 as i32);
        let y1 = (own.1 + own.3 as i32).min(clip.1 + clip.3 as i32);
        if x1 > x0 && y1 > y0 && n.decoration().visible {
            f(n, (x0, y0, (x1 - x0) as u32, (y1 - y0) as u32));
        }
    });
}

fn centre_de(s: &LoopState, id: &str) -> Result<(i32, i32), String> {
    let mut trouve = None;
    visibles(s, &mut |n, v| {
        if trouve.is_none() && id_de(n) == Some(id) {
            trouve = Some((v.0 + v.2 as i32 / 2, v.1 + v.3 as i32 / 2));
        }
    });
    trouve.ok_or_else(|| {
        let mut ids = Vec::new();
        visibles(s, &mut |n, _| ids.extend(id_de(n).map(str::to_string)));
        ids.truncate(60);
        format!("pas de #{id} à l'écran (visibles : {})", ids.join(", "))
    })
}

fn centre_boite(s: &LoopState, toile: &str, boite: &str) -> Result<(i32, i32), String> {
    use crate::ui::models::toile::{mesure_par_defaut, Toile};
    let mut trouve = Err(format!("pas de toile #{toile} à l'écran"));
    visibles(s, &mut |n, v| {
        if let UiNode::Control(c) = n
            && c.id == toile
            && let Some(t) = c.toile.as_deref()
        {
            trouve = match t.dessin.boites.iter().find(|b| b.id == boite) {
                Some(b) => {
                    let (w, h) = Toile::taille(b, &mesure_par_defaut);
                    let (x, y) = t.vers_ecran(v, b.x + w / 2.0, b.y + h / 2.0);
                    Ok((x as i32, y as i32))
                }
                None => Err(format!("#{toile} : pas de boîte « {boite} » (boîtes : {})", t.dessin.boites.iter().map(|b| b.id.as_str()).collect::<Vec<_>>().join(", "))),
            };
        }
    });
    trouve
}

fn touche(nom: &str) -> Option<u32> {
    Some(match nom {
        "entree" => 28,
        "echap" => 1,
        "tab" => 15,
        "espace" => 57,
        "retour" => 14,
        "suppr" => 111,
        "haut" => 103,
        "bas" => 108,
        "gauche" => 105,
        "droite" => 106,
        _ => return None,
    })
}

const CTRL: u32 = 29;
const KEY_V: u32 = 47;

/// Ce que l'app a coute pendant l'essai (voir `essai::Mesures`).
struct Compteurs {
    debut: std::time::Instant,
    dessins: u64,
    temps_dessins: Duration,
    dessin_max: Duration,
    /// Chaque action (clic, saisie, touche, molette) et son temps de
    /// reponse : de la commande a la fin de son redessin.
    actions: Vec<(String, Duration)>,
}

struct Pilote {
    s: LoopState,
    hote: EnMemoire,
    layout: Cell<KeyboardLayout>,
    compteurs: Compteurs,
}

/// Temps de processeur (programme + noyau), memoire max, reveils : de ce
/// processus (`RUSAGE_SELF`) ou de ses enfants attendus (`RUSAGE_CHILDREN`).
fn rusage(qui: libc::c_int) -> (Duration, u64, u64) {
    // SAFETY : structure C sans pointeur, remplie par getrusage.
    let mut r: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY : pointeur valide.
    unsafe { libc::getrusage(qui, &mut r) };
    let t = |v: libc::timeval| Duration::new(v.tv_sec.max(0) as u64, (v.tv_usec.max(0) as u32) * 1000);
    (t(r.ru_utime) + t(r.ru_stime), r.ru_maxrss.max(0) as u64 * 1024, (r.ru_nvcsw + r.ru_nivcsw).max(0) as u64)
}

impl Pilote {
    /// Un evenement, puis un tic : la fenetre redessine a la fin de l'un
    /// comme de l'autre ; s'il y a eu une image, son cout (le passage
    /// entier) compte comme un dessin.
    fn evenement(&mut self, e: WindowEvent) -> bool {
        let (avant, debut) = (self.s.perf.redessins, std::time::Instant::now());
        let continuer = sur_evenement(&mut self.s, &mut self.hote, e, &self.layout);
        self.compter(avant, debut);
        continuer
    }

    fn tic(&mut self) -> bool {
        let (avant, debut) = (self.s.perf.redessins, std::time::Instant::now());
        let continuer = sur_tic(&mut self.s, &mut self.hote, &self.layout);
        self.compter(avant, debut);
        continuer
    }

    fn compter(&mut self, avant: u32, debut: std::time::Instant) {
        if self.s.perf.redessins != avant {
            let d = debut.elapsed();
            let c = &mut self.compteurs;
            c.dessins += 1;
            c.temps_dessins += d;
            c.dessin_max = c.dessin_max.max(d);
        }
    }

    fn tics(&mut self, n: usize) {
        for _ in 0..n {
            self.tic();
        }
    }

    fn mesures(&self) -> String {
        let (cpu, memoire, reveils) = rusage(libc::RUSAGE_SELF);
        let (cpu_enfants, memoire_enfants, _) = rusage(libc::RUSAGE_CHILDREN);
        let c = &self.compteurs;
        let ms = |d: Duration| format!("{:.3}", d.as_secs_f64() * 1000.0);
        let mut out = vec![
            format!("duree_ms={}", ms(c.debut.elapsed())),
            format!("cpu_ms={}", ms(cpu)),
            format!("cpu_enfants_ms={}", ms(cpu_enfants)),
            format!("memoire_max={memoire}"),
            format!("memoire_enfants_max={memoire_enfants}"),
            format!("reveils={reveils}"),
            format!("dessins={}", c.dessins),
            format!("dessins_ms={}", ms(c.temps_dessins)),
            format!("dessin_max_ms={}", ms(c.dessin_max)),
        ];
        out.extend(c.actions.iter().map(|(a, d)| format!("action={a}|{}", ms(*d))));
        out.join("\n")
    }

    /// Une commande ; `Err` : a renvoyer comme erreur.
    fn commande(&mut self, c: &[String]) -> Result<Option<String>, String> {
        let arg = |i: usize| c.get(i).map(String::as_str).ok_or_else(|| format!("{} : argument {i} manquant", c[0]));
        match c[0].as_str() {
            "clic" => {
                let (x, y) = centre_de(&self.s, arg(1)?)?;
                self.evenement(WindowEvent::WindowMouseMove(x, y));
                self.evenement(WindowEvent::WindowMouseButton(BTN_LEFT, true));
                self.evenement(WindowEvent::WindowMouseButton(BTN_LEFT, false));
                self.tics(1);
            }
            // Une liste deroulante : l'ouvrir, puis cliquer l'option (valeur
            // ou libelle), comme quelqu'un - l'app recoit le clic.
            "choisir" => {
                let (id, valeur) = (arg(1)?, arg(2)?);
                let (x, y) = centre_de(&self.s, id)?;
                self.evenement(WindowEvent::WindowMouseMove(x, y));
                self.evenement(WindowEvent::WindowMouseButton(BTN_LEFT, true));
                self.evenement(WindowEvent::WindowMouseButton(BTN_LEFT, false));
                self.tics(1);
                let mut cible = Err(format!("#{id} n'est pas une liste ouverte"));
                visibles(&self.s, &mut |n, v| {
                    if let UiNode::Control(c) = n
                        && c.id == id
                        && c.open
                    {
                        let rangs = crate::ui::services::draw_control::option_rows(c, v);
                        cible = match c.options.iter().position(|(val, lib)| val == valeur || lib == valeur) {
                            Some(i) => rangs.get(i).map(|r| (r.0 + r.2 as i32 / 2, r.1 + r.3 as i32 / 2)).ok_or_else(|| "option hors de la liste".to_string()),
                            None => Err(format!("#{id} : pas d'option « {valeur} » ({})", c.options.iter().map(|o| o.1.as_str()).collect::<Vec<_>>().join(", "))),
                        };
                    }
                });
                let (ox, oy) = cible?;
                self.evenement(WindowEvent::WindowMouseMove(ox, oy));
                self.evenement(WindowEvent::WindowMouseButton(BTN_LEFT, true));
                self.evenement(WindowEvent::WindowMouseButton(BTN_LEFT, false));
                self.tics(1);
            }
            "survoler" => {
                let (x, y) = centre_de(&self.s, arg(1)?)?;
                self.evenement(WindowEvent::WindowMouseMove(x, y));
            }
            "souris" => {
                let x: i32 = arg(1)?.parse().map_err(|_| "x illisible".to_string())?;
                let y: i32 = arg(2)?.parse().map_err(|_| "y illisible".to_string())?;
                self.evenement(WindowEvent::WindowMouseMove(x, y));
            }
            "remplir" => {
                let (id, valeur) = (arg(1)?, arg(2)?);
                if !remplir(&mut self.s.event.ui_nodes, id, valeur)? {
                    centre_de(&self.s, id)?;
                    return Err(format!("#{id} n'est pas un champ"));
                }
                self.s.dirty = true;
                self.tics(1);
            }
            "taper" => {
                // Comme un collage au clavier : passe par la meme saisie.
                self.hote.presse_papiers = Some(arg(1)?.to_string());
                self.evenement(WindowEvent::WindowKeyPress(CTRL, true));
                self.evenement(WindowEvent::WindowKeyPress(KEY_V, true));
                self.evenement(WindowEvent::WindowKeyPress(KEY_V, false));
                self.evenement(WindowEvent::WindowKeyPress(CTRL, false));
                self.tics(1);
            }
            "touche" => {
                let k = touche(arg(1)?).ok_or_else(|| format!("touche inconnue : {} (entree, echap, tab, espace, retour, suppr, haut, bas, gauche, droite)", c[1]))?;
                self.evenement(WindowEvent::WindowKeyPress(k, true));
                self.evenement(WindowEvent::WindowKeyPress(k, false));
                self.tics(1);
            }
            "molette" => {
                let (x, y) = centre_de(&self.s, arg(1)?)?;
                let dy: f64 = arg(2)?.parse().map_err(|_| "molette : nombre attendu".to_string())?;
                self.evenement(WindowEvent::WindowMouseMove(x, y));
                self.evenement(WindowEvent::WindowScroll(dy));
                // Le defilement glisse sur quelques tics.
                self.tics(60);
            }
            "tics" => self.tics(arg(1)?.parse().map_err(|_| "tics : nombre attendu".to_string())?),
            // Le centre d'une boite d'une toile, a l'ecran : `x\ty`.
            "boite" => {
                let (x, y) = centre_boite(&self.s, arg(1)?, arg(2)?)?;
                return Ok(Some(format!("{x}\t{y}")));
            }
            // Appui en (x1, y1), quelques pas, relache en (x2, y2).
            "glisser" => {
                let n = |i| arg(i).and_then(|v| v.parse::<i32>().map_err(|_| "glisser : nombres attendus".to_string()));
                let (x1, y1, x2, y2) = (n(1)?, n(2)?, n(3)?, n(4)?);
                self.evenement(WindowEvent::WindowMouseMove(x1, y1));
                self.evenement(WindowEvent::WindowMouseButton(BTN_LEFT, true));
                for k in 1..=6 {
                    self.evenement(WindowEvent::WindowMouseMove(x1 + (x2 - x1) * k / 6, y1 + (y2 - y1) * k / 6));
                }
                self.evenement(WindowEvent::WindowMouseButton(BTN_LEFT, false));
                self.tics(1);
            }
            "mesures" => return Ok(Some(self.mesures())),
            "texte" => {
                let mut out = Vec::new();
                visibles(&self.s, &mut |n, _| out.extend(texte_de(n)));
                return Ok(Some(out.join("\n")));
            }
            "ids" => {
                let mut out = Vec::new();
                visibles(&self.s, &mut |n, _| out.extend(id_de(n).map(str::to_string)));
                return Ok(Some(out.join("\n")));
            }
            "capture" => {
                redraw(&mut self.s);
                let c = &self.s.canvas;
                let mut ppm = format!("P6\n{} {}\n255\n", c.width, c.height).into_bytes();
                for px in c.buffer.chunks(4) {
                    ppm.extend_from_slice(&[px[2], px[1], px[0]]);
                }
                std::fs::write(arg(1)?, ppm).map_err(|e| format!("capture : {e}"))?;
            }
            "taille" => {
                let n = |i| arg(i).and_then(|v| v.parse::<i32>().map_err(|_| "taille : nombres attendus".to_string()));
                self.evenement(WindowEvent::WindowResize(n(1)?, n(2)?));
                self.tics(1);
            }
            autre => return Err(format!("commande inconnue : {autre}")),
        }
        Ok(None)
    }
}

/// Donne `valeur` au champ `#id` (zone de texte, case, choix, curseur).
/// `Ok(false)` : pas de champ avec cet id.
fn remplir(nodes: &mut [UiNode], id: &str, valeur: &str) -> Result<bool, String> {
    for n in nodes {
        match n {
            UiNode::TextArea(t) if t.id == id => {
                t.text = valeur.to_string();
                t.cursor = valeur.chars().count();
                if let Some(r) = &mut t.rich {
                    r.styles = vec![Default::default(); t.cursor];
                }
                return Ok(true);
            }
            UiNode::Control(c) if c.id == id => {
                if !c.options.is_empty() {
                    let i = c.options.iter().position(|(v, l)| v == valeur || l == valeur).ok_or_else(|| format!("#{id} : pas d'option « {valeur} » ({})", c.options.iter().map(|o| o.1.as_str()).collect::<Vec<_>>().join(", ")))?;
                    c.selected = i;
                } else if let Ok(v) = valeur.parse::<f64>() {
                    c.value = v;
                } else {
                    c.checked = matches!(valeur, "true" | "oui" | "1");
                }
                return Ok(true);
            }
            UiNode::Container(c) => {
                if remplir(&mut c.children, id, valeur)? {
                    return Ok(true);
                }
            }
            _ => {}
        }
    }
    Ok(false)
}

/// La boucle sans ecran : un tic toutes les 16 ms, les commandes entre deux.
pub(crate) fn piloter(fenetre: AzureWindow, socket: PathBuf) {
    let (w, h) = std::env::var("AZURE_PILOTE_TAILLE")
        .ok()
        .and_then(|t| t.split_once('x').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))))
        .unwrap_or_else(|| fenetre.taille_demandee().unwrap_or((1280, 820)));
    let header = HeaderBar::new(String::new(), None, ButtonLayout::mac(), true);
    let compteurs = Compteurs { debut: std::time::Instant::now(), dessins: 0, temps_dessins: Duration::ZERO, dessin_max: Duration::ZERO, actions: Vec::new() };
    let mut p = Pilote { s: fenetre.en_etat(w, h, header, true), hote: EnMemoire { presse_papiers: None }, layout: Cell::new(KeyboardLayout::Qwerty), compteurs };
    let _ = std::fs::remove_file(&socket);
    let ecoute = match UnixListener::bind(&socket) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("AzureWindow (pilote) : {} : {e}", socket.display());
            return;
        }
    };
    let Ok((flux, _)) = ecoute.accept() else { return };
    let _ = flux.set_read_timeout(Some(Duration::from_millis(16)));
    let (Ok(lecture), mut ecriture) = (flux.try_clone(), flux) else { return };
    let mut lecteur = BufReader::new(lecture);
    redraw(&mut p.s);
    let mut tampon = String::new();
    loop {
        if !p.tic() {
            break;
        }
        match lecteur.read_line(&mut tampon) {
            Ok(0) => break,
            Ok(_) if tampon.ends_with('\n') => {
                let c = champs(&tampon);
                tampon.clear();
                if c.first().is_some_and(|x| x == "fermer") {
                    let _ = writeln!(ecriture, "ok");
                    break;
                }
                let debut = std::time::Instant::now();
                let resultat = p.commande(&c);
                // Le temps de reponse d'une action de la personne.
                if resultat.is_ok() && matches!(c[0].as_str(), "clic" | "remplir" | "taper" | "touche" | "molette" | "glisser" | "choisir") {
                    let quoi = format!("{} {}", c[0], c.get(1).map(String::as_str).unwrap_or("")).trim().to_string();
                    p.compteurs.actions.push((quoi, debut.elapsed()));
                }
                let reponse = match resultat {
                    Ok(None) => "ok".to_string(),
                    Ok(Some(donnees)) => ligne(&["ok", &donnees]),
                    Err(e) => ligne(&["erreur", &e]),
                };
                if writeln!(ecriture, "{reponse}").is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(_) => break,
        }
    }
    terminer(&mut p.s);
    let _ = std::fs::remove_file(&socket);
}
