# Production UI regression checks

Requirements: UI-02/11/17, NFR-08/14/15, ENG-02/05. These are browser acceptance
checks on Windows Edge. They do not establish packaged WebView2, native audio,
installer lifecycle, Narrator or endurance acceptance.

## Run

Install locked UI and contract dependencies and the pinned Rust toolchain as
described in the [quickstart](quickstart.md). From the repository root:

```powershell
# Full development suite, then optimized acceptance suite; guard qualifies first.
npm.cmd --prefix ui run e2e

# Focus on the optimized application code and fixture adapters.
npm.cmd --prefix ui run e2e:production-canvas -- --workers=1

# Qualify the uncaught-error guard in isolated Edge pages.
npm.cmd --prefix ui run e2e:guard
```

CI uses one worker. Both browser profiles use the existing disposable Rust
backend or deterministic route adapter. The optimized build emits only
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

- `audiorouter-playwright-results`: development failure traces/screenshots.
- `audiorouter-production-playwright-results`: optimized failure artifacts.
- `audiorouter-browser-guard-*`: positive/negative logs and qualification JSON.

CI retains guard evidence even on successful runs and both browser artifact
directories after failure. Screenshots use test output paths. Keep the tested
SHA/run URL, environment and original failure when diagnosing; a rerun is not
first-pass evidence. README screenshot generators and optional external HTTP
fixtures remain explicitly opt-in; a skip is not a product pass.

## Separate packaged gate

The [quality roadmap](../plans/active/application-quality.md) still requires an
exact built-shell smoke with its matching sidecars, isolated database/WebView
profile, native bridge, stopped first launch, save/reload and recovery evidence.
The ordinary app uses the default pipe/single-instance guard, so do not launch
that smoke while the owner's daily app is running. Closing or replacing the
owner's process is not an incidental test action. Keep missing packaged,
hardware/accessibility and release evidence explicit in the active plan.
