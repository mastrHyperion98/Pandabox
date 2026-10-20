# CS-3 — Restore from folder + conflict handling

**Epic:** CS — Cloud Sync
**Merges into:** `main`
**Branch prefix:** `feature/cs-3-*`
**Depends on:** CS-1, CS-2
**Blocks:** CS-4
**Spec:** [DESIGN.md](../DESIGN.md) D4 · [SCHEMA.md](../SCHEMA.md) (conflict copies) · [ADR 0003](../adr/0003-conflict-detect-ask-keep-all.md)

## Goal

Let the user restore the vault from a folder (fresh machine) and make divergence impossible to lose: detect via fingerprint, always confirm, always preserve the losing copy.

### Task CS-3.1 — Restore flow (fresh machine)

**Do:**
- `restore_from_folder_with_password(path, password)` in `restore.rs`: read `pandabox.pbx`, derive the engine key from (password, blob's public salt), AEAD-decode, validate, and decrypt the vault's master record with the engine key (second authentication check) before `import_vault`.
- Return `RestoredVault { master_key, engine }` so the app can build a `Session` without re-authenticating.
- The UI wiring (folder picker, password prompt reusing the Authenticate flow) lands in CS-4.

**Acceptance:**
- [ ] Fresh-install test: folder containing a blob from another DB file ⇒ after password entry, local vault identical (records + master row).
- [ ] Wrong password ⇒ auth error shown, local DB untouched.
- [ ] Folder without `pandabox.pbx` ⇒ clear "no vault found" error, no crash.

### Task CS-3.2 — Divergence detection

**Do:**
- `compare_with_local(blob, local_db)`: decode blob with active session key; compare `vault_fingerprint` of blob JSON vs local vault JSON.
- Equal ⇒ "already in sync". Different ⇒ diverged; compute mtime of file for suggestion only (D4).

**Acceptance:**
- [ ] Test: identical state ⇒ InSync; edited local record ⇒ Diverged (and vice versa).
- [ ] Undecodable blob (wrong/rotated key) ⇒ Undecodable error state, never treated as InSync.

### Task CS-3.3 — Conflict prompt + preservation

**Do:**
- On diverged state (at restore, or on the CS-4 manual "Sync now" check), present the D4 prompt: Import / Keep local / Cancel.
- Losing side preserved: local-kept ⇒ export current vault to `<folder>/pandabox.pbx.conflict-<YYYYMMDDTHHMMSS>`; imported ⇒ copy incoming file to `<app_dir>/pandabox.pbx.conflict-<YYYYMMDDTHHMMSS>`.
- Cancel ⇒ nothing written, state stays Diverged.

**Acceptance:**
- [ ] Test: diverged + Import ⇒ local records become blob's; local's prior state exists as conflict copy (decode-able, fingerprints match).
- [ ] Test: diverged + Keep local ⇒ folder still holds the diverged file AND the conflict copy of the local vault.
- [ ] Test: diverged + Cancel ⇒ no file changes anywhere (hash both locations before/after).
- [ ] No code path overwrites `pandabox.pbx` without the prompt (code review checklist item).
