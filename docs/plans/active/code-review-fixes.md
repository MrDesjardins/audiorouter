# Active plan — code-review fixes (2026-11-01, owner request)

Owner request, 2026-11-01: "do the two code review markdown in the future
folder. Set them in current and tackle all of them." Sources:
[code review 2026-11-01](../future/code-review-2026-11-01.md) and the items
still open in [code review 2026-10-07](../future/code-review-2026-10-07.md).

- Branch `feature/code-review-fixes-2026-11-01`, created from
  `review/code-review-2026-11-01` (`9a0c4067`). No pull request without the
  owner's approval (AGENTS.md).
- Builds and test suites: the owner allowed them on 2026-11-01 while their
  app runs ("Build now").
- One commit per item or small group, each with its regression test.
  Mechanical file splits are separate pure-move commits.

## Owner decisions (2026-11-01)

1. **P2-7 backups:** raise the backup budget so it covers the audio media
   limit, and say so in STATE-12 (option "raise the budget"). Not a separate
   media export.
2. **2026-10-07 P2-5 remainder:** encrypt network audio and protect a
   restarted receiver from replay now, not later.
3. Work on a new branch from the review branch.

## Requirement IDs

PLUG-05 and PLUG-07 (plugin latency and failure policy), SEC-13 and
GRAPH-16 (network audio), STATE-12 (backup budget), ARCH-04 and the
audio-callback rules (work on the audio service thread), AUTO-18 (API and
MCP output), SND-* (Soundboard memory), UI-17 (unsaved-state report).

## Triage and tasks

Status key: **todo**, **done** (code and regression test committed),
**blocked** (needs Windows hardware, a measurement or the owner).

### From the 2026-11-01 review

| Item | Task | Status |
| --- | --- | --- |
| P2-2 | VST3 worker answers Latency from its cache on the pipe thread; a plugin-requested restart is posted to the UI thread and runs under a processing lock; the worker's UI timeout drops below the host's 5 s response timeout. Fixture mode on the Rust worker for a slow latency answer; host test that a slow poll does not fail the worker | done (16a17403) |
| P2-1 | One unchanged-audio watch for a whole VST2 chain (chain input vs. final output); the chain is guarded when any member is; tests on bridge behavior | done (16a17403) |
| P2-4 | One exhaustive `node_latency_samples` in the domain, used by the catalog and `inspect_routes`; engine sample rate instead of 48; exhaustive cost class; remove unused `routeLatencyText` or use it | done (86bf5943) |
| P2-3 | Plugin shared memory on `MmapRaw`, raw-pointer access, atomic payload words and acquire/release fences | implemented; Windows tests pass (see progress) |
| P2-7 | Backup budget covers the media budget; source opened read-only without migration; STATE-12 and a decision record | done (0d163449) |
| P2-8 | Soundboard Play: manifest parsed once and indexed; file read and hash inside `while_servicing_audio`; blobs loaded in bounded steps | done |
| P2-9 | Full integrity check once per app start; light open for the shell's small reads; corrupt-database result logged | done (0d163449) |
| P2-5 | `App.tsx` and `SessionFlowCanvas.tsx` split by pure moves; engine and windows-audio splits | todo |
| P3-1 | Doc comment; HTTP header `BufReader`; sample-rate-derived poll and watch windows; `unsavedReport` diagnostic; MCP stdio bounded byte reads with -32700 for bad UTF-8; SID passed from the I/O thread and a pipe read timeout; session-wide Soundboard memory budget; denormal bench | todo |
| P2-6 | Carried items from 2026-10-07 (below) | see below |
| Branch | `pluginSkipReason` exhaustiveness is on the unmerged `feature/plugin-library-ux`; fix it on that branch, not here | todo |

### From the 2026-10-07 review

| Item | Task | Status |
| --- | --- | --- |
| P0-1 remainder | SQLite reads/writes at Play and export/import moved to worker connections; backups already run in the separate CLI process | implemented; live continuity measurement open |
| P2-1 remainder | Typed method table for `dispatch` | implemented; 137 discovered methods covered exactly once |
| P2-2 remainder | `AppContent` handlers into hooks (with P2-5 above) | todo |
| P2-5 remainder | Network audio encryption (ChaCha20 with HMAC-SHA256, encrypt-then-MAC, packet version 3) and replay protection for a restarted receiver (authenticated sender time, bounded skew) | todo |
| P2-7 | Windows sine-continuity run for the optimized DSP loops | blocked: needs VB-Cable route and a quiet PC |
| P3 UI bundle | Lazy-load rarely used panels if a fresh build still shows a large main chunk | implemented; main 629.49 -> 513.06 kB; browser checks pass |
| P3 silent catches | Diagnostic categories for the remaining swallowed UI errors | todo |
| P3 unsafe audit | Bridge shared memory `MmapRaw` (with P2-3) | bridge already uses raw mappings; plugin mapping repaired with P2-3 |

## Validation matrix

| Check | When |
| --- | --- |
| `cargo fmt --all -- --check`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | every Rust commit |
| Clippy workspace and `src-tauri`, all targets and features, `-D warnings` | every Rust commit |
| `cargo test` for the touched crates (`plugin-host` with `test-fixtures`, `domain`, `storage`, `control`, `protocol`, `windows-audio`, `cli`, `transport`) | per item |
| VST3 worker build (`tools/m06-vst3-worker/build.ps1`) and its acceptance script | P2-2 |
| UI format, lint, typecheck, Vitest; Playwright for moved UI | UI items |
| `npm --prefix contracts run check:drift`, API reference regeneration | API output changes |
| `tests/acceptance/docs.ps1` | every docs change |
| Sine-continuity harness, `AudioServicePassTiming` measurements | blocked items; record when run |

## Risks

- P2-2 changes the VST3 worker's threading; a restart racing a process call
  must be impossible (processing lock) and is covered by review plus the
  acceptance fixture.
- Network audio version 3 does not pair with AudioRouter 0.0.26 or older;
  both computers must be updated. The receiver says so ("senderOutdated").
- Clock skew larger than the allowed window between two paired computers
  would stop audio; the receiver reports `clockSkew` so the user can fix the
  time setting.
- Raising the backup budget makes a backup of a full library take longer.

## Rollback

Revert the item's commit. No storage migration is added; the network packet
version change is reverted together with its UI note.

## Progress

- 2026-11-01: plans tidied (Soundboard, Output Switch, Wave Link first
  round, release 0.0.24/0.0.25 and the old `current.md` archived); this plan
  created.
- 2026-11-01: P2-2 and P2-1 (VST3 latency on the pipe thread, guarded
  VST2 chains; `again.vst3` acceptance and the plugin-host suite pass), P2-4
  (one exhaustive node latency), P2-7 and P2-9 (backup budget with media,
  read-only backups, integrity check once per start) and P2-8 with the
  session-wide Soundboard budget (manifest read once, reads and decodes on a
  worker; `soundboard_load_reads_the_manifest_once_and_bounds_the_session`).
  The sample-rate poll and watch windows of P3-1 landed with P2-1.

## Next action

Finish P2-3 verification, then the remaining items in the tables.

### Resumption and P2-3 verification (2026-11-01)

The interrupted edit compiled after correcting its stale `MmapMut` import.
Plugin slots now use raw mappings and aligned atomic 32-bit payload accesses;
the state word remains atomic 64-bit. Release/acquire fences bracket the
payload copy, without borrowing the mapping as a byte slice. Frame validation
no longer clones the samples before writing. Invalid frames leave the previous
published slot intact.

Windows MSVC, pinned Rust 1.96.0, this workspace:

- `cargo check -p audiorouter-plugin-host`: passed.
- `cargo test -p audiorouter-plugin-host --features test-fixtures`: passed
  (82 library, 2 worker unit and 39 worker-process tests; 20 fixture/private
  tests ignored by their explicit opt-in gates). Includes an actual child
  worker mapped-slot exchange, concurrent full-slot reads/writes, and rejected
  invalid writes preserving the last frame.
- Both Rust format checks and workspace and shell all-target/all-feature Clippy with
  `-D warnings`: passed.
- Miri and attended native-plugin/hardware continuity have not been run.
  The child-worker test is transport evidence, not physical-audio evidence.
- `tests/acceptance/docs.ps1`: passed (225 Markdown files, 1,245 local links).

Other committed progress recovered from Git: HTTP buffering and unsaved-report
diagnostic (`b731cc48`), bounded MCP UTF-8 handling (`5bb1caf0`), pipe SID lookup
and stalled-request timeout (`481244c3`), and denormal-tail benchmark
(`993dc18d`). These address the corresponding P3-1 subtasks; they must not be
reimplemented from the stale aggregate todo row.

### P0-1 remaining storage work: approach

Keep the control plane's original connection present while a scoped helper
opens a second connection to the same database. Worker opens must not migrate,
prune recovery or repeat integrity scans. Route preparation, asset export and
import use this worker connection through the existing audio-service loop.
In-memory test databases need a unique named shared-memory SQLite target so
the same path is tested without substituting a synchronous fallback. Production
file-backed connections retain WAL. Test worker visibility, independent memory
database isolation, errors, and the real dispatch upload/export/import paths;
run storage/control suites, formatting and both Clippy scopes. Rollback is a
revert with no schema change.

P0-1 implementation and verification, Windows MSVC / Rust 1.96.0:

- `with_storage_worker` keeps the original connection present during service
  passes. The worker opens an existing database without integrity checks,
  migrations or pruning. In-memory connections share a unique process-local
  URI per database; separate test databases remain isolated.
- Play's Audio File and Soundboard blobs, saved plugin state and slider files,
  bundle export preparation, bundle restoration, upload blob writes and the
  complete temporary-take import now run on helpers. No schema, REST method,
  MCP method or response shape changed.
- `cargo test -p audiorouter-storage -p audiorouter-control`: passed,
  104 storage / 315 control tests, 11 opt-in control tests ignored. Includes
  real dispatcher media uploads, temporary-take import and cross-database
  bundle export/import, plus worker thread identity, shared visibility,
  independent database isolation and service passes retaining the original
  connection. A worker refuses to create a missing file database.
- Workspace and shell all-target/all-feature Clippy with `-D warnings`:
  passed. Both format checks and `git diff --check`: passed.
- Live `AudioServicePassTiming` under a large library / concurrent route and
  hardware DSP continuity remain open; these tests do not waive them.

### Typed dispatch table (2026-11-01)

`dispatch_methods.rs` replaces the top-level string match with typed function
pointers. The table distinguishes ordinary calls, calls without parameters,
method families and backend-resolved node targets. Parameter validation,
authorization, mutating-notification rejection, error mapping and post-mutation
refresh stay in their original order. Diagnostics construction moved into its
own handler. No public methods or schema changed.

Verification on Windows MSVC / Rust 1.96.0:

- `cargo check -p audiorouter-control`: passed.
- `cargo test -p audiorouter-control -p audiorouter-cli`: passed (316 control
  tests, 11 opt-in ignored; 44 CLI unit tests and its stdio integration suite).
  Includes default MCP reachability and real authorized dispatcher calls.
- New `every_discovered_method_has_exactly_one_typed_handler` checks all 137
  API entries, duplicates and undiscovered handlers in both directions.
- Both all-target/all-feature Clippy scopes with `-D warnings`: passed.
- Rollback: revert the table commit; no database or wire migration.

### UI mechanical split, first slice (2026-11-01)

Moved the unchanged MCP/Logs polling state and effect from `AppContent` into
`useWorkbenchStatus`; moved canvas kind labels and families into
`canvasNodeKinds`. Imports and exports are the only glue changes. This is an
initial slice of P2-5, not completion of the larger App/canvas/engine/WASAPI
splits. Behavior changes and lazy loading belong in separate commits.

- `npm.cmd --prefix ui run typecheck:all`, `format`, `lint`: passed (unused
  imports found by the first lint were removed; final lint has zero warnings).
- `npm.cmd --prefix ui test`: 783 tests / 86 files passed.
- Edge Playwright `workbench-organization.pw.ts --workers=1`: all 8 scenario
  assertions passed, including dark/light/high-contrast and three sidebar
  widths. Restricted-launcher run stalled after all assertions and was
  cancelled (exit 1); rerun outside that launcher exited 0, 8 passed in 14.1 s.
  Status screenshots were inspected in all three themes under
  `%TEMP%/audiorouter-playwright-results/workbench-organization.pw.*/`.
- Warnings examined: the Node color-environment warning is from conflicting
  `NO_COLOR`/`FORCE_COLOR` environment variables. React Flow warns about hidden
  attribution; no licensing determination or attribution setting changes are
  made as part of this mechanical move. Neither is a UI lint warning.
- Fresh production baseline before splitting: main chunk 629.49 kB, Vite
  over-500-kB warning. This confirms the lazy-panel follow-up is warranted.

### Lazy workbench panels (2026-11-01)

API, Logs, settings, recording, timing, Apps and session-file panels now use
dynamic imports. The existing side-panel error boundary contains load failures.
A Suspense boundary covers the tab body, and tab clicks use a transition so
the previous tab remains usable until the requested panel is ready. No source
or tool controls changed, so no property-guide recapture is required.

- Fresh `npm.cmd --prefix ui run build`: typecheck and production build pass;
  main 513.06 kB / 150.50 kB gzip, down from 629.49 / 179.65. The remaining
  size warning is retained deliberately: the main chunk contains the shared
  canvas, inspector and backend adapter. Splitting core editor code solely to
  cross 500 kB would not reduce the initial modules needed by this screen.
  This change does not raise or suppress Vite's warning threshold.
- Format, lint and all-project typecheck pass. Vitest: 783 / 86 files pass.
  First run caught a synchronous Apps-panel assertion; it now awaits the
  requested asynchronous panel without weakening its heading assertions.
- Edge development Playwright: 19 panel/API/recording/session-file scenarios
  pass, plus the dedicated delayed-panel regression (20 total).
- Edge compiled-fixture Playwright with `playwright.ci.config.ts`: 20 pass
  (22.7 s), including dark/light/high-contrast and blocked dynamic loading.
  The new regression holds the real panel module response, asserts that
  playback controls, tabs and canvas bounds remain available, then releases
  it and verifies the requested panel appears. It covers both development
  modules and compiled chunks, with the compiled-asset request guard active.
- First compiled-fixture rebuild hit EPERM deleting old generated assets in
  the restricted launcher; the same build passed outside that launcher.
- Rollback: revert this UI commit; stored preferences and session data do
  not change.
