# Schema — Cloud Sync

## Vault blob (`pandabox.pbx`)

A single ChaCha20-Poly1305 AEAD ciphertext (tag inside the ciphertext) over the canonical vault JSON. No plaintext prefix, no unencrypted headers — the file must be indistinguishable from noise (D1).

| Segment | Size | Notes |
|---------|------|-------|
| magic | 4 B | `PBXS` — **stored inside the ciphertext**, i.e. part of the plaintext JSON, not a visible file prefix. File magic is undefined on purpose. |
| version | 1 u8 | `1`. Stored as `"version": 1` in the JSON. Unknown version ⇒ reject with a clear error, never attempt decode. |
| nonce | 12 B | Random per write (existing `CryptEngine::generate_nonce`); never reused across writes with the same key. |
| ciphertext + tag | var | `CryptEngine::encrypt_record(json_bytes, session_master_key)`. |

Decryption: `CryptEngine::decrypt_record` with the candidate master key; the Poly1305 tag authenticates both content and key (wrong master password ⇒ auth failure, not garbage).

### Canonical vault JSON (the plaintext)

```json
{
  "magic": "PBXS",
  "version": 1,
  "exported_at": "2026-02-14T12:34:56Z",
  "master": {
    "encrypted_master_key": "<base64>",
    "nonce": "<base64>",
    "salt": "<base64>"
  },
  "records": [
    {
      "id": 1,
      "service": "example.com",
      "email": "<base64 or plaintext per current storage>",
      "username": "<…>",
      "password": "<as stored in records table>",
      "notes": "<…>"
    }
  ]
}
```

Field rules:
- Field names mirror `schema.rs` column names exactly; base64 is used for the binary columns of `master_table` (`encrypted_master_key`, `nonce`, `salt`).
- `records.password` is stored in the DB as `base64(nonce || ciphertext)` (encrypted per record, CS-1.1 verified); the JSON carries that stored string verbatim. `service`, `email`, `username`, `notes` are stored as text and pass through unchanged.
- `records` is **sorted by `id`** so the JSON is byte-canonical; the content hash (D4) is BLAKE2s-256 over the canonical JSON **with `exported_at` excluded** (so two exports of identical state hash equal).
- No keys beyond those listed. Unknown keys on import ⇒ reject (forward-compat safety), with the error surfaced per D5.

### Conflict copies

`pandabox.pbx.conflict-<YYYYMMDDTHHMMSS>` — a full copy of the blob that lost the D4 prompt, placed in the winner's folder (sync folder for local-kept wins; local app dir for imported wins). Never overwritten or auto-deleted.

## Sync config (`~/.Pandabox/sync-config.json`)

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `enabled` | bool | `false` | Off by default (D7) |
| `folder` | string (abs path) | — | Required when `enabled` is true; missing/empty ⇒ treated as disabled |

Plaintext, no secrets. Parse errors ⇒ sync treated as disabled, logged. Written atomically (temp+rename) like the blob.

Canonical write example:

```json
{ "enabled": true, "folder": "/home/user/Dropbox/pandabox" }
```

Type homes: `src/sync/config.rs` (config), `src/sync/blob.rs` (blob), `src/sync/vault_export.rs` / `vault_import.rs` (JSON DTO, `serde`).
