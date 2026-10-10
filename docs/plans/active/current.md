# Active plan — post-0.0.8 maintenance

Driver-agent resumption: read the [Claude / next-agent handoff](virtual-cable-agent-handoff.md)
first. It records the prepared candidate, exact next guest step, completed
checks, open defects and host/VM safety boundaries as of 2026-10-10.

Updated 2026-10-07. Completed work through the code review P0/P1 fixes is in the
[2026-10-04 to 2026-10-07 execution record](../archived/2026-10-07-maintenance-0.0.12-to-code-review.md);
the current work is the code review P2/P3 follow-ups below.

## Objective and scope

Support users of the published releases and close the remaining qualification
gates. Preserve sessions, local API credentials, device formats and published
assets. Requirements DSP-19, GRAPH-08/14/15, UI-05/08/11/12/13/17, AUTO-15,
ARCH-04, SEC-01/10 and DIST-01–08 stay traceable through the release evidence.

## Where things stand

- [v0.0.7](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.7)
  (tag `ac29df97`): Rust workspace 979/0, shell 44, UI 458, full development
  Edge 202/0, exact-package fresh install and published-asset verification
  pass. Unsigned.
- Completed work of 2026-10-03 (0.0.6/0.0.7) is recorded in the
  [execution record](../archived/2026-10-03-releases-0.0.6-0.0.7.md). Earlier
  history is indexed in [archived plans](../archived/README.md).
- Other active plan: [app/installer signing](release-signing-and-publication.md),
  blocked on the user's provider choice and enrollment.

## Decision (2026-10-05, user): GPL-3.0-only with commercial licensing (DEC-19)

The user asked to stop others from profiting from AudioRouter while staying
open source and keeping the option to earn money later. CC BY-NC was
considered and rejected: it is not an open-source license and Creative Commons
advises against it for software. Chosen: `GPL-3.0-only` for versions after
0.0.13, with commercial licenses offered by the sole copyright holder
(Patrick Desjardins, author of every commit). Releases up to 0.0.13 stay MIT.

- Changed: `LICENSE` (standard GPL-3.0 text from GitHub's license API, since
  gnu.org was unreachable), workspace and shell `Cargo.toml` license fields,
  README License section, new `CONTRIBUTING.md` (contributors grant the
  copyright holder a right to relicense, which dual licensing needs), and
  website wording ("GPL-3.0 open source", footer).
- Compatibility checked: shipped Rust dependencies are MIT/Apache-2.0/BSD/
  ISC/Zlib/Unicode/MPL-2.0 or LGPL-3.0 (LAME), all GPL-3.0 compatible; VST2
  uses AudioRouter's own ABI declarations, not Steinberg's VST2 SDK; the VST3
  SDK is MIT. The MS-PL driver stays a separate program under its own license.
- Not legal advice; a lawyer should review the commercial-license terms
  before any are sold. Rollback: restore the MIT `LICENSE` and fields.
- Next: the next release ships under GPL-3.0; no code change is needed.

## Decision (2026-10-05, user): ship AudioRouter's own virtual cable (DEC-18)

Current driver task (2026-10-09): qualify sustained tone continuity after the
render-publication repair. Latest packet-clock r2: preparation and 30 seconds
pass, but the five-minute attempt failed and exposed a 38.5-minute synchronous
reporting block that extended active recording past its deadline. The harness
now uses bounded deferred reporting and a process watchdog; host regressions
and the checksummed bounded-tone update are complete. Its 30-second guest
check passes: leases stop at 30,001 ms, child exit 0 without timeout, zero
error counters. Its five-minute run ends on time but loses audio: both audio
loops stall about 310 ms and the control loop about 317 ms in the same
observation interval. Root cause remains unidentified; next is an opt-in guest
scheduler trace, preserving WSL and the failed continuity gate. Its initial
GeneralProfile startup failed with 0x80070032 before tone began. The minimal
profile's two-second guest probe now starts/saves successfully; the copied
trace decodes with scheduling/DPC/ISR event families and zero reported lost
events. The 300-second traced run now fails with 7,056 capture underrun and
14,832 render overrun frames. Saved trace analysis finds worker waits up to
35.7 ms, predominantly before readiness; the longest wake runs at priority 24
within 30.2/161 us for capture/render. An offline reader interprets the common
version-5 context-switch fields and matches independent event counts. Guest
timer versus host/VirtualBox attribution remains open. Next: prepare paired
host/guest timing diagnostics before another audio run; do not repeat the
unchanged guest-only test or alter buffers/counters/WSL based on correlation.
Paired recorder/clock probe scripts are prepared and local checks pass. Real
host recording remains unavailable to the agent's non-administrator Windows
token. The user has now run that host probe successfully: saved event coverage and
zero-lost counts pass review, and WPR is idle afterward. The paired two-second
probe also passes trace coverage/loss review. Guest UTC drifts relative to QPC;
normalized QPC brackets remain compatible within 22.45631 ms during this short
probe. Attribution must use raw QPC with that uncertainty, not UTC alignment.
The separate copy-only paired-tone update is published and independently
verified (35 files, all 32 base files unchanged). The paired 30-second run now
passes counters/deadline, both trace loss checks and compatible QPC brackets
(23.0877 ms). The user reports continuous hiss mixed with the beep through
Cable B, ending with the script. The saved Cable A WAV does not verify that
path. Hold longer runs; the [automatic direct audio diagnostic](../../operations/virtual-cable-direct-audio.md)
generates Cable A audio and records Cable B without default/microphone fallback
or Audacity installation. The first guest attempt passed status but the new
recorder exited before readiness with empty output. PE inspection confirms a
dynamic Visual C++ runtime dependency missed by the original packaging.
Repair uses static linking, a PE import gate and saved numeric/hex startup
exits; no redistributable or driver installation. The r2 recorder now succeeds,
but the run fails with a 288-ms gap shared by all three audio workers and
12,912 lost frames in each bridge direction. Both active WAVs span 29.73 s.
Direct Cable B samples are clean between the single loss event; the user's
continuous hiss points to further Listen/speaker playback investigation.
Cable A also has separate phase breaks at 12.037–12.050 s. The offline
analyzer now retains waveform metrics when duration fails, with unchanged
pass criteria; both saved WAVs still fail. Hold further guest runs while
reviewing these separate failures. See the
[direct waveform evidence](evidence/2026-10-09-m03-direct-audio-preparation.md#direct-r2-recordings-and-reporting-repair-2026-10-09).
Sustained continuity, audible quality and exact cause remain unresolved.
User also hears hiss on saved-file replay through Speakers; that device only
offers PCM16 at 16/22.05/44.1 kHz. Keep its supported 44.1-kHz setting and
Cable B at 48 kHz. The clean recorded section quantizes identically to a
generated reference. New offline PCM16 reference files are published.
The user now reports static in all three sections of the independently
generated 17-second **44.1-kHz** speaker reference. Offline reinspection
confirms its header, clean sine samples and silent separators. Read-only VM
configuration/log review shows HDA, HostAudioWas and default Focusrite speaker
output; it does not identify a defective component. With Cable B Listen off,
the user now hears much less static but several distinct crackles remain in
the early/final portions, and the middle is not perfectly clean either. Keep
Listen off. The bounded guest speaker-loopback recording is now reviewed:
after a 64-frame alignment, the first 749,260 stereo frames (16.990023 s)
match the reference exactly, including both reported noisy sections and
silent separators. Audible crackles in those sections arise after this
capture boundary; the exact downstream component is unproved. The final
440 reference frames differ and further tone sections continue; packet
position gaps/flags are retained, not silently waived. No whole-recording
or driver pass is claimed. Next is read-only VirtualBox playback/backend
review before preparing a reversible comparison, as documented in the
[direct audio runbook](../../operations/virtual-cable-direct-audio.md#completed-supported-format-speaker-reference).
Installed-version source review now confirms a VM-specific DirectSound
comparison requires both backend selection and an override to avoid silent
WAS substitution. The powered-off guard, saved baseline, startup-log check
and exact rollback are prepared in the runbook. User applied the reviewed
host block and booted normally. Fresh startup log confirms DSoundAudio.
The guest recorder now stops at 25.111494 s after receiving 34.996825 nominal
seconds of samples and reaching its storage limit. Its first 16.990023 s of
samples remain exactly equal to the reference. User reports quiet separators,
scratching during capture and clean sound after capture ends. The subsequent
capture-off playback also scratches variably, sometimes six to eight times,
with more scratching as playback continues. This supersedes the provisional
capture-only explanation; DirectSound is not a demonstrated improvement.
The current boot log also reports a 248.755-second virtual-clock catch-up
failure and a 249-second guest heartbeat gap. Installed-version source shows
HDA uses the synchronous virtual clock, whose recovery can accelerate DMA.
This is a concrete timing hypothesis, not aligned proof of the audible cause.
Host power-event inspection was denied; no sleep or host cause is established.
User has now shut the VM down and completed the guarded rollback. XML confirms
default/WAS, retained HDA and no backend override; VM processes are absent.
The closed-session log records 11 HDA output transfers skipped while a
completion interrupt remained pending; source confirms this pauses DMA while
the host consumer continues. Whole-session totals do not identify individual
scratches. Later overnight HostSuspend/HostResume and host endpoint changes
are also logged and must not be attributed to the earlier listening run.
Next engineering step: source review confirmed that event render DMA can replay
uncommitted circular-buffer bytes without incrementing bridge counters. The
owning reader now tracks accepted absolute packet identities, silences/counts
missing frames and commits atomically with position progress. Fresh kernel
review found no remaining source blocker in this scoped repair; 620 offline
helper checks and x64/ARM64 WDK/catalog/source acceptance pass. Legacy clients before/without packet commits and
one-slot continuity remain unqualified. This finding does not prove the saved
phase-break cause or explain the independent HDA scratches. Keep the VM off
and further recorder/bridge runs held. The clean-source `28b989f5` candidate
is ready at `C:\VMs\ar-share\repair-20261010-render-commits`: 33 package checks
and all 32 manifest hashes pass. Both clean guest smokes passed
2026-10-10; next is guest install plus status/format inventory, as recorded
below; no audio run yet.
No installed driver or host setting is changed by the agent. See the
[repair and validation record](evidence/2026-10-10-m03-render-commit-validity.md) and the
[clock review](evidence/2026-10-09-m03-direct-audio-preparation.md#directsound-playback-with-capture-off-and-clock-review-2026-10-10).
Preserve WSL and the separate measured scheduling/loss failures.
Commands are in
[Administrator PowerShell](../../operations/virtual-cable-paired-trace.md).
See the
[packet-clock review](evidence/2026-10-09-m03-packet-clock-review.md) and active
virtual cable plan. The [earlier review record](evidence/2026-10-09-m03-render-publication-review.md)
and [virtual cable execution plan](virtual-cable.md) record the guest's passing
smoke cycles, four-endpoint format inventory, and 30-second tone run. The
first 10-minute attempt failed with earlier audio counter errors and a
14m55s VirtualBox guest-execution stall. VirtualBox reports NEM “Snail” mode
under the active Hyper-V hypervisor. The user requires WSL to remain
available; do not disable Hyper-V-backed features. The guide's 4-vCPU/8-GB
settings are already in use. A supervised 300-second run with the VM kept
awake and foreground completed without a long pause, but recorded four bursts
of audio errors (16,704 capture underrun frames / 348 ms; 19,152 render
overrun frames / 399 ms; maximum pump gaps about 163 ms). The user confirms
the VM did not sleep and no screen saver ran. This rules out those proposed
causes, but does not identify why the VM was intermittently descheduled. Keep
the continuity gate failed; review the run evidence and scheduling diagnostics
before any further long run. Source review confirms the counters track actual
loss: the recorded WAV is short by 40 ten-millisecond blocks, closely matching
the render-overrun total. The driver's bounded queues provide only tens of
milliseconds of headroom; absorbing a 163 ms pause would require comparable
buffer headroom and transient latency, conflicting with the virtual-cable
latency target. The deeper review subsequently found packet-count drift,
unbounded capture catch-up, a one-packet timestamp error and fractional carry
retained across STOP, plus long-uptime timestamp conversion overflow.
These are repaired in the
[packet-clock review](evidence/2026-10-09-m03-packet-clock-review.md).
Host regressions/builds pass; fresh VM continuity is still pending. Next:
use the [bounded retest procedure](../../operations/virtual-cable-packet-clock-retest.md).
No test driver has been installed on the host; WSL and Hyper-V remain enabled.

AudioRouter will ship its own signed virtual cable so users do not need
VB-Cable; VB-Cable/Voicemeeter stay supported. Lowest-cost signing
(attestation, EV certificate bought only for driver-release windows).
Recorded in [15-delivery DEC-18](../../spec/15-delivery.md); full plan,
costs and phases in the [driver track](../future/M03-driver-signing.md).

- Requirements: VDEV-01–12, SEC-08, NFR-16, DIST-01/03/07 (amended).
- Prerequisites for phase 1: a Hyper-V VM (Windows 11, Gen2) or a spare PC,
  created by the user; test signing only inside it. Nothing is installed or
  changed on the daily workstation.
- **Execution plan with work packages WP-00…WP-15:**
  [virtual cable plan](virtual-cable.md). Design: [17 Virtual
  cable](../../spec/17-virtual-cable.md). Signing runbook:
  [virtual cable signing](../../operations/virtual-cable-signing.md).
  WP-01 host baseline passed; WP-02 package tooling complete (2026-10-05),
  with [build/signing evidence](evidence/2026-10-05-virtual-cable-wp02.md).
  No driver has been loaded; VM evidence is pending.
- Test procedure (stages A–E, VM to main PC): [virtual cable
  testing](../../operations/virtual-cable-testing.md).
- WP-03 smoke tooling is prepared and WP-04 host bridge hardening is reviewed;
  their VM gates (install/remove, Verifier/fuzz, second-user denial) remain
  pending. WP-05 host endpoint, precision and format work is in progress; see
  its execution record and evidence in the [virtual cable plan](virtual-cable.md).
  Current WP-05 cable descriptors compile for x64/ARM64 and pass portable
  INF/conversion checks, but remain non-loadable until timing/cleanup work,
  WP-06 bridge/Rust compatibility and the VM gates are complete. Host builds
  are not runtime or sound-quality evidence.
- Needs a separate user go-ahead: buying the EV certificate (phase 5) and
  the first public driver release.
- Cables: up to 8 (A–H), user enables 1–8, default 2; develop and test with
  2, qualify all 8 before signing (user, 2026-10-05).
- Open choices: see WP-00 D2–D6 in the [virtual cable plan](virtual-cable.md).
- Rollback: the driver is a separate optional package; releases without it
  keep today's VB-Cable workflow. Revert DEC-18 doc changes to return to
  DEC-16.

## Decisions (2026-10-03, user)

- Signing: keep publishing unsigned prereleases while the project builds the
  public track record SignPath Foundation expects; apply later. The signing
  plan stays parked.
- Authorized now, in this order after items 1–2: Siege compressor
  simulation (item 3); Surround to headphones for 44.1/96 kHz 5.1/7.1 devices
  by resampling; the API request builder from the
  [external app integrations plan](../future/external-app-integrations.md);
  spatial speaker mode and distance/room controls from the
  [spatial audio plan](../future/spatial-audio.md). Each gets its own section
  here with requirement IDs, steps, validation and rollback before coding.

## Open work

### Code review P2/P3 follow-ups (2026-10-07, user request)

- **P2-1 (control-plane split).** `crates/control/src/lib.rs` went from
  33,117 to 1,290 lines. Code moved, without behavior change, into 22
  domain modules. The largest is `api_output_schema` at 2,045 lines. Tests
  moved to `<module>_tests.rs`, with shared fixtures in `test_support.rs`.
  Checks: 244 tests before and after, an identical `pub` item list, and
  item-by-item equivalence. Workspace Clippy (windows-gnu, `-D warnings`)
  and fmt are clean. The tests were not run here (they need Windows), and
  the test binary does not link in the container (`-lSwdevice` is
  missing). The typed method table was not done. Module list in the
  [review](../future/code-review-2026-10-07.md#p2-1-the-control-plane-is-one-21500-line-file).
  Next action: a Windows CI run of `cargo test -p audiorouter-control`.

- **P2-7 (DSP loops).** Engine block kernels (fixed delay, gain ramp,
  interleave/PCM16 conversions, channel matrix, `map_from`,
  `mix_mapped_from`, linear and streaming resamplers, voice-chain
  interleave) borrow planar channel slices once and run channel-major with
  `zip`/`chunks_exact`; their per-sample `unwrap`s are gone. The fixed
  per-route cost was `BlockMeter::observe` (80% of a bare Gain route in
  callgrind: eight meters per quantum, each making about eight passes); it now
  makes one vectorizable pass plus the sequential f64 sums. Output is
  bit-identical (`tests/planar_kernels_match_reference.rs` passes on the old
  and new code). `cargo bench --bench realtime` (Linux container, release):
  Gain 23.4 → 3.5 µs/quantum, ParametricEq 24.3 → 3.7, Denoise 66.3 → 46.4,
  route-32-tools 680.0 → 507.6. Continuity harness on Windows not run.
- **P2-2 (UI lint, format, App.tsx).** ESLint (typescript-eslint,
  react-hooks) and Prettier added to `ui/`; one mechanical format commit
  (`2697774b`); CI runs `format:check` and `lint`, the pre-commit hook
  formats staged UI files. 0 lint errors, 25 reviewed `exhaustive-deps`
  warnings. A proven stale closure is fixed: Ctrl+Alt+S after Save
  started a temporary preview instead of the saved session
  (`e2e/shortcut-after-save.pw.ts`). `App.tsx` 8,543 (formatted) → 3,069
  lines: helpers, four polling hooks and seven panels moved verbatim to
  their own files. Linux container: tsc clean, Vitest 56 files / 498
  tests before and after, build OK, harness-only Playwright (Linux
  Chromium) 152 → 153 passed, HTML of every tab and node inspector
  identical in three themes. Not run: backend-harness Playwright, Windows
  CI. Next: moving AppContent's action handlers (state flow) into hooks.

**P3-1 unsafe audit.** Every `unsafe` block and impl now has a
`// SAFETY:` comment with its invariants (pointer and handle validity,
owner and lifetime, buffer lengths, COM thread). Before: 188 missing
(windows-audio 98, transport 27, plugin-host 19, engine allocator test 4,
shell 26, `tools/m00-wasapi-probe` 14); after: 0. The lints
`clippy::undocumented_unsafe_blocks` and `clippy::missing_safety_doc` are
`deny` in `[workspace.lints.clippy]` (each crate has
`[lints] workspace = true`) and in `[lints.clippy]` of `src-tauri` and the
probe, so all targets, tests and examples included, are covered.
Soundness fixes: `SharedCapture::next_packet_into` and
`SharedRender::submit_bytes` reject `bytes_per_frame` above the stream's
`nBlockAlign` (a larger value read or wrote past the WASAPI buffer);
`created_callback` clones its sender before sending, so `create` may free
the context while `send` runs; `token_user_sid_string` reads `TOKEN_USER`
with `read_unaligned` from its byte buffer; the OS-transition and plug-in
editor windows no longer free their boxed context in `WM_NCDESTROY`, which
could double-free when `CreateWindowExW` fails after `WM_NCCREATE`; the
probe's `raw_capture_initialize` no longer calls `CoUninitialize` before
its interfaces are released.
Open: `NativeBridgeRegion` and `SharedAudioRegion` write through pointers
taken from `MmapMut`'s shared slice (`map.as_ptr()`), which Rust's aliasing
model does not allow; move them to `memmap2::MmapRaw`. The `enumerate_*`
helpers return a `windows::core::Error` (which may hold COM error info)
after `CoUninitialize`.
Local evidence (Linux container, pinned 1.96.0, `x86_64-pc-windows-gnu`):
workspace and shell Clippy with `-D warnings` clean; probe Clippy clean;
`cargo fmt` checks clean; engine, storage, domain, dsp, recording,
protocol and plugin-host tests pass. Windows CI and hardware runs not done
here; the Jev rule check was not run (no tool checkout or key).
Rollback: revert the commit.

- **P2-4 supply chain:** Dependabot, `deny.toml` and a `supply-chain` CI
  job (cargo deny for both workspaces, `npm audit --omit=dev` for ui,
  contracts and Stream Deck); actions bumped to Node 24 releases. Five
  unmaintained rust-unic advisories in `src-tauri` are ignored until
  2027-01-07 (needs a Tauri update checked on Windows). Local: both
  `cargo deny` runs and all three audits pass. Details in the
  [review](../future/code-review-2026-10-07.md#p2-4-dependency-and-supply-chain-checks-are-missing).
  Next action: first CI run of the `supply-chain` job.
- **P2-8 coverage and soak:** nightly/manual `quality.yml` (Windows):
  `cargo llvm-cov` and Vitest coverage (not gated), a 20-minute engine soak
  (`crates/engine/examples/soak.rs`, gates on heap growth, allocations and
  p99 per quantum), and the backend part of NFR-14 timed by
  `fresh_install_shell`. Local: 10 s soak passes; UI coverage 74.5 % of
  lines; portable-crate coverage 86.5 %. UI-ready startup time is not
  measured yet. Next action: dispatch `quality.yml` once and record the
  first numbers.

- **P2-5 network pairing key** (SEC-13, GRAPH-16; branch
  `worktree-agent-ae33fb03459795579`). Network Send/Receive take an
  optional `pairingKey` (domain validates blank or 16–128 printable ASCII).
  Wire format: version 2 = version 1 plus a 16-byte truncated HMAC-SHA256
  tag over header and samples; key = HMAC-SHA256(pairing key, fixed
  context). Tagging runs on the send I/O thread and verification plus the
  replay window (`crates/protocol/src/network_audio.rs`, portable) on the
  receive thread, never on the audio thread. Counters `authFailures`,
  `authProblem` and `replayedPackets` reach telemetry, `network.jsonl` and
  the inspector; the key is never logged (no log writes parameter values;
  MCP activity also drops `pairing*` argument names, and the inspector's
  change summary hides the value). A key change applies in place on
  both nodes. Decision: version 1 and 2 are mutually exclusive, so a paired
  node cannot talk to an older AudioRouter (documented).
  Evidence (Linux): protocol unit tests (pinned tag computed with Python
  `hmac`); domain validation tests; Windows-target Clippy clean for
  protocol, domain, windows-audio and control; the `network_audio.rs` socket
  tests run on Linux in a throwaway harness (stubs for the crate types):
  all pass except the pre-existing IPv6-loopback test, which also fails on
  the unchanged file in this container; Vitest and the new
  `ui/e2e/network-pairing.pw.ts` in Chromium, three themes, no layout
  shift. Windows CI must still run `windows-audio` and the extended
  `network_settings_corrected_while_playing_take_effect_without_restart`
  control test. Rollback: revert the commit; sessions with a key then fail
  validation until the key is removed.
  Next action: Windows CI on the branch, then a two-PC attended check with
  the same key, a wrong key, and one side blank.

- **P2-3 request IDs, verbose mode, support bundle** (commit `00ecb1ed`).
  Protocol: optional top-level `requestId` member, which older backends
  ignore. The logic is portable and unit-tested in
  `crates/protocol/src/diagnostics.rs`. New methods are
  `diagnostics.getVerbose` (`read`) and `diagnostics.setVerbose`
  (`sessionControl`, one hour, expires by itself). The Logs tab has a Verbose
  switch with a fixed-width countdown (UI-17) and "Copy support bundle"
  (`export_support_bundle`, a ZIP in the logs folder, path-filtered, local
  only). Decision: the ID travels beside `JsonRpcRequest` in wrappers
  (`CorrelatedRequest`/`IncomingRequest`), not as a new struct field, to
  leave its ~290 literal constructions unchanged. The HTTP forward signature
  now takes the ID.
  Verified on Linux: `cargo test -p audiorouter-protocol` and `-p
  audiorouter-domain`, the support-bundle tests in a scratch crate,
  `npm --prefix ui test` (493 passed), `tsc`, the Logs e2e in Chromium in
  dark, light and high-contrast, Windows-GNU Clippy for the workspace and
  `src-tauri`, and `tools/docs/validate.mjs`.
  **Next:** let Windows CI run the transport, shell (`http_api`,
  `support_bundle`, shell log) and `mcp_stdio` tests and
  `check-drift.mjs`. Then, in an attended run, click Play with verbose on,
  confirm one ID in the client row, `shell.jsonl` and `backend.jsonl`, and
  open a support bundle.

### Release packaging: one download (2026-10-06, user request)

User found the 0.0.13 release page confusing (16 assets) and asked for one
package containing the app, backend, frontend and Stream Deck plugin.
Finding: the installer already held the shell (UI embedded, backend in
process), CLI and plugin worker; the loose `.exe`s, `audiorouter-ui.zip`,
`run-vb-cable-desktop.ps1` and examples ZIP were duplicates or developer
files. Changes (requirement area M08 distribution, HTTP/Stream Deck):

1. `tauri.release.conf.json` bundles the packed Stream Deck plugin;
   `prepare-artifacts.ps1` and `m08-installer-smoke.ps1` pack it before NSIS.
2. Shell command `install_streamdeck_plugin` opens the bundled file with
   `ShellExecuteW` (Stream Deck asks to confirm); API tab gains
   `StreamDeckPluginPanel` with a reserved status line (UI-17).
3. Published assets: installer, `.streamDeckPlugin`, `SHA256SUMS.txt`,
   `release-manifest.json`, SBOMs, notices. `verify-artifacts.ps1`, its test
   (which still lacked the 0.0.12 Stream Deck entries) and `m08-release.ps1`
   updated; `m08-release.ps1` now rejects loose copies of bundled files.
4. `create-draft-release.ps1` prepends a "Which file do I download?" section.

Checks run (Linux container, 2026-10-06): `vitest` for
`StreamDeckPluginPanel`/`ApiPanel` pass; UI typecheck clean;
`npm run pack` in `tools/streamdeck` produced a package with
`com.mrdesjardins.audiorouter.sdPlugin/manifest.json`.
Not run (need Windows): shell compile, PowerShell release scripts and
tests, NSIS build, installing the plugin from the installed app, three-theme
screenshots of the API tab. Next: on Windows run `test-verify-artifacts.ps1`,
`m08-installer-smoke.ps1`, `m08-release.ps1`, then install the 0.0.14
candidate and press **Install Stream Deck plugin** with and without the
Stream Deck app. Rollback: revert the commit; earlier releases are unchanged.

### Defect 2026-10-05: backend dead after resume ("Backend refresh failed")

Reproduction (v0.0.12 release shell, PID 17188): sign-in start at
2026-10-04 19:05 with `patrick-main-native` running; lock + sleep at 19:20
(only `lock` was journaled); resume 2026-10-05 16:30:01, when the backend
still ran endpoint inventory (`discovery.jsonl`, `getDevicePeriod`
0x88890008). Opening the window at 19:39 showed "Unable to load your saved
session: Backend refresh failed", and every shell RPC was `transportError`.
The `\\.\pipe\audiorouter-control` pipe no longer existed, the process's live
threads totalled ~1.5 min CPU of 2 h 23 min (the 1 ms service thread had
exited), and `recovery_crashes` was empty, so the supervisor's error path
never ran: the control thread **panicked**. A panic bypassed the
`BackendSupervisor` loop, and release builds have no console, so the panic
location was lost.

Fix: `serve_control_connections_forever_observed` catches a control-plane
panic, wakes and joins its I/O thread (which otherwise keeps the only pipe
instance; max instances is 1), and returns an error. The shell supervisor
also wraps each attempt in `catch_unwind`, so a panic is counted and
restarted like any other failure (safe mode after repeated failures). A
panic hook writes a `shell.panic` record (thread name, file and line; no
message text) to `shell.jsonl`. Regressions:
`stopped_control_plane_releases_the_pipe_for_a_restarted_server` (fails with
the wake disabled) and `shell_panic_log_keeps_location_and_thread_but_no_message`.
Checks 2026-10-05: `cargo test -p audiorouter-transport --lib` 26/26,
`cargo test --manifest-path src-tauri/Cargo.toml` 61 passed, 1 ignored.

Released in 0.0.13 (2026-10-05): see the
[execution record](../archived/2026-10-05-release-0.0.13.md) and
[release evidence](evidence/2026-10-05-release-0.0.13.md).

Still open (next action): the panic site itself is unknown. After the next
occurrence, read the `shell.panic` record, fix the owning code and qualify a
sleep/resume with a native route running. A restarted backend leaves routes
stopped until Play.

## Risks, evidence and rollback

Stats.cc's undocumented feed can change; unknown state releases Duck.
Independent clock drift and queue latency remain limits. Keep the previous
installer and compatible configuration/recording backups for rollback; no
migration or driver install. Published assets are immutable; repairs use a new
version.
