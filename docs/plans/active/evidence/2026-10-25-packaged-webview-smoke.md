# Packaged WebView2 smoke investigation - 2026-10-25

Result: the local packaged smoke passes after correcting its launch environment,
diagnostic probe and selectors. No application source change was required.
The [local cancellation/failure qualification](2026-10-25-packaged-smoke-guard-qualification.md)
subsequently passed 11/11; current hosted CI remains required before the
nightly gate is declared complete.

## Build and environment

- Product source: `11737991135112c7085f6b45a88770f36287fd04` (`v0.0.23`).
  The working tree also contains uncommitted quality tooling/docs changes.
- Executable: `src-tauri/target/release/audiorouter-shell.exe`, SHA-256
  `02d38c70a8ca845cc704f2d056f2c2a501200d4a1e07770916d076184182ce9e`.
- Windows 11 Home `10.0.26300`, WebView2 `155.0.4283.62`, Node `22.22.3`.
- Run outside the agent tool's restricted process sandbox, in interactive
  session 1. A WindowsPrincipal check in that launch context returned
  `AdministratorToken=false`; no UAC or Windows policy change was made.
- Disposable database, roaming data and WebView2 profile per run; production
  default pipe preflight; inherited device grants and debugger/probe switches
  removed. No playback was started; device access remained denied.

The UI and release shell were built on 2026-10-24 with `npm.cmd ci --prefix
ui`, `npm.cmd --prefix ui run build`, and `cargo build --manifest-path
src-tauri/Cargo.toml --release --locked --features custom-protocol`.

## Controlled findings

1. **Restricted launch environment.** Prior sandboxed launches exposed a
   blank/`about:blank` WebView target and no native response. Running the same
   executable and original runner outside that process sandbox produced the
   frontend-owned native response. This supersedes the prior inference that
   the product's embedded assets might be broken. It does not establish which
   individual sandbox restriction caused the failed navigation.
2. **One-shot probe perturbed the UI.** The original runner enabled
   `AUDIOROUTER_SHELL_PROBE_FILE`. `ui/src/main.tsx` sends the probe's
   `system.describe`; the real backend adapter also requests `system.describe`
   during snapshot refresh (`ui/src/backend.ts`). The shell's
   `write_probe_marker` uses `create_new(true)` and propagates an existing-file
   error from `rpc_request`. Thus the first request wrote a marker, but later
   requests failed and the UI showed `Unable to load your saved session:
   Backend refresh failed`. The retained screenshot and HTML confirm this.
   The smoke now clears this inherited switch and calls the normal native
   `system.describe` API before and after restart, retaining compact results.
3. **Selector matched hidden legacy controls.** The first corrected live run
   reached usable canvas and passed native/consent checks, then the session
   label matched both a hidden legacy input and the visible workbench input.
   The runner now selects the visible textbox by role. Restart verification
   checks both the visible selected session and its backend representation.
4. **Debugger setup.** WebView2 writes `DevToolsActivePort` inside the
   runtime-added `EBWebView` directory. Chromium rejects the explicit
   `http://localhost` WebSocket origin without its allowlist flag. The runner
   enables only the required child debugging flags, binds to loopback and
   allows that origin. An initial target list can still say `about:blank`
   while navigation is starting; usable canvas and native responses determine
   success.

The environment helper regression verifies removal of inherited grants,
one-shot probes and debugger waits, case-insensitive Windows variable
replacement, preservation of ordinary runtime variables, and no parent
environment mutation. Both normal UI startup and repeated API calls now run
without the one-shot probe.

## Commands and retained results

Command, from the repository root and outside the tool process sandbox:

```powershell
node tools/quality/packaged-shell-smoke.mjs --shell src-tauri/target/release/audiorouter-shell.exe --timeout-ms 30000
```

- Original unsandboxed runner: failed at canvas readiness because the probe
  disrupted snapshot refresh. Evidence under
  `%TEMP%/audiorouter-packaged-smoke-0eb9dcba-6c1d-42a6-856b-7967de3b466c/`:
  `result.json`, `failure.png`, `failure.html`, `trace.zip`, bounded logs.
- Probe removed: failed at duplicate session-name selector. Evidence under
  `%TEMP%/audiorouter-packaged-smoke-87b7ed0f-08db-4920-bb36-f09b0b75af8c/`.
- Selectors corrected: passed. Report at
  `%TEMP%/audiorouter-packaged-smoke-8ec6b2fc-a810-48b1-a748-a87a66fe6912/result.json`.
  Fresh-profile UI-ready 762 ms; restart UI-ready 702 ms.
- Runner with tested environment helper: passed. Report at
  `%TEMP%/audiorouter-packaged-smoke-92f67be5-59e9-41c2-b654-4c2095491d0b/result.json`.
  Fresh-profile UI-ready 804 ms; restart UI-ready 718 ms; total 2,153 ms.
  Both native describes returned protocol 1.0 / `desktop-shell`; no uncaught
  UI errors or cleanup failure. No `audiorouter-shell` process remained.
- Final runner with launch timing measured before spawn, separate save duration,
  and explicit recorded startup/restart consent state: passed. Report at
  `%TEMP%/audiorouter-packaged-smoke-d5e59c40-7dde-4248-9a7f-a01f003879cf/result.json`.
  Fresh-profile UI-ready 794 ms; restart UI-ready 758 ms; save 74 ms;
  total 2,007 ms. Both state responses recorded zero active sessions and
  `deviceAccessAllowed=false`.
- Missing executable qualification: expected runner exit 1 with failed JSON
  report and no executable hash, verified at
  `%TEMP%/audiorouter-smoke-missing-240734fe88ba44cc93c912da70f86ae6/result.json`.
- `node --test tools/quality/policy.test.mjs tools/quality/verify.test.mjs`:
  65 passed, including the child-environment regression.
- Docs validation: 196 Markdown files, 1,086 local links. Node syntax,
  runner/environment Prettier, workflow YAML and extracted PowerShell parsing,
  and `git diff --check` passed. `actionlint` is unavailable here.

## Requirement coverage and remaining limits

UI-11 session rename/save/restart/reload and native bridge are locally covered.
NFR-14 usable-canvas readiness is reported; these single fresh-profile and
restart measurements use a warm machine and do not establish OS-cold startup
or a timing regression baseline. Timings remain informational pending Q5.
ENG-02 has the environment regression; ENG-05 records the exact binary/source
and retained reports. Existing browser tests own UI-17 geometry/layout evidence.
This smoke does not by itself establish network-disabled NFR-15, installer,
hardware-audio, Narrator or endurance qualification.
The page-error listener begins after CDP attachment; errors before attachment
are not individually retained by that listener. The guard qualifier now seeds a document-initialization error on reload after
attachment and verifies detection; the earliest pre-attachment errors remain
a limitation of the page listener.

Next: obtain current Windows CI evidence for the reviewed branch. Local
cancellation and injected bridge/readiness failure qualification is recorded
in the linked guard evidence. PR opening still requires the owner's approval.
