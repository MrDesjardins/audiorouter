# Generic MCP UI flow correction — 2026-10-11

## Objective and scope

Clarify the MCP setup screen so it distinguishes the standard local MCP server,
ChatGPT's optional OpenAI Secure MCP Tunnel, and the separate remote connection
Cowork requires. Hide actions that cannot work in the current state. Requirements
AUTO-13, AUTO-16, AUTO-17, SEC-14, UI-17; user decision DEC-21.

## Decisions

- Local stdio remains the vendor-neutral MCP transport for clients that can
  launch a local command.
- OpenAI Secure MCP Tunnel is an optional OpenAI transport for ChatGPT, not an
  MCP protocol/server and not Cowork support.
- Cowork is a cloud connector and cannot invoke the local stdio command. A
  generic remote Streamable HTTP server plus authenticated reachability is
  required; this work does not expose a public endpoint.
- Keep current-user grants, enrollment and backend authorization. Do not use
  the REST API, unencrypted LAN listener, anonymous access, or a static shared
  bearer secret as a shortcut.

## Implementation

- Reframed Setup → MCP around standard MCP, with separate ChatGPT and Cowork
  cards and local stdio instructions.
- ChatGPT connector settings appear only after the tunnel reports connected.
  Connect/Retry, Disconnect, setup, Update and Forget match configured state;
  initial status checking has no premature actions.
- Local configuration copy actions are hidden if the CLI is missing. A saved
  tunnel ID is usable without retyping it.
- MCP tab selection resets the sidebar to its heading. Reserved action slots
  keep the panel size stable while connection status changes.
- Added a deterministic Playwright fixture for disconnected/connected and
  missing-CLI states, with dark/light/high-contrast layout screenshots.
- Recorded the standard remote MCP requirement and security/hosting gates in
  AUTO-17, SEC-14, DEC-21, and
  [the generic remote MCP future plan](../future/generic-remote-mcp.md).

## Verification and evidence

Environment: Windows, `C:\code\audiorouter`, 2026-10-11.

- `npm --prefix ui run format` and `npm --prefix ui run format:check`: passed.
- `npm --prefix ui run build`: passed (TypeScript + production Vite build).
- `npm --prefix ui run lint`: passed with 25 existing warnings, no errors.
- Focused Edge/Playwright test for the desktop workspace MCP flow: 1 passed.
  The tunnel and CLI states are deterministic fixtures; this is UI evidence,
  not ChatGPT account or live tunnel evidence. Dark, light, high-contrast and
  connected screenshots were visually reviewed before cleaning test output.
- UI unit suite: 56 files and 521 tests passed; one unrelated existing
  `idempotency.test.ts` assertion failed because it expects the older
  `ui-session-create-*` format while current code returns `ui-<uuid>`.
- Docs acceptance: 157 Markdown files and 880 local links passed.
- `git diff --check`: passed.

See the [detailed evidence](../active/evidence/2026-10-11-generic-mcp-ui-flow.md).

## Limitations and rollback

ChatGPT account/workspace interoperability remains pending in the separate
active OpenAI-tunnel plan. Cowork remains unavailable until remote MCP and
secure reachability are designed and implemented. The unit assertion above is
unrelated and remains separate. Rollback the Workbench, CSS, browser test and
this execution record together; keep the vendor-neutral specification and
future plan because they capture the user's transport decision.

## Handoff

The immediate UI correction is complete. The next engineering work is the
generic remote MCP architecture and service decision in the future plan. No
remote listener or hosted relay has been deployed.
