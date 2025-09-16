use std::rc::Rc;
use base64::Engine;
use slint::SharedString;
use crate::database::manager::DatabaseManager;
use crate::encrypt::cryptography::CryptEngine;

pub struct Session {
    // Whether the session is active or not
    is_active: bool,
    // The unencrypted session key
    key: Vec<u8>,
    // crypto engine
    crypto_engine: CryptEngine,
    database_manager: Rc<DatabaseManager>
}

impl Session {
    pub fn new(key: Vec<u8>, crypto_engine: CryptEngine, db_manager: Rc<DatabaseManager>) -> Session {
        Session {
            is_active: true,
            key: key,
            crypto_engine: crypto_engine,
            database_manager: db_manager
        }
    }

    fn get_key(&self) -> &Vec<u8> {
        &self.key
    }

    pub fn insert_entry(&self, service: &SharedString, email: &SharedString, username: &SharedString, password: &SharedString, notes: &SharedString) -> bool
    {
        let encrypted_password = match self.crypto_engine.encrypt_record(password.as_str().as_ref(), self.get_key().clone()) {
            Ok(encrypted) => base64::engine::general_purpose::STANDARD.encode(encrypted),
            Err(e) => {
                eprintln!("Failed to encrypt password: {}", e);
                return false;
            }
        };    
        
        match self.database_manager.insert_entry(
            service.as_str(),
            email.as_str(),
            username.as_str(),
            encrypted_password.as_str(),
            notes.as_str(),
        ) {
            Ok(_) => true,
            Err(e) => {
                eprintln!("Failed to insert entry: {}", e);
                false
            }
        }
    }

}
