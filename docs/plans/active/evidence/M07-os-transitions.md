# M07 OS-transition policy evidence

## 2026-09-17 - focused executable recovery refresh

The current worktree passed the focused control transition run with all seven
`os_transition` tests: sorted lock handling, repeated bounded
suspend/resume decisions, native/recording protection, sign-out/sleep release,
idempotency replay, and active-recording refusal. The Windows shell listener
run also passed all four `os_transition_windows` tests covering notification
mapping, unrelated-message rejection, nonblocking bounded delivery, and
listener startup/teardown. These are executable policy/listener checks only;
they do not prove a real workstation transition, endpoint re-enumeration, or
automatic native reopen.

## 2026-09-17 - continuation recovery regression refresh

The focused control transition run passed 7 tests: sorted lock handling,
bounded repeated suspend/resume decisions, native/recording protection,
sign-out/sleep release, idempotency replay, and active-recording refusal. The
Tauri shell run passed all 29 tests, including Windows notification mapping,
bounded nonblocking delivery, listener startup/teardown, safe-mode latching,
and tray authority. These checks remain policy/listener evidence; they do not
prove an actual workstation transition, endpoint re-enumeration, or automatic
native reopen.

## 2026-09-17 - elevated shell acceptance rerun

The elevated `tests/acceptance/m07-shell-rpc.ps1` acceptance passed through
the real Tauri/WebView initialization, native RPC command, and authenticated
backend `system.describe` path. The bounded run used disposable resources and
changed no audio endpoint or persistent machine configuration. This is current
shell transport evidence only; attended lock/sign-out/sleep/resume delivery,
endpoint re-enumeration, and automatic native reopen remain unverified.

## 2026-09-17 - transition idempotency replay regression

Added a control-plane regression for `system.osTransition`: the first sleep
request is journaled, an identical request with the same idempotency key
replays the original result without reapplying stop/release, and reusing that
key for a different transition is rejected. The focused test and full control
library suite passed (176 non-ignored tests, 3 guarded live tests ignored).
This strengthens durable recovery semantics without claiming real OS
notification delivery or automatic native endpoint reopen.

## 2026-09-17 - continuation shell transport refresh

The elevated `tests/acceptance/m07-shell-rpc.ps1` acceptance passed again
through the real WebView initialization, Tauri command, and authenticated
backend `system.describe` path. Its disposable backend, pipe, database, and
child processes were cleaned up. This refreshes the shell transport evidence
only; it does not claim attended OS power/session delivery, endpoint
re-enumeration, or automatic native reopen.

## 2026-09-17 — elevated shell transport requalification

The guarded elevated `tests/acceptance/m07-shell-rpc.ps1` acceptance passed
the real Tauri/WebView initialization, native RPC command, and authenticated
backend `system.describe` path. It used disposable database, pipe, and marker
resources, cleaned its child processes, and changed no audio endpoint or
persistent machine configuration. This closes the shell transport boundary;
it does not claim attended lock/sign-out/sleep/resume delivery or endpoint
re-enumeration evidence.

The native endpoint bridge now exposes its negotiated same-rate graph sample
rate to control-plane activation. This corrects the prior 48 kHz activation
assumption for validated 44.1 kHz endpoint pairs. Cross-rate capture/render
conversion is intentionally still gated; mismatched endpoint formats remain
rejected before activation.

Date: 2026-09-15

The control crate now exposes a side-effect-free `plan_os_transition` contract
for STATE-11. Lock keeps the explicitly running session set. Sign-out and
sleep request stop/release for every running session. Resume only returns
non-native, non-recording sessions for endpoint revalidation before restart;
protected sessions remain stopped. IDs are sorted and deduplicated before a
decision is returned.

The control plane now applies the stop/release boundary through
`ControlPlane::handle_os_transition`. It refuses a sleep/sign-out transition
when an active recorder still needs explicit finalization, preserving the
running session rather than silently losing recording data. A successful sleep
stores only eligible portable sessions; resume consumes that bounded set and
returns `revalidateBeforeRestart` without starting audio itself.
On resume, an existing endpoint monitor is force-refreshed through its
read-only snapshot path before the response is returned; the response reports
`endpointInventory: refreshed` or `notStarted`.

Sleep also preserves the exact IDs of native-owned sessions separately from
portable restart candidates. Resume returns those IDs as `nativeSessionIds`
and requires `revalidateBeforeRestart`; it does not restart or reopen a native
endpoint. This keeps hardware and driver recovery explicit while preserving
the identity needed by the future exact-rebind operation.

The same boundary is exposed as the authenticated, idempotent
`system.osTransition` JSON-RPC method so a native notification adapter can use
the shared backend authority. Its input and output schemas are included in
method discovery and the readable API reference. The TypeScript contract and
UI backend adapter now expose the same typed method to connected hosts; the
disconnected adapter fails closed.

The MCP adapter now exposes a focused `os_transition` tool with the same
transition enum and required idempotency key. It is non-read-only and
idempotent, and dispatches through the authenticated control plane; the MCP
adapter owns no transition state.

The CLI now exposes the same operation as `os-transition <transition>` with a
durable database and explicit idempotency key. Invalid transition names are
rejected before storage is opened; valid requests use the shared authenticated
dispatcher.

The connected UI exposes a resume-validation panel. It displays the backend's
endpoint-inventory result and exact portable/native revalidation candidates;
the panel does not start routes during validation and explicitly keeps native
routes stopped. A separate deliberate action can restart only the portable
IDs returned by the backend, with per-route failure reporting; native IDs are
never passed to session start.

After either validation or the explicit portable restart, the panel refreshes
the authoritative snapshot so sidebar and session status do not remain stale
after a partial or successful recovery.

The Tauri shell now owns a Windows message-only listener in
`src-tauri/src/os_transition_windows.rs`. WTS session notifications map the
current user's lock and logoff events; `WM_POWERBROADCAST` maps suspend and
automatic resume. Delivery uses a bounded `SyncSender::try_send`, while a
separate shell thread forwards authenticated RPC requests. The listener uses
the actual listener thread ID for shutdown and releases its window callback
state during `WM_NCDESTROY`.

Validation:

```text
cargo test -p audiorouter-control --locked --lib -- --test-threads=1
165 passed; 2 ignored; 0 failed
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
clean
cargo test --manifest-path src-tauri/Cargo.toml --locked -- --test-threads=1
22 passed; 0 failed
npm.cmd run typecheck --prefix contracts
clean
npm.cmd run typecheck --prefix ui
clean
npm.cmd test --prefix ui -- --run
218 passed; 0 failed
npm.cmd run build --prefix ui
production build passed (214 modules)
cargo test -p audiorouter-cli --locked
38 passed; 0 failed
```

This is portable policy evidence, not Windows power-notification evidence.
The native listener startup/teardown smoke test now passes, but attended
delivery still needs a guarded lock/sign-out/sleep/resume acceptance and
endpoint re-enumeration with
before/after identity proof. No machine power state, audio endpoint, driver,
or user configuration was changed by this work.

## 2026-09-17 - notification mapping regression

`cargo test --manifest-path src-tauri/Cargo.toml --locked --
--test-threads=1` passed 28 shell tests. New focused tests cover the exact
`WM_WTSSESSION_CHANGE` lock/logoff and `WM_POWERBROADCAST` suspend/resume
mappings, plus rejection of unrelated message parameters. The listener
startup/teardown test also remains green. These are bounded message-translation
and lifecycle tests only; they do not simulate a real OS transition or prove
endpoint reopening, native restart, or attended delivery.

The shell transition forwarder uses a process-and-timestamp-qualified
idempotency key, preventing a newly launched shell from colliding with a
durable transition operation from an earlier shell instance. The uniqueness
regression is covered by the native shell test suite.
## 2026-09-15 — repeated suspend/resume policy cycles

Added `repeated_suspend_resume_cycles_remain_bounded_and_fail_closed`, which
runs 100 portable Sleep/Resume decisions over a mixed desktop/native session
set. Every cycle returns `StopAndRelease` for the sorted running set and
`RevalidateBeforeRestart` only for the desktop session; the native session is
never automatically resumed. The focused control transition suite passed 6
tests with formatting and diff checks clean. This validates the side-effect-
free policy boundary only; it does not claim OS power-notification delivery,
endpoint reopening, or native restart evidence.

## 2026-09-17 - current shell and lint refresh

cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
clean (exit 0). This confirms the current source tree has no Clippy warnings under the denied-warning gate; it does not expand the attended OS-transition, endpoint-reopen, driver, or release claims above.

## 2026-09-17 - attended transition gate audit

The repository contains no safe acceptance wrapper that drives an actual
Windows lock, sign-out, suspend, or resume transition and then proves native
endpoint reopen. The available guarded live checks qualify endpoint lifecycle
and exact stopped rebind separately; the shell listener and control policy
tests qualify message mapping, authentication, idempotency, and fail-closed
resume behavior. A workstation power/session transition was therefore not
forced by automation. Real transition delivery, endpoint re-enumeration, and
transition-triggered native reopen remain explicitly unverified rather than
being inferred from the lower-level tests.

## 2026-09-17 - bounded listener delivery regression

The Windows shell listener now routes recognized notifications through one
small nonblocking forwarder used by the window procedure. A focused regression
fills and drains a capacity-one channel to prove recognized notifications are
delivered when capacity exists and are dropped without waiting when the
control-plane queue is full. The Tauri shell suite passed 29 tests, strict
Clippy passed with denied warnings, and formatting passed. This strengthens
the shell-to-control handoff only; it does not turn synthetic messages into
real lock, sleep, sign-out, resume, or native-reopen evidence.

## 2026-09-17 - focused recovery policy and listener regression refresh

The focused locked control test run passed six OS-transition tests, including
the repeated suspend/resume boundedness and fail-closed cases, plus the
active-recording refusal rule. The focused `src-tauri` Windows listener run
passed four tests covering recognized notification mapping, unrelated-message
rejection, nonblocking bounded delivery, and listener startup/teardown. These
are repeatable policy and listener-component checks; they do not promote real
lock, sign-out, sleep, resume, endpoint re-enumeration, or native-reopen
behavior to attended evidence.

## 2026-09-17 - continuation listener and recovery refresh

`cargo test --manifest-path src-tauri/Cargo.toml --locked --
--test-threads=1` passed all 29 shell tests again, including bounded
notification delivery, unrelated-message rejection, exact power/session
mapping, listener startup/teardown, safe-mode latching, and tray authority
checks. This refresh is synthetic/policy evidence only; real OS transition
delivery, endpoint re-enumeration, native reopen, and endurance remain open.

## 2026-09-17 - resume endpoint-change retention fix

The control recovery path now retains the exact endpoint changes returned by a
resume resnapshot and publishes the bounded `devices.changed` event through the
same path as read-only inventory. This prevents resume from consuming a change
without leaving it available for deliberate endpoint rebind. It does not select
a replacement or reopen native audio automatically. The locked control library
suite passed 177 tests with three guarded live tests ignored; real power/session
delivery, endpoint re-enumeration caused by an OS transition, and native reopen
remain unverified.

## 2026-09-18 - interactive-surface availability recheck

The computer-use inventory was rechecked before attempting attended acceptance
and again returned `apps: []` and `browsers: []`. No lock, sign-out, sleep,
resume, accessibility, scaling, first-run, or drag/drop action was attempted.
The component and policy evidence above remains valid, while the attended
transition and native-reopen gates remain open.

The same-day focused refresh then passed 7 locked `audiorouter-control` OS-
transition tests and all 29 locked `audiorouter-shell` tests (including
listener delivery, startup/teardown, safe-mode, tray, and transition-key
coverage). These are portable/component checks only; no OS state or audio
endpoint was changed.

## 2026-09-18 - headless recovery and parity refresh

`tests/acceptance/m07-headless.ps1` passed on the current worktree. The
acceptance covered 36 CLI tests, 3 MCP stdio interoperability tests, 177
control tests with 4 guarded live tests ignored, 70 plugin-host tests, 13
worker-process tests, and the M01 CLI acceptance. All executed tests passed.
This refresh confirms headless parity and recovery-policy regressions only;
the guarded live tests and real OS-transition, endpoint-reenumeration, native
reopen, attended-shell, and endurance gates remain open.
