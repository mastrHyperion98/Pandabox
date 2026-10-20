//! Canonical vault export (docs/cloud-sync DESIGN D1, SCHEMA.md).
//!
//! The export is the plaintext that goes under the vault blob. Field
//! names mirror the Diesel schema, binary columns are base64, and
//! records are sorted by `id` so identical vault state serializes to
//! identical bytes.

use crate::database::manager::DatabaseManager;
use base64::Engine;
use blake2::Digest;
use serde::{Deserialize, Serialize};

pub const VAULT_MAGIC: &str = "PBXS";
pub const VAULT_VERSION: u8 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultExport {
    pub magic: String,
    pub version: u8,
    pub exported_at: String,
    pub master: MasterExport,
    pub records: Vec<RecordExport>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MasterExport {
    pub encrypted_master_key: String,
    pub nonce: String,
    pub salt: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecordExport {
    pub id: i32,
    pub service: String,
    pub email: String,
    pub username: String,
    pub password: String,
    pub notes: String,
}

fn encode64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Serialize the local vault into its canonical form.
pub fn export_vault(manager: &DatabaseManager) -> Result<VaultExport, diesel::result::Error> {
    let master = manager.get_master_record()?;
    let records = manager.get_all_records()?;

    let mut records: Vec<RecordExport> = records
        .into_iter()
        .map(|r| RecordExport {
            id: r.id,
            service: r.service,
            email: r.email,
            username: r.username,
            // Stored exactly as it sits in the DB: base64(nonce || ciphertext).
            password: r.password,
            notes: r.notes,
        })
        .collect();
    records.sort_by_key(|r| r.id);

    Ok(VaultExport {
        magic: VAULT_MAGIC.to_owned(),
        version: VAULT_VERSION,
        exported_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        master: MasterExport {
            encrypted_master_key: encode64(&master.encrypted_master_key),
            nonce: encode64(&master.nonce),
            salt: encode64(&master.salt),
        },
        records,
    })
}

/// Canonical, deterministic serialization of the vault content
/// (includes `exported_at`; exclude it when comparing states).
pub fn canonical_bytes(vault: &VaultExport) -> Vec<u8> {
    serde_json::to_vec(vault).expect("a serializable vault cannot fail to serialize")
}

/// Wipe the plaintext fields of `vault` in place (service names,
/// emails, usernames, notes, master-record encodings). Call this after
/// the data has been consumed (serialized / sealed).
pub fn zeroize_vault(vault: &mut VaultExport) {
    vault.magic.as_mut_bytes().fill(0);
    vault.exported_at.as_mut_bytes().fill(0);
    vault.master.encrypted_master_key.as_mut_bytes().fill(0);
    vault.master.nonce.as_mut_bytes().fill(0);
    vault.master.salt.as_mut_bytes().fill(0);
    for r in vault.records.iter_mut() {
        r.service.as_mut_bytes().fill(0);
        r.email.as_mut_bytes().fill(0);
        r.username.as_mut_bytes().fill(0);
        r.password.as_mut_bytes().fill(0);
        r.notes.as_mut_bytes().fill(0);
    }
}

/// BLAKE2s-256 fingerprint used for sync conflict detection (DESIGN D4).
/// Computed over the canonical JSON *without* `exported_at`, so two
/// exports of identical vault state produce the same fingerprint.
pub fn vault_fingerprint(vault: &VaultExport) -> [u8; 32] {
    #[derive(Serialize)]
    struct FingerprintPayload<'a> {
        magic: &'a str,
        version: u8,
        master: &'a MasterExport,
        records: &'a [RecordExport],
    }

    let payload = FingerprintPayload {
        magic: &vault.magic,
        version: vault.version,
        master: &vault.master,
        records: &vault.records,
    };
    let bytes = serde_json::to_vec(&payload).expect("a serializable vault cannot fail to serialize");

    let mut hasher = blake2::Blake2s256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::test_util::TempDb;

    #[test]
    fn export_shape_and_determinism() {
        let db = TempDb::new();
        db.seed(3);

        let a = export_vault(&db.manager).expect("export");
        let b = export_vault(&db.manager).expect("export");

        // SCHEMA.md shape.
        assert_eq!(a.magic, VAULT_MAGIC);
        assert_eq!(a.version, VAULT_VERSION);
        assert!(a.exported_at.ends_with('Z'));
        assert_eq!(a.records.len(), 3);
        assert!(a
            .records
            .windows(2)
            .all(|w| w[0].id < w[1].id),
            "records sorted by id");

        // Base64 fields decode back to the seeded bytes.
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(&a.master.salt)
                .expect("salt b64"),
            [0xA5; 32]
        );

        // Same state ⇒ same fingerprint even though exported_at may differ.
        assert_eq!(vault_fingerprint(&a), vault_fingerprint(&b));
        assert_eq!(a.records, b.records);
        assert_eq!(a.master, b.master);
    }

    #[test]
    fn fingerprint_changes_with_vault() {
        let db = TempDb::new();
        db.seed(2);
        let before = vault_fingerprint(&export_vault(&db.manager).expect("export"));

        db.manager
            .insert_entry("extra", "e@example.com", "extra", "pw", "n")
            .expect("insert");
        let after = vault_fingerprint(&export_vault(&db.manager).expect("export"));

        assert_ne!(before, after);
    }
}
