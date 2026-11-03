# Active plan — code-review fixes (2026-11-01, owner request)

Owner request, 2026-11-01: "do the two code review markdown in the future
folder. Set them in current and tackle all of them." Sources:
[code review 2026-11-01](code-review-2026-11-01.md) and the items
still open in [code review 2026-10-07](code-review-2026-10-07.md).

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
| P2-3 | Plugin shared memory on `MmapRaw`, raw-pointer access, atomic payload words and acquire/release fences | done (165b5720); Windows transport tests pass |
| P2-7 | Backup budget covers the media budget; source opened read-only without migration; STATE-12 and a decision record | done (0d163449) |
| P2-8 | Soundboard Play: manifest parsed once and indexed; file read and hash inside `while_servicing_audio`; blobs loaded in bounded steps | done |
| P2-9 | Full integrity check once per app start; light open for the shell's small reads; corrupt-database result logged | done (0d163449) |
| P2-5 | `App.tsx` and `SessionFlowCanvas.tsx` split by pure moves; engine and windows-audio splits | bounded concern splits implemented (7abc2e1b, aa1f6dbd); large compiler/endpoint files remain a maintainability risk |
| P3-1 | Doc comment; HTTP header `BufReader`; sample-rate-derived poll and watch windows; `unsavedReport` diagnostic; MCP stdio bounded byte reads with -32700 for bad UTF-8; SID passed from the I/O thread and a pipe read timeout; session-wide Soundboard memory budget; denormal bench | implemented across 16a17403, b731cc48, 5bb1caf0, 481244c3 and 993dc18d; see progress |
| P2-6 | Carried items from 2026-10-07 (below) | see below |
| Branch | `pluginSkipReason` exhaustiveness is on the unmerged `feature/plugin-library-ux`; fix it on that branch, not here | done on owning branch (ed1e41ec); 791 UI tests pass |

### From the 2026-10-07 review

| Item | Task | Status |
| --- | --- | --- |
| P0-1 remainder | SQLite reads/writes at Play and export/import moved to worker connections; backups already run in the separate CLI process | implemented; live continuity measurement open |
| P2-1 remainder | Typed method table for `dispatch` | implemented; 137 discovered methods covered exactly once |
| P2-2 remainder | `AppContent` handlers into hooks (with P2-5 above) | recording actions and workbench polling extracted; existing UI/browser checks pass |
| P2-5 remainder | Network audio encryption (XChaCha20 with HMAC-SHA256, encrypt-then-MAC, packet version 3) and fresh authenticated receiver challenges preventing replay after restart | implemented; Windows loopback/dispatcher and UI checks pass; see evidence below |
| P2-4 remainder | Include the shell dependency tree in release notices and retain its Cargo SBOM | implemented; fixtures and both real metadata trees checked |
| P2-7 | Windows sine-continuity run for the optimized DSP loops | done 2026-11-02 (live run below) |
| P3 UI bundle | Lazy-load rarely used panels if a fresh build still shows a large main chunk | implemented; main 629.49 -> 513.06 kB; browser checks pass |
| P3 silent catches | Diagnostic categories for the remaining swallowed UI errors | implemented; polling regressions cover redaction, recovery and keep-last-value |
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
- Receiver freshness is negotiated without clocks. A captured valid probe can
  consume bounded handshake work; IP filtering, queue bounds and response
  throttling remain necessary and do not provide denial-of-service immunity.
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

Finish large-media open timing and HTTP idle-response evidence, then prepare
the attended VB-Cable continuity/service-gap checks. Measurements are now
recorded below. The cable checks await the owner's safe-to-use confirmation;
no PR is authorized.

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

### Remaining silent catches (2026-11-01)

MCP setup, tunnel status and backend activity failures now produce safe
diagnostic categories. Repeated polling failures log once until a successful
read rearms that category; failed reads retain the last successful answer.
Virtual-cable polling uses the same one-message-per-failure-run rule, and a
failed inventory read after deletion records its category. The intentional
pre-App packaged-shell probe catch is documented at its call site. Expected
pointer-capture fallback and diagnostic-sink fallback remain intentional.

- New hook regressions exercise initial failures, unsafe error-name redaction,
  repeated failures, successful recovery, retained tunnel state, cancellation
  and late rejections after leaving the tab.
- Typecheck and lint: passed. Focused accessibility/hook run: 106 passed.
  Full run limited to four workers: 786 passed / 87 files.
- An unconstrained run exposed the first Advanced panel's cold import taking
  longer than the startup-status assertion's one-second window. The browser
  paths pass; the unit scenario now explicitly waits for the lazy panel, then
  tests registration with its original assertion. Final normal-worker run:
  786 passed / 87 files (30.61 s). This does not alter startup performance gates.

### Network v3 freshness decision (2026-11-01)

The owner's authorized scope is encryption and protection of a restarted
receiver. Timestamp freshness with allowed clock skew cannot fully provide the
latter: captured packets inside that skew window remain replayable. Replace
the provisional timestamp design with a key-authenticated receiver challenge.
This changes the transport design, not the authorized product scope.

Ordered implementation and verification:

1. Portable framing: version-3 data uses XChaCha20 with independently derived
   encryption and HMAC keys, a random 24-byte nonce, a 16-byte receiver token,
   and a 16-byte encrypt-then-MAC tag. Authenticate before decrypting.
2. Authenticated transport-only probes carry a fresh 16-byte sender challenge;
   receiver responses echo it and supply a fresh 16-byte receiver token.
   Reject unsolicited/old responses. Refresh periodically for restart recovery.
   A new receiver token invalidates every captured packet from its predecessor.
3. Keep CSPRNG, negotiation, encryption and decryption on network threads;
   retain bounded audio queues and callback packet construction. Unpaired
   version 1 remains available with its warning; paired receivers reject v1/v2.
4. Test altered headers, ciphertext and tags, wrong keys, nonce/token mismatch,
   replay after receiver restart, challenge replay, live key/target changes,
   and real loopback send/receive through backend dispatch. Keep the replay
   window within a negotiated token too.
5. Update SEC-13, GRAPH-16, pairing descriptions and compatibility guidance,
   contracts/catalogs, dependency notices and checks. Run protocol, native
   network, backend, source-combination, drift and UI checks. Record hardware
   limits separately. Revert the complete v3 commit for rollback; no schema
   migration or new setting is needed.

Both peers must update together when paired; no silent v2 fallback. Clock
skew is no longer a transport risk. The earlier clock-skew risk paragraph is
superseded by this decision.

### Network v3 verification and guide audit (2026-11-01)

SEC-13 / GRAPH-16: encryption, authenticated receiver freshness, periodic
renegotiation after restart, same-key no-op edits and explicit paired-v2 refusal
are implemented. Default MCP tools configure pairing; the recipe explains the
upgrade boundary. REST/contract output includes `senderOutdated`.

Experiments caught two live-path defects before handoff. Rotating the receiver
token on every periodic probe discarded genuine packets in flight; periodic
probes now reconfirm the live token, while restart/key/peer changes replace it.
Retargeting a connected UDP socket retained queued Windows ICMP errors from its
old port; an actual destination change now opens a fresh socket. The original
dispatcher live-edit regression and encrypted continuity scenario pass.

Windows MSVC / Rust 1.96.0 verification:

- Protocol: 25 passed. Native audio library: 145 passed, 4 opt-in ignored.
  Includes tampered header/ciphertext/tag, wrong key, legacy rejection,
  captured audio after receiver restart, old response rejection, live sender
  recovery and same-key edits retaining replay history.
- Control: 317 passed, 11 opt-in ignored. CLI: 45 unit tests plus stdio
  integration passed. Paired loopback traverses real dispatcher entry points
  in one backend and in two independent backends, with zero lost/replayed
  packets in the continuity assertions. This is local UDP evidence, not
  cross-computer or physical-audio latency qualification.
- Source combinations: 5 passed; processor combinations: 4 passed.
- Both Rust format checks, workspace and shell all-target/all-feature Clippy,
  UI format/lint/typecheck, 786 UI tests and contract drift passed.
  First UI run caught one old note-text expectation; updated and reran all.
- Edge: 9 network pairing/address-hint scenarios passed, including all three
  themes, field styling and reserved warning-slot geometry. Screenshots under
  `%TEMP%/audiorouter-playwright-results/network-pairing.pw.*/` inspected.
- Network Send/Receive and Plugin Properties-only portrait recordings refreshed;
  all three silent overview videos passed actual Firefox click/playback checks.
  Frames inspected under `target/tool-guide-overviews/`; Firefox results in
  `target/tool-guide-firefox-results.json`. These demonstrate configuration,
  not a live network or licensed-plugin effect.
- Guide audit: 37 backend kinds / 221 parameter descriptors accounted for;
  grouped picker fields explicitly map to their controls, and 4 API-only
  descriptors have reasons. Soundboard mode choices and the existing Plugin
  unchanged-audio control were missing from guide metadata and are now covered.
  Regeneration preserves exemptions instead of inventing audible evidence.
  Coverage: 38 guides / 252 properties. Video format check: 150 clips pass.
- Documentation acceptance: 225 Markdown files / 1,246 links pass.
- `cargo deny --offline --locked check` for both workspaces passed against
  the cached advisory database. Full JSON diagnostics retained in
  `target/review-deny-{root,shell}.jsonl`. Duplicate-version warnings cover
  base64, bitflags, block-buffer, cpufeatures, crypto-common, digest,
  getrandom, hashbrown, indexmap, miniz_oxide, sha2, syn, thiserror and its
  derive crate, toml/toml_datetime/winnow and Windows crate families. Their
  upstream dependencies require incompatible major/minor versions; this
  transport change does not force incompatible dependency replacements.
  Unencountered ignored-advisory warnings are shared-config bookkeeping;
  the shell still needs the documented temporary Tauri/unic exceptions.
  Offline checks are not evidence that the advisory database was refreshed.

Residual limits: weak user keys are still guessable; metadata is visible;
bounded authenticated negotiation does not prevent denial of service. The
existing replay cache retains eight stream IDs, with 64 sequences per stream;
restart protection comes from a fresh receiver token rather than that cache.
No attended physical route, two-PC network or release executable was exercised.
Rollback is the complete v3 commit with its compatibility copy and guide assets.

### Combined release notices (2026-11-01)

Old P2-4 remainder: release preparation now retains `sbom.shell.cargo.json`
alongside the root Cargo SBOM, and generates notices from their union.
Duplicates share name/version/source; conflicting license metadata fails
instead of dropping a notice. The verifier and M08 acceptance inventory require
the shell SBOM, and the distribution runbook names it.

- `test-cargo-notices.ps1 -RealMetadata`: fixtures pass for shell-only/root-only
  packages, duplicates, two versions, fallback labels, input order and license
  conflicts. Real offline locked metadata yields 508 packages and includes
  Tauri, XChaCha20/cipher/inout. Artifact: `target/review-cargo-notices.txt`.
- `test-verify-artifacts.ps1`, `test-app-only-release.ps1`: pass, including a
  missing shell-SBOM manifest entry rejection. Documentation acceptance passes.
- Full release preparation was not run: it builds/packages release inputs
  beyond this notice fix. No release, installer or signing claim is made.
- Rollback: revert this tooling change. Existing release archives remain intact;
  the updated verifier requires the additional SBOM for newly checked manifests.

### Remaining mechanical splits: execution order

P2-5 / old P2-2: move the recording-library actions from App into a hook,
canvas connector measurement into its own module, engine delay compensation into
a module, and native packet codecs into a module. Preserve public exports,
callbacks, state lifetime and existing tests. These are first bounded concern
boundaries; larger compiler/endpoint moves can follow independently. Verify
typecheck immediately per UI move, compile per Rust move, then existing full
UI/Rust and browser suites. No API, audio policy or control changes; rollback
each mechanical commit without data migration.

Implemented concern boundaries: `useRecordingActions.ts` owns recording-library
mutation serialization and preview/recovery ordering; `CanvasConnectorMeasurements.tsx`
owns connector readiness/repair; `latency_compensation.rs` owns branch delay
preparation and processing; `packet_codec.rs` owns native planar/interleaved
conversion. Public Rust exports and App/Canvas callers are preserved. This
bounded pass does not claim that the remaining large compiler and endpoint
implementations have become small files.

- Immediate typecheck/compile passed after each move. Initial canvas lint found
  three now-unused imports; removed, then lint and format checks pass.
- Existing UI suite: 786 tests / 87 files pass. Edge: 14 recording-library,
  real WAV recording/settings and workbench scenarios pass (13.6 s), including
  all themes and unchanged control geometry.
- Full engine and native-audio test command passes, including 33 deterministic
  audio tests (113.76 s), 6 planar reference comparisons and all source/tool
  combination tests; native library 145 passed / 4 opt-in ignored. Explicit
  private-session and physical-endpoint tests remain gated.
- Both all-target/all-feature Clippy scopes pass; both Rust format checks pass.
  No audio/UI behavior, control or API changed, so no guide recapture applies.
- Connector-specific Edge regressions: 7 pass (7.3 s), including cold repair in
  dark/light/high-contrast, reload/commit and exhausted bounds. React Flow's
  missing-handle warnings are expected during intentionally removed bounds;
  successful repair and exhaustion are explicitly asserted. Its attribution
  warning and Node's conflicting color-environment warning remain as previously
  recorded; no license entitlement is inferred by this change.

### Measurement follow-through

P2-9: use a new target-only database with eight 50 MiB synthetic opaque blobs;
alternate six checked and six ordinary opens, retain CSV, and verify first/last
blob contents. This compares current checked-vs-light paths under warm OS cache,
not an old-binary or cold-disk startup benchmark. The opt-in example refuses an
existing generated directory and never touches the owner's database.
Old P2-6: print the existing real idle-accept test's connect-to-response time;
retain its no-polling, complete-response and stop/queue assertions. This is
socket intake evidence, not a full UI/Stream Deck latency claim.

Windows MSVC / Rust 1.96.0 results:

- `cargo bench -p audiorouter-engine --bench realtime --locked`: pass.
  Silent tails range 2.2–44.0 microseconds/128-frame quantum. Spectral Gate's
  tail is 30.4 vs. 26.6 microseconds on voice; no order-of-magnitude tail
  slowdown appeared. Pitch's voice processing is 54.2 microseconds; the
  32-tool route is 330.5 microseconds (12.39% of the 2.667 ms budget).
  These are local optimized microbenchmarks while the owner's app runs,
  not device scheduling, reference-PC release NFRs or a physical latency gate.
- `cargo run --release -p audiorouter-storage --example review_open_timing
  --locked`: pass. Checked opens 318.390–440.378 ms; ordinary opens
  1.649–2.977 ms, six each in alternating order on the same 400 MiB media DB.
  First/last blobs verified unchanged. CSV:
  [storage open timing](evidence/2026-11-01-storage-open-timing.csv).
  Generated DB retained only in `target/review-storage-open-40028`.
  Initial example compilation caught StorageError lacking std::error::Error;
  it now maps errors at the example boundary without changing storage errors.
- Full shell Windows unit suite: 86 passed, 4 opt-in ignored. Initial `--lib`
  invocation correctly reported no library target; reran the real binary target.
  Full transport suite: 34 library tests and the glitch detector pass;
  explicitly attended/private/device scenarios remain ignored.
- `idle_accept_waits_in_the_kernel_instead_of_polling -- --nocapture`:
  pass; connect-to-response after 300 ms idle is 1.0696 ms in this run.
  One sample through the socket pool is not an HTTP API percentile claim.
- Both all-target/all-feature Clippy scopes and both Rust format checks pass.

### Owning branch and document lifecycle

`feature/plugin-library-ux` has `ed1e41ec`: both compatibility and error-code
switches enforce `never`, and regression fixtures enumerate the declared
unions. Typecheck/format/lint and 791 UI tests pass on that branch; its plan
retains the independent desktop-app qualification gate. It was not merged into
this branch, and the working checkout returned to the review-fixes branch.

Both review records physically moved from `future/` into `active/`; historical
findings remain unchanged and point to this status record. The future index
no longer lists authorized active work. Soundboard and the other completed
execution plans remain archived as recorded earlier.

### Live gate preparation (executed 2026-11-02, see "Live continuity run")

Read-only `cargo run -p audiorouter-cli --locked -- devices list --json`
confirms the harness's CABLE and CABLE-B endpoint IDs still exist at 48 kHz.
CABLE-B render is eight channels; its capture is stereo. The default capture
is CABLE-A, which this harness does not open. No microphone or speaker endpoint
is part of the proposed test. The optimized harness was built with
`cargo test -p audiorouter-transport --test live_audio_continuity --release
--locked --no-run`.

Pending owner confirmation because the two cables may feed a running call or
stream: baseline and optimized-chain synthetic-tone runs, 20 seconds each,
on a disposable private backend/pipe. Set `AUDIOROUTER_LIVE_CONTINUITY=1`,
`AUDIOROUTER_CONTINUITY_SECONDS=20` and the explicit four confirmed endpoint
IDs; run `live::live_backend_service_keeps_a_routed_tone_continuous` with
`--ignored --exact --nocapture`. Baseline chain is `gain`; second chain is
`gain,parametricEq,graphicEq,bassTreble,dehum,compressor,gate,limiter,delay,meter`.
Retain console output and service phase maxima; require a clean reference and
no unexplained routed glitches. Clear unrelated optional continuity/plugin
environment settings in the test process before launch. A failure is evidence
to investigate, not a waived gate. Large-library concurrent Play/export/import
measurement and the latency-changing commercial-plugin run remain separate
follow-ups; the baseline cannot stand in for either.

### Live continuity run (2026-11-02, owner-approved)

The owner confirmed the cables were free ("I dont play"). CABLE-B Input is
now set to 8 channels in Windows, and both workers refuse it
(`nativePaths.prepare`: "output fan-out endpoints must be stereo IEEE
float32"; endpoint mode: "audio frame size was invalid"). With the owner's
approval the result path used CABLE-A instead: tone → CABLE Input, route
CABLE Output → chain → CABLE-A Input, result recorded from CABLE-A Output
(`AUDIOROUTER_CONTINUITY_ROUTE_RENDER_ID` and `_RESULT_CAPTURE_ID`
overrides; the harness still prints "CABLE-B Output" as a fixed label).
The release harness ran for 20 s per chain, with the owner's release shell
running on its own pipe.

| Chain | Reference glitches | Routed glitches | Silent runs | Late gaps | Max gap | Underruns |
| --- | --- | --- | --- | --- | --- | --- |
| `gain` | 0 | 0 | 0 | 0 | 3.83 ms | 0 |
| gain, parametricEq, graphicEq, bassTreble, dehum, compressor, gate, limiter, delay, meter | 0 | 0 | 0 | 0 | 4.80 ms | 0 |

Logs: [baseline](evidence/2026-11-02-live-continuity/baseline.log),
[optimized chain](evidence/2026-11-02-live-continuity/chain.log). This
closes the 2026-10-07 P2-7 continuity gate. Still open: large-library
concurrent Play/export/import service timing and the latency-changing
commercial-plugin run. A side finding, not fixed here: a route whose output
is a multichannel (8-channel) endpoint cannot start, so the owner's own
routes to CABLE-B would fail while it stays at 8 channels. The Gate's
telemetry showed a subnormal `gainReductionDb` (1.7e-43); the silent-tail
bench found no slowdown from it.

### Repeat runs and an isolation finding (2026-11-02)

After the owner set CABLE-B Input back to stereo, six more 20 s `gain` runs
alternated the result cable (tone → CABLE Input, route CABLE Output →
CABLE-B or CABLE-A). Two passed; four failed with 2–10 routed glitches
(exact-silence runs of 10–86 ms, mostly in the first 5 s). The backend
service was never late in these runs (0 late gaps, pump ≤ 0.24 ms), and the
reference path, which contains no AudioRouter code, also glitched in two of
the three CABLE-B runs. Logs: [repeat/](evidence/2026-11-02-live-continuity/repeat/).

Cause of the doubt: the cables were not free. The owner's shell (PID 35804)
has played its main session since 2026-11-01 (backend log `session.start`,
no stop). That route renders the voice to CABLE-A Input, loopback-captures
CABLE-B Input (`siege-in`) and sends game sound to CABLE Input. A CABLE-B run
therefore formed a loop (result → `siege-in` → CABLE Input → the test's
route input), and a CABLE-A run mixed with the owner's voice output. The
morning runs used CABLE-A under the same condition. Their zero-glitch result
stands, but they are not isolated evidence, and the failures above cannot be
attributed to AudioRouter or to the system until the runs are repeated with
the owner's route stopped. Next action: with the owner's approval, stop the
main route, repeat three runs per cable, then restart it.
