# 2026-09-29 — Feedback review, visual groups and local HTTP

Environment: Windows x64, PowerShell, installed VB-Cable endpoints, headed Edge.
Requirements: GRAPH-11, UI-03/16, API-01–12, SEC-01–04/10/12, HTTP-01–06.
Contracts and active-plan decisions were written before implementation. The user
confirmed localhost-only HTTP; no network interface beyond 127.0.0.1 is bound.

## Result and boundaries

Known software feedback returns now appear in graph-plan warnings. Saving needs
explicit acknowledgment; native playback still refuses the return. Endpoint
selection is retained, never silently rebound. Exact endpoint/driver metadata
identifies known VB-Cable pairs; arbitrary external applications and acoustic
feedback cannot be proven from the graph alone.

Tools → Group adds a named rounded rectangle below nodes and wires. Properties
edits color and opacity (default 25%); caption dragging and border resizing work.
Groups are per-session local presentation state, without audio ports/revisions.
They are not yet included in session exports or remotely editable API state.

API tab starts/stops an optional loopback listener (default port 17891), reveals
its activation-scoped bearer token explicitly, and opens bundled local Swagger.
All 100 backend methods use the existing pipe/backend and desktop grant. API
changes refresh clean frontend state; dirty drafts are preserved with a conflict
notice. HTTP is disabled at launch. See the [operator guide](../../../operations/local-http-api.md)
and [contract](../../../spec/16-local-http-api.md).

## Verification

- `cargo test -p audiorouter-control -p audiorouter-domain --locked --target-dir target/sender-routing-20260929`:
  control 201 passed, seven opt-in checks ignored; domain 70 passed.
- Shell tests with `cargo test --manifest-path src-tauri/Cargo.toml --locked --target-dir target/sender-routing-20260929`:
  37 passed, one disposable HTTP fixture ignored. Final HTTP-only rerun:
  five passed, fixture ignored. Cases cover same-backend plan/commit/conflict,
  authorization, host/origin, malformed and oversized requests, chunking rejection,
  bounds, port collision, token revocation, permissions and fragmented packets.
- `npm.cmd test -- --run` in `ui`: 383 passed across 33 files.
  After final persistence fixes, focused CanvasGroups/ApiPanel/SessionFlowCanvas
  rerun: 31 passed; `npm.cmd run typecheck` passed.
- Headed Edge Playwright: six group/API-panel cases passed across dark, light and
  high-contrast themes; one real HTTP integration case passed. The latter forwards
  through an isolated Windows named pipe into a production in-memory backend,
  checks clean refresh and dirty-draft preservation, and authorizes Swagger's
  GET status Try it out (200). Swagger loads 103 operations with no external
  requests. UI panel command styling uses a fake Tauri bridge; packaged shell
  Start/Stop clicks remain an attended check.
- Real PowerShell HTTP POST succeeded; wrong-token POST returned 401, including
  .NET's Expect: 100-continue behavior.
- `node tools/contracts/check-drift.mjs`: passed, 100 methods, 33 node kinds,
  17 processors and 21 event categories.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/docs.ps1`:
  passed, 75 Markdown files and 408 local links. Tracked-source diff check passed.
  Staging also checked newly added files and identified four upstream trailing
  spaces in Swagger's bundled license comments; vendor notices are preserved
  verbatim. `git diff HEAD^ --check -- . ':!src-tauri/http-assets'` passed for
  authored changes.
- Native continuity command:
  `cargo test -p audiorouter-transport --test live_audio_continuity --locked --target-dir target/sender-routing-20260929 live_backend_service_keeps_a_routed_tone_continuous -- --ignored --nocapture`.
  Environment: `AUDIOROUTER_LIVE_CONTINUITY=1`, `AUDIOROUTER_CONTINUITY_PRE_MIXER=1`,
  `AUDIOROUTER_CONTINUITY_DISABLED_MIX_INPUT=1`,
  `AUDIOROUTER_CONTINUITY_CHAIN=parametricEq`, `AUDIOROUTER_CONTINUITY_TONE_HZ=47`,
  `AUDIOROUTER_CONTINUITY_SECONDS=15`, `AUDIOROUTER_CONTINUITY_TOGGLE=tool-0`.
  Passed: four live toggles applied; reference, mixed CABLE-B output and direct
  Recorder branch each had zero glitches, silence runs or WASAPI discontinuity
  flags. Telemetry still counted four startup underruns and one 8.77 ms service
  gap; these did not produce analysed discontinuities. No microphone used.

## Visual evidence and corrected experiments

Reviewed [dark groups](2026-09-29-groups-dark.png),
[light groups](2026-09-29-groups-light.png),
[high-contrast groups](2026-09-29-groups-high-contrast.png),
[dark API](2026-09-29-api-running-dark.png),
[light API](2026-09-29-api-running-light.png),
[high-contrast API](2026-09-29-api-running-high-contrast.png),
[dirty-draft refresh](2026-09-29-http-refresh.png) and
[Swagger](2026-09-29-swagger.png). Swagger screenshot precedes token entry.

Selected groups initially rose above audio nodes due to React Flow selection
elevation; disabled elevation and asserted stacking. Initial resize persistence
wrote speculative state inside a React updater, losing groups on reload; persist
committed state in an effect, preserve valid geometry and measured dimensions,
and test resize/reload/delete. Color fields now use the global full-width style.

Windows accepted sockets inherited nonblocking mode, causing fragmented HTTP
reads to reset; explicitly select blocking mode with bounded timeouts and test
fragmented requests. Error replies drain a bounded pending body before close so
Windows resets do not hide 401 responses. Expect: 100-continue is accepted only
after authentication and request bounds are validated.

An early refresh test incorrectly expected external changes to overwrite a dirty
draft. Existing conflict behavior is intentional; test clean refresh separately
and assert dirty-draft preservation. No production merge behavior was changed.

## Local test artifacts

Built UI first, then shell with `--release --features custom-protocol --locked`
and matching plugin worker, using target directory `target/groups-http-20260929`.
Temporary frontendDist override restored. Shell embeds `index-BjdXm7lV.js` and
`index-BAqoZ1mx.css`; binary timestamp is later than the rebuilt UI index.

- Executable: `C:\code\audiorouter\target\groups-http-20260929\release\audiorouter-shell.exe`
  (22,461,952 bytes, 2026-09-30 04:34:26 UTC).
  SHA-256: `0D194FEAA1A06CA05B3944DF7FDE7E9AFC1C1A41E50B5BC1E355E9C5BA182D22`.
- Adjacent `audiorouter-plugin-worker.exe` (968,192 bytes, 04:31:29 UTC).
  SHA-256: `954D3BB3D99DC511979D15B827E08A3FA0931B7AFADDC3DB06B4607F17E24869`.

Swagger UI 5.33.0 assets, Apache license, notice and bundled dependency notices
are embedded, with a local license link. Distribution/SBOM review remains an M08
gate. No official release, installer or clean-machine qualification is claimed.
User database and existing shell were untouched. Disposable fixtures/browser
servers were stopped and token-bearing test traces removed. Rollback: stop HTTP
and run the previous live-bypass build; revert this slice without schema migration.
Next: attended new-build API controls, cable correction and Discord mic test.
