# Repository code review — 2026-11-01

A second repository-wide review, after the
[2026-10-07 review](code-review-2026-10-07.md). It re-checks that review's
open items and reads the newest and riskiest code: the plugin latency
and licence guard (U4/U6) and the HTTP and tunnel entry points. A second
pass the same day covered storage, transport, recording, the Soundboard,
the CLI MCP server, the WASAPI code, the driver IOCTLs and the desktop
shell. It is a targeted review, not a line-by-line read of the whole
repository; what each pass read is listed below. Findings are ranked P0
to P3. Like every plan in this folder, this
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
- **Read:** the plugin-host runtime bridge and shared memory, the VST3
  worker's latency path (`tools/m06-vst3-worker/main.cpp`), control
  `plugins.rs`, the domain catalog and `inspect_routes`,
  `src-tauri/src/http_api.rs`, the tunnel launcher (`mcp_tunnel.rs`), a
  lock/allocation scan of `network_audio.rs`, a `NodeKind` catch-all scan
  of all production Rust, and a UI size and swallowed-error scan.
- **Read in the second pass:** the transport pipe framing, accept path and
  the control/audio service loop; storage open, WAL, backup, restore and
  audio-media code; the recording crate's outline and limits; the
  Soundboard media loading (`crates/control/src/sound_library.rs`); the MCP
  stdio loop (`crates/cli/src/lib.rs`); a blocking/allocation scan of
  `windows-audio/src/lib.rs`; filter and envelope state in `dsp`; the
  driver's control-device IOCTL dispatch and device ACL; the desktop
  shell's Tauri commands, CSP and close handling.
- **Still not read line by line:** the engine graph compiler (covered by
  the exhaustive source and tool combination suites), the WASAPI stream
  setup and driver-bridge internals, the WAV writer, the driver's WaveRT
  stream code (it has its own active plan), and the UI beyond the scans
  named above.
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
| [P2-7](#p2-7-backups-fail-once-the-audio-library-passes-100-mib) | P2 | Reliability | Backup and restore refuse databases over 100 MiB; the audio library may hold 512 MiB |
| [P2-8](#p2-8-soundboard-play-reads-and-hashes-sounds-on-the-audio-service-thread) | P2 | Audio | Starting a Soundboard reads, hashes and re-parses library files on the thread that services audio |
| [P2-9](#p2-9-every-database-open-runs-a-full-integrity-check) | P2 | Performance | Every `Storage::open` runs a full `PRAGMA integrity_check`, several times per app start |
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
  indefinitely with no silence and no hint. The `passThroughAllowed`
  description that `list_tool_kinds` gives assistants says that any plugin
  fed by a recording device is silenced (`crates/control/src/catalog.rs:165`),
  which is not true for chain members. DEC U6 scopes the guard to "a single plugin",
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
  Before U4 the latency was not polled while playing. The poll now
  repeats twice a second, so the window is reachable in normal use. The mismatched timeouts also mean the host kills the
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
  unit tests under Miri, if it can run them, to check.
- **Verify.** Miri on the region tests, if Miri can run them (file
  mappings may need an in-memory stand-in); the existing worker-process
  continuity tests.

### P2-4 Two latency tables and catch-all arms in the domain catalog

- **Evidence.** `capability_for_kind` declares per-kind delays (Limiter 240,
  Declick, spectral tools, FIR, Fun Voice Changer) with `_ => 0`, and the
  real-time cost class with `_ => "device-bound"`
  (`crates/domain/src/lib.rs:466-477`). `inspect_routes` computes each
  path's `latency_samples` from a separate list that counts only Pitch
  (hard-coded 1,024) and Delay (`delayMs × 48`), with `_ => 0`
  (`lib.rs:2836-2848`). That number goes out through the `inspect_routes`
  API and MCP output. `ui/src/graphView.ts:20` (`routeLatencyText`) formats
  it, but has no production caller, only tests.
- **Impact.** Through the API and MCP, a route with a Limiter, Denoise or
  FIR filter reports less delay than the engine adds, and plugin delay is
  never included. The playing route's runtime stage delays are a separate
  path and were not checked here. A new
  processor would silently get latency 0 and the "device-bound" cost
  class. That is the hand-kept list the owner rule (2026-10-29) forbids.
- **Fix.** One exhaustive `fn node_latency_samples(node, sample_rate)` with
  no `_` arm, used by both the catalog and `inspect_routes`. Use the
  engine's sample rate instead of 48, and the reported plugin delay when the
  route is playing. Make the cost-class match exhaustive too.
- **Verify.** A domain test that walks every `NodeKind` and checks that
  the path estimate equals the sum of `node_latency_samples` for each node
  (the function takes the node, so Delay's parameter counts). Remove
  `routeLatencyText` or use it.

### P2-5 The largest files grew back

- **Evidence.** `ui/src/App.tsx` was 2,588 lines at the last review and was
  split under P2-2. It is now 3,750 lines. `SessionFlowCanvas.tsx` grew
  from 1,310 to 3,680, and `App.accessibility.test.tsx` is 3,057. On the
  Rust side, `crates/engine/src/lib.rs` (18,054) and
  `crates/windows-audio/src/lib.rs` (13,772) were not split as `control`
  was.
- **Impact.** These are the four largest source files. The two UI files
  grew by 1,162 and 2,370 lines since the last split. Size alone is not a
  defect, so this is a maintainability item.
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

### P2-7 Backups fail once the audio library passes 100 MiB

- **Evidence.** Imported audio (Audio File and Soundboard sounds) is stored
  as blobs in the database, up to 64 MiB each and 512 MiB in total
  (`MAX_AUDIO_MEDIA_BYTES`, `MAX_AUDIO_MEDIA_TOTAL_BYTES`,
  `crates/storage/src/lib.rs:37-38`). `backup_to` writes the backup and then
  deletes it with `DocumentTooLarge` if it is over `MAX_BACKUP_BYTES` =
  100 MiB (`validate_backup_size`, `lib.rs:99-109`). `restore_backup`
  refuses any source over the same limit. The only callers are the CLI
  `backup` and `restore` commands (`crates/cli/src/lib.rs:345-372`).
- **Impact.** Someone who imported a few long sounds (two 64 MiB files, or
  many short ones) can no longer back up their database. The command does
  all the work first and then fails. Restoring an older large backup is
  refused too. The backup command also opens the source with
  `Storage::open`, which runs migrations on it, so "backing up" an older
  database changes it first.
- **Spec.** STATE-12 (`docs/spec/12-persistence.md`) sets a "documented
  100 MiB default configuration backup budget". The spec states no audio
  media budget, so storing up to 512 MiB of audio inside the configuration
  database is where code and spec part ways. The fix needs a decision
  record and a STATE-12 update, not only a new constant.
- **Fix.** Pick one with the owner: keep configuration backups at 100 MiB
  and back up audio media separately (a media-free configuration backup
  plus a media export), or raise the budget to cover the media limit and
  say so in STATE-12. In both cases, open the source read-only for a
  backup, without migrating.
- **Verify.** A storage test that stores audio media past 100 MiB, backs up
  and restores, and checks the restored media bytes.

### P2-8 Soundboard Play reads and hashes sounds on the audio service thread

- **Evidence.** `load_soundboard_media` (`crates/control/src/sound_library.rs:187`)
  runs at Play on the control thread, which also services running audio.
  For each library sound, `decode_board_sound` calls `read_library_sound`.
  That function reads and parses the whole 67 kB manifest again, reads the
  file (up to 2 MiB) and computes its SHA-256, all outside
  `while_servicing_audio`; only the decode runs inside it (`lib.rs:269-283`).
  A user sound is loaded from SQLite with `load_audio_media` the same way.
  Each sound ID is loaded once per Play (the `media` map), a category slot
  expands to the whole category, and the manifest lookup is linear per
  sound.
- **Impact (measure).** Starting a route with a large Soundboard while
  another route plays can delay the next audio pass past the 10 ms
  period. In one request that is up to about 105 manifest parses, file
  reads and hashes (every sound in the shipped `library.json`), plus up to
  9 slots × 64 (`MAX_SLOT_SOUNDS`) = 576 blob reads for the user's own
  sounds. This is the P0-1 remainder in a new place.
- **Fix.** Parse the manifest once per Play and index it by ID. Move the
  file read and hash into the `while_servicing_audio` closure with the
  decode. Load the user's blobs before entering it, or in a short bounded
  step per sound.
- **Verify.** **(measure)** Watch `AudioServicePassTiming` (or the
  `lateGaps` counter) while starting a full-category board with another
  route playing; require no pass later than a few milliseconds.

### P2-9 Every database open runs a full integrity check

- **Evidence.** `Storage::open` runs `PRAGMA integrity_check`, then
  migrations and recovery pruning, every time (`crates/storage/src/lib.rs:763-775`).
  The desktop shell opens it at least twice per start: `recovery_safe_mode`
  (`src-tauri/src/main.rs:1632`), then the backend (`main.rs:1642`). After a
  backend failure it opens it again, in `record_backend_failure` and the
  restart (and in a safe-mode check at `main.rs:1728`). Every CLI command
  and every `mcp serve` launch opens it again.
- **Impact (measure).** `integrity_check` reads every page. With the
  audio library allowed to reach 512 MiB, app start and each MCP client
  launch can pay a full read of the database, at least twice per app start. A corrupt
  database also makes `recovery_safe_mode` quietly return `None`
  (`.ok()?`), and the shell reads that as "not in safe mode"
  (`unwrap_or(false)`, `main.rs:1632`) with no log line; the backend's own
  open fails afterwards.
- **Fix.** Run the full check once per app start (or after an unclean
  shutdown) and use `PRAGMA quick_check` or no check for other opens. Give
  the shell's small reads a light open that skips migrations. Log a
  corrupt-database result where it is swallowed.
- **Verify.** **(measure)** Time `Storage::open` on a database with
  400 MiB of media before and after; a storage test for the lighter open.

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
- **MCP stdio: one non-UTF-8 line ends the session.** `run_mcp_stdio`
  reads with `stdin.lock().lines()` and propagates its error with `?`
  (`crates/cli/src/lib.rs:2143-2144`). Invalid UTF-8 returns `InvalidData`
  and stops the server, unlike invalid JSON, which gets -32700. The whole
  line is also buffered before the 4 MiB check. Read bytes with a bounded
  `read_until` and answer -32700 for bad encoding.
- **Pipe I/O: per-request SID lookup and no read timeout.**
  `dispatch_control_frame` resolves the client SID from its PID on the
  control thread for every request (`crates/transport/src/lib.rs:1834`),
  although `accept_client` already checked the user. Pass the SID from the
  I/O thread instead. The I/O threads read frames with blocking pipe reads
  and no timeout, so a stalled same-user client holding all four instances
  blocks every other client until it disconnects.
- **Soundboard memory budget is per board.** `MAX_BOARD_DECODED_BYTES`
  (256 MB) applies to each enabled board separately
  (`sound_library.rs:20`, `:251`), so several large boards have no total
  bound. Count the budget across the session.
- **Denormals (measure).** Only the Fun Voice Changer flushes tiny values
  (`crates/dsp/src/voice_changer.rs:57`), and no thread sets
  flush-to-zero. Biquads keep `f64` state and the compressor works in dB,
  so the risk is low; the other tools' state was not checked. Time a block
  of silence after a loud tone through each tool in `benches/realtime.rs`.

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
2. **P2-7**: users with a large audio library cannot back up today. It
   needs an owner decision and a STATE-12 update, then a small code change.
3. **P2-1**: close the privacy gap or make the limit explicit in code and
   tests.
4. **P2-8** and **P2-9**: measure first, then fix together with the P0-1
   remainder (work on the audio service thread and at start-up).
5. **P2-4**: small, removes a forbidden hand-kept list.
6. **P2-3**: soundness, best done with Miri in CI.
7. **P2-5** and the P3 items alongside feature work.
