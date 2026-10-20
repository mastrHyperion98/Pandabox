//! Test helpers for the sync module: throwaway databases in unique
//! temp directories, so parallel tests never share a `HOME`-based DB.

use crate::database::manager::DatabaseManager;
use crate::encrypt::cryptography::CryptEngine;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// The master password all TempDb vaults use (tests only).
pub const TEST_MASTER_PASSWORD: &str = "correct horse battery staple";

pub struct TempDb {
    pub dir: std::path::PathBuf,
    pub manager: Rc<DatabaseManager>,
}

impl TempDb {
    pub fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "pandabox-cs1-{}-{}",
            std::process::id(),
            n
        ));
        let manager = Rc::new(DatabaseManager::with_path(dir.join("vault.db")));
        Self { dir, manager }
    }

    /// Seed a **real** master record with a randomly generated salt
    /// (derived from `TEST_MASTER_PASSWORD`) and `count` records.
    pub fn seed(&self, count: usize) {
        self.seed_with_salt(&CryptEngine::generate_salt(), count)
    }

    /// Seed a real master record using the given salt and `count` records.
    pub fn seed_with_salt(&self, salt: &[u8], count: usize) {
        let manager = &self.manager;
        let engine = CryptEngine::new(TEST_MASTER_PASSWORD, salt).expect("engine");
        let master_key = CryptEngine::generate_master_key();
        let (nonce, ciphertext) = engine.encrypt_master_key(&master_key).expect("encrypt");
        manager
            .create_master_record(&salt, &ciphertext, &nonce)
            .expect("seed master record");
        for i in 1..=count {
            manager
                .insert_entry(
                    &format!("service-{i}"),
                    &format!("user{i}@example.com"),
                    &format!("user-{i}"),
                    &format!("base64pw{i}"),
                    &format!("note-{i}"),
                )
                .expect("seed record");
        }
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
