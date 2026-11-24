# Synthetic service timing: bundle export/import, Play preparation and slow methods — 2026-11-24

Closes the measurement half of two open items, on synthetic data only:

- the code-review-fixes plan's "large-library concurrent Play/export/import
  service timing"
  ([code-review fixes](../code-review-fixes.md#synthetic-service-timing-2026-11-24));
- resilience finding RES-6, "the 5-second UI timeout is shorter than work the
  backend allows"
  ([audit](../../archived/2026-11-13-resilience-audit.md#res-6-the-5-second-ui-timeout-is-shorter-than-work-the-backend-allows),
  [implementation](../../archived/2026-11-13-resilience-implementation.md)).

Related requirements (this is evidence, not a gate for any of them): API-07
(an idempotent retry makes a timed-out mutation safe), UI-13 (what the window
shows when the backend does not answer), ARCH-05 and NFR-04 (audio scheduling;
the real-time callback is not exercised here). Raw data:
[res6](2026-11-24-service-timing-res6.csv),
[cadence](2026-11-24-service-timing-cadence.csv),
[play](2026-11-24-service-timing-play.csv),
[pipe](2026-11-24-service-timing-pipe.csv).

## Environment and caveats

- Intel Core Ultra 9 285K (24 cores), 31 GiB RAM, Windows 11 Home build 26300,
  Rust 1.96.0, release profile, system SSD (Lexar NM1090 PRO).
- **Noisy.** The PC was in use during the runs (the owner's
  `audiorouter-shell.exe` was running and not touched; other builds and tests
  shared the machine). The harness ran with `CARGO_BUILD_JOBS=6`. Read the
  medians as "typical on this PC while it is in use", not as a benchmark.
  The `res6`, play and pipe CSVs were re-run after the harness was extended,
  so the tables below quote the final run set only; an earlier pipe run set
  (before the six-poller scenario was added) showed a longest quick-request
  wait of about 3 s, and the same noise `exportFile` took about 1.5 s in the
  cadence run and about 1.8 s in the `res6` run.
- Every database, bundle and pipe belongs to the harness: databases under
  `target/service-timing-*`, a pipe named after the process ID, and `TEMP`
  pointed at a per-run directory (`sessions.importFile` otherwise stages under
  the fixed `%TEMP%\audiorouter-session-import`, which the ordinary test suite
  also uses, so two runs at once could trample each other's staging; this is a
  collision risk of the product design, noted here, not changed). No audio
  device was opened. Nothing touched the owner's database, logs or default
  pipe.
- Each measurement ran three times (`AUDIOROUTER_TIMING_RUNS=3`); odd runs
  reverse the order of the fixtures. The tables give median and maximum of
  the three; the CSVs hold every row.

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
cargo test -p audiorouter-control --release --lib --locked service_timing_tests::play_path_media_load_and_compile -- --ignored --exact --nocapture
cargo test -p audiorouter-control --release --lib --locked service_timing_tests::export_of_an_asset_over_the_import_limit -- --ignored --exact --nocapture
cargo test -p audiorouter-transport --release --locked --test service_timing_pipe -- --ignored --exact pipe_quick_requests_wait_behind_bundle_work --nocapture
```

`AUDIOROUTER_TIMING_SCALE=8` shrinks every blob for a quick smoke run. The
harnesses are `crates/control/src/service_timing_tests.rs` and
`crates/transport/tests/service_timing_pipe.rs`. Every measuring test is
`#[ignore]`, so CI does not run them; one small catalog check
(`deadline_classes_match_the_contract_catalog`) runs normally and keeps the
harness's deadline classes in step with `METHOD_DEADLINES`. A passing control
run deletes its `target/service-timing-*` directory; a failing one, and every
pipe run (its serve threads keep the databases open until the process exits),
leaves it, so remove `target/service-timing-*` by hand afterwards.

## A. Does a bundle export/import or Play preparation hold the audio service?

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
- Play's media preparation (`session_audio_media`) loads each Audio File
  through `with_storage_worker` and decodes it under `while_servicing_audio`,
  so it is serviced the same way; the graph compile afterwards is inline.

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
< 0.01 ms (`service_running_native_audio`, 2,000 passes per run, median
< 0.001 ms, maximum 0.009 ms), so the cadence is dominated by the 1 ms sleep.

**Inline steps with no service pass.** After the staged restore, the import
inserts the session on the control thread; that insert took a median of
0.8–1.1 ms (max 3.1 ms). Importing the same small bundle 100 times into one database
(each copy searches the `-imported-N` suffix) took 5.0 ms for the first copy
and 6.1–7.8 ms for copies 10–100, so the suffix search does not grow in a
way that matters (the session cap is 128). Neither number can by itself
create an audible gap.

### A2. Play preparation

Starting a route needs real endpoints (`nativePaths.prepare`), so the harness
cannot press Play. `play_path_media_load_and_compile` times what Play does on
the control thread before any device is opened, with eight 100 s stereo
16-bit files (a Mixer takes at most 8 sources; 293 MiB decoded):

| Step | 48 kHz files: median / max (ms) | 44.1 kHz files (resampled): median / max (ms) |
| --- | --- | --- |
| load and decode, cold cache (serviced) | 705 / 734 | 601 / 604 |
| load and decode, warm cache | 0.008 / 0.013 | 0.006 / 0.010 |
| multi-input graph compile with the media (inline) | 0.22 / 0.26 | 0.17 / 0.22 |
| `session.start` on the one-shot "cli" plane (simulated runtime; no media, no device) | 0.32 / 0.40 | 0.34 / 0.39 |

The only unserviced step is the compile, at a fraction of a millisecond. The
decode holds the control thread for 0.6–0.7 s, with the service passes
running meanwhile and quick requests waiting (see A3). The 15-node export
fixture is not a playable route (one Mixer takes 8 inputs), which is why Play
is measured on its own fixture.

### A3. How long does a quick request wait behind an export or import?

`pipe_quick_requests_wait_behind_bundle_work` runs the production serve loop
(`serve_control_connections_forever_with_grant`) on private pipes. A poller
sends `diagnostics.getVerbose` (class `quick`, 5 s UI deadline) every 20 ms
over a real pipe while the heavy request runs. Because the loop handles one
request at a time, the poll sent after the heavy request starts waits for all
of it. Median / max of 3 runs (last run set):

| Heavy request | Heavy request (ms) | Longest poll wait (ms) | Polls over 5 s | Failed polls |
| --- | --- | --- | --- | --- |
| none (idle, 1 s window) | — | 0.2 / 0.3 | 0 | 0 |
| `exportFile`, compressible (3.2 MB zipped) | 430 / 463 | 428 / 446 | 0 | 0 |
| `exportFile`, noise (96 MiB zipped) | 2,628 / 2,663 | 2,620 / 2,662 | 0 | 0 |
| `importFile`, compressible bundle | 1,659 / 1,916 | 1,650 / 1,915 | 0 | 0 |
| `importFile`, noise bundle | 889 / 961 | 882 / 951 | 0 | 0 |
| `exportFile` and a re-`importFile` sent together (the imported media is already in that database, so the import restores less than it did the first time) | 2,353 / 2,356 | 2,155 / 2,335 | 0 | 0 |
| `exportFile`, noise, with **6 pollers** at once | 2,633 / 2,746 | 2,622 / 2,728 | 0 | **3 per run (9 over 3 runs)** |

With one poller, the longest quick-request wait in this run set was 2.66 s
(53 % of the 5 s UI deadline), with no poll over the deadline and no failure;
an earlier run set saw about 3 s. Notes:

- The pipe export of the noise bundle took about 2.6 s, against 1.5–1.8 s for
  the same `exportFile` dispatch in B. The pipe run builds three databases of
  hundreds of MiB moments before it, so the extra second is most likely disk
  write-back competing with the ZIP write; this was not verified.
- The combined case is queued, not parallel: the second request starts when
  the first returns.

### A4. Finding: the pipe has four instances, and extra clients fail rather than wait

The pipe server has `SERVER_PIPE_INSTANCES = 4`, each connected client holds
one until its reply, and `connect_client` waits at most `PIPE_BUSY_WAIT` (2 s)
for a free instance. With a long request in flight, the heavy client plus
three pollers fill the instances; in the 6-poller run, 3 of the 9 polls sent
during the export (in every run) failed about 2 s after they were sent (median
poll time 2,000 ms). The error text was not captured; `connect_client`'s
message for this case is "timed out waiting for a free named-pipe instance",
so that is the likely cause, not an observed string. Those are quick requests that would not have exceeded
their deadline had they been allowed to wait. Whether the real adapters
(window, CLI, REST, MCP, Stream Deck) ever have four requests outstanding
during an export was not measured, so this is a candidate defect, not a
confirmed one. Reproduction: the last row of the table above, with the pipe
command in "Commands". Not fixed here; diagnosed and fixed in the
[pipe-busy follow-up](#pipe-busy-follow-up-2026-11-24) below.

### A5. Conclusion and limits

On this PC a maximum-size bundle export or import occupies the control thread
for up to about 3 s, and the service closure keeps its 1 ms cadence with no gap
over 8 ms during that work. App-started routes are pumped by their own thread
anyway. Play's own media decode holds the control thread for about 0.7 s, also
serviced. That answers the plan's question for the code as it is: **no
service-gap defect was found**; the pipe-instance limit in A4 is the one
thing worth a decision.

What this does not show:

- The harness had no audio device, so `audioService.lateGaps` was never
  incremented (it only counts while a worker runs) and no route was actually
  playing. The cadence is of the service closure, not of audio. It does not
  qualify an endpoint, duplex, render-source or cable route, nor a slower
  disk or CPU (a mechanical disk or antivirus scanning would lengthen file
  work, though the closure runs on a different thread). The Play numbers
  cover decode and compile, not `nativePaths.prepare` with real endpoints.
- The synthetic library is not the owner's: no graph history, plugin
  states or real sound library.
- The live gate with a route playing on isolated VB-Cable stays open in the
  plan; this file does not stand in for it.

## B. RES-6: slow methods against their deadlines

`METHOD_DEADLINES` gives `quick` methods the ordinary 5 s deadline and the
others 60 s (`requestTimeoutMs`, `SLOW_REQUEST_TIMEOUT_MS`). Every method
below was timed through `ControlPlane::dispatch`
(`res6_slow_method_timings`), median / max of three runs, with the source
library above (the pipe timing in A3 adds the wait behind any request in
flight).

| Method | Class | Case | Median / max (ms) | Of its deadline (max) |
| --- | --- | --- | --- | --- |
| `sessions.approveExport` | long | — | 0.4–0.6 / 0.6 | < 0.01 % |
| `sessions.exportFile` | long | compressible, 240 MiB expanded | 462 / 492 | 0.8 % |
| `sessions.exportFile` | long | noise, 96 MiB | 1,836 / 1,837 | 3.1 % |
| `sessions.importFile` | long | compressible, 240 MiB expanded | 1,891 / 1,981 | 3.3 % |
| `sessions.importFile` | long | noise, 96 MiB | 826 / 864 | 1.4 % |
| `sessions.importPlan` | long | 16-node document | 0.1 / 0.2 | < 0.01 % |
| `sessions.importCommit` | long | 16-node document | 0.3–0.4 / 0.4 | < 0.01 % |
| `audioMedia.beginUpload` | long | 20 MiB or 61 MiB | < 0.1 / < 0.1 | < 0.01 % |
| `audioMedia.uploadChunk` | long | 192 KiB, slowest of 107 or 326 | 1.7–1.8 / 3.7 | < 0.01 % |
| `audioMedia.uploadChunk` | long | all chunks, 20 MiB (119 s, 44.1 kHz stereo 16-bit) | 150 / 169 | 0.3 % |
| `audioMedia.uploadChunk` | long | all chunks, 61 MiB (83 s, 96 kHz stereo float32) | 447 / 473 | 0.8 % |
| `audioMedia.finishUpload` | long | 20 MiB file, decode, resample, store | 227 / 255 | 0.4 % |
| `audioMedia.finishUpload` | long | 61 MiB file, decode and store | 479 / 538 | 0.9 % |
| `audioMedia.delete` | long | 20 MiB / 61 MiB | 16 / 26 and 60 / 64 | 0.1 % |
| `sessions.get` | quick | — | < 0.1 / < 0.1 | < 0.01 % |
| `sessions.list` | quick | 100+ sessions | 1.0–1.2 / 1.3 | < 0.03 % |
| `recordings.list` | quick | one page of 500 | 7.1 / 7.9 | 0.2 % |
| `recordings.list` | quick | 2,000 rows, four pages | 25.8 / 27.6 | 0.6 % |

**Result:** no measured method approaches its deadline. The slowest, an
`importFile` of the 240 MiB-expanded bundle, took 2.0 s against 60 s, and the
slowest quick call took 28 ms against 5 s. Per request the handler is
dominated by I/O, not queueing.

What it means for the deadline classes:

- The 60 s deadline for `*.exportFile`/`importFile` has more than 30× headroom
  on this PC for the maximum bundle sizes the limits allow.
- A `quick` request is the one that can exceed its deadline, because it waits
  behind a running heavy request (A3): up to about 3 s here against 5 s. This
  holds only while a bundle operation is in flight, and a slower disk would
  push the wait toward 5 s. RES-5's stale-status path is therefore reachable
  on slow hardware during a large export, not on this PC.

**Not measured, and why:**

| Method | Reason |
| --- | --- |
| `session.stop`, `recorders.stop` | Need a running route or recorder, which needs audio devices; recorder finalization is bounded at 10 s in `threaded_recorder.rs`. |
| `system.checkSetup` | Probes the installed audio and driver setup. |
| `virtualDevices.provision` | Installs or changes a driver. |
| `audioMedia.importTemporaryRecording` | Needs a completed recording in a recorder root. |
| `nativePaths.prepare`, `plugins.*` | Need real endpoints or a plugin (already `long`). |

### Observations (not fixed)

- **What a decodable upload can be.** `audioMedia.beginUpload` accepts up to
  64 MiB, but `finishUpload` also applies the decoder's own limits
  (`audio_file_source.rs`): one or two channels, at most 120 s, and at most
  64 MiB of decoded f32 samples (16 Mi samples, frames × channels). A 119 s
  stereo file at 96 kHz and 16 bits was refused during development of the
  harness because it holds 22.8 M samples, although it is only 46 MB (the
  exact refusal text was not kept); the common 16-bit 48 kHz stereo file caps at
  about 23 MB. A 64 MiB upload is therefore possible only as high-rate
  float or 24-bit audio (the 61 MiB 96 kHz float32 file was accepted), and the
  user learns of an over-limit file only after sending all of it. The refused
  case needs a code edit to reproduce (a 96 kHz 16-bit stereo `wav_with_format`
  upload in `res6_slow_method_timings`). Low severity: it costs time, not data.
- **Export and import agree on asset size.** The importer allows 64 MiB per
  media entry and 16 MiB for other entries, so a 20 MiB audio asset
  round-trips. `export_of_an_asset_over_the_import_limit` (ignored probe)
  stored one, exported it and imported the bundle: both succeeded. The
  16 MiB limit applies to plugin state entries; exporting an oversized plugin
  state was not tried.
- **Global caps bite before size.** Importing the 16- and 7-node bundles into
  a database that already held 107 nodes stopped with "global.nodes: 130
  exceeds maximum 128" (the first pipe run). The pipe harness then used 60
  fillers. That is the documented cap, not a timing issue.

## RES-6 note (kept here, not in the plan summary)

RES-6's open item, "representative large-import/export timings", is
answered for the device-free methods: every method listed above stays under
4 % of its deadline at the limits. The residual risks are the waiting time of
a quick request behind a running bundle operation (about 3 s of 5 s on this
PC), the pipe's four-instance limit (A4), and the methods that need devices
(stop, setup check, driver provisioning), which still have no measurement. No
operation-status poll for a mutation whose reply is lost after its deadline
was added or judged necessary by this data.

## Pipe-busy follow-up (2026-11-24)

Diagnosis of the candidate defect in A4, and its fix. Branch
`fix/pipe-busy-during-bundle-work`.

### 1. The error text, captured

The pipe harness now keeps each failed poll's error (`Poll` in
`crates/transport/tests/service_timing_pipe.rs`). One run
(`AUDIOROUTER_TIMING_RUNS=1`, same command as in "Commands"), scenario
"export-noise with 6 pollers":

```text
run 0 export-noise with 6 pollers: poll sent at 2020 ms failed after 2001 ms: Windows("timed out waiting for a free named-pipe instance")
run 0 export-noise with 6 pollers: poll sent at 2021 ms failed after 2001 ms: Windows("timed out waiting for a free named-pipe instance")
run 0 export-noise with 6 pollers: poll sent at 2021 ms failed after 2001 ms: Windows("timed out waiting for a free named-pipe instance")
```

Three failures, all the same text, each after 2.00 s: the pipe-busy hypothesis
is confirmed. Mechanism: `spawn_control_io` runs four I/O threads, each holding
one pipe instance while its client waits for the single control thread
(`crates/transport/src/lib.rs:2033-2040`, instance count `:1380`). The export plus
three pollers fill all four; the other three pollers get `ERROR_PIPE_BUSY` and
`connect_client` stopped waiting after the fixed `PIPE_BUSY_WAIT` of 2 s,
although the export freed an instance 0.5 s later (2.5 s in all). Run data:
[pipe-busy-before.csv](2026-11-24-pipe-busy-before.csv).

### 2. Which real clients can fill four instances

Every adapter opens **one connection per request** (no persistent
connection); a connection holds an instance from connect until its reply.

| Adapter | How it connects | Evidence |
| --- | --- | --- |
| Window (UI bridge) | One `round_trip_with_timeout` per `rpc_request` call, on a Tauri async thread, so calls run concurrently; the page allows 64 pending requests. Pipe timeout 60 s, except `status.get` and `events.subscribe` (2 s) and `system.quit` (15 s). | `src-tauri/src/main.rs:800-811`, `:1429-1437`; `ui/src/host.ts:61,142` |
| HTTP API (Stream Deck, scripts) | Four workers, each forwards one request over its own pipe connection (HTTP-adapter work shares the window's identity, with its own budget). Up to 4 more connections. | `src-tauri/src/http_accept.rs:17`, `src-tauri/src/main.rs:637-653` |
| MCP (`mcp serve`) | One `round_trip` (60 s) per tool call, sequential per process; each assistant session is its own process. | `crates/cli/src/lib.rs:3573` |
| CLI commands | One `round_trip` each; the "is the app running" probe is one more (`system.describe`). The probe treats any error as "the app is not running", so a busy app (all instances held) made it answer "not running" after 2 s, and the CLI could then act on the app's database; with the fix it waits for its turn. A second user-reachable effect of the same defect. | `crates/cli/src/lib.rs:2436-2458` |
| Tray | One forward per click or status poll. | `src-tauri/src/main.rs:2543-2960` |

Can four be outstanding during an export? Yes, from the window alone. While an
export or import holds the control thread, the window has these in flight, each
holding an instance:

1. the export itself;
2. `events.subscribe`, polled every 1 s with a guard against overlap
   (`ui/src/useWorkspaceEvents.ts:198`);
3. diagnostics refresh every 50 ms while a route runs, one at a time
   (`ui/src/useDiagnosticsRefresh.ts:9,64`);
4. native counters pump every 1 s while a route runs, one at a time
   (`ui/src/useNativeCounters.ts:18,125`);
5. the 2 s activity refresh, which sends `listClients` and others together
   without an overlap guard (`ui/src/useWorkbenchStatus.ts:91`);
6. OBS status polls when OBS is connected (`ui/src/useObsStatus.ts:6`).

A Stream Deck plugin polling over HTTP, or an MCP/CLI call during the export,
adds more. So four instances are reachable with the window and a playing route
alone, and the fifth caller fails after 2 s with the error above. It happens
only while the control thread is held for more than 2 s: a maximum-size bundle
export or import (2.5 s here, longer on a slower disk). It is not a
steady-state problem. I did not run the shell (no launching it on this
machine), so the window-side count above is from code, not from a live
observation.

User-visible effect: a window request that failed this way fails with "control
pipe request failed: timed out waiting for a free named-pipe instance"
(`main.rs:1438`); a failed event poll marks the snapshot stale with an error
(`useWorkspaceEvents.ts` catch block). The same requests would have been served
a moment later. HTTP and MCP callers get the error as a failed call.

### 3. Decision and fix

Real, user-reachable, small. Fixed in the owning layer, the pipe client:

- `round_trip_with_timeout` (the primitive behind the window, HTTP adapter,
  MCP, CLI and tray, via `round_trip`'s 60 s default) now waits for a free
  instance within **its own timeout** instead of a fixed 2 s
  (`connect_client_waiting`, `crates/transport/src/lib.rs:1551`, used at
  `:1622`). The deadline is taken before connecting and shared with the
  response read, so the whole exchange still never exceeds `timeout` (this
  also keeps the function's "bounded by `timeout`" contract true; an earlier
  draft started the response clock after connecting, which allowed up to twice
  the timeout). Callers with a short timeout keep a short wait: the shell's `status.get` and
  `events.subscribe` (2 s) fail as before, so the tray and the event poll
  still surface "backend busy" promptly rather than hanging.
- The other client entry points (`round_trip_many`, `round_trip_session*`,
  `send_oneway`) keep the 2 s `PIPE_BUSY_WAIT`; they have no timeout to scale
  by, and `send_oneway` is the wake-up that `stop_control_io` repeats.
- Not changed: `SERVER_PIPE_INSTANCES` stays 4. A higher count would raise
  memory (each instance has two `MAX_FRAME_BYTES + 4` buffers) and the number
  of same-user clients the I/O layer holds, and would only move the cliff
  (the single control thread still serves one request at a time). The 5 s
  stalled-client read deadline (`PIPE_REQUEST_DEADLINE`) is what bounds an
  instance held by a stuck client, and it is untouched, so a waiting client
  cannot be starved past that by a stalled peer.
- Trade-off: a window request the page already gave up on (5 s UI deadline)
  can now wait in a Tauri thread for an instance for part of its 60 s pipe
  budget, instead of 2 s, and run late. Read requests are harmless; mutations carry idempotency keys
  (API-07). It is bounded by the pipe timeout and happens only while the
  control thread is held.
- Not helped: `events.subscribe` and `status.get`, whose shell timeout is 2 s.
  During a bundle export the event poll still reports a stale snapshot; that
  is the existing UI-13 behaviour, not this defect.

Tests (`crates/transport/tests/busy_pipe_wait.rs`, not ignored, unique pipe
names, no database or disk; the servers hold their four instances behind a gate
instead of running a real export):

- `a_client_waits_for_a_free_instance_for_as_long_as_its_timeout`: saturates
  all four instances, starts a 30 s client, requires it to still be waiting
  after 2.6 s, then releases the gate and requires it to be served after at
  least 2.5 s.
- `a_short_timeout_bounds_the_wait_for_a_free_instance`: a 300 ms client fails
  with the busy-pipe error within 1.5 s.

Both **fail without the fix** (reverted `connect_client_waiting` call
temporarily; first: "the client gave up while its 30 s timeout had not run
out", second: "a 300 ms timeout waited 2.0034439s for an instance") and pass
with it. The first takes about 2.6 s, so the pair runs in 2.6 s. A first
draft of the first test shut the servers down while the waiting client could
still need them (a race on a slow runner); it now opens the gate, joins the
client and only then shuts down. The final form was run 10 times in a row
(10 of 10 passed, 2.61 s each) and re-checked against the reverted fix (both
failed again, the short-timeout one after 2.014 s).

The shell's own pipe tests (`src-tauri/src/main.rs:3875-3935`, run through
`forward_rpc_request`) retry on a missing pipe or succeed; none saturates the
four instances, so none would now wait for a busy one.

End to end, the same harness scenario with the fix
([pipe-busy-after.csv](2026-11-24-pipe-busy-after.csv)): "export-noise with 6
pollers" 0 failed polls (before: 3), 6 polls sent during the export and all
served, longest 3,115 ms (the final code; an earlier run of the first draft
gave 0 failures and 2,686 ms; the noise export itself took 2.5 s then and
3.2 s now, so read these as "no failure", not as a timing).

### 4. Commands and results

All on this PC, Windows 11, Rust 1.96.0, `CARGO_BUILD_JOBS=6`, in the branch's
own worktree; no shell launched, default control pipe and the owner's database
untouched.

| Command | Result |
| --- | --- |
| harness before fix, `AUDIOROUTER_TIMING_RUNS=1`, release, ignored test | 3 failed polls, error text above |
| `cargo test -p audiorouter-transport --locked --test busy_pipe_wait` with the fix reverted | 2 failed (messages above) |
| same, fix restored | 2 passed, 2.61 s |
| harness after fix, same command as the first row | 0 failed polls; `test result: ok`, 44.8 s |
| `cargo fmt --all -- --check`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test -p audiorouter-transport -p audiorouter-cli -p audiorouter-control --locked` | all passed (transport lib 41, `busy_pipe_wait` 2, cli lib 58, `mcp_stdio` 5, `security_boundaries` 1, control lib 454; ignored tests not run) |
| `node tools/quality/verify.mjs --checks docs` | passed: 296 Markdown files, 1652 local links |

Not run: `src-tauri` Clippy and tests (no `src-tauri` file changed; its tests
need `ui/dist` and open pipes in the shell binary), the other `service_timing`
ignored harnesses, the three-run harness (one run only), `fresh_install_shell`
(no installer built), any run against the running app, and a live check with a
route playing. The window-side instance count is from code reading.
