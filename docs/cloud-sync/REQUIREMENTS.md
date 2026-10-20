# Requirements

**Design:** [DESIGN.md](./DESIGN.md) · **Epics:** [epics/](./epics/)

As a Pandabox user I want my vault backed up to a folder I control (often a cloud-synced folder) so that I can restore it on another machine, all without creating any cloud account with Pandabox and without any party ever seeing my credentials in plaintext.

## Functional user requirements

- I can **opt in** to sync in settings by choosing a folder; sync is **off by default** and can be turned off.
- While my vault is unlocked, changes (add / edit / delete) are reflected in the sync folder automatically without me pressing "sync".
- On any machine where I install Pandabox, I can **point it at the folder and restore** my full vault: entries, notes, and the master-password record.
- If the file in the sync folder differs from my local vault, I am **told and asked**, and nothing is silently overwritten; both versions survive.
- A sync failure (missing folder, permission error, …) is **visible in the UI** and does not break normal vault use.
- Disabling sync removes the setting and stops writes; it **never deletes** the folder or the file in it.

## Non-functional

- Zero-knowledge: the blob is fully encrypted and opaque; only a **path and timestamps** are stored in plaintext config.
- No new heavyweight dependencies: reuse existing `ChaCha20-Poly1305` crypto from `CryptEngine`; no OAuth / provider SDKs.
- Must work inside the current flatpak sandbox (`--filesystem=home` covers the default case; exotic paths are a documented limitation).

## Acceptance criteria

- All tasks under [epics/](./epics/) completed.
- App builds and unit/integration tests pass on Linux (flatpak build unaffected).
- Concrete proof: on machine A the user enables sync into `~/Dropbox/pandabox`, adds an entry, sees `pandabox.pbx` appear; on machine B the user points Pandabox at that folder, restores, unlocks with the master password, and sees the entry. A deliberately corrupted/diverged file in the folder produces the conflict flow (prompt + preserved `.conflict-*` copy), not data loss.
