# CS-1 — Vault serialization + encrypted blob

**Epic:** CS — Cloud Sync
**Merges into:** `main`
**Branch prefix:** `feature/cs-1-*`
**Depends on:** —
**Blocks:** CS-2, CS-3
**Spec:** [DESIGN.md](../DESIGN.md) D1 · [SCHEMA.md](../SCHEMA.md) · [ADR 0002](../adr/0002-whole-vault-encrypted-blob.md)

## Goal

Produce and consume the `pandabox.pbx` blob with zero UI: canonical vault JSON (master record + all records), encrypted with the existing `CryptEngine` ChaCha20-Poly1305 path, fully unit-tested and byte-canonical.

### Task CS-1.1 — Canonical vault JSON DTO + export

**Do:**
- Create `src/sync/` module: `vault_export.rs` with a serde DTO matching SCHEMA.md field-for-field.
- Read `master_table` + `records` via `DatabaseManager` (`get_master_record`, `get_all_records`); base64 the binary columns; verify how `records.password` is currently stored (see `Session::get_decrypted_password`) and record the storage truth in SCHEMA.md.
- Sort `records` by `id`; serialize with fixed key order (serde derive order) so output is byte-canonical.
- Add `exported_at` (RFC3339 UTC).

**Acceptance:**
- [ ] `vault_export::export_vault(&DatabaseManager) -> Vec<u8>` returns deterministic bytes for the same DB state (two calls, same bytes).
- [ ] Unit test: DB with 3 records + master row round-trips into JSON matching SCHEMA.md shape (snapshot test).
- [ ] SCHEMA.md note updated with the verified `password` storage form.

### Task CS-1.2 — Blob encode/decode

**Do:**
- `blob.rs`: `encode_vault_blob(json: &[u8], session_master_key: &[u8]) -> Vec<u8>` and `decode_vault_blob(blob: &[u8], candidate_master_key: &[u8]) -> Result<Vec<u8>, _>` using `CryptEngine::encrypt_record` / `decrypt_record` (fresh random nonce per encode).
- Version check: parsed JSON must contain `"version": 1`; unknown version or unknown top-level keys ⇒ typed error, surfaced message text.
- All error variants carry user-facing one-line strings (for D5 status later).

**Acceptance:**
- [ ] Round-trip test: encode → decode with same key yields identical JSON bytes; wrong key fails with auth error (Poly1305 tag), never returns garbage.
- [ ] Two encodes of identical JSON differ (fresh nonce) yet both decode.
- [ ] Nonce-reuse guard: no code path can pass a fixed nonce (review + test asserting random nonces differ).
- [ ] `cargo clippy` clean; no new dependencies added to `Cargo.toml`.

### Task CS-1.3 — Import application

**Do:**
- `vault_import.rs`: `import_vault(json: &[u8], &DatabaseManager)` — replace `master_table` row and `records` rows (delete-all + reinsert preserving ids) inside one transaction; validate version/keys before touching the DB.
- Expose a pure `vault_fingerprint(json: &[u8]) -> [u8; 32]` (SHA-256 or BLAKE3 per existing deps; pick one already in Cargo.lock) for later hash comparison (D4).

**Acceptance:**
- [ ] Import test: export from DB-A file, import into a freshly created DB-B file, `get_all_records` equal on both.
- [ ] Corrupt/unknown-version JSON: import fails, DB-B untouched (row counts unchanged).
- [ ] `vault_fingerprint` stable across export calls for identical DB state.
