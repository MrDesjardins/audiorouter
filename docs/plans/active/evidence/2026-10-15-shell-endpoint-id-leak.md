# Shell memory leak while playing — endpoint ID strings (2026-10-15)

## Summary

The desktop shell's private bytes grew about 9.3 MiB/h while a session
played, linearly for 8 hours (~220 MiB/day for an always-on tray app). Cause:
every `IMMDevice::GetId` call in `crates/windows-audio/src/lib.rs` copied the
returned CoTaskMem string with `to_string()` and never freed it, leaking about
150 bytes per endpoint on every endpoint walk. Fix: copy, then
`CoTaskMemFree`, through one helper (`take_endpoint_id`) at all six call
sites. With the fix, the same session played for 40 minutes with flat memory,
and the owner's 8-hour overnight soak passed (0.11 MiB/h, 2026-10-16).

- Requirements: NFR-06 (long-run stability), ARCH-04.
- Affected versions: every release since the read-only Windows audio adapter
  (`64dd36a7`, 2026-09-05; v0.0.1 through v0.0.16).
- Found by: the owner's overnight app soak (stability round, finding 7 in the
  [active plan](../current.md)).

## Detection: owner overnight soaks

`tools/quality/run-app-soak.ps1 -Hours 8 -AutoPlay`, release shell built from
`87545f1b`, a copy of the owner's database (active session "Patrick Main
Session (Siege Footstep EQ)", 19 nodes: microphone voice chain, Siege chain,
Discord application capture, two mixers, duck, recorder; 62 Windows audio
endpoints, 15 active). Two consecutive runs on 2026-10-15:

| Check | Run 1 | Run 2 |
| --- | --- | --- |
| Shell private bytes slope after warm-up | 9.26 MiB/h (REVIEW) | 9.31 MiB/h (REVIEW) |
| WebView private bytes slope | +0.17 MiB/h | −0.27 MiB/h |
| Shell handles / thread spread | 509, 2 | 509, 3 |
| Log history at measured rate | > 30,000 h | > 30,000 h |
| Shell CPU over 8 h | 2,242 s | 2,263 s |

The growth was smooth (0.1 MiB steps every 30 s sample) and already
10.2 MiB/h between minutes 10 and 40. The working set stayed flat (~45 MiB)
while private bytes rose: memory written once and never read again.

## Attribution

All probes ran a separately launched shell on database copies with their own
`LOCALAPPDATA` and WebView2 profile, never beside the owner's AudioRouter.

| Probe | Result | Conclusion |
| --- | --- | --- |
| Shell idle (nothing playing), 15 min | One 8 MiB step at ~5 min, then flat | Needs a playing session |
| Idle, window path flooded through DevTools with `rpc_request("system.diagnostics")`: 777,000 calls (~10 h at 20 Hz) | +1.4 MiB, then flat | Window → Tauri → pipe → backend request path does not leak |
| Voice chain only / Siege chain only, 25 min each | Both drift 4–8 MiB/h with 1 MiB heap steps | Inconclusive: every 25-minute run drifts during warm-up, idle included |
| dhat build (`dhat::Alloc` global allocator), full session, profiled minutes 10–40 | Live Rust heap ~6 KB, flat; process private bytes +8 MiB/h at the same time | Leak is native, not a Rust allocation |
| `cdb -pv` snapshots of the process heap (`!heap -s -h <heap>`), minutes 10 and 35, full session | Only the 128-byte LFH bucket grew: 15,112 → 43,039 blocks (18.7/s ≈ 8.2 MiB/h); all other sizes ±0 | One allocation type leaks |
| `s -u` over the heap, then `du` of the growing blocks | 42,328 strings `{0.0.0.00000000}.{…}` / `{0.0.1.00000000}.{…}`; 361 decoded blocks name active endpoints ~9–10× each and inactive ones ~5× each | Leaked blocks are endpoint ID strings (56 UTF-16 characters, 112 bytes plus header) |

The dhat profile also showed about 82,000 short-lived allocations per second
while playing, all freed: the 20 Hz diagnostics refresh (control-plane JSON
building, the shell decoding the response, and the backend logger decoding
every response again even for routine reads it does not log). That churn is
not the leak; it is recorded as a follow-up.

## Cause

`IMMDevice::GetId` returns a CoTaskMem-allocated `PWSTR`. In `windows` 0.62
that is a bare pointer with no destructor; the caller must free it with
`CoTaskMemFree`. Six call sites copied it with `to_string()` and dropped the
pointer, and two of them carried a comment claiming "the windows crate wrapper
copies and frees". The other CoTaskMem sites in the file (connector device
IDs, `GetMixFormat`, session display names, `PROPVARIANT`s) were already freed.

Leak per call, measured on the owner's PC with a scratch program calling each
public walk 2,000 times (process private bytes delta):

| Function | Bytes left per call |
| --- | --- |
| `enumerate_active_endpoints` (15 active) | 2,263 |
| `enumerate_active_endpoint_display_info` | 2,064 |
| `enumerate_default_endpoint_bindings` (6 roles) | 809 |
| `enumerate_endpoint_states` (all 62) | 8,251 |
| `enumerate_application_audio` | 25 (noise) |
| `enumerate_applications` | 0 |

`SharedCapture`/`SharedRender::open_internal` leaked the same way on every
stream open (one ID per active endpoint of that direction).

Not proven: which caller repeats about once a second while playing (~2.3 KiB/s
matches one active walk per second). The likely candidate is the control
plane's `EndpointMonitor`, created at Play: it re-runs
`enumerate_active_endpoints` whenever its notification client is dirty, and
`OnPropertyValueChanged` for any property of any endpoint sets it dirty. Idle
has no monitor, which fits the flat idle result. The fix covers every caller.
Later measured: the caller was the cable status, not the monitor (see
"Repeating caller found"). The UI polls the cable status idle too, so the
15-minute idle run (one 8 MiB step, then flat) was likely too short to show
the ~2.6 KB/s growth through heap commit steps, and playing was probably
not required.

## Fix

- `take_endpoint_id(id: PWSTR) -> Result<String, AudioError>`: copies the
  string, frees it once with `CoTaskMemFree`, returns the copy (documented
  safety contract).
- Used at all six `GetId` sites: `SharedCapture::open_internal`,
  `SharedRender::open_internal`, and the active, defaults, display-info and
  all-state inventory walks. Error mapping is unchanged.
- No behaviour, API or data change; no migration.

## Verification

- Regression `windows-audio` `tests::endpoint_walks_free_every_endpoint_id`
  (live; `#[ignore]` on CI, whose runners have no audio endpoints): runs all
  four inventory walks 500 times and requires the process heap's allocated
  bytes (`HeapSummary`) to grow by less than 16 bytes per endpoint per walk.
  Before the fix (same test on `87545f1b`): FAILED, 5,599,816 bytes left
  after 500 walks of 62 endpoints. After: passed.
  `cargo test -p audiorouter-windows-audio -- --ignored --exact tests::endpoint_walks_free_every_endpoint_id`
- `cargo test -p audiorouter-windows-audio`: 141 passed, 4 ignored.
  `cargo clippy -p audiorouter-windows-audio --all-targets --all-features -- -D warnings`
  clean; `cargo fmt --all -- --check` and the `src-tauri` check clean.
- Real app: release shell from `87545f1b` plus only this fix, owner's full
  session playing for 40 minutes (database copy, `-AutoPlay`):

  | | Before (same commit, unfixed) | After |
  | --- | --- | --- |
  | 128-byte blocks, minute 10 → 35 | 15,112 → 43,039 | 141 → 141 |
  | Private bytes after warm-up | 8–10 MiB/h | 32.10 → 32.46 MiB over 30 min |

- Owner overnight soak on the fixed build, 2026-10-16:
  `run-app-soak.ps1 -Hours 8 -AutoPlay -Shell C:\ard\src-tauri\target\release\audiorouter-shell.exe`
  (`87545f1b` plus only this fix, SHA-256 `4B9C16DF14707527…`), copy of the
  owner's database, 951 samples every 30 s, session playing. Every check
  PASS:

  | Check | Before (run 2) | After |
  | --- | --- | --- |
  | Shell private bytes slope after warm-up | 9.31 MiB/h, 104.8 MiB at end (REVIEW) | 0.11 MiB/h, 30.9 MiB at end (PASS) |
  | WebView private bytes slope (6 processes) | −0.27 MiB/h | −0.22 MiB/h, 553.8 MiB |
  | Shell handles / thread spread | 509, 3 | 509, 2 |
  | Log history at measured rate | > 30,000 h | 36,359 h backend, 30,469 h shell |
  | Error lines in logs | 0 / 0 | 0 / 0 |
  | Shell / WebView CPU over 8 h | 2,263 s / — | 2,257 s / 1,290 s |

  Real AudioRouter and WebView folders were unchanged.

## Reproduce or re-check

1. Close AudioRouter. Soak a build:
   `powershell -ExecutionPolicy Bypass -File .\tools\quality\run-app-soak.ps1 -Hours 8 -AutoPlay [-Shell <exe>]`.
2. To attribute growth, compare Rust heap and native heap: a dhat build
   (temporary `dhat` optional dependency and `#[global_allocator]`) shows
   whether live Rust bytes grow; if not, take two process-heap snapshots with
   the Windows SDK debugger, minutes apart, while the app runs:
   `cdb -pv -p <pid> -c "? poi(@$peb+0x30); qd"` gives the process heap, then
   `cdb -pv -p <pid> -c "!heap -s -h <heap>; qd"`. Diff the "Buckets info"
   block counts, then search and dump the growing size
   (`s -u <heap> L?<size> "<text>"`, `du <address>`).
3. Keep every `cdb -pv` attach to a few quick commands. It suspends the
   target while attached. One long dump session froze the probe shell, and
   killing cdb left its threads suspended (the shell had to be closed).

## Repeating caller found (2026-10-16)

Measured on the owner's PC, release shells playing the owner's session
(`run-app-soak.ps1 -AutoPlay`, database copy, isolated profile):

- An endpoint notification probe (`IMMNotificationClient` registered
  beside the app) counted **0** notifications in 30 s idle, 0 during an
  endpoint walk, and 0 in 4 minutes with 0.0.17 playing. The endpoint
  monitor does not re-walk while playing; filtering property keys would
  change nothing.
- A temporary build (not committed) counting each public walk every 30 s,
  same session playing for 3 minutes: the active-format walk ran 4 times
  (startup only), while the display-info walk and the all-state walk each
  ran about 7.5 times per 30 s, one every 4 s.
- Caller: `virtualCable.status`. The UI polls it every 2 s and the
  backend cached it for 2 s, so a fresh check ran about every 4 s, and
  each runs the driver helper's `cable_endpoints` (display info plus all
  62 endpoint states). Before the ID fix that leaked 2,064 + 8,251 bytes per
  check, 2.6 KB/s or about 9 MiB/h: the measured leak rate, and the 2:1
  ratio of active to inactive IDs in the heap dump.

Change: the status cache now lasts until Windows reports an endpoint change
(a lazily started read-only `EndpointNotificationSubscription` in the
control plane), with a 30 s backstop for driver state an endpoint change
does not announce. Without a subscription it keeps the 2 s cache. Same
instrumented build with the change, 3 minutes playing: display and
state walks once per 30 s (from 7.5). Regression
`virtual_cable::tests::status_cache_lasts_until_an_endpoint_change_or_the_backstop`.
Control tests 250 passed, 9 ignored; workspace and shell Clippy and
`cargo fmt --check` clean.

Side note: `run-app-soak.ps1 -AutoPlay` kept waiting for "running" on
these runs although `session.start` returned `running` in the backend
log; the overnight run detected it. Not investigated.

## Rollback and risks

Rollback: revert the `windows-audio` change; the leak returns, nothing else
changes. Risk: low. Each freed pointer comes from a successful `GetId` and is
freed once after its only read.

## Follow-ups

1. ~~Measure how often the endpoint monitor's notification fires while
   playing.~~ Done 2026-10-16; the repeating caller was the cable status,
   not the monitor (see "Repeating caller found").
2. Reduce the 20 Hz refresh's allocation churn; in particular, the backend
   logger should not decode responses of routine reads it does not log.
3. ~~Confirm with the owner's next overnight soak.~~ Done 2026-10-16 (see
   Verification).
