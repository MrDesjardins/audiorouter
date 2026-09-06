# AudioRouter headless runbook

This runbook describes the currently verified portable control workflow. It does not claim that native audio routing, virtual devices, driver installation, or production signing are available.

## Inspect without side effects

```powershell
audiorouter status --json
audiorouter schema --json
audiorouter session list --database C:\path\to\audiorouter.sqlite --json
```

Use absolute paths for databases and JSON files.

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

## Backup and restore

Backups require a new destination; the storage layer refuses to overwrite an existing recovery copy. Bundle imports are staged and validated before persistence:

```powershell
audiorouter backup --database C:\path\audiorouter.sqlite --output C:\path\recovery.sqlite --json
audiorouter restore --backup C:\path\recovery.sqlite --database C:\path\restored.sqlite --json
```

The backup and restore commands require absolute paths and refuse existing
destinations. Restore validates SQLite integrity before writing the new file.

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

## Recovery boundaries

If a plan reports a revision conflict, reread the session and create a new plan. Preserve failed or partial recording files for inspection. Do not delete recordings as part of configuration cleanup. The current repository does not provide a signed installer or managed virtual-audio driver; a successful portable test or MCP response is not evidence that those components are installed or that machine audio has changed.
