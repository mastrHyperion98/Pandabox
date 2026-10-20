use std::rc::Rc;
use base64::Engine;
use diesel::prelude::*;
use slint::SharedString;
use zeroize::Zeroizing;
use crate::database::manager::DatabaseManager;
use crate::encrypt::cryptography::CryptEngine;
use log::error;

pub struct Session {
    // Whether the session is active or not
    is_active: bool,
    // The unencrypted session key, zeroized when the session drops
    key: Zeroizing<Vec<u8>>,
    // crypto engine
    crypto_engine: CryptEngine,
    database_manager: Rc<DatabaseManager>,
    // Optional callback invoked after every successful vault mutation
    // (cloud sync arming; docs/cloud-sync DESIGN D3).
    on_vault_changed: Option<Rc<dyn Fn() + 'static>>,
}

impl Session {
    pub fn new(key: Vec<u8>, crypto_engine: CryptEngine, db_manager: Rc<DatabaseManager>) -> Session {
        Session {
            is_active: true,
            key: Zeroizing::new(key),
            crypto_engine: crypto_engine,
            database_manager: db_manager,
            on_vault_changed: None,
        }
    }

    pub fn set_on_vault_changed(&mut self, hook: Rc<dyn Fn() + 'static>) {
        self.on_vault_changed = Some(hook);
    }

    fn fire_on_vault_changed(&self) {
        if let Some(hook) = &self.on_vault_changed {
            hook();
        }
    }

    fn get_key(&self) -> &[u8] {
        &self.key
    }

    /// The 256-bit session (master) key. Owned by this session in a
    /// `Zeroizing` buffer and wiped when the session drops; callers
    /// borrow it read-only (per `CryptEngine::seal`'s zeroization contract).
    pub(crate) fn master_key(&self) -> &[u8] {
        &self.key
    }

    /// The Argon2-derived key (password + salt). Re-derivable on any
    /// machine that knows the password and salt — this is the key the
    /// sync blob is sealed with (docs/cloud-sync ADR 0002, v2 format).
    pub(crate) fn engine_key(&self) -> &[u8] {
        self.crypto_engine.derived_key()
    }

    pub fn database_manager(&self) -> &Rc<DatabaseManager> {
        &self.database_manager
    }
    
    /// Get all records from the database
    pub fn get_all_records(&self) -> QueryResult<Vec<crate::database::models::Record>> {
        self.database_manager.get_all_records()
    }

    pub fn insert_entry(&self, service: &SharedString, email: &SharedString, username: &SharedString, password: &SharedString, notes: &SharedString) -> QueryResult<crate::database::models::Record>
    {
        let encrypted_password = match self.crypto_engine.encrypt_record(password.as_str().as_ref(), self.get_key()) {
            Ok(encrypted) => base64::engine::general_purpose::STANDARD.encode(encrypted),
            Err(e) => {
                error!("Failed to encrypt password: {}", e);
                return Err(diesel::result::Error::DeserializationError(
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Encryption error: {}", e)
                    ))
                ));
            }
        };    
        
        let record = self.database_manager.insert_entry(
            service.as_str(),
            email.as_str(),
            username.as_str(),
            encrypted_password.as_str(),
            notes.as_str(),
        )?;
        self.fire_on_vault_changed();
        Ok(record)
    }

    pub fn update_entry(&self, record_id: i32, service: &SharedString, email: &SharedString, username: &SharedString, password: &SharedString, notes: &SharedString) -> bool
    {
        let encrypted_password = match self.crypto_engine.encrypt_record(password.as_str().as_ref(), self.get_key()) {
            Ok(encrypted) => base64::engine::general_purpose::STANDARD.encode(encrypted),
            Err(e) => {
                error!("Failed to encrypt password: {}", e);
                return false;
            }
        };

        match self.database_manager.update_record(
            record_id,
            service.as_str(),
            email.as_str(),
            username.as_str(),
            encrypted_password.as_str(),
            notes.as_str(),
        ) {
            Ok(_) => {
                self.fire_on_vault_changed();
                true
            }
            Err(e) => {
                error!("Failed to update entry: {}", e);
                false
            }
        }
    }

    pub fn delete_entry(&self, record_id: i32) -> bool
    {
        match self.database_manager.delete_entry(
            record_id
        ) {
            Ok(_) => {
                self.fire_on_vault_changed();
                true
            }
            Err(e) => {
                error!("Failed to delete entry: {}", e);
                false
            }
        }
    }

    /// Decrypt a password on-demand (for copying to clipboard)
    pub fn decrypt_password(&self, encrypted_password_base64: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Decode from base64
        let encrypted_bytes = base64::engine::general_purpose::STANDARD.decode(encrypted_password_base64)?;
        
        // Decrypt using the session key
        let decrypted_bytes = self.crypto_engine.decrypt_record(&encrypted_bytes, self.get_key())
            .map_err(|e| format!("Decryption error: {:?}", e))?;
        
        // Convert to string
        let password = String::from_utf8(decrypted_bytes)?;
        
        Ok(password)
    }

    /// Get encrypted password from database by record ID and decrypt it
    pub fn get_decrypted_password(&self, record_id: i32) -> Result<String, Box<dyn std::error::Error>> {
        // Fetch the record from database
        let record = self.database_manager.get_record_by_id(record_id)?;
        
        // Decrypt the password
        self.decrypt_password(&record.password)
    }
}
