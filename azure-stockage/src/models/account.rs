// Un compte cree par une app pour proteger ses donnees partagees : le mot de
// passe n'est jamais garde, seulement PBKDF2(mot de passe, sel).
use crate::crypto::hmac::{constant_time_eq, pbkdf2_sha256};
use crate::crypto::random::random_bytes;
use crate::models::wire::{Reader, Writer};
use azure_core::models::storage_model::Role;

pub const PBKDF2_ITERATIONS: u32 = 100_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub user: String,
    pub role: Role,
    iterations: u32,
    salt: [u8; 16],
    hash: [u8; 32],
}

impl Account {
    pub fn new(user: &str, password: &str, role: Role) -> Result<Account, String> {
        if password.is_empty() {
            return Err("Le mot de passe ne peut pas etre vide".to_string());
        }
        let mut salt = [0u8; 16];
        random_bytes(&mut salt)?;
        let mut hash = [0u8; 32];
        pbkdf2_sha256(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut hash);
        Ok(Account { user: user.to_string(), role, iterations: PBKDF2_ITERATIONS, salt, hash })
    }

    pub fn check_password(&self, password: &str) -> bool {
        let mut hash = [0u8; 32];
        pbkdf2_sha256(password.as_bytes(), &self.salt, self.iterations, &mut hash);
        constant_time_eq(&hash, &self.hash)
    }

    pub fn write(&self, writer: Writer) -> Writer {
        writer.str(&self.user).u32(self.role.code()).u32(self.iterations).bytes(&self.salt).bytes(&self.hash)
    }

    pub fn read(reader: &mut Reader) -> Result<Account, String> {
        let user = reader.str()?;
        let role = Role::from_code(reader.u32()?).ok_or("Role inconnu")?;
        let iterations = reader.u32()?;
        let salt = reader.bytes()?.try_into().map_err(|_| "Sel invalide")?;
        let hash = reader.bytes()?.try_into().map_err(|_| "Empreinte invalide")?;
        Ok(Account { user, role, iterations, salt, hash })
    }
}
