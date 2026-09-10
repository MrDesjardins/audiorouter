# AudioRouter native shell

This is the Tauri 2 desktop shell for the existing AudioRouter UI. It is a
standalone Cargo workspace so portable workspace builds do not acquire desktop
runtime dependencies.

The `rpc_request` command uses the existing authenticated Windows named-pipe
transport. Set `AUDIOROUTER_CONTROL_PIPE` only when connecting to a deliberately
started AudioRouter control service; the default is
`\\\\.\\pipe\\audiorouter-control`. The shell does not install a driver,
register plugins, change Windows audio endpoints, or start an unconfigured
service.

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
