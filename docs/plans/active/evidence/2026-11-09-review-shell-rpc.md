# R5 packaged-shell RPC measurement

Windows, 2026-11-09. Branch `fix/code-review-2026-11-09`, after `125a145e`.
Built with `npm --prefix ui run build`, then
`cargo build --manifest-path src-tauri/Cargo.toml --features custom-protocol`.
The debug custom-protocol exe was newer than the bundled UI. No other shell
was running. The database, LOCALAPPDATA, APPDATA and WebView2 profile were
isolated under `target/review-runtime-baseline2/`. No device was selected or
started; scanning inspects metadata and does not execute plugin code.

Temporary `eprintln!` instrumentation recorded process/thread IDs at `main`
and the thread/duration after forwarding `plugins.scan` in `rpc_request`.
It was removed after measurement. Logs contain no audio or credentials.

From the real Settings → Plug-ins page, scan
`C:\Program Files\Common Files\VST3` (92 files, 11 discovered candidates):

```text
Baseline:
REVIEW_MAIN_THREAD pid=31972 ThreadId(1)
REVIEW_RPC_THREAD pid=31972 ThreadId(1) duration_ms=1835

With #[tauri::command(async)] on rpc_request:
REVIEW_MAIN_THREAD pid=31788 ThreadId(1)
REVIEW_RPC_THREAD pid=31788 ThreadId(25) duration_ms=1466
```

The changed shell returned a live UI Automation/screenshot observation of
“Scanning selected directory...” and the disabled Scan button before the
RPC finished. A native title-bar drag succeeded afterward. Exact drag/scan
overlap was not instrumented; do not claim that timing from this observation.
The instrumented changed exe SHA-256 was
`1B3A09641CBE913E0F8CAA77DF4A154B8B86036FC3766C88A974541675322A0E`.
This is diagnostic evidence, not a release artifact.

The first isolated launch set AUDIOROUTER_CONTROL_PIPE, which intentionally
disables the owned backend and produced “Backend refresh failed.” It was
discarded. The two valid measurements used the normal owned backend and an
isolated database/profile, without that override or a probe-file marker.

Remaining final evidence: repeat with the final uninstrumented build and a
generated-audio route, check the live readings during scan, and exercise
hold/release and refused deletion. No microphone/hardware evidence is
claimed by the measurements above.
