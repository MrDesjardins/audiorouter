# Earlier review integration — 2026-11-02

## Objective and authority

Integrate `feature/code-review-fixes-2026-11-01` at `b8cc974f` into the
isolated UX checkout based on `5012aaae`. Parent owns push, PR, main integration
and rebuilt native artifacts. Do not access real profiles, devices or audio.

Requirements: M05/M08, UI-17/27, AUTO-18, PLUG-05/07, SEC-13,
GRAPH-16, STATE-12, ARCH-04 and SND memory limits. Prior decisions and
qualification are in [review fixes](code-review-fixes.md) and
[review findings](code-review-2026-11-01.md) after merge.

## Audit and ordered tasks

Clean starting tree; current main dependency updates are already ancestors of
the UX head. Earlier branch has 28 nonmerge commits absent cached origin/main.
Its concrete delta includes plugin latency/shared-memory repairs, storage
workers and backup/integrity bounds, Soundboard loading bounds, typed dispatch,
bounded transport/MCP reads, encrypted network v3 and receiver freshness,
release shell SBOM/notices, lazy panels, diagnostic polling and mechanical UI
splits. These are substantive source changes, not merely differing hashes.

1. Merge the earlier branch, preserving parent session confirmation, setup
   actions, preview clearing after applied saves and Properties keyboard exit.
2. Preserve main's dependency versions; reconcile typed dispatch with setup.check.
3. Immediately compile/typecheck; inspect conflicts and combined changes.
4. Run focused regressions, full UI and relevant Rust checks with two jobs,
   contract drift, format/lint/docs and diff checks where available.
5. Commit integration and record exact results and remaining gates.

## Validation, risks and rollback

Windows MSVC and pinned toolchain are prerequisites for Rust checks; no audio
hardware is needed for fixture suites. Network v3 requires both peers updated.
Existing review measurements remain historical evidence, not rerun evidence.
Parent's pre-integration packaged smoke cannot qualify this source: rebuild
embedded UI and matching CLI, then repeat disposable-profile smoke. Native
plugin, hardware, attended theme/Narrator/usability and release gates remain
with parent. Rollback: revert merge with first parent; retain earlier branch.

## Integration decisions and results

Merged all earlier commits, including the historical guide videos and review
evidence. `git cherry HEAD feature/code-review-fixes-2026-11-01` before merging
showed one patch-equivalent marketing note (`92b7e1d7`); the remaining commits
were not patch-equivalent. The merge conserves their ancestry and actual source
delta. Dependencies retain main's rusqlite 0.41.1 and ZIP 9.0.4; encryption
adds cipher/inout through the existing ChaCha20 package.

Conflicts resolved in control dispatch, storage, App, Workbench and current
plan. The typed table now includes `system.checkSetup`; setup module and shared
device eligibility rules remain. Storage retains read-only diagnostic inventory
with the new worker connection field and both typed rusqlite backup callbacks.
The original diagnostic inventory regression was restored. Workbench preserves
latest tab/panel semantics and forward Properties exit inside the lazy-panel
Suspense boundary. The keyboard regression now waits for transition completion.
Setup refresh-key calculation moved to a small shared module so App does not
statically import the lazy Status panel. No new method, setting or migration.
The additive network `senderOutdated` result is present in Rust and TS schemas.

Windows / Rust 1.96.0 / Node 22.22.3 checks (logs in local `target/`):

- Workspace `cargo check --workspace -j 2`: passed (`integration-check.log`).
  Initial compile caught the missing diagnostic inventory helpers; repaired.
- `cargo test --workspace --locked -j 2`: 1,275 passed, 32 opt-in ignored
  (`integration-rust-tests.log`), including source/tool combinations, typed
  dispatch discovery, setup dispatcher, default MCP reachability and network
  protocol/loopback tests. Restored inventory regression separately passed
  (`integration-inventory-test.log`). No real audio/device test was requested.
- Plugin-host with `test-fixtures`: 82 library, 2 worker and 39 process tests
  passed; 20 explicitly gated tests ignored (`integration-plugin-tests.log`).
- Workspace and shell all-target/all-feature Clippy, `-j 2 -- -D warnings`:
  passed (`integration-clippy-final.log`, `integration-shell-clippy.log`).
- Full UI: 818 tests / 92 files passed (`integration-ui-tests-final.log`).
  After helper extraction, 34 focused keyboard/setup/lifecycle tests passed
  (`integration-focused-final.log`). Initial full run: 817 passed, one obsolete
  synchronous keyboard assertion failed; repaired as described above.
- UI typecheck/all, zero-warning lint, format/check and production build passed.
  Main chunk 522.60 kB / 153.80 kB gzip; Vite's over-500-kB advisory remains.
  The ineffective-dynamic-import warning found during integration is fixed.
- Contract drift passed: 138 methods, 37 kinds, 20 processors, 22 events
  (`integration-drift.log`). Existing exhaustive parameter catalog, default
  MCP dispatch and API reference changes remain integrated (AUTO-18).
- Guide catalog: 37 kinds / 221 public descriptors passed; guide guards 8
  passed (`integration-site-catalog.log`, `integration-site-guards.log`).
- Both Rust format checks and working diff whitespace checks passed. Staged
  diff check identifies four trailing blank lines in imported raw continuity
  logs (baseline, chain, repeat A3/B1); retain these unchanged as historical
  evidence. All source/docs outside those raw logs pass the staged check.
  Documentation
  acceptance: 234 Markdown files / 1,356 links passed after repairing links to
  the earlier branch's archived plans (`integration-docs.log`).

Release notices and artifact-verifier fixture tests passed
(`integration-notices.log`, `integration-artifacts.log`). Shell tests passed:
86 tests, 4 opt-in ignored (`integration-shell-tests.log`). A redundant
no-default-features shell run was cancelled before execution; it is not a
validation gate or a pass. No owner native app, real database/profile, audio device,
driver, installation, push or PR was touched.
No browser/Firefox/Narrator/three-theme run or guide recapture was performed;
the integrated earlier branch retains its recorded guide qualification, which
is not new evidence for this combined snapshot.

Next: finish shell/fixture checks, commit this merge; parent rebuilds embedded
UI and matching companions, then repeats disposable packaged smoke and final
review before its authorized PR/main work. This is source integration, not a
release qualification or a claim that every historical review gate is closed.
