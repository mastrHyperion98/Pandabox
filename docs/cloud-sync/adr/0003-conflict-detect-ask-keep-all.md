# ADR 0003 — Conflict policy: detect, ask, keep all

| Field | Value |
|-------|--------|
| Status | Accepted |
| Date | 2026-02-14 |
| Epic | CS-3 |
| Merges into | main |

## Context

The vault can diverge from the synced file: edits while offline, two machines with the same folder, or a manual copy. For a password manager the cost of a wrong automatic choice is credential loss. Provider metadata is unavailable (ADR 0001), so the only signals are file mtime and the content hash of the decrypted vault JSON (D4).

## Decision

On any divergence between local vault and incoming blob, **never overwrite silently**: surface a prompt ("Import this file / Keep local / Cancel"). The side that loses the prompt is preserved as a full copy `pandabox.pbx.conflict-<YYYYMMDDTHHMMSS>` (SCHEMA.md). Newer mtime breaks obvious ties by pre-selecting the suggested option; the user always confirms.

## Alternatives considered

### Alt 1 — Last-write-wins
**Rejected.** For credential data, silent LWW risks destroying the copy that holds unique recent entries (e.g. the other machine's).

### Alt 2 — Per-record version vectors / 3-way merge
**Rejected.** No server or metadata channel exists; merging records by id is ambiguous (same id, different content across machines) and would require a schema change and far more code than the feature justifies.

### Alt 3 — Refuse import when diverged
**Rejected.** Worse UX: the user is stuck and would be forced to delete the file; keeping the losing side automatically is strictly better with the same safety.

## Consequences

### Positive
- No data-loss path from sync; the safety property is easy to state and test.
- Prompt is a single modal choice — small UI cost.
- Conflict copies give the user a manual merge escape hatch.

### Negative / costs
- Diverged state requires user attention; users who never look will accumulate conflict copies (bounded by user behavior, not app).
- mtime is a weak signal (fs/copy tools change it), so it only pre-selects; the hash check remains authoritative.
