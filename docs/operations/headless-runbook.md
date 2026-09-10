# AudioRouter headless runbook

This runbook describes the currently verified portable control workflow. It does not claim that native audio routing, virtual devices, driver installation, or production signing are available.

## Inspect without side effects

```powershell
audiorouter status --json
audiorouter schema --json
audiorouter session list --database C:\path\to\audiorouter.sqlite --json
```

Use absolute paths for databases and JSON files.

## Run the bounded native backend

On Windows, the external backend can expose the same authenticated control
pipe used by the desktop shell:

```powershell
audiorouter backend serve `
  --database C:\path\to\audiorouter.sqlite `
  --pipe \\\.\pipe\audiorouter-control `
  --connections 256
```

The current Windows user must already be enrolled in the database. The server
accepts at most 500 connections and then exits, making the lifecycle explicit
for development and acceptance tests. It uses the existing owner-only pipe
ACL, same-user peer validation, and stored client grant; it does not bootstrap
permissions, open an audio endpoint, install a driver, or modify Windows audio
settings. The Tauri shell forwards its requests to this pipe and does not open
the database itself.

## Interactive Tauri shell acceptance

For the remaining desktop-shell gate, use a disposable database and pipe. Run
from an elevated PowerShell session after building the CLI and shell. Enroll
only the current Windows user's SID as an observer, start the bounded backend,
and launch the shell:

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

The manual gate passes only when the shell opens and the UI reports a connected
backend after a read-only refresh. It does not open an audio stream, install a
driver, or change persistent audio settings. Do not use the normal user
database for diagnosis.

## Plan and apply a graph change

Create a plan against the observed revision, inspect it, then apply it once with a unique idempotency key:

```powershell
audiorouter graph plan session-id `
  --base-revision 0 `
  --file C:\path\candidate.json `
  --output C:\path\change.plan.json `
  --database C:\path\audiorouter.sqlite `
  --json

audiorouter graph inspect C:\path\change.plan.json --json

audiorouter graph apply C:\path\change.plan.json `
  --idempotency-key change-20260906-001 `
  --database C:\path\audiorouter.sqlite `
  --json
```

Apply rereads the current revision and refuses a stale plan. If the caller loses the response, query `operations.get` through `api call` before retrying.

## Plan and apply virtual-bus desired state

Virtual-bus lifecycle changes use the same explicit plan/apply boundary. The
CLI requires the `deviceAdministration` scope and persists desired state in
the selected database; until the managed driver is available, apply reports
the operation as applied while its endpoint availability remains unavailable:

```powershell
audiorouter virtual-devices plan `
  --operation C:\path\virtual-bus-operation.json `
  --database C:\path\audiorouter.sqlite `
  --json

audiorouter virtual-devices apply virtual-plan-1 `
  --idempotency-key virtual-change-20260907-001 `
  --database C:\path\audiorouter.sqlite `
  --json
```

The operation file contains one lifecycle object, for example
`{"action":"create","id":"desktop-in","name":"Desktop In"}`. These
commands do not install a driver, create a Windows endpoint, or change audio
configuration.

## Watch bounded state events

Replay state events for one session from an opaque cursor. Repeat `--category`
to select only the event categories the client needs; the backend bounds the
filter to 32 category names and the replay to 500 events:

```powershell
audiorouter watch session-id `
  --after 0 `
  --limit 100 `
  --category graph.committed `
  --category session.deleted `
  --database C:\path\audiorouter.sqlite `
  --json
```

An expired cursor requires the returned snapshot/resynchronization flow. This
command only reads persisted control state; it does not open audio or change
machine configuration.

## Backup and restore

Backups require a new destination; the storage layer refuses to overwrite an existing recovery copy. Bundle imports are staged and validated before persistence:

```powershell
audiorouter backup --database C:\path\audiorouter.sqlite --output C:\path\recovery.sqlite --json
audiorouter restore --backup C:\path\recovery.sqlite --database C:\path\restored.sqlite --json
audiorouter backup prune --directory C:\path\recovery --json
```

The backup and restore commands require absolute paths and refuse existing
destinations. Restore validates SQLite integrity before writing the new file.
The explicit retention command removes only direct files named
audiorouter-backup-*.sqlite beyond the newest ten lexically timestamped
names; pre-migration backups and unrelated files are preserved. It never
prunes recordings.

```powershell
audiorouter export-bundle session-id --database C:\path\audiorouter.sqlite --output C:\path\session.audiorouter --json
audiorouter import-bundle C:\path\session.audiorouter --database C:\path\new.sqlite --staging C:\path\staging --json
```

Imported sessions are stopped and do not install drivers, execute plugins, arm recording, or enable startup automatically.

## MCP stdio

An enrolled local client can launch:

```powershell
audiorouter mcp serve --client-id enrolled-client --database C:\path\audiorouter.sqlite
```

Use `--pipe \\.\pipe\AudioRouter` when a running backend exposes that local named pipe. MCP stdout is reserved for newline-delimited JSON-RPC; diagnostics belong on stderr. The adapter exposes read tools/resources and forwards API calls through enrolled permissions. It does not accept remote HTTP connections or stream raw audio to tools.

The resource catalog includes capabilities, node schemas, bounded session
snapshots, redacted diagnostics, and headless workflow guidance. Focused tools
publish read-only, destructive, and idempotency hints, but the backend remains
the authority for permission and mutation checks. Confirmed recording recycle
is destructive; preview is read-only.

Device and application discovery failures retain their structured JSON-RPC error
data through the CLI and MCP boundaries. Inspect `data.code`, `data.hresult`,
`data.retryable`, and `data.remediation`; in particular, `deviceInUse` is a
contention result and is distinct from `invalidArgument`. An empty inventory is
therefore a successful empty result, not a replacement for a discovery error.

The native transport supports bounded persistent connections for clients that
need to send distinct requests without reconnecting; each connection is capped
at 500 request frames and is closed by the owning server after its session
budget. A production daemon still owns the outer shutdown/restart policy.

## Recovery boundaries

If a plan reports a revision conflict, reread the session and create a new plan. Preserve failed or partial recording files for inspection. Do not delete recordings as part of configuration cleanup. The current repository does not provide a signed installer or managed virtual-audio driver; a successful portable test or MCP response is not evidence that those components are installed or that machine audio has changed.
