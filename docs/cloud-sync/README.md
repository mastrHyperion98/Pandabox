# Cloud Sync — plan index

Pandabox's vault is a local, plaintext-structured SQLite file (`~/.Pandabox/pandabox.db`), so today it cannot be backed up or moved to another machine without exposing service names, emails, and usernames. This package scopes an **optional cloud sync** feature: on change, Pandabox writes a **whole-vault, encrypted, opaque blob** to a **user-chosen folder** (which the user may have synced by any means — Google Drive, Dropbox, Nextcloud, USB). No provider accounts, no OAuth, no vendor lock-in.

| Doc | Role |
|-----|------|
| [REQUIREMENTS.md](./REQUIREMENTS.md) | User intent and acceptance bar |
| [DESIGN.md](./DESIGN.md) | Locked decisions (D0–D7) |
| [SCHEMA.md](./SCHEMA.md) | On-disk blob layout and sync config file |
| [adr/](./adr/) | Why (0001–0003) |
| [epics/](./epics/) | CS-1 … CS-5 implementation tasks |

**Do not** implement provider APIs (Google Drive, Dropbox, …), delta/partial sync, or multi-device merge in this package. The `SyncProvider` abstraction is a seam for a later package, not work here.

## Epic order

1. [CS-1](./epics/CS-1-vault-blob.md) — vault serialization + encrypted blob (core, no UI)
2. [CS-2](./epics/CS-2-sync-writer.md) — sync config, folder selection, atomic writer + triggers
3. [CS-3](./epics/CS-3-restore-conflicts.md) — import/restore and conflict handling
4. [CS-4](./epics/CS-4-settings-ui.md) — settings page, status display, status strings
5. [CS-5](./epics/CS-5-packaging-docs.md) — flatpak, metainfo, README, final test pass

Merges into `main`. Branch prefix: `feature/cs-<n>-*`.
