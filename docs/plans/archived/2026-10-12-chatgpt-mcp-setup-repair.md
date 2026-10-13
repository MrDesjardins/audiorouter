# ChatGPT MCP setup repair — completed 2026-10-12

## Objective and requirements

Repair the initialization/tool discovery failure for the owner's existing
Secure MCP Tunnel. Requirements AUTO-06, AUTO-16, SEC-14, UI-17. Scope is
local MCP compatibility, runtime lifecycle, deployment and ChatGPT plugin
setup; full remote tool permissions/lifecycle qualification remains active.

## Reproduction, decisions and implementation

ChatGPT rejected setup despite AudioRouter showing Tunnel ready. The runtime
was launching a stale CLI sidecar; another poller survived a prior shell exit.
A process regression also showed that valid legacy request metadata was
mistaken for modern negotiation and rejected during initialization.

Preserve standard local stdio, both MCP protocol eras, scoped client enrollment
and protected runtime credentials. The OpenAI tunnel remains an optional
transport. No new tunnel/key or grant is created by this repair.

1. Implement dual-era protocol compatibility and process interoperability
   coverage, preserving backend authorization.
2. Inspect the actual host runtime, stop the verified orphan, stage the matching
   CLI beside the shell and verify a read against the existing backend.
3. Correct legacy metadata detection; add a bounded protocol/backend preflight
   before tunnel startup and actionable safe errors for stale CLI/backend failure.
4. Use a suspended process and kill-on-close Windows job to contain runtime
   descendants even when the shell exits without destructors.
5. Add CLI staging/hash verification independent of driver packaging; rebuild
   and stage the shell/CLI at the owner's existing launch path, preserving backups
   and the running audio session.

## Evidence and result

- [Initial protocol repair](../active/evidence/2026-10-11-mcp-protocol-compatibility.md).
- [Live repair, exact commands, hashes and limitations](../active/evidence/2026-10-12-chatgpt-tunnel-repair.md).
- Focused MCP process interop passed; CLI unit tests: 40 passed. Supervisor
  tests: 5 passed, 1 guarded probe ignored in the ordinary run. Host probes
  separately verified the final CLI and rejected both stale versions.
- Real backend read, runtime polling, native job descendant cleanup, Rust
  formatting, workspace/Tauri Clippy, production UI/CLI/shell builds and
  documentation acceptance passed. UI source in this repair was unchanged;
  the prior three-theme-reviewed flow is embedded.
- On 2026-10-12 the user reported successful ChatGPT connection and supplied
  [the created plugin](https://chatgpt.com/plugins/plugin_asdk_app_6acd77c0830881919e8b719807f198a8).
  This closes the setup failure using user-confirmed account evidence. The
  agent did not inspect the authenticated plugin page or perform a remote tool
  call: Firefox automation was policy-blocked and the in-app browser logged out.
- Archival documentation acceptance passed: 159 Markdown files, 885 local
  links; `git diff --check` passed. No new code or audio checks were needed
  for this user-confirmed status/documentation update.

## Remaining integration gates and rollback

Follow-up 2026-10-12: the user confirmed actual tool use and supplied the
ChatGPT request/response plus resulting AudioRouter session screenshots, then
AudioRouter connection and numbered ChatGPT plugin setup screenshots. All four
were reviewed and added unaltered to the [worked example](../../operations/chatgpt-mcp-example.md).
They document session listing and permitted creation of **PD200X Radio Voice**,
with PD200X → Advanced EQ → CABLE-A and Scarlett headphones, left stopped.
The evidence records source/copy hashes and the remaining qualification boundary.

No AudioRouter tools are exposed to this Codex chat. Remaining integration
qualification is denied writes, Disconnect/reconnect, credential revocation
and privacy checks. Session creation requires the
owner's Editor grant; plugin creation alone is not evidence of that authority.
Keep these tasks in [the active integration plan](../active/current.md#chatgpt-mcp-integration-plan-2026-10-11-active).

Rollback: disconnect the tunnel, restore the saved CLI/shell backups and revert
the paired compatibility/supervisor/staging changes. Preserve credentials,
grants and session/database contents. No driver or audio configuration was changed.
