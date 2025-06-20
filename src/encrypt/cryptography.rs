use rand_core::RngCore;
use argon2::{Argon2};
use argon2::password_hash::{rand_core::OsRng, Error as PasswordHashError};
use argon2::{ParamsBuilder};
use chacha20poly1305::{Key, Nonce, ChaCha20Poly1305, AeadCore, Error as ChaChaError, KeyInit};
use chacha20poly1305::aead::{Aead, };
use argon2::Algorithm::Argon2id;
use chacha20poly1305::aead::generic_array::GenericArray;
use chacha20poly1305::consts::U12;

const SALT_LENGTH: usize = 32;

pub struct CryptEngine {
    
    key: Vec<u8>, // Store the Argon2 output (not the ChaCha20 key directly)
}

impl CryptEngine {
    pub fn new(password: &str, salt: &[u8]) -> Result<Self, PasswordHashError> {
        let derived_key = CryptEngine::derive_key(password, salt);


        Ok(CryptEngine {
            key: derived_key, // Store the whole PasswordHashString
        })
    }
    
    //Derive key.  This is a separate function so that it can be called after password verification
    fn derive_key(password: &str, salt: &[u8]) -> Vec<u8> {
        let params = ParamsBuilder::new()
            .m_cost(19456)
            .t_cost(2)
            .p_cost(4)
            .output_len(32)
            .build()
            .unwrap();

        let derived_key_length = params.output_len(); // Get the output length
        let mut derived_key = vec![0u8; derived_key_length.unwrap()];
        let argon2 = Argon2::new(Argon2id, argon2::Version::V0x13, params);
        argon2.hash_password_into(password.as_bytes(),salt, &mut derived_key).unwrap();

        derived_key
    }

    // Function to generate a random master encryption key
    pub fn generate_master_key() -> Vec<u8> {
        let mut key = [0u8; 32]; // 256 bits for AES-256
        OsRng.fill_bytes(&mut key);
        key.to_vec()
    }

    // Function to encrypt the master key using ChaCha20Poly1305
    pub fn encrypt_master_key(
        &self,
        master_key: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), ChaChaError> {
        let key = Key::from_slice(self.key.as_slice()); // Derive key from stored hash
        let nonce = Self::generate_nonce();
        let cipher = ChaCha20Poly1305::new(key);
        let ciphertext = cipher.encrypt(&nonce, master_key.as_ref())?;
        Ok((nonce.to_vec(), ciphertext))
    }

    // Function to decrypt the master key using ChaCha20Poly1305
    pub fn decrypt_master_key(
        &self,
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, ChaChaError> {
        let key = Key::from_slice(self.key.as_slice());
        let nonce = Nonce::from_slice(nonce);
        let cipher = ChaCha20Poly1305::new(key);
        let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref())?;
        Ok(plaintext)
    }

    // Helper function to generate a random nonce
    fn generate_nonce() -> GenericArray<u8, U12> {
        let mut nonce = [0u8; 12]; // 12-bit nonce for ChaCha20Poly1305
        OsRng.fill_bytes(&mut nonce);
        Nonce::from_slice(&nonce).clone()
    } // Recommended salt length (in bytes)

    pub fn generate_salt() -> Vec<u8> {
        let mut salt = vec![0u8; SALT_LENGTH];
        OsRng.fill_bytes(&mut salt);
        salt
    }
}