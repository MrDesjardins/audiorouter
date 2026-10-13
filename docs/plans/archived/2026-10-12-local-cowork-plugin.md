# Local Cowork plugin implementation — 2026-10-12

Status: local adapter/export implementation complete. Actual Cowork account
import and tool execution remain an attended gate in the
[active plan](../active/current.md); this does not mark the whole integration
or M07 complete.

## Scope and decision

Owner requested Cowork compatibility and selected **Local**. Requirements
AUTO-06/10/13, SEC-02/04/09 and UI-17. Preserve existing backend authorization,
audio, sessions and driver state. No public transport or cloud hosting.

Anthropic's [plugin guide](https://support.claude.com/en/articles/13837440-use-plugins-in-claude)
and [architecture overview](https://support.claude.com/en/articles/14479288-claude-cowork-architecture-overview)
distinguish supported local Cowork deployments from new cloud tasks.
Local plugin MCP runs on the host; Desktop chat configuration and cloud remote
connectors are different mechanisms. This corrects prior blanket statements
that all Cowork needs remote MCP. Manifest format checked against the
[official reference](https://code.claude.com/docs/en/plugins-reference).

## Implementation

- MCP tab exports a local Cowork plugin ZIP to Downloads after explicit action.
  A separate `audiorouter-cowork-local` client starts as Observer; existing
  grants are preserved. Re-exporting a revoked client explicitly re-enrolls
  it as Observer. Editor requires the owner to choose it in Client permissions.
- Native export preflights the actual installed CLI using modern discovery,
  legacy initialization, tool listing and a backend read with that identity.
  The same bounded job-controlled probe continues to serve ChatGPT unchanged.
- ZIP has exactly `.claude-plugin/plugin.json` and `README.md`; the manifest
  uses structured command/arguments with the exact CLI/database/pipe paths.
  No secrets, binaries, database contents or audio. Paths are personal
  configuration; the guide advises against publishing the ZIP.
- Export rejects redirected output ancestors and uses a unique create-new
  filename. Failed writes remove the partial ZIP. Blocking file/process work
  runs off the UI thread. Buttons require native availability and a connected
  backend; fixed action/feedback slots prevent layout movement.
- Corrected automation contract, future remote plan, website and added the
  [Cowork runbook](../../operations/cowork-mcp.md).

## Verification

Windows x64 host, pinned Rust 1.96.0, installed Claude Code 2.1.296, Edge
browser harness. No audio endpoints or driver settings changed.

- `cargo test --manifest-path src-tauri/Cargo.toml`: final 80 passed, 4 ignored;
  both new opt-in plugin fixtures are separately verified below.
  Restricted-sandbox run failed 17 socket/storage/pipe checks;
  outside-sandbox rerun passed. This was not a source repair.
- `cargo test -p audiorouter-cli --test mcp_stdio local_cowork -- --nocapture`:
  both tests pass, actual CLI legacy/modern MCP and Observer denied writes,
  plus authenticated Windows pipe backend.
- With `AUDIOROUTER_COWORK_CLI` pointing at the packaged release CLI,
  `cargo test --manifest-path src-tauri/Cargo.toml export_preflights_real_cli_and_saves_exact_plugin -- --ignored --nocapture`
  passes. Disposable database/pipe; actual preflight and saved ZIP bytes match
  the production archive builder. First attempt reached all export assertions
  but failed fixture cleanup on SQLite WAL/SHM; explicit companion cleanup
  fixed the fixture, then rerun passed.
- Generated production-builder artifact via opt-in
  `write_plugin_validation_fixture`, then
  `claude plugin validate src-tauri/target/cowork-validation/plugin --strict`:
  validation passed. Informational marketplace advice is inapplicable to this
  personal ZIP; no marketplace is configured. Generated files are ignored
  under `src-tauri/target/`, not user-install artifacts.
- UI build/typecheck pass; targeted App lifecycle and host unit checks:
  34 passed. UI format/check clean; lint 0 errors, same 25 existing warnings.
- Four targeted Edge tests pass: workspace availability/disconnected state,
  new Observer enrollment, Editor preservation, revoked-client enrollment,
  progress/success/error recovery and stable height. First browser attempt
  tried to click the correctly disabled button on a disconnected fixture;
  connected route fixture added for the export checks.
- Dark, light and high-contrast screenshots visually reviewed:
  [dark](../../images/cowork-mcp-dark.png),
  [light](../../images/cowork-mcp-light.png),
  [high contrast](../../images/cowork-mcp-high-contrast.png).
- Rust fmt/check and both workspace/shell all-target/all-feature Clippy with
  `-D warnings` pass. Cargo reports filesystem hard-link cache warnings.
- Documentation acceptance passes: 162 Markdown files, 909 local links.
- Custom-protocol release build passes, current UI string confirmed embedded
  in the asset bundle, executable newer than `ui/dist/index.html`.

## Artifact and rollback

Updated app staged at
`C:\code\audiorouter\src-tauri\target\release\audiorouter-shell.exe`.
SHA-256: `F3607AD2D208FBA28EE7A34427C1FE31213B70186DBD43BA618F871E263516EC`.
Previous binary retained as
`C:\code\audiorouter\src-tauri\target\release\audiorouter-shell-before-cowork-20261012.exe`.
No app process was stopped. Existing packaged CLI is reused and its real
preflight passed. This is a local staged build, not a published installer.

Rollback: uninstall the Cowork plugin, revoke its local client, restore the
previous executable while the app is closed, and revert adapter/UI/contract
changes. No database migration. Export again after installation path changes.

## Remaining gate and exact next task

No running Claude Desktop/local Cowork account was available for an attended
check. Import the ZIP exported by the actual app in local Cowork, observe a
session-list read in MCP activity, then owner-approved stopped-session
creation, and confirm denial after revocation. Record actual deployment,
plugin/account version and matching activity; CLI validation does not satisfy
this gate. Cloud Cowork remains separate future remote transport work.
