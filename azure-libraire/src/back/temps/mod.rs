// Les dates sans fuseau : des secondes depuis le 1er janvier 1970,
// ecrites `AAAA-MM-JJ` ou `AAAA-MM-JJ HH:MM:SS`.

/// Nombre de jours depuis le 1er janvier 1970 -> (annee, mois, jour).
pub fn civil(jours: i64) -> (i64, u32, u32) {
    let z = jours + 719_468;
    let ere = z.div_euclid(146_097);
    let jour_ere = z.rem_euclid(146_097);
    let annee_ere = (jour_ere - jour_ere / 1460 + jour_ere / 36_524 - jour_ere / 146_096) / 365;
    let jour_annee = jour_ere - (365 * annee_ere + annee_ere / 4 - annee_ere / 100);
    let mp = (5 * jour_annee + 2) / 153;
    let jour = (jour_annee - (153 * mp + 2) / 5 + 1) as u32;
    let mois = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (annee_ere + ere * 400 + i64::from(mois <= 2), mois, jour)
}

/// (annee, mois, jour) -> nombre de jours depuis le 1er janvier 1970.
pub fn jours(annee: i64, mois: u32, jour: u32) -> i64 {
    let a = annee - i64::from(mois <= 2);
    let ere = a.div_euclid(400);
    let annee_ere = a.rem_euclid(400);
    let mp = i64::from(if mois > 2 { mois - 3 } else { mois + 9 });
    let jour_annee = (153 * mp + 2) / 5 + i64::from(jour) - 1;
    ere * 146_097 + annee_ere * 365 + annee_ere / 4 - annee_ere / 100 + jour_annee - 719_468
}

fn jours_du_mois(annee: i64, mois: u32) -> u32 {
    match mois {
        4 | 6 | 9 | 11 => 30,
        2 if annee % 4 == 0 && (annee % 100 != 0 || annee % 400 == 0) => 29,
        2 => 28,
        _ => 31,
    }
}

/// `2025-01-31`, `2025-01-31 18:30`, `2025-01-31T18:30:05` ou `31/01/2025`
/// -> secondes depuis 1970.
pub fn lire(texte: &str) -> Option<i64> {
    let t = texte.trim();
    let (date, heure) = match t.split_once([' ', 'T']) {
        Some((d, h)) => (d, h.trim().trim_end_matches('Z')),
        None => (t, ""),
    };
    let nombres = |s: &str, sep: char| s.split(sep).map(|p| p.trim().parse::<i64>().ok()).collect::<Option<Vec<i64>>>();
    let (annee, mois, jour) = match (nombres(date, '-'), nombres(date, '/')) {
        (Some(p), _) if p.len() == 3 => (p[0], p[1], p[2]),
        (_, Some(p)) if p.len() == 3 => (p[2], p[1], p[0]),
        _ => return None,
    };
    if !(1..=9999).contains(&annee) || !(1..=12).contains(&mois) || jour < 1 || jour > i64::from(jours_du_mois(annee, mois as u32)) {
        return None;
    }
    let secondes = if heure.is_empty() {
        0
    } else {
        // Les fractions de seconde sont ignorees.
        let p = nombres(heure.split('.').next().unwrap_or(""), ':')?;
        let (h, m, s) = match p.as_slice() {
            [h, m] => (*h, *m, 0),
            [h, m, s] => (*h, *m, *s),
            _ => return None,
        };
        if !(0..24).contains(&h) || !(0..60).contains(&m) || !(0..60).contains(&s) {
            return None;
        }
        h * 3600 + m * 60 + s
    };
    Some(jours(annee, mois as u32, jour as u32) * 86_400 + secondes)
}

pub fn date(secondes: i64) -> String {
    let (a, m, j) = civil(secondes.div_euclid(86_400));
    format!("{a:04}-{m:02}-{j:02}")
}

pub fn heure(secondes: i64) -> String {
    let s = secondes.rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

pub fn date_heure(secondes: i64) -> String {
    format!("{} {}", date(secondes), heure(secondes))
}

/// Maintenant, en secondes depuis 1970 (heure universelle).
pub fn maintenant() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_jours_et_les_dates_se_repondent() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(19_431), (2023, 3, 15));
        assert_eq!(civil(-25_567), (1900, 1, 1));
        assert_eq!(jours(2000, 2, 29), 11_016);
        for j in [-800_000, -1, 0, 59, 60, 365, 11_016, 20_000, 2_932_896] {
            let (a, m, d) = civil(j);
            assert_eq!(jours(a, m, d), j);
        }
    }

    #[test]
    fn les_dates_se_lisent_et_s_ecrivent() {
        assert_eq!(lire("1970-01-02"), Some(86_400));
        assert_eq!(lire("02/01/1970"), Some(86_400));
        assert_eq!(lire(" 2023-03-15 18:30 ").map(date_heure).as_deref(), Some("2023-03-15 18:30:00"));
        assert_eq!(lire("2023-03-15T18:30:05.250Z").map(date_heure).as_deref(), Some("2023-03-15 18:30:05"));
        assert_eq!(lire("1969-12-31 23:59:59"), Some(-1));
        assert_eq!(date(-1), "1969-12-31");
        assert_eq!(heure(-1), "23:59:59");
        for faux in ["2023-02-29", "2023-13-01", "2023-01-01 24:00", "hier", "2023-01", ""] {
            assert_eq!(lire(faux), None, "{faux}");
        }
        assert!(lire("2024-02-29").is_some());
    }
}
