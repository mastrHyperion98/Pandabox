# CS-2 — Sync config, atomic writer, triggers

**Epic:** CS — Cloud Sync
**Merges into:** `main`
**Branch prefix:** `feature/cs-2-*`
**Depends on:** CS-1
**Blocks:** CS-4
**Spec:** [DESIGN.md](../DESIGN.md) D0, D2, D3, D6, D7 · [SCHEMA.md](../SCHEMA.md) (config) · [ADR 0001](../adr/0001-sync-via-user-folder.md)

## Goal

Wire the blob into a running app: opt-in config, crash-safe writes into the user folder, triggered by vault mutations and app exit — with sync failures fully isolated from vault operation.

### Task CS-2.1 — Config store

**Do:**
- `src/sync/config.rs`: read/write `~/.Pandabox/sync-config.json` per SCHEMA.md (atomic temp+rename); `enabled: false` default.
- Missing file, invalid JSON, or empty `folder` ⇒ `SyncConfig::Disabled` (never a hard error).

**Acceptance:**
- [ ] Unit tests: absent file ⇒ Disabled; valid file ⇒ Enabled with path; corrupt file ⇒ Disabled + `log::error`.
- [ ] No secrets in the file; round-trip test of write → read.

### Task CS-2.2 — SyncEngine writer

**Do:**
- `src/sync/mod.rs`: `SyncEngine` owning `SyncConfig` + optional `Arc<Mutex<Session>>`; `arm_sync()` records dirty state; a Slint `Timer` (2 s, single-shot re-armed) coalesces writes.
- `writer.rs`: export → encode (session master key) → write `pandabox.pbx.tmp` → `rename` → on success record last-sync time + clear dirty; on any error log `error!` and record the message (consumed by CS-4 status).
- Write only while a session is active (D6); `SyncEngine` is constructed in `main.rs` next to `DatabaseManager`, session registered in `on_authenticate`.
- Write config atomically when enable/disable/folder changes; disable never deletes (D7).

**Acceptance:**
- [ ] Integration test (temp folder): 5 rapid `arm_sync()` calls during a session ⇒ exactly one `pandabox.pbx` written (count writes via file mtime/harness).
- [ ] No session armed ⇒ no file created.
- [ ] Read-only folder ⇒ app continues to work; error string recorded; no panic (test with read-only dir).
- [ ] No `pandabox.pbx.tmp` left behind after a simulated mid-write failure.

### Task CS-2.3 — Mutation + exit hooks

**Do:**
- Add `on_vault_changed` hook called on success in `Session::insert_entry` / `update_entry` / `delete_entry` (engine passed or weak ref; no behavior change when sync disabled).
- Clean-exit path (existing quit handling in `main.rs`) flushes a pending dirty state and disables nothing.

**Acceptance:**
- [ ] Add/edit/delete each produce a blob whose decoded records match post-change DB (3 integration tests).
- [ ] Quit with pending edit ⇒ file contains the edit (test).
- [ ] Existing suite passes unchanged; no UI-visible change yet.
