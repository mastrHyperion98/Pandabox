# CS-5 — Packaging, docs, final test pass

**Epic:** CS — Cloud Sync
**Merges into:** `main`
**Branch prefix:** `feature/cs-5-*`
**Depends on:** CS-4
**Blocks:** — (final)
**Spec:** [DESIGN.md](../DESIGN.md) D0–D7

## Goal

Ship-readiness: flatpak correctness, changelog/README coverage, and the end-to-end proof from REQUIREMENTS.md.

### Task CS-5.1 — Flatpak + distribution

**Do:**
- Review `com.github.mastrHyperion98.Pandabox.yml`: confirm `--filesystem=home` covers default sync folders; document the limitation for paths outside `home` (e.g. `/mnt/…` network mounts) — do **not** add a blanket `--filesystem=all` (D0/ADR 0001 security posture).
- `metainfo.xml`: release note line for the new "Cloud sync (folder-based)" feature.
- Re-run `build-flatpak.sh`; app launches, settings page present, sync writes into a home-dir folder under the sandbox.

**Acceptance:**
- [ ] Flatpak build succeeds; blob written into `~/<provider>/pandabox` from inside the sandbox.
- [ ] Metainfo and build both pass in CI.

### Task CS-5.2 — Documentation

**Do:**
- README.md "Features" + new "Backup & Sync" section: how to enable, which folders work, what the `.pbx` file is, conflict behavior (keep-all), flatpak path limitation.
- CONTRIBUTING.md: pointer to `docs/cloud-sync/` for sync-related changes.
- Index `docs/cloud-sync/` from the repo README (docs section) so the package is not orphaned.

**Acceptance:**
- [ ] A new user following only the README can complete the two-machine proof in REQUIREMENTS.md.
- [ ] No doc contradicts DESIGN.md (review checklist).

### Task CS-5.3 — End-to-end proof + regression sweep

**Do:**
- Execute the REQUIREMENTS.md acceptance proof manually/scripted: A adds entry → blob appears in B's chosen folder → B restores → unlock → entry present → divergence test produces prompt + `.conflict-*` copy.
- Full test suite (unit + integration) green; clippy clean; memory-zeroize audit: nothing from the blob export path leaks keys (reuse existing `zeroize` patterns from `main.rs` create/lock flows).

**Acceptance:**
- [ ] Two-machine proof passes as written in REQUIREMENTS.md.
- [ ] Divergence test passes (prompt, both copies preserved).
- [ ] `cargo test && cargo clippy` green; zeroize audit checklist signed in the PR.
