# Diagnostics and review build — 2026-10-02

Scope: CAP-01, API-09, SEC-10, UI-13, ENG-05. Windows x64 workstation.
User requested better future failure logs and a current executable to review.
No VST implementation changed; no user session or running app was modified.

## Changes

- Inventory errors carry fixed operation labels through the control API.
  A separate discovery log retains failures that are skipped because an
  endpoint disappeared. Direction/index identify the enumeration step without
  logging device names/IDs. Three disappearance HRESULTs alone are skipped;
  access, service and format errors still fail visibly.
- Shell/backend failure summaries retain typed category, operation, retryability,
  numeric/hex HRESULT and fixed guidance. Raw error messages and arbitrary
  request/error data are excluded. Each record identifies version/build/process.
- Device-list success records show inventory counts. Successful event polls are
  omitted; event errors remain. Discovery uses a same-user named mutex with a
  bounded wait and rotation, so CLI and shell share two files safely.
- Collection instructions are in the
  [privacy guide](../../../operations/privacy-permissions.md#diagnostics-and-support).

## Checks

| Command | Result |
| --- | --- |
| `cargo check -p audiorouter-control --offline` | Passed |
| `cargo check -p audiorouter-transport --locked` | Passed |
| `cargo test -p audiorouter-windows-audio --lib discovery_ --locked` | 2 passed; existing unused_mut warning |
| `cargo test -p audiorouter-control --lib audio_error_response_preserves --locked` | 1 passed |
| `cargo test -p audiorouter-transport --lib --locked` | 25 passed, including native pipe/diagnostic serialization |
| `cargo test --manifest-path src-tauri/Cargo.toml --offline shell_` | 6 passed |
| `npm.cmd run build --prefix ui` | Passed; chunk-size advisory |
| Documentation acceptance and `git diff --check` | Passed before plan cleanup; repeated after cleanup |

The discovery and transport tests were rerun after the final logging mutex and
operation-label changes. Shell tests preceded those shared-library changes;
the final shell and CLI compiled against them. Build commands used
`AUDIOROUTER_BUILD_ID=diagnostics-20261002-review`:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --release --locked --features custom-protocol --target-dir target/diagnostics-20261002
cargo build --release --locked -p audiorouter-cli -p audiorouter-plugin-host --bin audiorouter-cli --bin audiorouter-plugin-worker --target-dir target/diagnostics-20261002
```

Both commands passed. Normal restricted Node builds had EPERM; the authorized
build succeeded outside that restriction. No dependency version was upgraded;
each Cargo lock adds only the windows-audio serde_json dependency.

## Artifacts

Folder: `C:\code\audiorouter\target\diagnostics-20261002\release`.
Version remains 0.0.2; this is an uncommitted review snapshot, not a new tag.
All three pass `Assert-X64PortableExecutable` from the release PE validator.

| Executable | Bytes | SHA-256 |
| --- | --- | --- |
| audiorouter-shell.exe | 23166464 | D6A895DB20DE3575B69BC41811C6435EDA14896BCC8A61EE91195672B3515F8C |
| audiorouter-cli.exe | 13540352 | 8E996B76F39BECF4674F03A4D7CC2E8532B7080B8539DA685F83CBB68876ACC6 |
| audiorouter-plugin-worker.exe | 968192 | 878D86A0AA7CCD14AD61CBB311054C26950EA7DB79866A3571F53B704431F327 |

UI index timestamp: 18:32:48 UTC; shell: 18:39:56 UTC; CLI: 18:40:34 UTC;
worker: 18:37:06 UTC. The optimized JS includes both `Network audio direction`
and `Keyboard graph controls`; shell uses custom-protocol and is newer than UI.
Binary inspection also confirms the exact current JS/CSS asset names and
`diagnostics-20261002-review` build identity inside the shell executable.
Initial PE verification needed a process-scoped PowerShell execution-policy
bypass; rerun passed. An initial rg glob was invalid on Windows; directory
search with `-g 'index-*.js'` passed.

## Limitations and next action

User's existing shell PID 58604 remained running. No second shell was launched,
so the exact build's fresh-install gate is still open. No Joe-machine or live
audio validation is claimed. These three executables are a review folder,
not a prepared installer or published release. SEC-10's full audit retention
gate is not closed by this diagnostics change: three diagnostic streams each
retain a 5 MiB current and previous file; the aggregate audit budget needs
reconciliation during M08 review.

Next: review the app, then run fresh-install qualification with no other shell;
collect Joe's new discovery/shell/backend logs if his input list still fails.
Rollback: use the previous app folder; no storage migration is introduced.

## User-requested repository cleanup

Removed 21 repository-local skills registered from `heygen-com/hyperframes`
and their now-empty `skills-lock.json`. Verified resolved targets stayed in
`C:\code\audiorouter\.agents\skills` with no reparse-point descendants before
native PowerShell removal. Initial sandbox deletion denied protected .agents
access; the explicitly authorized elevated retry removed all folders. Final
checks confirm both `.agents` and `skills-lock.json` are absent. Global skills
and existing video artifacts were outside the requested folder cleanup.

Archived the accumulated 2,179-line plan as superseded execution history,
rebasing evidence links. The replacement is 100 lines with current decisions,
release state, validation matrix, rollback and exact next task. HyperFrames
work is discontinued by user instruction; M08 is not marked complete.
Documentation validation passes: 88 Markdown files, 472 local links.
`git diff --check` passes after removing an extra final blank line.
