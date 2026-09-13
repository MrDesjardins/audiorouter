# AudioRouter native shell

This is the Tauri 2 desktop shell for the existing AudioRouter UI. It starts a
per-user control backend on its default launch path, so a fresh
shell has a real connected status surface without opening an audio endpoint.
It is a standalone Cargo workspace so portable workspace builds do not acquire
desktop runtime dependencies.

The `rpc_request` command uses the existing authenticated Windows named-pipe
transport. Set `AUDIOROUTER_CONTROL_PIPE` only when connecting to a deliberately
started AudioRouter control service; the default is
`\\\\.\\pipe\\audiorouter-control`. The default database is
`%LOCALAPPDATA%\\AudioRouter\\state.sqlite`; `AUDIOROUTER_DATABASE` can provide
an absolute test path. First launch enrolls only the current Windows user as
an operator for graph/session control; device administration remains explicit.
The shell does not install a driver,
register plugins, change Windows audio endpoints, or start an unconfigured
service.

For an explicitly authorized native adapter test, set both
`AUDIOROUTER_CAPTURE_ENDPOINT_ID` and `AUDIOROUTER_RENDER_ENDPOINT_ID` to the
exact IDs returned by `devices.list`. The shell then prepares matching stopped
WASAPI clients; pressing **Start session** is still required to start them.
If either ID is missing, stale, direction-mismatched, or format-incompatible,
the worker is rejected and the control backend remains available without
opening a substitute endpoint. Clear both variables to return to the normal
control-only launch path.

Compile without launching or packaging:

```text
cargo check --manifest-path src-tauri/Cargo.toml
```

## Interactive control-plane check

The shell/backend boundary can be checked without opening an audio endpoint by
using a disposable database and pipe. Run from an elevated PowerShell session
after building the CLI and shell. The backend grant is bound to the current
Windows user's SID:

```powershell
$database = Join-Path $env:TEMP "audiorouter-shell-check.sqlite"
$pipe = "\\.\pipe\audiorouter-shell-check"
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$params = Join-Path $env:TEMP "audiorouter-shell-check.json"
Set-Content -LiteralPath $params -NoNewline -Value ('{"clientId":"' + $sid + '","role":"observer","idempotencyKey":"shell-check-enroll"}')
.\target\debug\audiorouter-cli.exe api call clients.authorize $params --database $database --json
Start-Process .\target\debug\audiorouter-cli.exe -ArgumentList @("backend", "serve", "--database", $database, "--pipe", $pipe, "--connections", "1")
$env:AUDIOROUTER_CONTROL_PIPE = $pipe
.\src-tauri\target\debug\audiorouter-shell.exe
Remove-Item Env:AUDIOROUTER_CONTROL_PIPE -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $params, $database -Force -ErrorAction SilentlyContinue
```

The expected observation is that the shell opens and its normal UI reports a
connected backend after a read-only refresh. This does not prove driver or
audio routing behavior. Do not use the user's normal database for diagnosis.

The tray's **Close window** action hides the editor while leaving the backend
and session running. **Quit and stop audio** sends an authenticated
`session.stop` for the stable desktop session and exits only after the backend
confirms `stopped`; a refused or failed stop leaves the shell running and
reports the refusal in the tray status item.

The tray's **Toggle privacy mute** action reads the authoritative backend
privacy state before changing it. It uses the same scoped API as the editor;
it does not alter Windows microphone permissions, endpoint mute, or master
volume.

For a repeatable control-plane check without manual UI observation, build the
debug CLI and shell, then run `tests/acceptance/m07-shell-rpc.ps1` from an
elevated PowerShell session. It opts into a temporary frontend initialization
probe through `AUDIOROUTER_SHELL_PROBE_FILE`; the marker is written only after
the authenticated native command receives `system.describe`, and the script
removes its database, marker, processes, and pipe-related state afterward.
