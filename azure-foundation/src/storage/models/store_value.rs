// Ce qu'on peut ranger dans le stockage et relire, sans conversion a la
// main : texte, nombres, booleens, octets. Les nombres et booleens sont
// stockes en texte ("42", "true") : un `i64` range se relit en `u32`, et la
// valeur reste lisible par une autre app qui la recoit partagee.

pub trait ToStore {
    fn to_store(&self) -> Vec<u8>;
}

pub trait FromStore: Sized {
    fn from_store(bytes: &[u8]) -> Result<Self, String>;
}

impl<T: ToStore + ?Sized> ToStore for &T {
    fn to_store(&self) -> Vec<u8> {
        (**self).to_store()
    }
}

impl ToStore for str {
    fn to_store(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl ToStore for String {
    fn to_store(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl FromStore for String {
    fn from_store(bytes: &[u8]) -> Result<String, String> {
        String::from_utf8(bytes.to_vec()).map_err(|_| "La valeur n'est pas du texte UTF-8".to_string())
    }
}

impl ToStore for [u8] {
    fn to_store(&self) -> Vec<u8> {
        self.to_vec()
    }
}

impl ToStore for Vec<u8> {
    fn to_store(&self) -> Vec<u8> {
        self.clone()
    }
}

impl FromStore for Vec<u8> {
    fn from_store(bytes: &[u8]) -> Result<Vec<u8>, String> {
        Ok(bytes.to_vec())
    }
}

macro_rules! as_text {
    ($($t:ty),*) => {$(
        impl ToStore for $t {
            fn to_store(&self) -> Vec<u8> {
                self.to_string().into_bytes()
            }
        }

        impl FromStore for $t {
            fn from_store(bytes: &[u8]) -> Result<$t, String> {
                let text = String::from_store(bytes)?;
                text.trim().parse().map_err(|_| format!("'{text}' n'est pas un {}", stringify!($t)))
            }
        }
    )*};
}

as_text!(i8, i16, i32, i64, u8, u16, u32, u64, usize, isize, f32, f64, bool, char);
