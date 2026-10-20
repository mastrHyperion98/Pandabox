# ADR 0001 — Sync to a user-chosen folder, not a provider API

| Field | Value |
|-------|--------|
| Status | Accepted |
| Date | 2026-02-14 |
| Epic | CS-2 |
| Merges into | main |

## Context

Pandabox is a zero-knowledge, flatpak-distributed desktop password manager. Users asked for "cloud sync". The literal reading (Google login, Drive REST API) requires OAuth flows, token storage/refresh, provider quotas, an unverified-app consent screen at distribution, and extra flatpak sandbox wiring — a large dependency surface for one optional feature, and it would lock the app to a single vendor.

## Decision

Implement sync as **writing the encrypted vault blob into a directory the user selects** (D0). Users who want Google Drive point Pandabox at their Drive-for-Desktop (or other provider's) sync folder. The feature stays provider-agnostic, requires no accounts, and needs no new network/portal permissions beyond the existing `--filesystem=home`.

A `SyncProvider`-style seam is kept open (the writer only needs a destination `PathBuf`), so a first-class Drive backend can be added later without rework.

## Alternatives considered

### Alt 1 — Google Drive REST API + OAuth
**Rejected.** OAuth webview/loopback flow in Slint, token persistence and refresh, Drive API quotas, Google consent/verification for public distribution, single-provider lock-in. Roughly an order of magnitude more surface area and a trust relationship with Google for a feature most users want only as "backup".

### Alt 2 — Self-hosted sync server
**Rejected.** Operations burden for end users of a consumer app; out of scope.

## Consequences

### Positive
- Minimal new code: file IO + existing crypto.
- Works with any sync mechanism (Drive, Dropbox, Nextcloud, Syncthing, USB), inside the flatpak sandbox as-is.
- Cloud providers see only an opaque ciphertext; zero-knowledge claim stays intact.

### Negative / costs
- Users must have a provider client installed for true "cloud" sync; raw local folders still covered.
- Exotic paths outside `home` (e.g. `/mnt` network mounts) are a documented flatpak limitation (CS-5).
- Conflict handling is file-based (mtime + content hash) rather than provider metadata — acceptable given single-file sync (ADR 0003).
