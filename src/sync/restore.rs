//! Restore + conflict handling (docs/cloud-sync DESIGN D4, ADR 0003).
//!
//! Divergence detection compares vault fingerprints; resolution never
//! overwrites silently: the user chooses, and the losing side is
//! preserved as `pandabox.pbx.conflict-<YYYYMMDDTHHMMSS>`.

use crate::database::manager::DatabaseManager;
use crate::encrypt::cryptography::CryptEngine;
use crate::session::session::Session;
use crate::sync::blob;
use crate::sync::vault_export::{export_vault, vault_fingerprint, VaultExport};
use crate::sync::vault_import::{import_blob, parse_vault, VaultImportError};
use crate::sync::writer::BLOB_FILE_NAME;
use base64::Engine;
use std::fs;
use std::io;
use std::path::Path;
use zeroize::Zeroizing;

/// The decrypted master key and Argon2 engine needed to build a
/// `Session` after a fresh-machine restore. The key is a `Zeroizing`
/// buffer; hand a copy to `Session::new` (e.g. `master_key.to_vec()`) —
/// the buffer itself is wiped when `RestoredVault` drops.
pub struct RestoredVault {
    pub master_key: Zeroizing<Vec<u8>>,
    pub engine: CryptEngine,
}

#[derive(Debug)]
pub enum RestoreError {
    NoVaultFile,
    Io(String),
    Decoding,
    Import(VaultImportError),
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoVaultFile => write!(f, "No vault file found in the folder"),
            Self::Io(detail) => write!(f, "Could not read the vault file: {detail}"),
            Self::Decoding => write!(f, "Vault file could not be decrypted"),
            Self::Import(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RestoreError {}

/// Restore the vault from `folder` on a machine with no prior state.
///
/// The master password is validated against the blob's public salt
/// (AEAD authentication); only after the vault authenticates and
/// parses does it replace the local database.
pub fn restore_from_folder_with_password(
    folder: &Path,
    password: &str,
    manager: &DatabaseManager,
) -> Result<RestoredVault, RestoreError> {
    let file = folder.join(BLOB_FILE_NAME);
    let bytes = match fs::read(&file) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(RestoreError::NoVaultFile),
        Err(e) => return Err(RestoreError::Io(e.to_string())),
    };
    let (json, engine) =
        blob::open_blob_with_password(&bytes, password).map_err(|_| RestoreError::Decoding)?;
    let vault = parse_vault(&json).map_err(RestoreError::Import)?;
    // The vault's master key decrypts with the derived key; this second
    // authentication check validates the password against the vault's
    // own master record.
    let master_key = Zeroizing::new(
        engine
            .decrypt_master_key(
                &base64::engine::general_purpose::STANDARD.decode(&vault.master.nonce)
                    .map_err(|e| RestoreError::Import(VaultImportError::Base64(e.to_string())))?,
                &base64::engine::general_purpose::STANDARD.decode(&vault.master.encrypted_master_key)
                    .map_err(|e| RestoreError::Import(VaultImportError::Base64(e.to_string())))?,
            )
            .map_err(|_| RestoreError::Decoding)?,
    );
    import_blob(&bytes, engine.derived_key(), manager).map_err(RestoreError::Import)?;
    Ok(RestoredVault {
        master_key,
        engine,
    })
}

/// Comparison between the local vault and the synced file (DESIGN D4).
#[derive(Debug)]
pub enum SyncComparison {
    NoFile,
    Io(String),
    /// Blob exists but fails decryption/parse with the current key.
    Undecodable,
    InSync,
    Diverged {
        #[allow(dead_code)]
        local: [u8; 32],
        #[allow(dead_code)]
        remote: [u8; 32],
        /// mtime-based hint for the UI's preselection; the user
        /// confirms in all cases (never auto-resolved).
        remote_newer: bool,
        remote_bytes: Vec<u8>,
    },
}

pub fn compare_with_local(folder: &Path, session: &Session) -> SyncComparison {
    let file = folder.join(BLOB_FILE_NAME);
    let bytes = match fs::read(&file) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return SyncComparison::NoFile,
        Err(e) => return SyncComparison::Io(e.to_string()),
    };

    let json = match blob::decode_vault_blob(&bytes, session.engine_key()) {
        Ok(j) => j,
        Err(_) => return SyncComparison::Undecodable,
    };
    let remote = match parse_vault(&json) {
        Ok(v) => v,
        Err(_) => return SyncComparison::Undecodable,
    };
    let local_vault: VaultExport = match export_vault(session.database_manager()) {
        Ok(v) => v,
        Err(_) => return SyncComparison::Io("Local vault export failed".to_owned()),
    };

    let local_fp = vault_fingerprint(&local_vault);
    let remote_fp = vault_fingerprint(&remote);
    if local_fp == remote_fp {
        return SyncComparison::InSync;
    }

    // mtime hint only; the hash comparison above is authoritative.
    let file_mtime = fs::metadata(&file)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::UNIX_EPOCH)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let db_mtime = fs::metadata(session.database_manager().database_path())
        .and_then(|m| m.modified())
        .unwrap_or(std::time::UNIX_EPOCH)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    SyncComparison::Diverged {
        local: local_fp,
        remote: remote_fp,
        remote_newer: file_mtime > db_mtime,
        remote_bytes: bytes,
    }
}

/// User's answer to the D4 prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    Import,
    KeepLocal,
    Cancel,
}

#[derive(Debug)]
pub enum ResolveError {
    Crypto,
    Io(String),
    Import(VaultImportError),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Crypto => write!(f, "Vault file could not be decrypted"),
            Self::Io(detail) => write!(f, "Conflict resolution failed: {detail}"),
            Self::Import(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ResolveError {}

fn seal_local_vault(session: &Session) -> Result<Vec<u8>, ResolveError> {
    let vault = export_vault(session.database_manager())
        .map_err(|e| ResolveError::Io(e.to_string()))?;
    let salt = base64::engine::general_purpose::STANDARD
        .decode(&vault.master.salt)
        .map_err(|e| ResolveError::Io(e.to_string()))?;
    let json = Zeroizing::new(crate::sync::vault_export::canonical_bytes(&vault));
    blob::encode_vault_blob(&json, session.engine_key(), &salt)
        .map_err(|_| ResolveError::Crypto)
}

/// Unique conflict-copy file name; never overwrites an existing copy.
fn conflict_file_name(dir: &Path) -> std::path::PathBuf {
    let base = format!(
        "pandabox.pbx.conflict-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S")
    );
    let mut name = base.clone();
    let mut i = 2u32;
    while dir.join(&name).exists() {
        name = format!("{base}-{i}");
        i += 1;
    }
    dir.join(name)
}

/// Apply the user's D4 choice. The losing side is preserved as a full
/// conflict copy before anything is written:
/// - `Import`: the local vault's prior state is saved in the local app
///   directory, then the remote vault replaces it.
/// - `KeepLocal`: the local vault is saved as a conflict copy in the
///   sync folder; the folder's `pandabox.pbx` is left untouched.
/// - `Cancel`: nothing is written.
pub fn resolve_conflict(
    folder: &Path,
    session: &Session,
    choice: ConflictChoice,
    remote_bytes: &[u8],
) -> Result<(), ResolveError> {
    match choice {
        ConflictChoice::Cancel => Ok(()),
        ConflictChoice::Import => {
            let local_blob = seal_local_vault(session)?;
            let db_path = session.database_manager().database_path();
            let app_dir = db_path
                .parent()
                .ok_or_else(|| ResolveError::Io("No local app directory".to_owned()))?;
            fs::write(conflict_file_name(app_dir), local_blob)
                .map_err(|e| ResolveError::Io(e.to_string()))?;
            import_blob(remote_bytes, session.engine_key(), session.database_manager())
                .map_err(ResolveError::Import)?;
            Ok(())
        }
        ConflictChoice::KeepLocal => {
            let local_blob = seal_local_vault(session)?;
            fs::write(conflict_file_name(folder), local_blob)
                .map_err(|e| ResolveError::Io(e.to_string()))?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypt::cryptography::CryptEngine;
    use crate::sync::blob::decode_vault_blob;
    use crate::sync::test_util::{TempDb, TEST_MASTER_PASSWORD};
    use crate::sync::vault_export::vault_fingerprint;
    use crate::sync::vault_import::parse_vault;
    use crate::sync::writer;
    use std::fs;

    /// Salt shared by diverged-vault pairs so both engines derive the
    /// same key (the blob is sealed with the derived engine key).
    const SHARED_SALT: [u8; 32] = [0x42; 32];

    /// Build a real Session (Argon2-derived engine) over `db`.
    fn make_session(db: &TempDb) -> Session {
        let master = db.manager.get_master_record().expect("master record");
        let engine = CryptEngine::new(TEST_MASTER_PASSWORD, &master.salt).expect("engine");
        let key = engine
            .decrypt_master_key(&master.nonce, &master.encrypted_master_key)
            .expect("decrypt master key");
        Session::new(key, engine, db.manager.clone())
    }

    fn write_blob(folder: &Path, _db: &TempDb, session: &Session) {
        fs::create_dir_all(folder).expect("folder");
        writer::write_vault_blob(folder, session).expect("write blob");
    }

    #[test]
    fn compare_in_sync_after_flush() {
        let db = TempDb::new();
        db.seed(2);
        let folder = db.dir.join("sync-folder");
        let session = make_session(&db);
        write_blob(&folder, &db, &session);
        assert!(matches!(
            compare_with_local(&folder, &session),
            SyncComparison::InSync
        ));
    }

    #[test]
    fn compare_diverged_after_local_edit() {
        let db = TempDb::new();
        db.seed(2);
        let folder = db.dir.join("sync-folder");
        let session = make_session(&db);
        write_blob(&folder, &db, &session);

        db.manager.insert_entry("new", "n@example.com", "n", "pw", "n").expect("edit");
        match compare_with_local(&folder, &session) {
            SyncComparison::Diverged { local, remote, .. } => {
                assert_ne!(local, remote);
            }
            other => panic!("expected Diverged, got {other:?}"),
        }
    }

    #[test]
    fn compare_undecodable_for_wrong_key() {
        let db = TempDb::new();
        db.seed(1);
        let folder = db.dir.join("sync-folder");
        // A blob sealed with a different key.
        fs::create_dir_all(&folder).expect("folder");
        let json = crate::sync::vault_export::canonical_bytes(
            &export_vault(&db.manager).expect("export"),
        );
        let blob = blob::encode_vault_blob(&json, &[0x77; 32], &[0u8; 32]).expect("encode");
        fs::write(folder.join(BLOB_FILE_NAME), blob).expect("write");

        let session = make_session(&db);
        assert!(matches!(
            compare_with_local(&folder, &session),
            SyncComparison::Undecodable
        ));
    }

    #[test]
    fn compare_no_file() {
        let db = TempDb::new();
        db.seed(1);
        let folder = db.dir.join("empty-folder");
        fs::create_dir_all(&folder).expect("folder");
        let session = make_session(&db);
        assert!(matches!(
            compare_with_local(&folder, &session),
            SyncComparison::NoFile
        ));
    }

    #[test]
    fn restore_fresh_machine_with_password() {
        let source = TempDb::new();
        source.seed(3);
        let source_session = make_session(&source);
        let folder = source.dir.join("sync-folder");
        write_blob(&folder, &source, &source_session);

        // Fresh target database; restore validates the password via the
        // blob's public salt and replaces the vault.
        let target = TempDb::new();
        let restored =
            restore_from_folder_with_password(&folder, TEST_MASTER_PASSWORD, &target.manager)
                .expect("restore");
        assert_eq!(
            target.manager.get_master_record().expect("master").salt,
            source.manager.get_master_record().expect("master").salt,
            "restore copies the source vault's master record"
        );

        let got = target.manager.get_all_records().expect("records");
        let want = source.manager.get_all_records().expect("records");
        assert_eq!(got, want);

        // The restored engine can be used to build a working session.
        let session = Session::new(restored.master_key.to_vec(), restored.engine, target.manager.clone());
        assert!(matches!(
            compare_with_local(&folder, &session),
            SyncComparison::InSync
        ));
    }

    #[test]
    fn restore_wrong_password_leaves_db_untouched() {
        let source = TempDb::new();
        source.seed(2);
        let source_session = make_session(&source);
        let folder = source.dir.join("sync-folder");
        write_blob(&folder, &source, &source_session);

        let target = TempDb::new();
        target.seed(1);
        let err = match restore_from_folder_with_password(&folder, "wrong password", &target.manager) {
            Ok(_) => panic!("wrong password must fail"),
            Err(e) => e,
        };
        assert!(matches!(err, RestoreError::Decoding));
        assert_eq!(
            target.manager.get_all_records().expect("records").len(),
            1,
            "wrong password must not reach import"
        );
    }

    #[test]
    fn restore_missing_file() {
        let db = TempDb::new();
        db.seed(1);
        let folder = db.dir.join("empty");
        fs::create_dir_all(&folder).expect("folder");
        assert!(matches!(
            restore_from_folder_with_password(&folder, "pw", &db.manager),
            Err(RestoreError::NoVaultFile)
        ));
    }

    /// Build a diverged remote blob: a fresh vault with different
    /// content, sealed with the shared salt's engine key.
    fn diverged_remote_bytes() -> Vec<u8> {
        let source = TempDb::new();
        source.seed_with_salt(&SHARED_SALT, 4); // different content ⇒ different fingerprint
        let source_session = make_session(&source);
        let vault = export_vault(&source.manager).expect("export");
        let json = crate::sync::vault_export::canonical_bytes(&vault);
        blob::encode_vault_blob(&json, source_session.engine_key(), &SHARED_SALT).expect("encode")
    }

    #[test]
    fn resolve_import_preserves_prior_local_state() {
        let db = TempDb::new();
        db.seed_with_salt(&SHARED_SALT, 2);
        let session = make_session(&db);
        let folder = db.dir.join("sync-folder");
        fs::create_dir_all(&folder).expect("folder");

        let remote = diverged_remote_bytes();
        let fp_before = vault_fingerprint(&export_vault(&db.manager).expect("export"));
        let remote_fp: [u8; 32] = {
            let json = decode_vault_blob(&remote, session.engine_key()).expect("decode");
            vault_fingerprint(&parse_vault(&json).expect("parse"))
        };
        assert_ne!(fp_before, remote_fp, "precondition: diverged");

        resolve_conflict(&folder, &session, ConflictChoice::Import, &remote).expect("resolve");

        // Local vault now equals the remote's.
        assert_eq!(
            vault_fingerprint(&export_vault(&db.manager).expect("export")),
            remote_fp
        );
        // The losing side (prior local state) is preserved in the app dir.
        let app_dir = {
            let db_path = db.manager.database_path();
            db_path.parent().unwrap().to_path_buf()
        };
        let conflicts: Vec<_> = fs::read_dir(app_dir)
            .expect("read app dir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("pandabox.pbx.conflict-")
            })
            .collect();
        assert_eq!(conflicts.len(), 1, "exactly one conflict copy");
        let copy = fs::read(conflicts[0].path()).expect("read copy");
        let json = decode_vault_blob(&copy, session.engine_key()).expect("decode copy");
        assert_eq!(
            vault_fingerprint(&parse_vault(&json).expect("parse copy")),
            fp_before
        );
    }

    #[test]
    fn resolve_keep_local_preserves_local_in_sync_folder() {
        let db = TempDb::new();
        db.seed_with_salt(&SHARED_SALT, 2);
        let session = make_session(&db);
        let folder = db.dir.join("sync-folder");
        fs::create_dir_all(&folder).expect("folder");

        let remote = diverged_remote_bytes();
        fs::write(folder.join(BLOB_FILE_NAME), &remote).expect("remote file");
        let fp_local = vault_fingerprint(&export_vault(&db.manager).expect("export"));
        let remote_bytes_before = fs::read(folder.join(BLOB_FILE_NAME)).expect("read");

        resolve_conflict(&folder, &session, ConflictChoice::KeepLocal, &remote).expect("resolve");

        // Incoming file untouched, local vault intact...
        assert_eq!(fs::read(folder.join(BLOB_FILE_NAME)).expect("read"), remote_bytes_before);
        assert_eq!(
            vault_fingerprint(&export_vault(&db.manager).expect("export")),
            fp_local
        );
        // ...and the local vault now also exists as a conflict copy in the folder.
        let copy = fs::read_dir(&folder)
            .expect("read dir")
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with("pandabox.pbx.conflict-"))
            .expect("conflict copy")
            .path();
        let json = decode_vault_blob(&fs::read(&copy).expect("read copy"), session.engine_key()).expect("decode");
        assert_eq!(vault_fingerprint(&parse_vault(&json).expect("parse")), fp_local);
    }

    #[test]
    fn resolve_cancel_writes_nothing() {
        let db = TempDb::new();
        db.seed(2);
        let session = make_session(&db);
        let folder = db.dir.join("sync-folder");
        fs::create_dir_all(&folder).expect("folder");

        let remote = diverged_remote_bytes();
        let before = {
            let vault = export_vault(&db.manager).expect("export");
            (vault_fingerprint(&vault), folder.join(BLOB_FILE_NAME).exists())
        };

        resolve_conflict(&folder, &session, ConflictChoice::Cancel, &remote).expect("resolve");

        let after = {
            let vault = export_vault(&db.manager).expect("export");
            (vault_fingerprint(&vault), folder.join(BLOB_FILE_NAME).exists())
        };
        assert_eq!(before, after, "cancel must not touch anything");
        assert!(
            !fs::read_dir(&folder)
                .expect("read dir")
                .filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy().starts_with("pandabox.pbx.conflict-")),
            "no conflict copies on cancel"
        );
    }
}
