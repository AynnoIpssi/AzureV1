// Un hasard qui se rejoue a l'identique avec la meme graine (donnees de
// test, melanges, tirages). Pas pour des cles : voir
// `chiffrement::random`.

/// xoshiro256** amorce par splitmix64.
#[derive(Debug, Clone)]
pub struct Alea([u64; 4]);

impl Alea {
    pub fn new(graine: u64) -> Alea {
        let mut x = graine;
        let mut suivant = || {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        Alea([suivant(), suivant(), suivant(), suivant()])
    }

    /// Une graine differente a chaque lancement.
    pub fn graine_du_moment() -> u64 {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        t ^ (std::process::id() as u64).rotate_left(32)
    }

    pub fn u64(&mut self) -> u64 {
        let s = &mut self.0;
        let resultat = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        resultat
    }

    /// Dans [0, 1[.
    pub fn f64(&mut self) -> f64 {
        (self.u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Dans [0, n[ (0 si n = 0).
    pub fn sous(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        // Sans biais : on rejette la queue qui ne tombe pas juste.
        let limite = u64::MAX - u64::MAX % n;
        loop {
            let x = self.u64();
            if x < limite {
                return x % n;
            }
        }
    }

    /// Un entier entre `min` et `max` compris.
    pub fn entre(&mut self, min: i64, max: i64) -> i64 {
        let (a, b) = if min <= max { (min, max) } else { (max, min) };
        let etendue = (b as i128 - a as i128 + 1) as u128;
        if etendue > u64::MAX as u128 {
            return self.u64() as i64;
        }
        (a as i128 + self.sous(etendue as u64) as i128) as i64
    }

    pub fn parmi<'a, T: ?Sized>(&mut self, liste: &[&'a T]) -> &'a T {
        liste[self.sous(liste.len() as u64) as usize]
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_meme_graine_rejoue_le_meme_hasard() {
        let (mut a, mut b, mut c) = (Alea::new(42), Alea::new(42), Alea::new(43));
        let suite = |x: &mut Alea| (0..8).map(|_| x.u64()).collect::<Vec<_>>();
        let premier = suite(&mut a);
        assert_eq!(premier, suite(&mut b));
        assert_ne!(premier, suite(&mut c));
    }

    #[test]
    fn les_tirages_restent_dans_leurs_bornes() {
        let mut a = Alea::new(7);
        let mut vus = [false; 6];
        for _ in 0..2000 {
            let n = a.entre(-2, 3);
            assert!((-2..=3).contains(&n));
            vus[(n + 2) as usize] = true;
            let f = a.f64();
            assert!((0.0..1.0).contains(&f));
        }
        assert!(vus.iter().all(|v| *v));
        // Bornes a l'envers, borne unique, toute l'etendue.
        assert!((1..=3).contains(&a.entre(3, 1)));
        assert_eq!(a.entre(5, 5), 5);
        a.entre(i64::MIN, i64::MAX);
        assert_eq!(a.sous(0), 0);
    }
}
