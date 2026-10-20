//! Test helpers for the sync module: throwaway databases in unique
//! temp directories, so parallel tests never share a `HOME`-based DB.

use crate::database::manager::DatabaseManager;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

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

    /// Seed a master record (arbitrary bytes — no real crypto needed)
    /// and `count` records with deterministic content.
    pub fn seed(&self, count: usize) {
        let manager = &self.manager;
        manager
            .create_master_record(&[0xA5; 32], &[0xB4; 32], &[0xC3; 12])
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
