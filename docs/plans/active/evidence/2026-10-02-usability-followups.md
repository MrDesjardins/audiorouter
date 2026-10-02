# Usability follow-ups and external app API review — 2026-10-02

Environment: Windows workstation, PowerShell, installed Rust/Node/Edge.
Requirements: UI-03/04/11/12, GRAPH-05/06/08/16, CAP-01, DIST-01,
AUTO-15, API-01/07/09/12 and HTTP-01/04/05. No new driver/credentials needed.
Existing release/video working-tree changes preserved. Running desktop PID
58604 was left alone. No desktop launch, device preparation or private audio.

## Changes and decisions

- Network Properties has a static sending/receiving PC diagram with audio
  direction, current IP and matching UDP port. No simulated signal activity.
- README begins with the published unsigned 0.0.2 installer, retains source
  setup, removes obsolete developer-variable enrollment instructions, and has
  three explicit video placeholders awaiting supplied links.
- Canvas List view switch removed. Advanced → Keyboard graph controls retains
  accessible node and topology actions. Existing focused tests now use it.
- Arrange and drag/drop already existed; no rewrite was needed. Production
  tests verify graph-order positions, Undo arrange, exact preview/drop placement
  and unchanged viewport in all three themes.
- Item 7 means external app integrations. A preliminary VST UI interpretation
  was withdrawn at the user's correction; component, controls and tests removed.
  No VST hosting/worker or VST UI control changes remain. Existing REST supports
  active/pinned session and named/ID nodes.set values/flags. Source review and
  nine control tests confirm current control contracts; they do not establish
  native menu/match sound. Guide includes Volume/Mixer and Duck examples.
  Request builder, node-reference discovery and multi-node/temporary-state
  decisions are proposed in the future plan, not implemented.

## Joe's empty input list

Supplied shell.jsonl contains nine failed devices.list calls, all code -32000,
HRESULT 3758096907 = 0xE000020B, previously categorized other. This is Windows
ERROR_NO_SUCH_DEVINST. The log contains no endpoint or failed-operation name;
an exact device/driver cause cannot be concluded. Source discovery propagates
per-endpoint failures and reads display properties for inactive endpoints too.
One stale/disappeared device can therefore abort discovery for healthy devices.

Discovery now isolates reads per endpoint and skips only ERROR_NO_SUCH_DEVINST,
its HRESULT form 0x800F020B, and AUDCLNT_E_DEVICE_INVALIDATED. Other errors still
propagate. Missing-device codes retain exact HRESULTs and become retryable
deviceInvalidated errors outside discovery. No selected input is substituted.
Regression injects healthy/missing/healthy entries, verifies retained endpoints,
and checks access-denied, service, format and unknown failures remain errors.
Local real endpoint enumeration passes; Joe's exact hardware remains untested.

## Commands and outcomes

| Command | Result / evidence |
| --- | --- |
| `npm.cmd run typecheck --prefix ui` | Pass |
| `npx.cmd vitest run --configLoader runner src/NetworkNodeEditor.test.tsx src/smartLayout.test.ts src/SessionFlowCanvas.test.ts src/App.accessibility.test.tsx` (ui) | 147/147 pass after final VST rollback |
| `cargo check -p audiorouter-windows-audio --locked` | Pass |
| `cargo test -p audiorouter-windows-audio --lib --locked` | 106/106 pass; existing unused_mut warning |
| `cargo test -p audiorouter-control --lib simple::tests --locked` | 9/9 pass |
| `npx.cmd vite build --config vite.canvas-production.config.ts --configLoader runner --outDir ../target/usability-20261002-ui` (ui, escalated) | Pass; optimized route harness assets in isolated target directory |
| `npx.cmd playwright test --config=playwright.usability.config.ts --workers=1` (ui, escalated) | 9/9 pass, exit 0; all three themes |
| `powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\docs.ps1` | Pass, 86 Markdown files / 457 links after evidence update |
| `git diff --check` | Pass |

Initial UI test selector matched both a field and library button; corrected to
exact label matching. Restricted Vite builds failed with EPERM writing existing
and new output; an isolated build under automatic escalation succeeded.
Restricted Playwright reported successful cases but stalled cleaning its own
server; stopped the task's test process and reran with escalation to get exit 0.
An initial command used ui/ui as the npm prefix and was corrected. No installer
or shell artifact was produced. Build warnings about bundle size remain.

Screenshots from the final optimized run are in
`%TEMP%/audiorouter-playwright-results/usability-followups.pw.ts--*/` (six network
captures). Dark/light/high-contrast diagrams were visually reviewed at
1280×720; form fields use the global style, text and IP/port remain readable,
diagram stays inside the sidebar. Arrange/drop screenshots also come from the
final suite's smart-layout and drag-place folders. No jsdom styling claim.

Next task: rebuild for Joe's device-list confirmation; review the proposed
request builder. Native live external-event audio, Narrator, 200% scaling and
release gates remain separate. Rollback only this UI/docs/inventory slice;
saved graph schema and VST processing are unchanged.
