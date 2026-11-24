# Synthetic service timing: bundle export/import and slow methods — 2026-11-24

Closes the measurement half of two open items, on synthetic data only:

- the code-review-fixes plan's "large-library concurrent Play/export/import
  service timing"
  ([code-review fixes](../code-review-fixes.md#live-continuity-run-2026-11-02-owner-approved));
- resilience finding RES-6, "the 5-second UI timeout is shorter than work the
  backend allows"
  ([audit](../../archived/2026-11-13-resilience-audit.md#res-6-the-5-second-ui-timeout-is-shorter-than-work-the-backend-allows),
  [implementation](../../archived/2026-11-13-resilience-implementation.md)).

Requirement IDs: ARCH-08, CAP-04, NFR-03 (control-thread occupancy and audio
service), AUTO (method deadlines). Raw data:
[res6 CSV](2026-11-24-service-timing-res6.csv),
[cadence CSV](2026-11-24-service-timing-cadence.csv),
[pipe CSV](2026-11-24-service-timing-pipe.csv).

## Environment and caveats

- Intel Core Ultra 9 285K (24 cores), 31 GiB RAM, Windows 11 Home build 26300,
  Rust 1.96.0, release profile, system SSD (Lexar NM1090 PRO).
- **Noisy.** The owner's `audiorouter-shell.exe` was running (playing, not
  touched), and other agents' builds and tests shared the PC. The harness ran
  with `CARGO_BUILD_JOBS=6`. Read the medians as "typical on this PC while it
  is in use", not as a benchmark.
- Every database, bundle and pipe belongs to the harness: databases under
  `target/service-timing-*`, a pipe named after the process ID, and `TEMP`
  pointed at a per-run directory (`sessions.importFile` otherwise stages under
  the fixed `%TEMP%\audiorouter-session-import`, which the ordinary test suite
  also uses, so two runs at once could trample each other's staging; this is a
  collision risk of the product design, noted here, not changed). No audio
  device was opened. Nothing touched the owner's database, logs or default
  pipe.
- Each measurement ran three times (`AUDIOROUTER_TIMING_RUNS=3`); odd runs
  reverse the order of the two bundle kinds. The tables give median and maximum
  of the three; the CSVs hold every row.

## What "large library" means here

The plan never quantifies it, so the fixture fills each store toward its own
cap:

| Store | Fixture | Cap it approaches |
| --- | --- | --- |
| Audio media | 25 blobs of 16 MiB − 1 KiB = ~400 MiB (15 compressible, 10 noise) | 512 MiB total (`MAX_AUDIO_MEDIA_TOTAL_BYTES`); the 2026-11-01 open-timing fixture was 400 MiB |
| Sessions | 100 one-node fillers + two export sessions (16 and 7 nodes) | 128 sessions, 128 nodes (`MAX_SESSIONS_GLOBAL`, `MAX_NODES_GLOBAL`) |
| Recordings | 2,000 rows | listing pages of 500 |
| Bundle | ~96 MiB incompressible (100,689,465 bytes zipped) and ~240 MiB expanded compressible (3,217,030 bytes zipped) | 100 MiB compressed, 250 MiB expanded, 1,000 entries (`MAX_BUNDLE_*`); media entries may each be up to 64 MiB, other entries 16 MiB |

The two bundle kinds exist because the limits interact: noise (like real
audio) barely compresses, so a bundle tops out near 100 MiB; the 250 MiB
expanded limit is reachable only with highly compressible data (the
compressible fixture repeats one sine period). Graph history, plugin states
and a real sound library were not populated.

## Commands

From the repository root, Windows, release mode:

```text
$env:CARGO_BUILD_JOBS = 6; $env:AUDIOROUTER_TIMING_RUNS = 3
$env:AUDIOROUTER_TIMING_CSV = "<csv path>"
cargo test -p audiorouter-control --release --lib --locked service_timing_tests::res6_slow_method_timings -- --ignored --exact --nocapture
cargo test -p audiorouter-control --release --lib --locked service_timing_tests::service_cadence_while_bundle_work_runs -- --ignored --exact --nocapture
cargo test -p audiorouter-transport --release --locked --test service_timing_pipe -- --ignored --exact pipe_quick_requests_wait_behind_bundle_work --nocapture
```

`AUDIOROUTER_TIMING_SCALE=8` shrinks every blob for a quick smoke run. The
harnesses are `crates/control/src/service_timing_tests.rs` and
`crates/transport/tests/service_timing_pipe.rs`. Every measuring test is
`#[ignore]`, so CI does not run them; one small catalog check
(`deadline_classes_match_the_contract_catalog`) runs normally and keeps the
harness's deadline classes in step with `METHOD_DEADLINES`.

## A. Does a bundle export/import hold the audio service?

### Background in the code

- Since PR #46 (2026-10-20) the multi-input worker, which every route started
  from the app uses, is pumped by a dedicated `audiorouter-audio` thread that
  only `try_lock`s the worker (`audio_thread.rs`). A long control request
  cannot stop that thread. The 2026-11-02 plan text still describes the
  control thread as the audio service thread; that now holds only for the
  other worker kinds the control thread pumps itself (endpoint, duplex,
  render source, cable route), the recorder drain and the periodic
  maintenance in `service_running_native_audio`.
- `sessions.exportFile` and `sessions.importFile` run their file work
  (ZIP, hashing, database restore) on a helper thread under
  `while_servicing_audio`, which calls `service_running_native_audio` every
  1 ms meanwhile. The control thread is still occupied for the whole request:
  it takes no other request until the handler returns.

### A1. Service-pass cadence while the bundle work runs

`service_cadence_while_bundle_work_runs` runs the real export/stage/restore
work under the production `run_while_servicing` with a service closure that
calls the real `service_running_native_audio` and timestamps each call, and
compares it with a same-length idle sleep. Median/max of 3 runs:

| Phase | Work (ms) median / max | Service passes (median) | Gap, median of runs (ms) | Gap, max (ms) | Gaps > 8 ms |
| --- | --- | --- | --- | --- | --- |
| compressible: prepare (database read) | 250 / 256 | 221 | 1.04 | 1.66 | 0 |
| compressible: ZIP write | 186 / 188 | 149 | 1.04 | 1.65 | 0 |
| compressible: idle baseline of the same length | 187 / 188 | 162 | 1.04 | 1.68 | 0 |
| compressible: stage (unzip, hash) | 387 / 397 | 324 | 1.04 | 2.38 | 0 |
| compressible: restore (database write) | 1,267 / 1,274 | 1,051 | 1.04 | 2.07 | 0 |
| noise: prepare | 96 / 97 | 82 | 1.04 | 1.68 | 0 |
| noise: ZIP write | 1,348 / 1,412 | 1,141 | 1.04 | 2.06 | 0 |
| noise: idle baseline of the same length | 1,348 / 1,413 | 1,188 | 1.04 | 1.85 | 0 |
| noise: stage | 181 / 205 | 151 | 1.04 | 1.70 | 0 |
| noise: restore | 559 / 693 | 454 | 1.04 | 1.73 | 0 |

The loaded gaps match the idle baseline: ZIP, hashing and SQLite on the
helper thread do not starve the 1 ms service closure on this 24-core PC. The
8 ms threshold is `AUDIO_SERVICE_LATE_GAP`. An idle pass (no route) costs
< 0.01 ms (`service_running_native_audio`, 2,000 passes per run, max 0.005 ms),
so the cadence is dominated by the 1 ms sleep.

**Inline steps with no service pass.** After the staged restore, an import
runs the occupancy check and the session insert on the control thread:
`ensure_session_loaded` + `create_session` took a median of 0.8–1.1 ms
(max 3.1 ms). Importing the same small bundle 100 times into one database
(each copy searches the `-imported-N` suffix) took 5.0 ms for the first copy
and 6.1–7.8 ms for copies 10–100, so the suffix search does not grow in a
way that matters (the session cap is 128). Neither number can by itself
create an audible gap.

### A2. How long does a quick request wait behind an export or import?

`pipe_quick_requests_wait_behind_bundle_work` runs the production serve loop
(`serve_control_connections_forever_with_grant`) on private pipes. A poller
sends `diagnostics.getVerbose` (class `quick`, 5 s UI deadline) every 20 ms
over a real pipe while the heavy request runs. Because the loop handles one
request at a time, the poll sent after the heavy request starts waits for all
of it:

| Heavy request | Heavy request, median / max (ms) | Longest poll wait, median / max (ms) | Polls over 5 s | Failed polls |
| --- | --- | --- | --- | --- |
| none (idle, 1 s window) | — | 0.2 / 0.4 | 0 | 0 |
| `exportFile`, compressible (3.2 MB zipped) | 417 / 452 | 411 / 433 | 0 | 0 |
| `exportFile`, noise (96 MiB zipped) | 2,687 / 3,266 | 2,668 / 3,247 | 0 | 0 |
| `importFile`, compressible bundle | 1,878 / 2,599 | 1,868 / 2,586 | 0 | 0 |
| `importFile`, noise bundle | 969 / 986 | 968 / 975 | 0 | 0 |
| `exportFile` and `importFile` sent together | 2,469 / 3,067 (both) | 2,261 / 2,460 | 0 | 0 |

The longest quick-request wait was 3.25 s: 65 % of the 5 s UI deadline, with
no request over it. Nothing failed. Notes:

- The pipe export of the noise bundle took 2.7 s, against 1.5 s for the same
  `exportFile` dispatch in A1/B. The pipe run builds three databases of
  hundreds of MiB moments before it, so the extra second is most likely disk
  write-back competing with the ZIP write; this was not verified.
- The combined case is queued, not parallel: the second request starts when
  the first returns.

### A3. Conclusion and limits

On this PC a maximum-size bundle export or import occupies the control thread
for up to ~3.3 s, and the service closure keeps its 1 ms cadence with no
gap over 8 ms during that work. App-started routes are pumped by their own
thread anyway. That answers the plan's question for the code as it is:
**no service-gap defect was found.**

What this does not show:

- The harness had no audio device, so `audioService.lateGaps` was never
  incremented (it only counts while a worker runs) and no route was actually
  playing. The cadence is of the service closure, not of audio. It does not
  qualify an endpoint, duplex, render-source or cable route, nor a slower
  disk or CPU (a mechanical disk or antivirus scanning would lengthen file
  work, though the closure runs on a different thread).
- The synthetic library is not the owner's: no graph history, plugin
  states or real sound library.
- The live gate with a route playing on isolated VB-Cable stays open in the
  plan; this file does not stand in for it.

## B. RES-6: slow methods against their deadlines

`METHOD_DEADLINES` gives `quick` methods the ordinary 5 s deadline and the
others 60 s (`requestTimeoutMs`, `SLOW_REQUEST_TIMEOUT_MS`). Every method
below was timed through `ControlPlane::dispatch`
(`res6_slow_method_timings`), median / max of three runs, with the source
library above (the pipe timing in A2 adds the wait behind any request in
flight).

| Method | Class | Case | Median / max (ms) | Of its deadline (max) |
| --- | --- | --- | --- | --- |
| `sessions.approveExport` | long | — | 0.3 / 0.5 | < 0.01 % |
| `sessions.exportFile` | long | compressible, 240 MiB expanded | 414 / 424 | 0.7 % |
| `sessions.exportFile` | long | noise, 96 MiB | 1,496 / 1,569 | 2.6 % |
| `sessions.importFile` | long | compressible, 240 MiB expanded | 1,624 / 1,802 | 3.0 % |
| `sessions.importFile` | long | noise, 96 MiB | 609 / 749 | 1.2 % |
| `sessions.importPlan` | long | 16-node document | 0.1 / 0.1 | < 0.01 % |
| `sessions.importCommit` | long | 16-node document | 0.3 / 0.4 | < 0.01 % |
| `audioMedia.beginUpload` | long | 20 MiB WAV | < 0.1 / 0.1 | < 0.01 % |
| `audioMedia.uploadChunk` | long | 192 KiB, slowest of 107 | 1.7 / 1.9 | < 0.01 % |
| `audioMedia.uploadChunk` | long | sum of 107 chunks | 156 / 156 | 0.3 % |
| `audioMedia.finishUpload` | long | 119 s 44.1 kHz stereo, decode and store | 180 / 201 | 0.3 % |
| `audioMedia.delete` | long | 20 MiB | 18 / 24 | 0.04 % |
| `sessions.get` | quick | — | < 0.1 / < 0.1 | < 0.01 % |
| `sessions.list` | quick | 100+ sessions | 0.8–1.3 / 1.3 | < 0.03 % |
| `recordings.list` | quick | one page of 500 | 6.4 / 11.3 | 0.2 % |
| `recordings.list` | quick | 2,000 rows, four pages | 14.9 / 23.5 | 0.5 % |

**Result:** no measured method approaches its deadline. The slowest, an
`importFile` of the 240 MiB-expanded bundle, took 1.8 s against 60 s, and the
slowest quick call took 23 ms against 5 s. Each handler returned in a
fraction of a second per the numbers above; the dominant cost is I/O, not
queueing.

What it means for the deadline classes:

- The 60 s deadline for `*.exportFile`/`importFile` has more than 30× headroom
  on this PC for the maximum bundle sizes the limits allow.
- A `quick` request is the one that can exceed its deadline, because it waits
  behind a running heavy request (A2): up to 3.3 s here against 5 s. This
  holds only while a bundle operation is in flight, and a slower disk would
  push the 3.3 s toward 5 s. RES-5's stale-status path is therefore
  reachable on slow hardware during a large export, not on this PC.

**Not measured, and why:**

| Method | Reason |
| --- | --- |
| `session.stop`, `recorders.stop` | Need a running route or recorder, which needs audio devices; recorder finalization is bounded at 10 s in `threaded_recorder.rs`. |
| `system.checkSetup` | Probes the installed audio and driver setup. |
| `virtualDevices.provision` | Installs or changes a driver. |
| `audioMedia.importTemporaryRecording` | Needs a completed recording in a recorder root. |
| `plugins.*`, `*.prepare` | Need a plugin or real endpoints (already `long`). |

### Observations (not fixed, no defect)

- **A 64 MiB upload cannot be a WAV.** `audioMedia.beginUpload` accepts up to
  64 MiB, but `finishUpload` refuses any file over 120 s, counted in source
  frames (119 s at 96 kHz was refused with "exceeds the 120 second duration
  limit"; 119 s at 44.1 kHz stereo 16-bit, 20 MiB, was accepted). The user
  learns of a too-long file only after sending all of it. Reproduce: run the
  res6 harness with the upload built from 96 kHz stereo. Low severity; it
  costs time, not data.
- **Export and import agree on asset size.** The importer allows 64 MiB per
  media entry and 16 MiB for other entries, so a 20 MiB audio asset round-trips.
  `export_of_an_asset_over_the_import_limit` (ignored probe, run with the same
  `--ignored --exact --nocapture` pattern) stored one, exported it and
  imported the bundle: both succeeded. The 16 MiB limit applies to plugin
  state entries; exporting an oversized plugin state was not tried.
- **Global caps bite before size.** Importing the 16- and 7-node bundles into
  a database that already held 107 nodes stopped with "global.nodes: 130
  exceeds maximum 128" (the first pipe run). The pipe harness then used 60
  fillers. That is the documented cap, not a timing issue.

## RES-6 note (kept here, not in the plan summary)

RES-6's open item, "representative large-import/export timings", is
answered for the device-free methods: every method listed above stays under
4 % of its deadline at the limits. The residual risk is the waiting time of a
quick request behind a running bundle operation (up to 3.3 s of 5 s on this
PC) and the methods that need devices (stop, setup check, driver
provisioning), which still have no measurement. No operation-status poll
for a mutation whose reply is lost after its deadline was added or judged
necessary by this data.
