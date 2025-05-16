use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use argon2::password_hash::Error as PasswordHashError;
pub struct CryptEngine{
    key : String,
}

impl CryptEngine {
    pub fn new(password: &str, salt: &[u8]) -> Result<Self, PasswordHashError> {
        let argon2 = Argon2::default();

        let salt_string = SaltString::encode_b64(salt)?;

        let password_hash = argon2.hash_password(password.as_bytes(), &salt_string)?;
        
        Ok(CryptEngine {
            key: password_hash.to_string(),
        })
    }
}