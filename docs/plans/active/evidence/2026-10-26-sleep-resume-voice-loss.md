# Sleep/resume voice loss — 2026-10-26

## Report and evidence

The owner left AudioRouter running through an approximately 18-hour sleep.
After wake, voice capture was silent. The top-bar Reconnect did not restore
it; Stop followed by Play did. The running log identifies a development
build reporting version 0.0.21, process 60800, with session
`patrick-main-siege-footstep` selected. The exact source commit of that
running binary was not established.

Windows Power-Troubleshooter event 1 reports sleep at 2026-10-26 05:07:41
UTC and wake at 23:10:07 UTC. The backend audio-health records immediately
before sleep and at 16:10:05 local on wake show the same multi-input worker
still active, including the `mic`, `siege-in`, and application inputs; the
reported audio-thread gap was approximately 64,928 seconds. The wake record
did not report a pump error. The backend logs show no `system.osTransition`
call after wake before the owner's Stop/Play. At 16:15:48 `session.stop`
succeeded and at 16:15:49 `session.start` succeeded with generation 2;
the owner confirmed voice returned. The subsequent health record reports
12 pump errors and a last binding-invalidated message, but its timing does
not establish that message as the original cause of silence.

The earlier chat diagnosis that the sleep transition stopped the route was
incorrect: the health record shows the old worker survived the transition.
The log format does not record the transition type for the 22:07:42 RPC;
it cannot independently prove whether that call was lock or sleep.
Reconnect in `ui/src/App.tsx` refreshes the UI/backend connection and
inventory; it does not reconstruct native audio clients.

## Source finding

`src-tauri/src/os_transition_windows.rs` creates its listener with
`HWND_MESSAGE` and expects `WM_POWERBROADCAST`. Microsoft documents that
[message-only windows do not receive broadcast messages](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#message-only-windows),
while [power transitions are broadcast](https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast-messages).
This is a concrete delivery defect consistent with the observed missing
resume RPC; the running binary's exact source has not been verified.
`handle_os_transition` in `crates/control/src/safety.rs` additionally stops
sessions on sleep and reports native sessions on resume without rebuilding
them. The shell discards the response. Therefore notification delivery alone
would leave native routes stopped and would not provide the requested repair.

## Proposed repair and acceptance

- Register explicitly for suspend/resume notification with the Windows
  notification API, or use a hidden top-level broadcast recipient. Confirm
  actual delivery on Windows, handle registration failure visibly, unregister
  on shutdown, and keep notification callbacks bounded and nonblocking.
  Explicit registration is documented by Microsoft for
  [suspend/resume notifications](https://learn.microsoft.com/en-us/windows/win32/api/powerbase/nf-powerbase-powerregistersuspendresumenotification).
  Do not infer successful delivery from a unit test that sends a synthetic
  window message. Account for duplicate resume events and queue overflow.
- Move orchestration into the backend: preserve the identity and active graph
  of previously running non-recording routes, release stale native streams,
  re-enumerate and validate the exact saved device and process bindings, then
  prepare and start the same route using the owning backend lifecycle. Retry
  boundedly as endpoints return. Reuse the preparation appropriate to the
  actual route, including multi-input paths; `session.start` alone is not
  sufficient. Handle wake without a prior suspend notification too.
- Preserve privacy mute and protected processor failure silence. Never
  substitute a microphone, restart a recording into its old file, silently
  restart a route the user stopped, or use a saved graph in place of a live
  preview with different content. If a route cannot be safely restored,
  leave it stopped or silent and expose the affected route and reason.
  User Stop, newer edits and subsequent transitions must cancel stale retries.
- Keep lock behavior separate from sleep. Sleep handling with active
  recorders currently returns an error before stopping any route; define and
  verify safe recorder finalization and recovery without suppressing recovery
  for unrelated routes.
- Verification needs deterministic lifecycle coverage for exact bindings,
  delayed and missing endpoints, repeated events, cancellation, privacy mute,
  processor failure, active recording and wake without suspend. On Windows,
  qualify actual OS notification delivery and real microphone-to-output
  audio across sleep/resume with the same multi-input route. Verify sound and
  running status without clicking Reconnect or Stop/Play. Hardware and the
  100-cycle NFR-11 endurance gate are not established by mocks.

## State and rollback

The candidate implementation uses a hidden top-level listener for power
broadcasts and schedules recovery of a previously running saved multi-path
route on the backend service thread. It reopens exact saved bindings, retries
up to 20 times, preserves privacy mute, and cancels on Stop, Play or a saved
revision change. Other native worker types and sessions recording at sleep
are not automatically recovered by this change. Delivery after a missed
suspend notification, exhausted-retry presentation, and real Windows audio
recovery remain unqualified. Rollback is to revert this branch and retain
the existing manual Stop/Play recovery. The separate CodeQL remediation
remains active in its original worktree.

## Local build — 2026-10-26

Windows, branch `fix/sleep-resume` based on freshly fetched `origin/main`
`7b1364dc`: `npm.cmd ci --prefix ui --no-audit --no-fund` and
`npm.cmd run build --prefix ui` succeeded. Vite reported its large-chunk
warning for the 608 kB application bundle. The optimized locked Rust shell,
CLI and VST2 worker builds succeeded with `--target-dir
target/sleep-resume-build`; the optimized VST3 worker was built from this
worktree's source against the existing SDK checkout. Both Rust formatting
checks and `git diff --check` passed. No automated tests, actual sleep cycle,
microphone audio check or endurance qualification was run.

The first executable was
`target/sleep-resume-build/release/audiorouter-shell.exe` (SHA-256
`B83A1EFA532EAC6D5E2864F9613F534A58F7E7831B68ADDA302DA6562537B428`),
with the matching CLI and plugin workers beside it. The owner will quit the
running app, start this executable, run their saved route, sleep and wake the
computer, and check whether voice returns without Reconnect or Stop/Play.
The build did not install or launch the candidate. The owner reported that it
showed “Hmmm… can't reach this page”: the shell build omitted
`--features custom-protocol` and therefore targeted the development server.
This initial executable must not be used for the sleep qualification. The
quickstart's shell build command also omitted the feature; correct it and
rebuild an isolated release shell with the embedded UI, retaining this record
of the failed handoff.

The corrected build used `cargo build --locked --release --features
custom-protocol --manifest-path src-tauri/Cargo.toml --target-dir
target/sleep-resume-embedded`. It completed on Windows. The resulting shell
contains the current `ui/dist` asset name `index-uuCbspt7.js` and has SHA-256
`57EF9DBC9DAE6390DE94675BE7ED804B64D499341744784EC8A52B041F9E11CF`.
The CLI and both plugin workers were copied beside it and their hashes match
the previous build's companions. The shell has not been launched or visually
verified: the owner may have an AudioRouter instance running, and the packaged
smoke requires the production control pipe to be free. This confirms the
asset was embedded, not that WebView2 rendered it or sleep recovery works.

## PR preparation checks — 2026-10-26

On Windows, `cargo test -p audiorouter-control --lib` passed (299 passed,
11 ignored); it does not qualify the new sleep recovery scenarios. Workspace
and shell Clippy with all targets/features and `-D warnings`, both Rust format
checks, `git diff --check`, and `tests/acceptance/docs.ps1` passed. Actual OS
notification delivery, real sleep/wake audio, and the lifecycle scenarios
above remain unverified. Next: qualify these before marking the change ready.
