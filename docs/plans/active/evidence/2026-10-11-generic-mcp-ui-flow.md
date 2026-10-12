# MCP UI flow and client boundary — 2026-10-11

## Scope

Clarify standard local MCP versus the optional OpenAI Secure MCP Tunnel, gate
actions by actual connection state, and describe Cowork's current limitation.
Requirements: AUTO-13, AUTO-16, AUTO-17, SEC-14, UI-17; DEC-21.

## Changes

- The MCP panel now leads with standard MCP and separates local stdio clients,
  ChatGPT's OpenAI-specific tunnel, and Cowork's not-yet-available remote path.
- ChatGPT connector settings appear only when the local tunnel reports
  connected. Connect/Retry and Disconnect follow current state; tunnel setup
  actions require a known, available runtime. Initial status checking shows no
  setup action. Local command copy actions require an installed CLI.
- A saved tunnel ID is accepted by the save action even when the owner leaves
  the prefilled field untouched.
- AUTO-17 and SEC-14 now require authenticated generic remote MCP and a
  reviewed relay/hosting design before cloud access is implemented. The
  existing REST listener and LAN mode are explicitly not substitutes.
- [Generic remote MCP future plan](../../future/generic-remote-mcp.md) captures
  protocol, identity/auth, relay/hosting, revocation, privacy and operations
  gates.

## Verification

- Environment: Windows, repository checkout `C:\code\audiorouter`, 2026-10-11.
- `npm --prefix ui run format`: passed; no unrelated UI formatting changes.
- `npm --prefix ui run build`: passed; TypeScript and production Vite build.
- `npm --prefix ui run lint`: passed with zero errors and the existing 25
  exhaustive-deps warnings.
- `powershell -NoProfile -ExecutionPolicy Bypass -File
  .\tests\acceptance\docs.ps1`: passed (157 Markdown files, 880 local links).
- `git diff --check`: passed.
- `npm --prefix ui test` first ran with the sandbox's ephemeral temp path and
  27 suites failed to import due `ENOENT`. Rerun with `TEMP`, `TMP` and
  `TMPDIR` inside the workspace: 56 files passed; 521 tests passed; one
  unrelated existing `src/idempotency.test.ts` assertion failed because it
  expects the old `ui-session-create-*` key format while the current function
  returns `ui-<uuid>`.
- The initial unprivileged Edge/Playwright run was blocked because the managed
  sandbox refused the local loopback socket. The final browser run passed with
  the local test harness. Its deterministic
  fixture verified disconnected/connected action visibility, local CLI setup
  and missing-CLI behavior, return-to-top navigation, stable card bounds during
  status/theme changes, and screenshots in dark, light and high-contrast themes.
  I visually reviewed the three theme screenshots and the connected-state
  screenshot. The fixture fakes tunnel state and is UI evidence only; it does
  not prove ChatGPT account interoperability or a real tunnel connection.
  Exact browser command: `playwright.cmd test --config=playwright.config.ts
  e2e/audio-tools.pw.ts --grep "desktop workspace keeps task-focused tabs,
  MCP setup, and diagnostics reachable" --timeout=90000 --reporter=line`;
  result: 1 passed.

## Remaining gates and rollback

The one failed unit assertion predates this change: it expects the old
`ui-session-create-*` key format while current `uiIdempotencyKey` returns
`ui-<uuid>`. No implementation file for idempotency was changed. Keep its
repair separate. ChatGPT account/tunnel interoperability and Cowork remote
connectivity remain open under their own plans. Roll back the UI/test changes
together; retain the vendor-neutral transport decision and future plan.
