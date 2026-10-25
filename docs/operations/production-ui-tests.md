# Production UI regression checks

Requirements: UI-02/11/17, NFR-08/14/15, ENG-02/05. These are browser acceptance
checks on Windows Edge. They do not establish packaged WebView2, native audio,
installer lifecycle, Narrator or endurance acceptance.

## Run

Install locked UI and contract dependencies and the pinned Rust toolchain as
described in the [quickstart](quickstart.md). From the repository root:

```powershell
# Shared local/CI full suite against bundled fixtures; guard qualifies first.
npm.cmd --prefix ui run e2e:ci

# Development server suite, then the focused optimized acceptance suite.
npm.cmd --prefix ui run e2e

# Focus on the optimized application code and fixture adapters.
npm.cmd --prefix ui run e2e:production-canvas -- --workers=1

# Qualify the uncaught-error guard in isolated Edge pages.
npm.cmd --prefix ui run e2e:guard
```

CI uses one worker. The full bundled profile inherits every `.pw.ts` scenario,
assertion timeout, diagnostic retry and `failOnFlakyTests` from the development
profile. It builds `index.html`, `harness.html`, `route-harness.html` and
`backend-harness.html` into `target/browser-acceptance-ui` before launching Edge.
This removes dev ESM/HMR traffic from the long acceptance run. PR #73's original
blank-page failure retained a `net::ERR_NO_BUFFER_SPACE` request failure for
`/src/routeHarness.tsx`; the precise OS resource cause remains unproven. Tests
are not reloaded or retried into success. `e2e:development` remains available
for development-server integration checks.

All profiles use the existing disposable Rust backend or deterministic route
adapter. The focused optimized build emits only
`route-harness.html` and `backend-harness.html` plus production chunks into
`target/canvas-production-ui`. Its dedicated output is emptied each time so
stale HTML/assets cannot make a missing build input appear to work. Ordinary
desktop production continues to emit `index.html` without test adapters.

The bounded optimized suite includes groups, alignment/size/distribution,
history, clipboard, connection creation, session save/reload/restart/import,
synthetic recording and live inspector/layout behavior in all three themes.
`verificationWiring.test.ts` checks the scripts, adapter inputs, selected file
existence and mandatory workflows so a renamed test cannot silently disappear.

## Readiness

`[data-testid="canvas-readiness"]` is a hidden diagnostic probe after the App
has applied a session snapshot. Tests wait for `data-ready="true"` together
with the expected `data-session-id` and `data-revision`. The probe requires
positive finite card dimensions, all expected handle IDs with valid bounds,
and loaded fonts. It uses the existing bounded measurement repair, with no
telemetry timer or unbounded retries. Exhaustion reports `data-state="exhausted"`
and `data-ready="false"`; a loading screen has no ready canvas.

The probe is recreated when session/revision changes. An empty session can be
ready with no connectors. Tests cover cold missing-bound repair, delayed first
snapshot, reload, commit and session changes, plus permanent missing bounds.
This measures usable browser geometry, not Windows launch time or audio health.

## Uncaught errors and evidence

Every active `.pw.ts` scenario uses the guarded browser fixture or real-backend
fixture. An uncaught page error fails a scenario even if its UI assertions
pass. No error-message allow-list or blanket console suppression was added.
The browser guard qualification runs a clean page (must exit zero) and a page
with an intentional unhandled error (must fail at the guard's exact assertion).
A missing module or failed browser launch cannot count as successful detection.

Outputs are under `%TEMP%` locally and `RUNNER_TEMP` in CI:

- `audiorouter-playwright-results`: full-suite failure traces/screenshots
  (bundled in shared verification, development for `e2e:development`).
- `audiorouter-production-playwright-results`: optimized failure artifacts.
- `audiorouter-browser-guard-*`: positive/negative logs and qualification JSON.

CI retains guard evidence even on successful runs and both browser artifact
directories after failure. Screenshots use test output paths. Keep the tested
SHA/run URL, environment and original failure when diagnosing; a rerun is not
first-pass evidence. README screenshot generators and optional external HTTP
fixtures remain explicitly opt-in; a skip is not a product pass.

## Separate packaged gate

The [quality roadmap](../plans/active/application-quality.md) requires an exact
release-shell smoke with an isolated database and WebView2 profile, native
bridge, usable canvas, stopped first launch and save/reload evidence. Build the
embedded UI and release shell, then run from the repository root:

```powershell
npm.cmd ci --prefix ui
npm.cmd --prefix ui run build
cargo build --manifest-path src-tauri/Cargo.toml --release --locked --features custom-protocol
$out = Join-Path $env:TEMP ('audiorouter-packaged-smoke-' + [guid]::NewGuid().ToString('N'))
node tools/quality/packaged-shell-smoke.mjs --shell src-tauri/target/release/audiorouter-shell.exe --output $out
```

The runner checks the production default control pipe and refuses to start if
it is occupied. Close AudioRouter yourself before running it; the runner never
closes or replaces an existing instance. It enables a loopback DevTools port
only in the child process, uses a disposable WebView2 profile/database, and
cleans up only the child it started. Evidence and bounded logs remain in `$out`.
Run native WebView2 acceptance from a normal interactive Windows process.
The agent tool's restricted process sandbox prevented navigation on the local
reference machine; rerunning outside that sandbox with a standard-user token
worked. This does not require UAC elevation or changing Windows security settings.
The runner removes inherited diagnostic markers and debugger waits. It verifies
the native bridge through normal `system.describe` calls; the older create-only
`AUDIOROUTER_SHELL_PROBE_FILE` probe disrupts repeated UI snapshot refreshes.
See the [controlled investigation](../plans/active/evidence/2026-10-25-packaged-webview-smoke.md).

Qualify the smoke's own failure/cancellation guards against the same binary:

```powershell
$out = Join-Path $env:TEMP ('audiorouter-packaged-qualification-' + [guid]::NewGuid().ToString('N'))
node tools/quality/qualify-packaged-shell-smoke.mjs --shell src-tauri/target/release/audiorouter-shell.exe --output $out
```

The qualifier runs 11 sequential cases: a healthy baseline, missing binary,
occupied control pipe, exited shell, invalid native response, blocked canvas
readiness, an uncaught error during document initialization, refused graceful
quit, and cancellation before attachment/after readiness/after restart. It
requires each seeded fault to reach its owning failed check and retain evidence;
an injection that never takes effect fails qualification. Page/bridge/readiness
and refused-quit cases must retain a screenshot, HTML and trace.

Cancellation uses a real Windows CTRL_BREAK event delivered to the runner's
own hidden console. The app has a separate process group, so its cleanup must
come from the runner's handler. The report must say `cancelled` and exit 130.
The qualifier snapshots the owned shell/WebView descendants and verifies that
the same PID/creation-time pairs have disappeared. The runner also handles
SIGINT/SIGTERM and qualification-parent disconnection. Uncatchable OS termination
cannot execute Node handlers; a process-tree supervisor remains responsible for
that form of termination.

Qualification control and fault options are confined to this tool's private
named-pipe protocol. Ordinary smoke runs enable no fault injection. Nightly
quality runs the qualifier before the ordinary smoke and uploads bounded case
reports/logs/screenshots/traces, excluding disposable browser profiles/databases.
This is packaged UI/backend evidence, not installer lifecycle, physical audio,
Narrator/accessibility or endurance acceptance. Keep those gates explicit in
the active plan.

The startup job explicitly checks Microsoft's WebView2 runtime registration
before either launch test. Edge Stable alone does not supply this dependency
([Microsoft distribution guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)).
If missing, the disposable GitHub runner installs the signed Microsoft Evergreen
bootstrapper; the helper refuses automatic installation outside GitHub Actions.
`metrics-startup/webview2-runtime.json` records detection before/after setup,
interactive session information and the downloaded bootstrapper hash. Packaged
artifacts include the bounded `shell.jsonl`, `backend.jsonl` and `shell.panic`
files as well as the smoke reports; raw profiles and databases remain excluded.

GitHub's administrator job token is unsuitable for this environment-based smoke:
WebView2 ignores `WEBVIEW2_*` overrides in elevated hosts, including the isolated
profile and debugger flags ([Microsoft privilege guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security)).
The nightly job uses `standard-user-process.ps1` to launch verification with a
restricted Medium token, verifies the actual suspended child token before it
runs, and retains `launch-token.json`. It preserves identity/session, environment,
streams, exit status and owned-tree timeout cleanup without changing machine
settings. Four Windows launcher probes run before the release build and upload
their evidence. The smoke itself refuses an elevated or Low token before starting
the shell; direct local runs must use a normal standard-user session.
