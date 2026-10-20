# ADR 0002 — Whole-vault opaque encrypted blob

| Field | Value |
|-------|--------|
| Status | Accepted |
| Date | 2026-02-14 |
| Epic | CS-1 |
| Merges into | main |

## Context

Sync must not leak the current plaintext structure of the local DB: `records.service`, `email`, `username`, `notes` are plaintext `Text` columns, so uploading the raw `pandabox.db` would expose the vault's shape and metadata to any cloud provider. The blob must also be self-contained so a fresh install can restore the vault, including the master-password record.

## Decision

The blob (`pandabox.pbx`) is **one ChaCha20-Poly1305 AEAD ciphertext over canonical JSON of the entire vault** — `master_table` row (salt, nonce, encrypted master key) plus all `records` rows — keyed with the session master key already held in `Session` (D1, SCHEMA.md). No unencrypted headers, no visible magic; version and magic live inside the ciphertext. Full-vault, whole-file writes; no delta sync.

## Alternatives considered

### Alt 1 — Encrypt the raw SQLite file
**Rejected.** WAL/journal sidecar files complicate atomic single-file sync; the SQLite format is version- and page-size-dependent across machines; the opaque-JSON DTO is simpler to validate, hash, and evolve.

### Alt 2 — Encrypt only per-record and upload rows individually
**Rejected.** Multiple small files in the cloud folder leak count/timing/structure, invite partial sync, and make conflict handling per-row instead of per-vault.

### Alt 3 — Derive a separate sync key (e.g. HKDF from master key)
**Rejected.** The session master key is already in memory, zeroized on logout, and the same ChaCha20-Poly1305 path is proven. A second derivation adds attack surface and key-lifecycle code for no benefit.

## Consequences

### Positive
- Fully opaque to providers; restore is a single file; content-hash conflict detection is trivial (D4).
- Reuses existing `CryptEngine` — no new crypto code or dependencies.

### Negative / costs
- Every change re-encrypts the whole vault; fine at password-manager scale (hundreds of entries), would need revisiting at very large vaults.
- Nonce is fresh per write; with a static session key this is sound under AEAD but must never reuse a nonce (CS-1 test).
- Importing a blob requires the *blob's* master password to be re-entered if the vault came from another machine (documented UX, by design).
