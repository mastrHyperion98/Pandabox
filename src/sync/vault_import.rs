//! Vault import (docs/cloud-sync DESIGN D4, CS-1.3).
//!
//! Parses a canonical vault JSON (the decrypted blob plaintext) and
//! replaces the local vault — master record and all records — in one
//! transaction. Validation happens before anything is touched.

use crate::database::manager::DatabaseManager;
use crate::database::models::{NewMasterRecord, Record};
use crate::sync::blob;
use crate::sync::vault_export::{VaultExport, VAULT_MAGIC, VAULT_VERSION};
use base64::Engine;

#[derive(Debug)]
pub enum VaultImportError {
    Malformed(String),
    InvalidMagic,
    UnsupportedVersion(u8),
    Base64(String),
    Crypto,
    Database(String),
}

impl std::fmt::Display for VaultImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(detail) => write!(f, "Vault file is not valid: {detail}"),
            Self::InvalidMagic => write!(f, "Vault file has the wrong magic"),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported vault version {v}"),
            Self::Base64(detail) => write!(f, "Vault file has invalid encoding: {detail}"),
            Self::Crypto => write!(f, "Vault file could not be decrypted"),
            Self::Database(detail) => write!(f, "Import failed: {detail}"),
        }
    }
}

impl std::error::Error for VaultImportError {}

fn decode64(s: &str) -> Result<Vec<u8>, VaultImportError> {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| VaultImportError::Base64(e.to_string()))
}

/// Parse and validate canonical vault JSON.
///
/// Rejects: malformed JSON, wrong magic, unknown version, and any
/// unknown fields (forward-compat safety per SCHEMA.md).
pub fn parse_vault(json: &[u8]) -> Result<VaultExport, VaultImportError> {
    let vault: VaultExport = serde_json::from_slice(json)
        .map_err(|e| VaultImportError::Malformed(e.to_string()))?;
    if vault.magic != VAULT_MAGIC {
        return Err(VaultImportError::InvalidMagic);
    }
    if vault.version != VAULT_VERSION {
        return Err(VaultImportError::UnsupportedVersion(vault.version));
    }
    Ok(vault)
}

/// Import a canonical vault JSON into the local database, replacing
/// the master record and all records atomically (record ids preserved).
pub fn import_vault(
    json: &[u8],
    manager: &DatabaseManager,
) -> Result<VaultExport, VaultImportError> {
    let vault = parse_vault(json)?;

    let master = NewMasterRecord {
        encrypted_master_key: &decode64(&vault.master.encrypted_master_key)?,
        nonce: &decode64(&vault.master.nonce)?,
        salt: &decode64(&vault.master.salt)?,
    };
    let records: Vec<Record> = vault
        .records
        .iter()
        .map(|r| Record {
            id: r.id,
            service: r.service.clone(),
            email: r.email.clone(),
            username: r.username.clone(),
            password: r.password.clone(),
            notes: r.notes.clone(),
        })
        .collect();

    manager
        .replace_vault(&master, &records)
        .map_err(|e| VaultImportError::Database(e.to_string()))?;
    Ok(vault)
}

/// Decrypt a sealed vault blob with the candidate key and import it.
///
/// This is the path that reaches `replace_vault` in production: the
/// AEAD authentication check runs first, so a wrong key or a tampered
/// blob fails before anything touches the database.
pub fn import_blob(
    blob_bytes: &[u8],
    key: &[u8],
    manager: &DatabaseManager,
) -> Result<VaultExport, VaultImportError> {
    let json = blob::decode_vault_blob(blob_bytes, key).map_err(|_| VaultImportError::Crypto)?;
    import_vault(&json, manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::test_util::TempDb;
    use crate::sync::vault_export::{canonical_bytes, export_vault, vault_fingerprint};

    #[test]
    fn export_import_round_trip_between_databases() {
        let source = TempDb::new();
        source.seed(3);
        let vault = export_vault(&source.manager).expect("export");
        let json = canonical_bytes(&vault);

        let target = TempDb::new();
        target.seed(1); // different pre-existing content
        import_vault(&json, &target.manager).expect("import");

        let got = target.manager.get_all_records().expect("records");
        let want = source.manager.get_all_records().expect("records");
        assert_eq!(got, want);
        assert_eq!(
            vault_fingerprint(&export_vault(&target.manager).expect("re-export")),
            vault_fingerprint(&vault)
        );
    }

    #[test]
    fn unknown_version_rejected_and_db_untouched() {
        let source = TempDb::new();
        source.seed(1);
        let vault = export_vault(&source.manager).expect("export");

        // A well-formed file with an unknown version must be rejected.
        let mut vault2 = vault;
        vault2.version = 3;
        let json2 = canonical_bytes(&vault2);

        let target = TempDb::new();
        target.seed(2);
        let err = import_vault(&json2, &target.manager).unwrap_err();
        assert!(matches!(err, VaultImportError::UnsupportedVersion(3)));
        assert_eq!(
            target.manager.get_all_records().expect("records").len(),
            2,
            "database must be untouched on rejected import"
        );
    }

    #[test]
    fn unknown_field_rejected() {
        let source = TempDb::new();
        source.seed(1);
        let vault = export_vault(&source.manager).expect("export");
        let mut map = serde_json::to_value(&vault)
            .expect("json")
            .as_object()
            .cloned()
            .expect("object");
        map.insert("surprise".to_owned(), serde_json::Value::Bool(true));
        let json = serde_json::to_vec(&map).expect("json");

        let target = TempDb::new();
        target.seed(1);
        assert!(matches!(
            import_vault(&json, &target.manager),
            Err(VaultImportError::Malformed(_))
        ));
        assert_eq!(
            target.manager.get_all_records().expect("records").len(),
            1,
            "unknown fields must be rejected before any DB write"
        );
    }

    #[test]
    fn import_blob_round_trip_and_wrong_key_protects_db() {
        use crate::sync::blob::{decode_vault_blob, encode_vault_blob};
        use rand::RngCore;

        let source = TempDb::new();
        source.seed(2);
        let vault = export_vault(&source.manager).expect("export");
        let json = canonical_bytes(&vault);

        let mut key = vec![0u8; 32];
        rand::rng().fill_bytes(&mut key);
        let master = source.manager.get_master_record().expect("master");
        let blob = encode_vault_blob(&json, &key, &master.salt).expect("encode");

        // Correct key: import succeeds, vault content matches.
        let target = TempDb::new();
        target.seed(1);
        import_blob(&blob, &key, &target.manager).expect("import blob");
        assert_eq!(
            target.manager.get_all_records().expect("records"),
            source.manager.get_all_records().expect("records")
        );

        // Wrong key: AEAD check fails, DB untouched.
        let mut other = vec![0u8; 32];
        rand::rng().fill_bytes(&mut other);
        let target2 = TempDb::new();
        target2.seed(3);
        let err = import_blob(&blob, &other, &target2.manager).unwrap_err();
        assert!(matches!(err, VaultImportError::Crypto));
        assert_eq!(
            target2.manager.get_all_records().expect("records").len(),
            3,
            "wrong key must not reach replace_vault"
        );

        // The decoded JSON is what import_vault consumes.
        let decoded = decode_vault_blob(&blob, &key).expect("decode");
        assert_eq!(decoded, json);
    }

    #[test]
    fn wrong_magic_rejected() {
        let json = br#"{"magic": "XXXX", "version": 1, "exported_at": "2026-01-01T00:00:00Z", "master": {"encrypted_master_key": "", "nonce": "", "salt": ""}, "records": []}"#;
        assert!(matches!(
            parse_vault(json),
            Err(VaultImportError::InvalidMagic)
        ));
    }
}
