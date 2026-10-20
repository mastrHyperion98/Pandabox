//! Cloud sync (docs/cloud-sync).
//!
//! CS-1: canonical vault export, encrypted blob codec, and import.
//! CS-2: config, debounced writer (`SyncEngine`), mutation triggers.
//! CS-3/CS-4: conflict handling and the settings UI.

pub mod blob;
pub mod config;
pub mod restore;
pub mod vault_export;
pub mod vault_import;
pub mod writer;

#[cfg(test)]
pub(crate) mod test_util;

use crate::session::session::Session;
use crate::sync::config::SyncConfig;
use crate::sync::writer::SyncError;
use slint::{Timer, TimerMode};
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use log::error;

const SYNC_DEBOUNCE: Duration = Duration::from_secs(2);

/// Sync status for the UI (consumed by CS-4).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Disabled,
    Locked,
    InSync,
    Pending,
    Error,
}

struct SyncEngineInner {
    config: std::cell::RefCell<SyncConfig>,
    #[allow(dead_code)] // read by CS-4 status plumbing
    config_path: Option<PathBuf>,
    dirty: Cell<bool>,
    // A debounced flush is already pending (prevents stacking one
    // single-shot timer per mutation).
    flush_scheduled: Cell<bool>,
    last_sync_at: std::cell::RefCell<Option<String>>,
    last_error: std::cell::RefCell<Option<String>>,
}

/// Debounced vault-blob writer (DESIGN D3).
///
/// `arm_with_timer` marks the engine dirty after a vault mutation and
/// schedules a 2 s debounced flush on the UI event loop. `flush` is
/// also called on clean app exit. All failures are logged and surfaced
/// via `status` — they never break vault operation.
#[derive(Clone)]
pub struct SyncEngine {
    inner: Rc<SyncEngineInner>,
}

impl SyncEngine {
    /// Engine with the default config location (`~/.Pandabox/sync-config.json`).
    pub fn new() -> Self {
        let config = SyncConfig::load();
        Self::with_inner(SyncEngineInner {
            config: std::cell::RefCell::new(config),
            config_path: None,
            dirty: Cell::new(false),
            flush_scheduled: Cell::new(false),
            last_sync_at: std::cell::RefCell::new(None),
            last_error: std::cell::RefCell::new(None),
        })
    }

    /// Engine whose config file lives at `path` (used by tests and
    /// `with_path`-style setups).
    #[allow(dead_code)] // exercised only by unit tests
    pub fn with_config_path(path: &std::path::Path) -> Self {
        let config = SyncConfig::load_from(path);
        Self::with_inner(SyncEngineInner {
            config: std::cell::RefCell::new(config),
            config_path: Some(path.to_path_buf()),
            dirty: Cell::new(false),
            flush_scheduled: Cell::new(false),
            last_sync_at: std::cell::RefCell::new(None),
            last_error: std::cell::RefCell::new(None),
        })
    }

    fn with_inner(inner: SyncEngineInner) -> Self {
        Self {
            inner: Rc::new(inner),
        }
    }

    pub fn config(&self) -> SyncConfig {
        self.inner.config.borrow().clone()
    }

    /// Update and persist the config. Disabling never touches the
    /// folder or the blob file (DESIGN D7).
    pub fn set_config(&self, config: SyncConfig) -> std::io::Result<()> {
        let path = match &self.inner.config_path {
            Some(p) => p.clone(),
            None => SyncConfig::default_path(),
        };
        config.save_to(&path)?;
        *self.inner.config.borrow_mut() = config;
        Ok(())
    }

    /// Mark the vault dirty. Returns whether sync is enabled.
    pub fn mark_dirty(&self) -> bool {
        if !self.inner.config.borrow().is_enabled() {
            return false;
        }
        self.inner.dirty.set(true);
        true
    }

    #[allow(dead_code)] // exercised only by unit tests
    pub fn is_dirty(&self) -> bool {
        self.inner.dirty.get()
    }

    /// Mark dirty and schedule the debounced flush on the event loop.
    /// At most one timer is outstanding: repeat mutations within the
    /// debounce window reuse the pending flush. The flush only runs
    /// while a session is active (DESIGN D6).
    pub fn arm_with_timer(&self, session_state: &Arc<Mutex<Option<Session>>>) {
        if !self.mark_dirty() || self.inner.flush_scheduled.get() {
            return;
        }
        self.inner.flush_scheduled.set(true);
        let engine = self.clone();
        let state = Arc::clone(session_state);
        let timer = Timer::default();
        timer.start(
            TimerMode::SingleShot,
            SYNC_DEBOUNCE,
            move || {
                engine.inner.flush_scheduled.set(false);
                if let Ok(guard) = state.try_lock() {
                    if let Some(session) = guard.as_ref() {
                        // flush() already logs and records failures; the
                        // dirty flag is kept for retry.
                        let _ = engine.flush(session);
                    }
                }
            },
        );
        // Single-shot timers fire exactly once; forgetting keeps the
        // timer alive until it fires (the standard slint pattern, same
        // as the toast timers in this app).
        std::mem::forget(timer);
    }

    /// Write the blob now. Only writes when sync is enabled and the
    /// engine is dirty; on failure the dirty flag is kept so the next
    /// mutation retries.
    pub fn flush(&self, session: &Session) -> Result<(), SyncError> {
        let config = self.inner.config.borrow().clone();
        if !matches!(config, SyncConfig::Enabled { .. }) || !self.inner.dirty.get() {
            return Ok(());
        }
        let folder = match config.folder() {
            Some(f) => f.clone(),
            None => return Ok(()),
        };
        match crate::sync::writer::write_vault_blob(&folder, session) {
            Ok(_) => {
                self.inner.dirty.set(false);
                self.inner.last_sync_at.replace(Some(
                    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                ));
                self.inner.last_error.replace(None);
                Ok(())
            }
            Err(e) => {
                error!("Cloud sync write failed: {}", e);
                self.inner.last_error.replace(Some(e.to_string()));
                Err(e)
            }
        }
    }

    /// Compare the local vault with the synced file (DESIGN D4).
    /// `NoFile` when sync is disabled.
    pub fn compare_with_remote(&self, session: &Session) -> crate::sync::restore::SyncComparison {
        match self.config().folder() {
            Some(f) => crate::sync::restore::compare_with_local(f, session),
            None => crate::sync::restore::SyncComparison::NoFile,
        }
    }

    /// Apply the user's D4 conflict choice in the configured folder.
    pub fn resolve_conflict(
        &self,
        session: &Session,
        choice: crate::sync::restore::ConflictChoice,
        remote_bytes: &[u8],
    ) -> Result<(), crate::sync::restore::ResolveError> {
        match self.config().folder() {
            Some(f) => crate::sync::restore::resolve_conflict(f, session, choice, remote_bytes),
            None => Ok(()),
        }
    }

    /// UI-facing status (consumed by CS-4).
    ///
    /// `locked` means no session is active (DESIGN D6): the app passes
    /// this in from `session_state`.
    pub fn status(&self, locked: bool) -> (SyncState, String) {        let config = self.inner.config.borrow().clone();
        if !config.is_enabled() {
            return (SyncState::Disabled, String::new());
        }
        if locked {
            return (SyncState::Locked, String::new());
        }
        if let Some(err) = self.inner.last_error.borrow().clone() {
            if self.inner.dirty.get() {
                return (SyncState::Error, err);
            }
        }
        if self.inner.dirty.get() {
            return (SyncState::Pending, String::new());
        }
        (
            SyncState::InSync,
            self.inner.last_sync_at.borrow().clone().unwrap_or_default(),
        )
    }
}

/// User-facing wording for a sync state (consumed by the UI).
pub fn sync_state_word(state: SyncState) -> &'static str {
    match state {
        SyncState::Disabled => "Disabled",
        SyncState::Locked => "Vault locked — sync paused",
        SyncState::InSync => "In sync",
        SyncState::Pending => "Pending changes not yet synced",
        SyncState::Error => "Sync error",
    }
}

#[cfg(test)]
mod state_word_tests {
    use super::*;

    #[test]
    fn all_states_have_words() {
        assert_eq!(sync_state_word(SyncState::Disabled), "Disabled");
        assert_eq!(sync_state_word(SyncState::Locked), "Vault locked — sync paused");
        assert_eq!(sync_state_word(SyncState::InSync), "In sync");
        assert_eq!(sync_state_word(SyncState::Pending), "Pending changes not yet synced");
        assert_eq!(sync_state_word(SyncState::Error), "Sync error");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypt::cryptography::CryptEngine;
    use crate::sync::blob::decode_vault_blob;
    use crate::sync::restore::ConflictChoice;
    use crate::sync::vault_export::{export_vault, vault_fingerprint};
    use crate::sync::writer::BLOB_FILE_NAME;
    use crate::sync::test_util::{TempDb, TEST_MASTER_PASSWORD};
    use std::fs;

    /// Build a real Session (Argon2-derived engine) over `db`.
    fn make_session(db: &TempDb) -> Session {
        let master = db.manager.get_master_record().expect("master record");
        let engine =
            CryptEngine::new(TEST_MASTER_PASSWORD, &master.salt).expect("engine");
        let key = engine
            .decrypt_master_key(&master.nonce, &master.encrypted_master_key)
            .expect("decrypt master key");
        Session::new(key, engine, db.manager.clone())
    }

    #[test]
    fn flush_writes_decodable_blob() {
        let db = TempDb::new();
        db.seed(2);
        let folder = db.dir.join("sync-folder");
        fs::create_dir_all(&folder).expect("folder");

        let engine = SyncEngine::with_config_path(&db.dir.join("sync-config.json"));
        engine
            .set_config(SyncConfig::Enabled { folder: folder.clone() })
            .expect("enable");
        let session = make_session(&db);
        assert!(engine.mark_dirty());

        engine.flush(&session).expect("flush");

        let blob_bytes = fs::read(folder.join(BLOB_FILE_NAME)).expect("blob file");
        let json = decode_vault_blob(&blob_bytes, session.engine_key()).expect("decode");
        let exported = export_vault(&db.manager).expect("export");
        assert_eq!(
            vault_fingerprint(&crate::sync::vault_import::parse_vault(&json).expect("parse")),
            vault_fingerprint(&exported),
            "synced blob must match the local vault"
        );
        assert!(!folder.join("pandabox.pbx.tmp").exists(), "no temp file left behind");
        assert!(!engine.is_dirty());
    }

    #[test]
    fn burst_of_changes_causes_one_write() {
        let db = TempDb::new();
        db.seed(1);
        let folder = db.dir.join("sync-folder");
        fs::create_dir_all(&folder).expect("folder");

        let engine = SyncEngine::with_config_path(&db.dir.join("sync-config.json"));
        engine
            .set_config(SyncConfig::Enabled { folder: folder.clone() })
            .expect("enable");
        let session = make_session(&db);

        // Five rapid mutations arm the same dirty flag.
        for _ in 0..5 {
            assert!(engine.mark_dirty());
        }
        let first_mtime = std::fs::OpenOptions::new()
            .read(true)
            .open(folder.join(BLOB_FILE_NAME))
            .map(|f| f.metadata().unwrap().modified().unwrap())
            .ok();

        engine.flush(&session).expect("flush 1");
        let mtime_after_first = fs::OpenOptions::new()
            .read(true)
            .open(folder.join(BLOB_FILE_NAME))
            .unwrap()
            .metadata()
            .unwrap()
            .modified()
            .unwrap();
        assert_ne!(first_mtime, Some(mtime_after_first), "first flush wrote the file");

        // Engine is no longer dirty: subsequent flushes must be no-ops.
        for _ in 0..5 {
            engine.flush(&session).expect("flush noop");
        }
        let mtime_after_rest = fs::OpenOptions::new()
            .read(true)
            .open(folder.join(BLOB_FILE_NAME))
            .unwrap()
            .metadata()
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(
            mtime_after_first, mtime_after_rest,
            "only one write for the burst of changes"
        );
    }

    #[test]
    fn disabled_sync_never_writes() {
        let db = TempDb::new();
        db.seed(1);
        let folder = db.dir.join("sync-folder");
        fs::create_dir_all(&folder).expect("folder");

        let engine = SyncEngine::with_config_path(&db.dir.join("sync-config.json"));
        let session = make_session(&db);
        assert!(!engine.mark_dirty(), "disabled ⇒ arm is a no-op");
        assert_eq!(engine.status(true).0, SyncState::Disabled);

        engine.flush(&session).expect("flush noop");
        assert!(!folder.join(BLOB_FILE_NAME).exists(), "no file when sync is off");
    }

    #[test]
    fn read_only_folder_fails_without_tmp_leftover() {
        let db = TempDb::new();
        db.seed(1);
        let folder = db.dir.join("ro-folder");
        fs::create_dir_all(&folder).expect("folder");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).expect("chmod");
        }

        let engine = SyncEngine::with_config_path(&db.dir.join("sync-config.json"));
        engine
            .set_config(SyncConfig::Enabled { folder: folder.clone() })
            .expect("enable");
        let session = make_session(&db);
        engine.mark_dirty();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // A root user can still write; skip the assertion in that case.
            let result = engine.flush(&session);
            if result.is_err() {
                assert!(
                    !folder.join("pandabox.pbx.tmp").exists(),
                    "no temp file left behind on failure"
                );
                assert!(matches!(
                    engine.status(false).0,
                    SyncState::Error
                ));
                assert!(engine.is_dirty(), "dirty flag kept for retry");
            }
            let _ = fs::set_permissions(&folder, fs::Permissions::from_mode(0o755));
        }
        #[cfg(not(unix))]
        {
            let _ = (engine, session, folder);
        }
    }

    /// Two-machine story (CS-4.3): machine A syncs a vault; fresh machine
    /// B (same salt, empty vault) restores it from the shared folder; then
    /// both diverge and machine A resolves by importing B's side (the
    /// loser is preserved as a conflict copy, D4/ADR 0003).
    #[test]
    fn two_machine_restore_and_conflict_flow() {
        let salt = [0x7E; 32];

        // Machine A: real vault, syncs to the folder.
        let a = TempDb::new();
        a.seed_with_salt(&salt, 3);
        let sync_dir = a.dir.join("sync");
        fs::create_dir_all(&sync_dir).unwrap();
        let session_a = make_session(&a);
        let engine_a = SyncEngine::with_config_path(&a.dir.join("cfg-a.json"));
        engine_a
            .set_config(SyncConfig::Enabled { folder: sync_dir.clone() })
            .unwrap();
        engine_a.mark_dirty();
        engine_a.flush(&session_a).unwrap();
        assert!(sync_dir.join(crate::sync::writer::BLOB_FILE_NAME).exists());

        // Machine B: fresh vault (same salt, zero records) restores.
        let b = TempDb::new();
        b.seed_with_salt(&salt, 0);
        let rv = restore::restore_from_folder_with_password(
            &sync_dir, TEST_MASTER_PASSWORD, &b.manager,
        )
        .expect("restore on fresh machine");
        let session_b = Session::new(rv.master_key.to_vec(), rv.engine, b.manager.clone());
        assert_eq!(
            b.manager.get_all_records().unwrap().len(),
            3,
            "restored record count"
        );

        // Both diverge: B adds a record and syncs; A adds one locally.
        let engine_b = SyncEngine::with_config_path(&b.dir.join("cfg-b.json"));
        engine_b
            .set_config(SyncConfig::Enabled { folder: sync_dir.clone() })
            .unwrap();
        a.manager.insert_entry("a-only", "a", "a", "p1", "").unwrap();
        b.manager.insert_entry("b-only", "b", "b", "p2", "").unwrap();
        engine_b.mark_dirty();
        engine_b.flush(&session_b).unwrap();

        // A detects the divergence and imports B's side (A is older by mtime).
        let cmp = engine_a.compare_with_remote(&session_a);
        let restore::SyncComparison::Diverged { remote_newer, remote_bytes, .. } = cmp else {
            panic!("expected Diverged, got {cmp:?}");
        };
        assert!(remote_newer, "B's blob was written after A's");
        engine_a
            .resolve_conflict(&session_a, ConflictChoice::Import, &remote_bytes)
            .unwrap();
        let a_records = a.manager.get_all_records().unwrap();
        assert_eq!(a_records.len(), 4, "A now has its record + B's record");
        assert!(
            a_records.iter().any(|r| r.service == "b-only"),
            "imported B's record"
        );
        // Import keeps the LOCAL vault (the loser) as a conflict copy in
        // the local app directory (restore::resolve_conflict).
        let has_conflict = fs::read_dir(&a.dir)
            .unwrap()
            .any(|e| e.unwrap().file_name().to_string_lossy().starts_with("pandabox.pbx.conflict-"));
        assert!(
            has_conflict,
            "the loser is kept as a conflict copy (ADR 0003)"
        );
        assert_eq!(
            engine_a.status(false).0,
            SyncState::InSync,
            "A's own vault is now the blob content → InSync"
        );
    }
}
