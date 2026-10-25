//! Sync configuration (docs/cloud-sync DESIGN D2, SCHEMA.md).
//!
//! Plaintext JSON at `<app data dir>/sync-config.json` (XDG-aware under
//! Flatpak, see `crate::app_paths`) holding only a directory path — no
//! secrets. Missing or corrupt config means "sync disabled", never a
//! hard error.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use log::error;

pub const CONFIG_FILE_NAME: &str = "sync-config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncConfigFile {
    enabled: bool,
    folder: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncConfig {
    Disabled,
    Enabled { folder: PathBuf },
}

impl SyncConfig {
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    pub fn folder(&self) -> Option<&PathBuf> {
        match self {
            Self::Enabled { folder } => Some(folder),
            Self::Disabled => None,
        }
    }

    /// Default config location: `<app data dir>/sync-config.json`.
    /// The data dir is XDG-aware under Flatpak (see `crate::app_paths`).
    pub fn default_path() -> PathBuf {
        crate::app_paths::app_data_dir().join(CONFIG_FILE_NAME)
    }

    /// Load from the default location. Missing/corrupt/empty ⇒ Disabled.
    pub fn load() -> Self {
        Self::load_from(&Self::default_path())
    }

    pub fn load_from(path: &Path) -> Self {
        match fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<SyncConfigFile>(&bytes) {
                Ok(file) if file.enabled && !file.folder.is_empty() => Self::Enabled {
                    folder: PathBuf::from(file.folder),
                },
                Ok(_) => {
                    error!("sync config is disabled or has no folder; treating sync as disabled");
                    Self::Disabled
                }
                Err(e) => {
                    error!("Failed to parse sync config {:?}: {}; treating sync as disabled", path, e);
                    Self::Disabled
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Self::Disabled,
            Err(e) => {
                error!("Failed to read sync config {:?}: {}; treating sync as disabled", path, e);
                Self::Disabled
            }
        }
    }

    /// Save to the default location (atomic).
    #[allow(dead_code)] // exercised only by unit tests
    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::default_path())
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        let file = match self {
            Self::Enabled { folder } => SyncConfigFile {
                enabled: true,
                folder: folder.as_os_str().to_string_lossy().into_owned(),
            },
            Self::Disabled => SyncConfigFile {
                enabled: false,
                folder: String::new(),
            },
        };
        let bytes = serde_json::to_vec_pretty(&file)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            e
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::test_util::TempDb;

    fn config_path(db: &TempDb) -> PathBuf {
        db.dir.join(CONFIG_FILE_NAME)
    }

    #[test]
    fn missing_file_is_disabled() {
        let db = TempDb::new();
        let cfg = SyncConfig::load_from(&db.dir.join("absent.json"));
        assert_eq!(cfg, SyncConfig::Disabled);
        assert!(!cfg.is_enabled());
    }

    #[test]
    fn round_trip_enabled() {
        let db = TempDb::new();
        let folder = db.dir.join("sync-target");
        let cfg = SyncConfig::Enabled { folder: folder.clone() };
        let path = config_path(&db);
        cfg.save_to(&path).expect("save");
        let loaded = SyncConfig::load_from(&path);
        assert!(loaded.is_enabled());
        assert_eq!(loaded, SyncConfig::Enabled { folder });
    }

    #[test]
    fn disabled_round_trip() {
        let db = TempDb::new();
        let path = config_path(&db);
        SyncConfig::Disabled.save_to(&path).expect("save");
        assert_eq!(SyncConfig::load_from(&path), SyncConfig::Disabled);
    }

    #[test]
    fn corrupt_file_is_disabled() {
        let db = TempDb::new();
        let path = config_path(&db);
        fs::write(&path, b"this is not json").expect("write");
        assert_eq!(SyncConfig::load_from(&path), SyncConfig::Disabled);
    }

    #[test]
    fn empty_folder_is_disabled() {
        let db = TempDb::new();
        let path = config_path(&db);
        let bytes = br#"{"enabled": true, "folder": ""}"#;
        fs::write(&path, bytes).expect("write");
        assert_eq!(SyncConfig::load_from(&path), SyncConfig::Disabled);
    }

    #[test]
    fn enabled_without_folder_key_is_disabled() {
        let db = TempDb::new();
        let path = config_path(&db);
        let bytes = br#"{"enabled": true}"#;
        fs::write(&path, bytes).expect("write");
        // `folder` missing ⇒ serde error ⇒ Disabled (never a hard failure)
        assert_eq!(SyncConfig::load_from(&path), SyncConfig::Disabled);
    }
}
