//! Cloud sync (docs/cloud-sync).
//!
//! CS-1: canonical vault export, encrypted blob codec, and import.
//! CS-2: config, debounced writer (`SyncEngine`), mutation triggers.
//! CS-3/CS-4: conflict handling and the settings UI.

pub mod blob;
pub mod config;
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

    /// UI-facing status (consumed by CS-4).
    ///
    /// `locked` means no session is active (DESIGN D6): the app passes
    /// this in from `session_state`.
    pub fn status(&self, locked: bool) -> (SyncState, String) {
        let config = self.inner.config.borrow().clone();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypt::cryptography::CryptEngine;
    use crate::sync::blob::decode_vault_blob;
    use crate::sync::vault_export::{export_vault, vault_fingerprint};
    use crate::sync::writer::BLOB_FILE_NAME;
    use crate::sync::test_util::TempDb;
    use std::fs;

    /// Build a real Session (Argon2-derived engine) over `db`.
    fn make_session(db: &TempDb) -> Session {
        let master = db.manager.get_master_record().expect("master record");
        let engine = CryptEngine::new("correct horse battery staple", &master.salt).expect("engine");
        let key = vec![0x11; 32]; // arbitrary but valid 256-bit session key
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
        let json = decode_vault_blob(&blob_bytes, &[0x11; 32]).expect("decode");
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
}
