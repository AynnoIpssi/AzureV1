// « Tout lancer » sur Azure, comme dans l'app : cargo compile et lance tous
// les tests pendant qu'une boucle a 60 Hz fait exactement ce que fait la
// fenetre (tic, `on_tick` de main.rs, redessin). On garde chaque tic : le
// pire, le 99e centile, les gels. Le retard du reveil (sleep de 16 ms qui
// dure plus) montre si la machine elle-meme est saturee par cargo.
//
// Long (compile tout Azure) : ignore par defaut.
//   cargo test --release -p azure-testeur --test banc -- --ignored tout_lancer --nocapture
// AZURE_BANC_DUREE=<s> : duree max (120 s par defaut), puis « Arreter ».
use crate::commun::*;
use azure_foundation::ui::services::interact;
use azure_testeur::clics::{cliquer, ranger};
use azure_testeur::ecran::{contexte, Rafraichir};
use azure_testeur::langage::Statut;
use std::time::{Duration, Instant};

const Z: &str = "lancer";

/// Temps CPU de ce thread (a comparer au temps ecoule : s'il est bien plus
/// court, le thread attendait - verrou ou processeur pris par cargo).
fn temps_cpu() -> Duration {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY : ecrit seulement dans `ts`.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}

fn centile(v: &mut [Duration], p: f64) -> Duration {
    v.sort();
    v.get(((v.len() as f64 - 1.0) * p) as usize).copied().unwrap_or_default()
}

pub fn mesurer() {
    eprintln!("\n=== tout lancer (Azure) ===");
    let Some(env) = environnement() else { return };
    let duree_max = Duration::from_secs(std::env::var("AZURE_BANC_DUREE").ok().and_then(|s| s.parse().ok()).unwrap_or(120));
    let t = testeur(Some(env));
    let mut nodes = construire(&t);
    let (_, d) = chrono(|| cliquer(&t, "tout-lancer", &Valeurs::de(&nodes)));
    noter(Z, "clic « Tout lancer »", true, d);
    nodes = construire(&t);

    let (mut tics, mut retards, mut redessins) = (Vec::new(), Vec::new(), Vec::new());
    let mut rafraichir = Rafraichir::new();
    let mut fin_dessinee = false;
    let debut = Instant::now();
    let mut pire_etape = (Duration::ZERO, String::new());
    let mut details: Vec<(Duration, String)> = Vec::new();
    loop {
        let tic = Instant::now();
        // with_context : les champs, a chaque tic.
        let valeurs = Valeurs::de(&nodes);
        // on_tick de main.rs.
        let (en_cours, etape) = {
            let e = t.lanceur.lire();
            (e.en_cours, e.lignes.last().cloned().unwrap_or_default())
        };
        if rafraichir.maintenant(&t) {
            let (r, cpu) = (Instant::now(), temps_cpu());
            let (_, a) = chrono(|| ranger(&t, &valeurs));
            let (_, b) = chrono(|| drop(t.lanceur.lire()));
            let (_, c) = chrono(|| drop(contexte(&t)));
            let (mut neuf, d) = chrono(|| construire(&t));
            interact::carry_scroll(&nodes, &mut neuf);
            let (_, e) = chrono(|| dessiner(&neuf));
            nodes = neuf;
            let (mur, cpu) = (r.elapsed(), temps_cpu() - cpu);
            details.push((mur, format!("cpu {cpu:.0?} ; ranger {a:.1?}, verrou {b:.1?}, donnees {c:.1?}, construire {d:.1?}, dessiner {e:.1?} ; sortie : {etape:.50}")));
            redessins.push(mur);
            fin_dessinee = !en_cours;
        }
        let duree = tic.elapsed();
        if duree > pire_etape.0 {
            pire_etape = (duree, etape);
        }
        tics.push(duree);
        // Fini, et le dernier redessin (le bilan) est fait.
        if fin_dessinee {
            break;
        }
        if debut.elapsed() > duree_max {
            eprintln!("  ({duree_max:?} ecoulees : Arreter)");
            t.lanceur.arreter();
            attendre(&t, Duration::from_secs(30));
            break;
        }
        let s = Instant::now();
        std::thread::sleep(IMAGE);
        retards.push(s.elapsed().saturating_sub(IMAGE));
    }
    let bilan = t.lanceur.lire().bilan.clone();
    {
        // Ce qui n'a pas reussi (et la ligne de cargo qui l'explique).
        let e = t.lanceur.lire();
        let mut ko: Vec<String> = e.resultats.iter().filter(|(_, r)| matches!(r.statut, Statut::Echoue | Statut::Erreur)).map(|(c, r)| format!("{} {c}", r.statut.code())).collect();
        ko.sort();
        for k in ko.iter().take(12) {
            eprintln!("  {k}");
        }
        if ko.len() > 12 {
            eprintln!("  ... et {} autre(s)", ko.len() - 12);
        }
        for l in e.lignes.iter().filter(|l| l.starts_with("error") || l.contains("could not compile")).take(8) {
            eprintln!("  cargo : {l}");
        }
    }
    eprintln!("  bilan : {bilan} ; {} tics, {} redessins en {:.1?}", tics.len(), redessins.len(), debut.elapsed());
    let gels = tics.iter().filter(|d| **d >= GEL).count();
    let saccades = tics.iter().filter(|d| **d >= IMAGE).count();
    noter(Z, format!("tic : le pire (sortie a ce moment : {:.60})", pire_etape.1), true, pire_etape.0);
    noter(Z, "tic : 99e centile", true, centile(&mut tics, 0.99));
    noter(Z, "tic : median", true, centile(&mut tics, 0.5));
    noter(Z, format!("redessin pendant l'execution : le pire ({} redessins)", redessins.len()), true, redessins.iter().max().copied().unwrap_or_default());
    // Les redessins les plus longs, puis le median, detailles.
    details.sort_by(|a, b| b.0.cmp(&a.0));
    for (d, x) in details.iter().take(5).chain(details.get(details.len() / 2)) {
        eprintln!("  redessin {d:>8.1?} : {x}");
    }
    noter(Z, "retard du reveil de la fenetre : le pire (machine saturee ?)", true, retards.iter().max().copied().unwrap_or_default());
    noter(Z, "retard du reveil : 99e centile", true, centile(&mut retards, 0.99));
    eprintln!("  {gels} tic(s) >= 100 ms, {saccades} tic(s) >= 16 ms sur {}", tics.len());
}

#[test]
#[ignore]
fn tout_lancer() {
    mesurer();
    fin_de_zone(Z);
}
