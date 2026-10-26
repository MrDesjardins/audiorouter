# Packaged smoke guard qualification - 2026-10-25

Result: all 11 local qualification cases passed, followed by a passing ordinary
smoke with fault injection disabled. The owner authorized finishing these
checks, committing and pushing `quality/packaged-webview-smoke`. Hosted CI is
the next gate; PR creation still requires explicit owner approval.

Requirements: UI-11, NFR-14 startup/consent checks, ENG-02/04/05. This extends
the [startup investigation](2026-10-25-packaged-webview-smoke.md) without
changing application source, APIs, device grants or Windows settings.

## Environment and build

- Windows 11 Home `10.0.26300`, WebView2 `155.0.4283.62`, Node `22.22.3`.
  Interactive standard-user execution outside the agent process sandbox.
- Product source `11737991135112c7085f6b45a88770f36287fd04`; quality tooling
  was uncommitted on the feature branch while qualification ran.
- Release shell SHA-256:
  `02d38c70a8ca845cc704f2d056f2c2a501200d4a1e07770916d076184182ce9e`.
- Every case used disposable data/profile paths and the default-pipe preflight.
  No audio was played. Ordinary smoke reports show zero active sessions and
  denied device access before and after restart.

## Mechanism

`qualify-packaged-shell-smoke.mjs` coordinates the runner through a private,
random named pipe. Fault switches require a supported qualification case and
that control pipe together. The ordinary smoke enables no injected faults.
The qualifier pauses at an explicit checkpoint, captures only its owned
shell/WebView process tree, then continues or cancels the test.

For Windows cancellation, a helper launches Node in its own hidden console,
attaches a second helper to that console, and sends a real CTRL_BREAK event.
The shell uses a separate process group; its termination comes from the
runner's SIGBREAK handler, not the console event. The handler aborts pending
work and terminates the currently owned shell tree. Reports retain `cancelled`,
the signal, child exit states and cleanup actions; cancellation exits 130.

The process helper reads only PID, parent PID and creation time. The qualifier
verifies that captured PID/creation-time pairs have disappeared after each
case, protecting against PID reuse. Emergency cleanup would fail qualification;
none was required in the final matrix. Hard, uncatchable termination of Node
cannot run its handlers and remains the external supervisor's responsibility.

Bridge and quit faults intercept the actual Windows IPC boundary, covering
both custom-protocol fetch and WebView2 postMessage. Hit counters must prove a
request encountered the fault. The bridge case returns a JSON-RPC error; quit
refusal rejects the native command without exiting the app. Readiness is held
false on the real probe. The page-error case installs an initialization script
and reloads the document after the listener is attached, seeding an uncaught
error during that document's initialization.

## Final matrix and evidence

```powershell
node tools/quality/qualify-packaged-shell-smoke.mjs --shell src-tauri/target/release/audiorouter-shell.exe --timeout-ms 5000
```

Overall: passed, 11/11. Summary:
`%TEMP%/audiorouter-packaged-qualification-26c0b5d2-c0f0-44ea-a508-7a351d589067/qualification.json`.
Each case's subdirectory retains its runner report, bounded logs and captured
process identities. Expected failure/cancellation reports are evidence that
the guard worked; the overall qualifier accepts only the expected reason,
exit code, injection confirmation and complete captured-process cleanup.

The final full rerun with creation-order validation in descendant discovery
also passed 11/11:
`%TEMP%/audiorouter-packaged-qualification-ad091a86-5bbd-4046-a60a-2b2b4bbb60b9/qualification.json`.
That validation excludes an older orphan whose recorded parent PID has since
been recycled. Final review then tightened successful-exit assertions and
paired the error-listener/reload promises; their affected cases were rerun:
baseline passed at
`%TEMP%/audiorouter-packaged-qualification-f0540544-09db-4ade-9942-31f1531d3080/qualification.json`
and page-error passed at
`%TEMP%/audiorouter-packaged-qualification-e318d9ca-eb5b-4297-ac32-8f4d87c0faae/qualification.json`.

- **Baseline:** runner passed, exit 0; save/restart/reload and both native state
  responses passed. All 14 captured processes from the two launches exited.
- **Missing binary:** runner failed, exit 1, ENOENT; no app child was launched.
- **Occupied pipe:** runner failed, exit 1, occupied-default-instance error;
  no app child was launched. The qualifier owned and closed the test listener.
- **Shell exit:** runner failed, exit 1, exited-before-WebView2 error;
  all 7 captured processes exited.
- **Bridge:** runner failed, exit 1, invalid-native-describe error; one IPC
  fault hit, screenshot/HTML/trace present, all 7 captured processes exited.
- **Readiness:** runner failed, exit 1, initial usable-canvas timeout;
  screenshot/HTML/trace present, all 7 captured processes exited.
- **Page error:** runner failed, exit 1, uncaught seeded page error;
  screenshot/HTML/trace present, all 7 captured processes exited.
- **Refused quit:** runner failed, exit 1, graceful-quit timeout; an actual IPC
  fault hit was recorded, forced `taskkill-tree` cleanup ran, screenshot/HTML/
  trace were retained, and all 7 captured processes exited.
- **Cancel before attachment:** runner cancelled, SIGBREAK, exit 130;
  tree cleanup recorded, all 7 captured processes exited.
- **Cancel after readiness:** runner cancelled, SIGBREAK, exit 130;
  tree cleanup recorded, all 7 captured processes exited.
- **Cancel after restart:** runner cancelled, SIGBREAK, exit 130; the first
  shell quit gracefully and the restarted shell tree was terminated. All 14
  captured processes from both launches exited.

The ordinary runner then passed without qualification options:

```powershell
node tools/quality/packaged-shell-smoke.mjs --shell src-tauri/target/release/audiorouter-shell.exe --timeout-ms 30000
```

Report:
`%TEMP%/audiorouter-packaged-smoke-7d0d6cf6-3732-427c-a35f-6252a2808a8b/result.json`.
Fresh-profile usable canvas 822 ms, restart 729 ms, save 82 ms; both shells
quit gracefully. These warm-machine samples remain informational for NFR-14.
No `audiorouter-shell` process remained after the checks.

Final local verification: `node tools/quality/verify.mjs --checks docs,policy`
passed (197 Markdown files, 1,092 local links; 65/65 policy/runner tests).
Verification JSON:
`%TEMP%/audiorouter-verification/1792959245613-d6bbf29b-221d-4e05-bd7f-9e3107f2acad.json`.
Node syntax and Prettier passed for the smoke/environment/qualifier; workflow
YAML parsed; the Windows helper and both added workflow PowerShell blocks
parsed; the helper is ASCII; `git diff --check` was clean. `actionlint` is not
installed locally. No Rust or UI source was edited.

## Failed experiments retained

- An initial detached Node runner had no console. AttachConsole failed with
  error 6; qualification failed and parent-disconnect cleanup stopped its app.
  One report is under
  `%TEMP%/audiorouter-packaged-qualification-0b975299-ee65-4501-a976-002ef615a20e/`.
  The hidden console launcher replaced that approach.
- Console delivery worked, but PowerShell initially lost the runner's exit
  code and returned 0. The qualifier rejected `0 != 130` even though the runner
  report said `cancelled` and all processes exited. Evidence:
  `%TEMP%/audiorouter-packaged-qualification-0ccd28c5-0b84-4f0d-a2c9-7f0f9696a5ad/`.
  Caching the process handle before waiting preserves the actual exit code.
- The first full matrix passed 9/11 and rejected ineffective bridge/quit
  injections. Tauri freezes its public API exports, and the local app also
  uses the Windows postMessage IPC fallback. Evidence:
  `%TEMP%/audiorouter-packaged-qualification-24d7ef33-d80a-49aa-9e9d-b7feef605ad4/`.
  Replacing the actual IPC transport and requiring hit counters fixed the
  qualification. These failed runs are not represented as first-pass success.

## Delivery and limits

Nightly quality runs the qualifier before the ordinary packaged smoke and
retains both artifact sets on failure. Profiles and databases are excluded
from uploads. Local qualification uses a 5-second readiness budget; the nightly
qualifier defaults to 15 seconds. No retries hide a failed qualification.

The error test covers document initialization after attachment. Errors that
occur before the very first CDP attachment are not individually retained by
the page listener; native bridge and readiness still gate successful startup.
Hosted runner compatibility and current CI results remain pending until the
branch is pushed and the workflows run. Installer, hardware audio, Narrator,
network-disabled and endurance qualification remain their separate gates.
