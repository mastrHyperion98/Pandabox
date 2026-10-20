//! Sync writer (docs/cloud-sync DESIGN D0/D3).
//!
//! Exports the local vault, seals it with the session master key, and
//! writes it atomically (temp file + rename) into the sync folder.
//! Failures are reported, never propagated into vault operation.

use crate::session::session::Session;
use crate::sync::blob;
use crate::sync::vault_export;
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

#[derive(Debug)]
pub enum SyncError {
    Export(String),
    Crypto,
    Io(std::io::Error),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Export(detail) => write!(f, "Vault export failed: {detail}"),
            Self::Crypto => write!(f, "Vault encryption failed"),
            Self::Io(e) => write!(f, "Sync write failed: {e}"),
        }
    }
}

impl std::error::Error for SyncError {}

/// Export the vault from `session`, seal it with the session master
/// key, and atomically write `pandabox.pbx` into `folder`.
///
/// Key lifetime: `session` owns the master key in a `Zeroizing`
/// buffer (wiped when the session drops); this function only borrows
/// it, as required by `CryptEngine::seal`'s contract.
pub fn write_vault_blob(folder: &Path, session: &Session) -> Result<PathBuf, SyncError> {
    // The export and its canonical JSON are plaintext (service names,
    // emails, usernames, notes): the JSON is kept in a Zeroizing buffer
    // and the export's fields are wiped explicitly once sealed.
    let mut vault = vault_export::export_vault(session.database_manager())
        .map_err(|e| SyncError::Export(e.to_string()))?;
    let json = Zeroizing::new(vault_export::canonical_bytes(&vault));
    let blob = match blob::encode_vault_blob(&json, session.master_key()) {
        Ok(b) => b,
        // Wipe the plaintext export before returning on failure too.
        Err(_) => {
            vault_export::zeroize_vault(&mut vault);
            return Err(SyncError::Crypto);
        }
    };
    vault_export::zeroize_vault(&mut vault);

    let target = folder.join(BLOB_FILE_NAME);
    let tmp = folder.join(BLOB_TMP_FILE_NAME);
    let write_result = (|| -> std::io::Result<()> {
        fs::write(&tmp, blob)?;
        fs::rename(&tmp, &target)
    })();
    if let Err(e) = write_result {
        // Never leave a half-written temp file behind.
        let _ = fs::remove_file(&tmp);
        return Err(SyncError::Io(e));
    }
    Ok(target)
}

pub const BLOB_FILE_NAME: &str = "pandabox.pbx";
pub const BLOB_TMP_FILE_NAME: &str = "pandabox.pbx.tmp";
