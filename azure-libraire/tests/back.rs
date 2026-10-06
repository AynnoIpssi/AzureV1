// Les modules du back verifies contre d'autres outils : ce que la
// librairie ecrit est relu par python3 (zlib, zipfile) et par `patch`, et
// l'inverse. Un outil absent : le test ne verifie rien (il le dit).
use azure_libraire::back::compression::zip::{Archive, Zip};
use azure_libraire::back::compression::zlib;
use azure_libraire::back::encodage::hexa::hexa;
use azure_libraire::back::hachage::{crc32::crc32, md5::md5, sha1::sha1, sha256::sha256};
use azure_libraire::back::hasard::Alea;
use azure_libraire::back::texte::diff;
use std::path::PathBuf;
use std::process::Command;

fn dossier(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("azure-libraire-back-{}-{nom}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// La sortie de `python3 -c script args...`, ou `None` sans python3.
fn python(script: &str, args: &[&str]) -> Option<String> {
    let sortie = Command::new("python3").arg("-c").arg(script).args(args).output().ok()?;
    assert!(sortie.status.success(), "python3 : {}", String::from_utf8_lossy(&sortie.stderr));
    Some(String::from_utf8_lossy(&sortie.stdout).trim().to_string())
}

// Du texte qui se repete, du hasard, et un melange des deux.
fn echantillons() -> Vec<Vec<u8>> {
    let mut alea = Alea::new(2026);
    let bruit: Vec<u8> = (0..70_000).map(|_| alea.u64() as u8).collect();
    let texte = "fn calcul(x: u32) -> u32 { x * 2 }\n// une ligne de code qui revient\n".repeat(900).into_bytes();
    let mut melange = Vec::new();
    for i in 0..400 {
        melange.extend(&texte[i * 7..i * 7 + alea.entre(5, 300) as usize]);
        melange.extend(&bruit[i * 11..i * 11 + alea.entre(0, 40) as usize]);
    }
    vec![Vec::new(), b"a".to_vec(), bruit, texte, melange]
}

#[test]
fn zlib_dans_les_deux_sens_avec_python() {
    let d = dossier("zlib");
    for (i, clair) in echantillons().iter().enumerate() {
        let (f_clair, f_notre, f_leur) = (d.join(format!("{i}.clair")), d.join(format!("{i}.notre")), d.join(format!("{i}.leur")));
        std::fs::write(&f_clair, clair).unwrap();
        std::fs::write(&f_notre, zlib::compress(clair)).unwrap();
        let script = "import sys,zlib\nclair=open(sys.argv[1],'rb').read()\nassert zlib.decompress(open(sys.argv[2],'rb').read())==clair\nopen(sys.argv[3],'wb').write(zlib.compress(clair,9))\nprint('ok')";
        let Some(vu) = python(script, &[f_clair.to_str().unwrap(), f_notre.to_str().unwrap(), f_leur.to_str().unwrap()]) else {
            eprintln!("python3 absent : zlib non verifie");
            return;
        };
        assert_eq!(vu, "ok");
        assert_eq!(&zlib::decompress(&std::fs::read(&f_leur).unwrap()).unwrap(), clair);
    }
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn zip_dans_les_deux_sens_avec_python() {
    let d = dossier("zip");
    let contenus = echantillons();
    let mut a = Archive::new();
    for (i, c) in contenus.iter().enumerate() {
        a.ajouter(&format!("dossier/fichier-{i}.bin"), c).unwrap();
    }
    a.ajouter("été.txt", "déjà".as_bytes()).unwrap();
    let (notre, leur) = (d.join("notre.zip"), d.join("leur.zip"));
    std::fs::write(&notre, a.finir()).unwrap();
    // python relit le notre (CRC verifies) et ecrit le sien avec les memes fichiers.
    let script = "import sys,zipfile\nz=zipfile.ZipFile(sys.argv[1])\nassert z.testzip() is None\nw=zipfile.ZipFile(sys.argv[2],'w',zipfile.ZIP_DEFLATED)\nfor n in z.namelist(): w.writestr(n,z.read(n))\nw.close()\nprint(','.join(z.namelist()))";
    let Some(noms) = python(script, &[notre.to_str().unwrap(), leur.to_str().unwrap()]) else {
        eprintln!("python3 absent : zip non verifie");
        return;
    };
    assert!(noms.ends_with("dossier/fichier-4.bin,été.txt"), "{noms}");
    let octets = std::fs::read(&leur).unwrap();
    let z = Zip::ouvrir(&octets).unwrap();
    assert_eq!(z.noms().len(), contenus.len() + 1);
    for (i, c) in contenus.iter().enumerate() {
        assert_eq!(&z.fichier(&format!("dossier/fichier-{i}.bin")).unwrap().unwrap(), c);
    }
    assert_eq!(z.texte("été.txt").unwrap().unwrap(), "déjà");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn les_empreintes_sont_celles_de_python() {
    let d = dossier("empreintes");
    for (i, clair) in echantillons().iter().enumerate() {
        let f = d.join(format!("{i}"));
        std::fs::write(&f, clair).unwrap();
        let script = "import sys,hashlib,zlib\nd=open(sys.argv[1],'rb').read()\nprint(hashlib.sha256(d).hexdigest(),hashlib.sha1(d).hexdigest(),hashlib.md5(d).hexdigest(),'%08x'%zlib.crc32(d))";
        let Some(vu) = python(script, &[f.to_str().unwrap()]) else {
            eprintln!("python3 absent : empreintes non verifiees");
            return;
        };
        assert_eq!(vu, format!("{} {} {} {:08x}", hexa(&sha256(clair)), hexa(&sha1(clair)), hexa(&md5(clair)), crc32(clair)));
    }
    let _ = std::fs::remove_dir_all(d);
}

// Notre diff unifie, applique a l'ancien texte par `patch` (ou, sans lui,
// par `git apply`), redonne le nouveau.
#[test]
fn un_autre_outil_applique_notre_diff() {
    let present = |outil: &str| Command::new(outil).arg("--version").output().is_ok();
    let commande: &[&str] = if present("patch") {
        &["patch", "--quiet", "f.txt", "f.diff"]
    } else if present("git") {
        &["git", "apply", "-p0", "f.diff"]
    } else {
        eprintln!("ni patch ni git : diff unifie non verifie");
        return;
    };
    let d = dossier("patch");
    let mut alea = Alea::new(11);
    for tour in 0..40 {
        let avant: Vec<String> = (0..alea.entre(0, 120)).map(|i| format!("ligne {} {}", i, alea.sous(5))).collect();
        let mut apres = avant.clone();
        for _ in 0..alea.entre(1, 12) {
            let ou = alea.sous(apres.len() as u64 + 1) as usize;
            match alea.sous(3) {
                0 => apres.insert(ou, format!("ajout {}", alea.sous(1000))),
                1 if ou < apres.len() => drop(apres.remove(ou)),
                _ if ou < apres.len() => apres[ou] = format!("change {}", alea.sous(1000)),
                _ => {}
            }
        }
        // Un tour sur quatre : pas de retour a la ligne final d'un cote.
        let texte = |l: &[String], fin: bool| if l.is_empty() { String::new() } else { l.join("\n") + if fin { "\n" } else { "" } };
        let (a, b) = (texte(&avant, tour % 4 != 1), texte(&apres, tour % 4 != 2));
        let unifie = diff::unifie("f.txt", "f.txt", &a, &b, 3);
        if a == b {
            assert_eq!(unifie, "");
            continue;
        }
        let (fichier, correctif) = (d.join("f.txt"), d.join("f.diff"));
        std::fs::write(&fichier, &a).unwrap();
        std::fs::write(&correctif, &unifie).unwrap();
        let sortie = Command::new(commande[0]).current_dir(&d).args(&commande[1..]).output().unwrap();
        assert!(sortie.status.success(), "tour {tour} : {}{}\n{unifie}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
        assert_eq!(std::fs::read_to_string(&fichier).unwrap(), b, "tour {tour}\n{unifie}");
    }
    let _ = std::fs::remove_dir_all(d);
}
