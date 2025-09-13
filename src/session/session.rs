use crate::encrypt::cryptography::CryptEngine;

pub struct Session {
    // Whether the session is active or not
    is_active: bool,
    // The unencrypted session key
    key: Vec<u8>,
    // crypto engine
    crypto_engine: CryptEngine
}

impl Session {
    pub fn new(key: Vec<u8>, crypto_engine: CryptEngine) -> Session {
        Session {
            is_active: true,
            key: key,
            crypto_engine: crypto_engine,
        }
    }
}
