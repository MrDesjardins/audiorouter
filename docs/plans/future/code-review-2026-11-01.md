# Repository code review — 2026-11-01

A second full review of the repository, after the
[2026-10-07 review](code-review-2026-10-07.md). It re-checks that review's
open items and reads the code added since then: the plugin latency and
licence guard (U4/U6), the Soundboard, network audio and the HTTP/MCP
surface. Findings are ranked P0 to P3. Like every plan in this folder, this
is a backlog, not authorized work. Each item needs the owner's go-ahead and
a place in the [active plan](../active/current.md).

## How this review was done

- **Source:** `origin/main` at `f2b84169` (merge of PR #99, the U4/U6
  plugin latency and licence work). The unmerged
  `feature/plugin-library-ux` branch (`2070c7ed`, UI only, 19 files) is
  covered in its own section below. The repository now has about 166,600
  lines of Rust (tests included) and 51,600 lines of TypeScript in `ui/src`.
- **Method:** static reading, guided by greps for the risky patterns:
  allocation, locks and blocking calls near the audio path; `_ =>` arms on
  `NodeKind`; `unsafe` and shared memory; swallowed errors; the HTTP and
  tunnel entry points. Each finding below was traced in the code, not only
  matched by a grep. Items that need a run or a measurement to confirm say
  **(measure)** or **plausible**.
- **Checks run (Windows 11, this PC, 2026-11-01):**
  - `cargo deny check` for the workspace and for `src-tauri`: advisories,
    bans, licences and sources all ok. Only duplicate-version warnings
    (`syn`, `base64`, `bitflags`, …). The five unic advisory ignores are
    reported as "not encountered" in the workspace because they apply only
    to `src-tauri`, which is expected.
  - `npm audit` in `ui` (production dependencies) and `contracts`: 0
    vulnerabilities.
  - GitHub: CI, CodeQL and the website deploy passed on `f2b84169`; the
    nightly quality run passed on `068b395a`.
- **Not run:** `cargo build`, `cargo test`, Clippy, Vitest and Playwright.
  The owner's `audiorouter-shell.exe` was running, and heavy compiles
  starve live audio. Nothing was measured with audio hardware. Line numbers
  refer to `f2b84169` and will drift.

## What is strong

- **Real-time plugin handoff.** `PluginRuntimeBridge::process`
  (`crates/plugin-host/src/lib.rs`, `RealtimePluginProcessor` impl) uses only
  bounded lock-free queues and atomics. It never waits, allocates or crosses
  into IPC; the runtime thread does the worker exchange. Network audio keeps
  its real-time side on `try_lock`.
- **Owner rules are visibly followed in new code.** `records_from_a_device`
  (`crates/control/src/plugins.rs`) classifies every `NodeKind` with no
  catch-all. The AUTO-18 reachability test guards REST and MCP parity.
- **HTTP API.** It checks the exact Host and Origin, compares the token in
  constant time, refuses chunked bodies and query strings, bounds header
  size and time, rate-limits, and keeps desktop-only methods (folder choice,
  device consent, OBS changes) off the token. The LAN listener accepts
  private and link-local peers only, and the UI and docs say plainly that
  its traffic is not encrypted.
- **MCP tunnel.** It starts only when the user asks. The runtime key is
  stored with DPAPI, zeroized on drop and passed by environment, not on the
  command line. The health URL must be loopback.
- **Supply chain.** It is clean in both Cargo workspaces and both npm
  packages, and CI enforces it.

## Summary

| ID | Priority | Area | Finding |
| --- | --- | --- | --- |
| [P2-1](#p2-1-the-licence-guard-does-not-watch-vst2-chains) | P2 | Privacy | The unchanged-audio guard never watches a shared VST2 chain, while the control plane marks its members as guarded |
| [P2-2](#p2-2-the-latency-poll-can-stall-a-plugin-for-5-s-then-kill-it) | P2 | Reliability | The 0.5 s latency poll can block a plugin's audio for 5 s and then end its worker |
| [P2-3](#p2-3-shared-memory-seqlock-is-not-sound-rust) | P2 | Soundness | Plugin shared-memory seqlock: aliasing through `MmapMut`, no reader fence (carried over) |
| [P2-4](#p2-4-two-latency-tables-and-catch-all-arms-in-the-domain-catalog) | P2 | Correctness | The route latency estimate ignores most DSP delays; two `_ =>` arms break the exhaustive-kind rule |
| [P2-5](#p2-5-the-largest-files-grew-back) | P2 | Maintainability | `App.tsx` and `SessionFlowCanvas.tsx` grew back past the size P2-2 split them from |
| [P2-6](#p2-6-open-items-carried-from-2026-10-07) | P2 | Mixed | Items from 2026-10-07 still open (not re-measured) |
| [P3-1](#p3-1-smaller-items) | P3 | Mixed | Smaller cleanups |

## P2

### P2-1 The licence guard does not watch VST2 chains

- **Evidence.** The runtime thread creates the unchanged-audio watch only
  for a single-member bridge:
  `let mut unchanged_watch = (members == 1).then(...)`
  (`crates/plugin-host/src/lib.rs:4380`). `start_plugin_bridges` merges
  adjacent, non-bypassed VST2 plugins joined by identity edges into one
  worker (`plugin_chain_groups`, `crates/control/src/plugins.rs:172-184`).
  It still calls `set_silence_if_unchanged(true)` on every member
  downstream of a microphone (`plugins.rs:274`). All members share the
  head's `silence_unchanged` flag (`lib.rs:4292`), so the last member's call
  wins for the whole chain. The unit test
  `unchanged_audio_is_silenced_only_downstream_of_a_recording_device`
  asserts that chained plugins are guarded (`["b", "c", "d"]`), but no
  watch ever runs for them.
- **Impact.** Microphone → VST2 A → VST2 B, where both plugins pass audio
  through unchanged (unlicensed or self-bypassed): the dry voice goes on
  indefinitely with no silence and no hint, while the API reports the
  guard's policy as applied. DEC U6 scopes the guard to "a single plugin",
  so this is a documented limit. However, the code and the test suggest
  wider coverage than the bridge delivers. VST3 (UADx) plugins each get
  their own worker, so the owner's UADx case is covered. A chain's
  `passThroughAllowed` also depends on member order, which is a latent bug
  if a chain watch is added.
- **Fix.** Either watch the chain as one unit (compare the chain's input
  with its final output; silence the whole chain when any member is
  guarded), or keep VST2 members on a guarded path out of shared chains.
  Make the policy flag per member, or compute one chain verdict (`any`)
  before calling the bridge. Make the test check the bridge behavior, not
  only `plugin_unchanged_guards`.
- **Verify.** A worker-process test with two unity-gain fixture plugins in a
  chain behind a `PhysicalInput`: output silent after about 2 s; Retry
  refuses while the audio is still unchanged.

### P2-2 The latency poll can stall a plugin for 5 s, then kill it

- **Evidence.** Every 24,000 frames the runtime thread calls
  `worker.plugin_latency_samples(now)` for each member
  (`crates/plugin-host/src/lib.rs:4585-4609`). It blocks in `read()` for up
  to `WORKER_RESPONSE_TIMEOUT` = 5 s (`lib.rs:51`); a failure calls
  `terminate_child` and `record_failure`. In the VST3 worker, after the
  plugin calls `restartComponent(kLatencyChanged)`, the next Latency
  request skips the cache and goes to the plugin's UI thread through
  `SendMessageTimeoutW`. That thread re-activates the component
  (`tools/m06-vst3-worker/main.cpp`, `audio_thread_main` and
  `restart_for_latency`). The worker's own UI timeout is 15 s
  (`kUiRequestTimeoutMs`, `main.cpp:1354`), three times the host's.
- **Impact (plausible, not reproduced).** A plugin whose UI thread is busy
  just after it reports a latency change, for example during an
  activation or licence dialog, an editor still loading, or a slow
  re-activation, stops this bridge's audio. No frames are exchanged while
  the poll waits, so the pipeline drains into output misses (silence).
  After 5 s the host kills the worker, which counts toward quarantine.
  Before U4 the latency was read only at start and after closing the
  editor. The poll now repeats twice a second, so the window is reachable
  in normal use. The mismatched timeouts also mean the host kills the
  worker before the worker's own guard can report the UI hang.
- **Fix.** Keep the poll off the UI thread. The pipe thread always answers
  from the cache and posts the re-activation to the UI thread, and the
  re-activation updates the cache when it finishes. Or give the poll a
  short deadline that leaves the worker alive when it is missed. Make the
  worker's UI timeout shorter than the host's response timeout.
- **Verify.** **(measure)** Add a fixture mode that sets the restart flag
  and then sleeps 6 s on the UI thread. Audio must keep flowing and the
  worker must not be failed. Run the sine-continuity harness with a
  latency-changing plugin.

### P2-3 Shared-memory seqlock is not sound Rust

- **Evidence.** `SharedAudioRegion` still holds a `MmapMut`
  (`crates/plugin-host/src/lib.rs:6447`), the open item from the
  2026-10-07 unsafe audit. `state()` (`lib.rs:6602`) builds an
  `&AtomicU64` from `self.map.as_ptr()`, a pointer from a `&[u8]` borrow,
  and writes through it. `write()` holds that pointer across
  `self.layout.write(&mut self.map, …)` (`lib.rs:6541`), a `&mut` borrow of
  the same bytes, and uses the pointer again afterwards. Readers copy the
  payload with plain loads while the other process may be writing it, then
  re-check the state word with an `Acquire` load and no
  `fence(Acquire)` between them (`read`/`read_into`, `lib.rs:6565`).
- **Impact.** It works on x86 today, and the torn-read check catches real
  races in practice. Under Rust's aliasing model, though, these are
  undefined behavior, and an acquire load does not keep the earlier payload
  loads before it, so a compiler may legally miss a torn frame. The
  bridge in `windows-audio` already moved to `MmapRaw`.
- **Fix.** Switch to `memmap2::MmapRaw` and access through raw pointers
  only. Copy the payload with `ptr::read_volatile` or atomic-sized loads,
  and put `fence(Acquire)` before the second state load (with the matching
  `fence(Release)` after the writer's odd store). Run the shared-memory
  unit tests under Miri, with a non-file mapping, to check.
- **Verify.** Miri on the region tests; the existing worker-process
  continuity tests.

### P2-4 Two latency tables and catch-all arms in the domain catalog

- **Evidence.** `capability_for_kind` declares per-kind delays (Limiter 240,
  Declick, spectral tools, FIR, Fun Voice Changer) with `_ => 0`, and the
  real-time cost class with `_ => "device-bound"`
  (`crates/domain/src/lib.rs:466-477`). `inspect_routes` computes each
  path's `latency_samples` from a separate list that counts only Pitch
  (hard-coded 1,024) and Delay (`delayMs × 48`), with `_ => 0`
  (`lib.rs:2836-2848`). The UI shows that number as "N samples estimated
  latency" (`ui/src/graphView.ts:20`).
- **Impact.** A route with a Limiter, Denoise or FIR filter reports less
  delay than the engine adds, and plugin delay is never included. A new
  processor would silently get latency 0 and the "device-bound" cost
  class. That is the hand-kept list the owner rule (2026-10-29) forbids.
- **Fix.** One exhaustive `fn node_latency_samples(node, sample_rate)` with
  no `_` arm, used by both the catalog and `inspect_routes`. Use the
  engine's sample rate instead of 48, and the reported plugin delay when the
  route is playing. Make the cost-class match exhaustive too.
- **Verify.** A domain test that walks every `NodeKind` and checks that
  the path estimate equals the sum of the catalog values for a chain of that
  kind.

### P2-5 The largest files grew back

- **Evidence.** `ui/src/App.tsx` was 2,588 lines at the last review and was
  split under P2-2. It is now 3,750 lines. `SessionFlowCanvas.tsx` grew
  from 1,310 to 3,680, and `App.accessibility.test.tsx` is 3,057. On the
  Rust side, `crates/engine/src/lib.rs` (18,054) and
  `crates/windows-audio/src/lib.rs` (13,772) were not split as `control`
  was.
- **Impact.** Parallel sessions edit the same few files, which led to the
  2026-10-04 commit mix-up. Reviews of those files are slow, and every
  App-level state change re-renders more (see the 20 Hz telemetry lesson).
- **Fix.** Do it the same way as P2-1/P2-2 last time: mechanical,
  one-area-at-a-time moves with no behavior change. Candidates: the
  plugin, network and soundboard handlers out of `App.tsx`; canvas
  interaction hooks out of `SessionFlowCanvas.tsx`; the graph compiler and
  stage processors out of `engine/src/lib.rs`; the WASAPI endpoint and
  driver-bridge modules out of `windows-audio/src/lib.rs`.
- **Verify.** Typecheck, lint, Vitest, Playwright and the Rust suites
  unchanged; `git diff --stat -M` shows moves.

### P2-6 Open items carried from 2026-10-07

Not re-measured in this review. Each still needs its recorded evidence:

- P0-1 remainder: SQLite reads and writes (saved plugin state at Play,
  imported assets, backups) still run on the thread that services audio.
- P2-7: the Windows sine-continuity run for the optimized DSP loops.
- P3 UI bundle: the main chunk in the local `ui/dist` is about 637 kB
  (this build may be stale). Lazy-load the rarely used panels if a fresh
  build agrees.

## P3-1 Smaller items

- **Misplaced doc comment.** `/// The live runtime bridge of a plugin node in
  a playing route.` now sits above `dispatch_plugins_resume`
  (`crates/control/src/plugins.rs:1012`). `plugin_bridge` (line 1036) lost
  its doc and the new method starts with the wrong sentence.
- **HTTP header read, one byte per call.** `handle` reads headers with a
  1-byte `read_exact` loop (`src-tauri/src/http_api.rs:288`), up to 16 KiB
  of system calls per request. A `BufReader` with the same limits is
  simpler and cheaper; keep the body reader using what is already buffered.
- **48 kHz assumptions.** `PLUGIN_LATENCY_POLL_FRAMES` ("half a second at
  48 kHz") and `UnchangedAudioWatch::WINDOW_FRAMES` (2 s) are frame
  counts. At another engine rate the timing changes. Derive them from the
  sample rate, or document that the engine is fixed at 48 kHz.
- **Silent catch.** `ui/src/unsavedReport.ts:12` drops `set_ui_unsaved`
  failures with `.catch(() => {})`. A failure there means the quit prompt
  could lose unsaved work without a trace. Record a UI diagnostic. The
  other three `.catch(() => undefined)` calls (`App.tsx` polls, `main.tsx`)
  are commented, intentional keep-last-value cases.

## Unmerged branch: `feature/plugin-library-ux`

Three commits on top of the U4/U6 work, now in `main`. UI and docs only, so
no contract change. Reviewed by reading the diff; tests not run here.

- **`pluginSkipReason`** (`ui/src/pluginLibrary.ts`) switches on
  `identity.compatibility` with no exhaustiveness check. A new
  compatibility value would fall through to the `errorCode` switch and
  show "File could not be checked". Add a `never` check so the compiler
  catches it (P3).
- The rest of the diff (library counting by file and format,
  folder outcomes, the scan summary) matches the plan in
  [plugin library UX](../active/plugin-library-ux.md). It still needs the
  three-theme screenshots and the desktop-app run the active plan lists
  before it is called done.

## Suggested order

1. **P2-2**: it can silence and fail a working plugin in normal use, and
   the fix is local to the poll and the VST3 worker.
2. **P2-1**: close the privacy gap or make the limit explicit in code and
   tests.
3. **P2-4**: small, removes a forbidden hand-kept list.
4. **P2-3**: soundness, best done with Miri in CI.
5. **P2-5** and the P3 items alongside feature work.
