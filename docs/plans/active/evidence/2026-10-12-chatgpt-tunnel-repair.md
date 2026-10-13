# Live ChatGPT tunnel repair — 2026-10-12

Requirements: AUTO-06, AUTO-16, SEC-14, UI-17. Windows x64, pinned Rust
1.96.0, user's existing tunnel `tunnel_6acc5b1bf6c8819195497c541c696b80`.
No new tunnel, key or grant was created. The existing audio shell/session
continued running throughout the repair.

## Findings and mitigation

Host inspection found two runtimes for this tunnel: PID 52228 had an exited
shell parent (34912); PID 52996 belonged to the current shell (54772). Each
had its own CLI child. The older poller had received three requests; a ready
new poller alone therefore did not establish which CLI handled ChatGPT setup.
Readiness was HTTP 200 while neither runtime verified modern MCP discovery.

Both launched `src-tauri/target/release/audiorouter-cli.exe` with SHA-256
`DF977C7C48AD5104CAE0EEDBCD0C55E68F5EC98460A9550337C96D38B04743CD`.
The protocol compatibility fix was built in `target/release/audiorouter-cli.exe`
with SHA-256
`09F78979E2EF8F875751124E50525DA02DF18BFF25E47384EC48D19DA9CEB643`.
Rebuilding the shell had not updated its separately staged sidecar.

Disconnected the current tunnel with the app's MCP control, stopped only the
verified orphan runtime and its stdio child, preserved the stale sidecar as
`src-tauri/target/release/audiorouter-cli-before-mcp-20261012.exe`, copied the
updated CLI beside the shell and verified its hash, then reconnected through
the app. Final host inspection found one runtime (50832), its MCP child (55996)
and the original audio shell (54772). `/readyz` returned `ready`; metrics
confirmed successful HTTP 200 tunnel metadata and HTTP 204 idle polling.

The direct live probe exercises the exact sidecar, `audiorouter-chatgpt`
identity, current database and real control pipe. It verifies modern
`server/discover`, legacy 2025-11-25 `initialize`, `tools/list` and a read-only
`describe_capabilities` tool call. It passed; the same probe rejected the
preserved stale sidecar. These establish a repaired local MCP/backend path,
not ChatGPT plugin acceptance or a tool call traversing OpenAI's tunnel.

A strengthened process regression then reproduced a second compatibility
defect: legacy `initialize` with valid `_meta.progressToken` returned no valid
initialization result. The adapter treated any `_meta` as modern negotiation.
Detection now uses discovery or explicit modern protocol/capability/info keys;
legacy progress tokens and extension metadata remain legacy. Initialization
and tools/list regressions failed before the fix and passed afterward. The
startup probe now includes legacy progress metadata. The final release CLI
passed this stronger probe against the live backend; both the original CLI and
the intermediate compatibility-only CLI were correctly rejected.

The earlier sandbox-only process inspection missed the host's running app.
`LOCALAPPDATA` also refers to a sandbox package directory in that context.
Host process inspection and live pipe/loopback checks were rerun in the approved
host execution context. The earlier absence claim is superseded by this evidence.

## Prevention

- A bounded, read-only MCP preflight now runs before starting the tunnel. It
  tests the actual staged CLI and backend, never receiving the OpenAI key.
  Protocol, enrollment/backend and timeout failures have fixed actionable
  messages; raw server errors, arguments, credentials and paths are not echoed.
- Legacy request metadata is accepted without weakening modern protocol
  validation or enrolled-client authorization.
- Runtime creation is suspended until assignment to a non-inheritable Windows
  job with `KILL_ON_JOB_CLOSE`. The shell owns the only job handle. Disconnect
  stops and reaps the runtime and descendants; Windows closes the job on shell
  exit even when Rust destructors do not run. Spawn/assignment/resume failures
  also reap the suspended child.
- `tools/stage-mcp-cli.ps1` builds/stages a matching CLI, compares hashes,
  preserves a changed sidecar backup, and reports in-use replacement failures.
  It is independent of the driver/NSIS packaging gate. The development launcher
  and quickstart now include this step.

## Checks and artifact

- `cargo test --locked --manifest-path src-tauri/Cargo.toml
  mcp_tunnel::tests:: -- --nocapture`: 5 passed, 1 guarded live probe ignored.
  Includes a subprocess that explicitly exits without destructors; both its
  runtime and descendant stop. Fixed-message regression checks redaction.
- Guarded live test, explicitly invoked on the host for both sidecars:
  `src-tauri/target/debug/deps/audiorouter_shell-2734075c2cf592ed.exe --exact
  mcp_tunnel::tests::live_mcp_sidecar_preflight --ignored --nocapture`.
  Environment: `AUDIOROUTER_MCP_SMOKE_CLI` selects the staged CLI or preserved
  backup, `AUDIOROUTER_MCP_SMOKE_DATABASE` selects the existing app database,
  `AUDIOROUTER_MCP_SMOKE_PIPE` selects `\\.\pipe\audiorouter-control`, and
  `AUDIOROUTER_MCP_SMOKE_EXPECT_STALE=1` only for a backup. Final cases passed:
  current CLI succeeds, original stale CLI is rejected, and the intermediate
  compatibility-only CLI is rejected for the metadata defect.
- `cargo test --locked -p audiorouter-cli --test mcp_stdio
  mcp_stdio_client_interoperates_with_cli_process -- --exact`: 1 passed.
- `cargo test --locked -p audiorouter-cli --lib`: 40 passed.
- Workspace and Tauri Clippy (`--all-targets --all-features -- -D warnings`):
  passed. Cargo emitted sandbox cache hard-link warnings, not code findings.
- Both Rust format commands and their `-- --check` runs: passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File
  .\tools\stage-mcp-cli.ps1 -Profile release`: passed, hashes match. Final
  staged CLI SHA-256:
  `AA5A86CDF99833F7EB1A2D8F85BD0A970D01AE3AAEBA26E761FCC699C23DAE70`.
  The intermediate mapped CLI was preserved as
  `audiorouter-cli-before-metadata-20261012.exe` without stopping audio.
- `npm.cmd --prefix ui run build`: passed (TypeScript + Vite). UI source was
  unchanged in this repair; this embeds the prior three-theme-verified MCP UI
  correction. Bundle `index-CVWdb5Ea.js` was found in the rebuilt executable.
- `cargo rustc --release --locked --features custom-protocol --manifest-path
  src-tauri/Cargo.toml --bin audiorouter-shell -- -o
  C:\code\audiorouter\src-tauri\target\release\audiorouter-shell-mcp-fixed.exe`:
  passed. An alternate output permitted building while the original audio shell
  ran. Rustc's two output-name/out-dir adaptation warnings were expected.
- The rebuilt shell was staged at the user's existing launch path,
  `src-tauri/target/release/audiorouter-shell.exe`, retaining the previous mapped
  executable as `audiorouter-shell-before-mcp-20261012.exe`. Hashes matched:
  `A7E48C9E90815AC974E11CB4A231F611333AC26F30D9E20AD511E1A39CC7C5CB`.
  The running shell was not stopped; it loads these prevention changes on its
  next launch. The already running MCP child retains the intermediate loaded
  CLI until Disconnect/reconnect; the final metadata fix is staged for the next
  MCP process. No second shell was launched.
- Documentation acceptance (158 Markdown files, 882 local links) and
  `git diff --check`: passed.

## Open gate and rollback

Update 2026-10-12, successful tool use: the user reported that the connected
plugin worked and supplied `Mcp_Result.png` and `Mcp_Result2.png` from Downloads
for documentation. Both images were visually reviewed. The first shows a
session-list result and ChatGPT reporting creation/opening of **PD200X Radio
Voice** with PD200X → Advanced EQ → CABLE-A plus Scarlett headphones; the second
shows the same named session and graph in AudioRouter, with playback stopped.
This is user-supplied evidence of a real read and permitted session/graph write
through ChatGPT, not an agent-performed MCP call or an audio-quality check.

The [worked example](../../../operations/chatgpt-mcp-example.md) embeds original,
unaltered copies in `docs/images/`. SHA-256 comparison confirmed the copies
match their Downloads sources:

- `chatgpt-mcp-request-response.png`:
  `4F6244A6C49FEFF9129DCEAAB7E50DBD9EAB69BA2C70CE06A62808DF4E5EA30F`.
- `chatgpt-mcp-radio-voice-session.png`:
  `00F39F2664FE8BECC6FCA647ACCC72687B93F20E6D8D7895C5488FBA55D52823`.

The owner also supplied `Mcp_AudioRouter_Setup.png` and `Mcp_ChatGPTPlugin.png`.
Both were reviewed and copied unaltered for the example's initial setup steps.
The first shows Tunnel ready in AudioRouter; the second shows the numbered
ChatGPT plugin form with Tunnel and No authentication selected, before creation.
Their source/copy SHA-256 hashes match:

- `chatgpt-mcp-audiorouter-setup.png`:
  `1A06D9C180DB88571B8292DA964F84FE5A2F9554803E762827D5C9753E0098CC`.
- `chatgpt-mcp-plugin-setup.png`:
  `FD8637D904A8CEB5D9B040007B3BE13D8B165B86718241A288511EE7755D5B15`.

Remaining remote qualification is denied-scope behavior, reconnect/revocation
and credential/privacy checks. The default read-only permission and requirement
for owner-authorized Editor access are retained in the example.
Documentation acceptance for this example passed: 160 Markdown files and 896
local links; `git diff --check` passed. All four screenshots were visually
reviewed and their source/copy hashes matched. Only documentation/assets
changed, so no new code, audio or UI tests were run for this update.

Update 2026-10-12: the user confirmed successful connection and supplied
[the created ChatGPT plugin](https://chatgpt.com/plugins/plugin_asdk_app_6acd77c0830881919e8b719807f198a8).
Plugin setup is now user-confirmed. The earlier unverified statement below
records the boundary at the end of the repair run. No AudioRouter tools are
exposed to this Codex chat. At that initial setup confirmation, remote calls
were still unqualified; the subsequent screenshots above provide the successful
read/write example. The
completed setup execution is [archived](../../archived/2026-10-12-chatgpt-mcp-setup-repair.md).

At the end of the repair run, ChatGPT-side plugin creation remained unverified.
The in-app browser was logged out. Native Firefox verification was blocked by the computer-use runtime's
browser URL policy; no cookies, account credentials or browser safety checks
were bypassed. The owner can retry the existing tunnel with **No authentication**;
no OAuth server is configured in AudioRouter's local stdio transport. Official
reference: [OpenAI Secure MCP Tunnel setup and troubleshooting](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels).

Rollback: disconnect the tunnel and restore the saved CLI/shell backups, then
revert these supervisor/tooling/spec changes together. Do not revert grants,
database contents or tunnel credentials. First Disconnect the current tunnel,
quit/reopen the staged app and Connect to load all fixes and avoid retaining an
older poller. Following the user-confirmed setup, the next gate is a read-only
MCP call and denied writes under Observer through
the same tunnel. Session creation remains subject to an owner-granted Editor role.
