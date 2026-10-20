# CS-4 — Settings page + status

**Epic:** CS — Cloud Sync
**Merges into:** `main`
**Branch prefix:** `feature/cs-4-*`
**Depends on:** CS-2, CS-3
**Blocks:** CS-5
**Spec:** [DESIGN.md](../DESIGN.md) D5, D7

## Goal

The visible surface: a `Page::Sync` settings page with opt-in toggle, folder picker, live status, and a manual sync/restore action — copy written so zero-knowledge and conflict behavior are clear to the user.

### Task CS-4.1 — `Page::Sync` in Slint

**Do:**
- Add `Page::Sync` to the `Page` enum in `ui/main.slint`; new `sync-view.slint` with: enable CheckBox, folder row (current path + "Choose folder…" button using `rfd::FileDialog` folder mode from Rust), status area (state word, last-sync time, one-line error), "Sync now" and "Restore from folder…" buttons.
- Wire callbacks in `main.rs`: toggle/choose persist via `SyncConfig` (CS-2.1); "Sync now" runs the CS-3.2 check and D4 prompt if diverged; restore reuses CS-3.1 flow.
- Add a settings affordance to the services view (small button in the page header).
- Conflict prompt as a modal in `sync-view.slint` (three labeled buttons, no free-text).

**Acceptance:**
- [ ] Toggle on + valid folder ⇒ config file shows `enabled: true`; first change produces a blob.
- [ ] Toggle off ⇒ no further writes, existing file untouched (hash check).
- [ ] Status area reflects: NotEnabled, InSync, Pending, Error(message) — one test each.
- [ ] All strings user-facing live in the Slint file; no Rust-side literals for UI copy.

### Task CS-4.2 — Status plumbing + copy review

**Do:**
- `SyncStatus` enum (CS-2.2) mapped to `SharedString` + `Timestamp` on the UI; updated on write success/failure, on "Sync now", and on session lock/unlock (locked ⇒ "Vault locked — sync paused" per D6).
- Copy pass: state the blob is fully encrypted, what sync does/doesn't do, and that conflicts are never auto-resolved (mirrors ADR 0003).

**Acceptance:**
- [ ] Lock vault ⇒ status shows paused; unlock ⇒ resumes; a dirty change after unlock still syncs.
- [ ] Copy review against REQUIREMENTS.md acceptance proof scenario, both machines.
