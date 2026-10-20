# Design — Cloud Sync (Draft 0.1)

**Requirements:** [REQUIREMENTS.md](./REQUIREMENTS.md) · **Schema:** [SCHEMA.md](./SCHEMA.md) · **ADRs:** [adr/](./adr/)

## 1. Problem statement

| Area | Today (`main`) | Needed |
|------|----------------|--------|
| Vault storage | Plaintext-structured SQLite at `~/.Pandabox/pandabox.db` (only passwords encrypted per-record; `service`, `email`, `username`, `notes` are plaintext `Text`) | Backup/transfer without plaintext exposure |
| Portability | None — DB file is machine-local, tied to `master_table` in the same file | Restore full vault on another machine |
| Backup trigger | None | Automatic on change, user-opted-in |
| Conflict handling | N/A | Detect divergence, never destroy data |

## 2. Goals and non-goals

**Goals**
- Optional, off-by-default sync into a user-chosen folder.
- Opaque encrypted blob (zero-knowledge) containing the *entire* vault incl. the master-password record, so restore works on a fresh install.
- Automatic, crash-safe (atomic) writes while a session is active.
- Visible status and errors in the UI.

**Non-goals (this package)**
- No provider accounts / OAuth / Drive API (ADR 0001). A `SyncProvider` trait is a seam, not work here.
- No delta or partial sync — every write is the whole vault.
- No automatic multi-device merge; conflicts surface to the user (ADR 0003).
- No change to per-record password encryption or the local DB schema.

## 3. Design principles

- Reuse, don't reinvent: crypto goes through the existing `CryptEngine` (`encrypt_record` / `decrypt_record`, ChaCha20-Poly1305 keyed with the session master key).
- The sync path must never break the app: every sync error is caught, logged (`log::error`), and surfaced via a status string — the vault keeps working.
- Fail closed on data: on any ambiguity, keep every copy rather than overwrite.

## 4. Locked decisions

- **D0 — Folder-based sync.** Sync means "write `pandabox.pbx` into a directory the user picks". No Google/Drive login, no network code, no provider SDK. (ADR 0001)
- **D1 — Whole-vault encrypted blob.** The blob is one ChaCha20-Poly1305 AEAD ciphertext containing canonical JSON of the entire vault (`master_table` row + all `records` rows). It is sealed with the **Argon2-derived session key** (password + salt, re-derivable on any machine) plus the vault's public salt appended for password-based restore (v2 format, SCHEMA.md). (ADR 0002)
- **D2 — Plaintext JSON config.** `~/.Pandabox/sync-config.json` holds `{ "enabled": bool, "folder": PathBuf }`. It contains a directory path only — no secrets, so plaintext storage is acceptable. Corrupt/missing config ⇒ sync disabled.
- **D3 — Write triggers.** The blob is rewritten (a) after every successful `insert_entry` / `update_entry` / `delete_entry` via `Session`, and (b) on clean app exit while a session is active. Writes are coalesced through a Slint `Timer` (2 s debounce) so bursty edits produce one write. Writes are atomic: write to `pandabox.pbx.tmp` in the same directory, `fs::rename` over `pandabox.pbx`.
- **D4 — Conflict policy: detect, ask, keep all.** Before importing, compare the vault fingerprint (BLAKE2s-256 over the decrypted canonical JSON, `exported_at` excluded) with the local vault's. If the incoming blob differs from local: (1) strictly newer (mtime) ⇒ offer "Import this file / Keep local"; (2) ambiguous ⇒ same prompt. Whichever the user keeps, the *other* side is preserved: the local vault is first written out as `pandabox.pbx.conflict-<YYYYMMDDTHHMMSS>` into the sync folder (or, when restoring, the incoming file is copied to that name in the local app dir). No silent overwrite, ever. (ADR 0003)
- **D5 — UI: new `Page::Sync` settings page.** Reached from a settings affordance on the services view. Contains: enable toggle, folder picker (existing `rfd::FileDialog` folder mode), status area (last sync time, success, or one-line error), and a "Restore from folder…" action available even when sync is disabled. All user-facing strings live in the Slint file; Rust passes `SharedString`s.
- **D6 — Sync requires an active session.** The blob is only written while a `Session` exists; the app never syncs from a locked or newly-created (pre-auth) state.
- **D7 — Off by default; disable never deletes.** Sync starts disabled. Disabling clears `enabled` and stops writes; the file in the folder and the config file are left untouched.

## 5. Sketches

Module layout (new `src/sync/`):

```
src/sync/
  mod.rs          // SyncEngine { config, timer }, SyncStatus enum
  vault_export.rs // read master_table + records -> canonical JSON (serde)
  vault_import.rs // JSON -> apply to local DB (master record replaced, records replaced)
  blob.rs         // PBXS envelope encode/decode (SCHEMA.md layout), via CryptEngine
  writer.rs       // atomic temp+rename write, debounce timer glue
  config.rs       // sync-config.json read/write
```

`SyncEngine` is constructed in `main.rs` after `DatabaseManager` exists, receives the `Session` on successful unlock, and registers the status string + last-sync time into the UI. Existing `Session::insert_entry/update_entry/delete_entry` gain a post-success hook (`on_vault_changed`) that arms the debounce timer.

Restore flow: `vault_import` writes the incoming master record to `master_table` and replaces `records` rows, then re-derives the session from the incoming master record (user re-enters master password if the file came from another machine with a different password — this is by design and stated in the UI copy).

## 6. Migration

None. No local DB schema change, no existing file format changes. Config file is created on first enable.

## 7. Epic map

| Epic | Decisions | Depends |
|------|-----------|---------|
| [CS-1](./epics/CS-1-vault-blob.md) vault serialization + blob | D1 | — |
| [CS-2](./epics/CS-2-sync-writer.md) config + writer + triggers | D0, D2, D3, D6 | CS-1 |
| [CS-3](./epics/CS-3-restore-conflicts.md) restore + conflicts | D4 | CS-1, CS-2 |
| [CS-4](./epics/CS-4-settings-ui.md) settings page + status | D5, D7 | CS-2, CS-3 |
| [CS-5](./epics/CS-5-packaging-docs.md) flatpak, metainfo, README, tests | all | CS-4 |
