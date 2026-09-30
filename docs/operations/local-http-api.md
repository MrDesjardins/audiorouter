# Use the local HTTP API

Open **API** in the desktop sidebar, choose a port (default **17891**), then
**Start API**. The displayed address is `http://127.0.0.1:17891` by default.
It accepts calls only from this PC. The API starts stopped each time you launch
AudioRouter; no extra service, driver or machine configuration is needed.

Choose **Open Swagger documentation** for the locally bundled interactive
reference at `/docs`. **Reveal API token**, then copy it into Swagger's
**Authorize** dialog. Authorization remains in memory. **Stop API** revokes it;
starting again creates a new token. Treat the token like access to the desktop
controls. Existing backend scopes still apply, including capture/recording roots
and device-administration permissions.

## Calls and graph changes

Read resources: `GET /api/v1/status`, `/api/v1/sessions`, `/api/v1/capabilities`.
Every backend method also has an action resource: replace the dot with a slash,
for example `graph.plan` → `POST /api/v1/graph/plan`. Send its discovery-defined
parameters as a plain JSON object and receive the plain result, without a
JSON-RPC envelope. Use `Content-Type: application/json` and
`Authorization: Bearer <token>` for API calls. The generated `/openapi.json`
contains exact schemas for all 100 methods and three read aliases.

PowerShell example (enter the token interactively, never put it in arguments):

```powershell
$apiUrl = 'http://127.0.0.1:17891'
$apiSecret = Read-Host 'API token'
$apiHeaders = @{ Authorization = "Bearer $apiSecret" }
Invoke-RestMethod "$apiUrl/api/v1/status" -Headers $apiHeaders
$sessions = Invoke-RestMethod "$apiUrl/api/v1/sessions" -Headers $apiHeaders
$candidate = $sessions.items[0]
$baseRevision = $candidate.revision
# Example: rename this session; gain/other parameters follow the same graph flow.
$candidate.name = 'Gaming'
$plan = Invoke-RestMethod "$apiUrl/api/v1/graph/plan" -Method Post `
  -Headers $apiHeaders -ContentType 'application/json' `
  -Body (@{ sessionId=$candidate.id; baseRevision=$baseRevision; candidate=$candidate } | ConvertTo-Json -Depth 64)
$plan # Read diff, warnings and affected destinations before committing.
# Add acknowledgments only after deliberately reviewing each returned warning.
$commit = @{ planId=$plan.planId; baseRevision=$baseRevision; idempotencyKey=[guid]::NewGuid().ToString() }
Invoke-RestMethod "$apiUrl/api/v1/graph/commit" -Method Post `
  -Headers $apiHeaders -ContentType 'application/json' `
  -Body ($commit | ConvertTo-Json -Depth 64)
Remove-Variable apiSecret, apiHeaders
```

Changes reach the existing backend and frontend refreshes automatically. A
clean editor adopts them. If it has unsaved edits, AudioRouter retains the draft
and reports that the session changed elsewhere; use Session → **Revert edits**
to load the authoritative graph, or resolve your draft against the new revision.
For a playing graph, inspect `activation.nativeState` in the commit result:
durable save does not guarantee that every topology change can apply live.

## Errors and limits

401: missing/wrong token; reveal the current one. 403: denied permission,
foreign browser Origin or wrong Host. Use the displayed **127.0.0.1** address,
not `localhost` or a LAN address. 409: stale revision; fetch and plan again.
429: request budget exhausted; back off. 503: backend unavailable; reconnect
in the app. A busy port prevents startup and says to choose another port.
Backend errors retain `error.code`, `message` and structured `data`.

The adapter bounds headers to 16 KiB, body to 4 MiB, header/body reads to a
two-second deadline and socket writes to two seconds. Four workers and 32
queued connections keep work off audio threads. A shared token bucket refills
20 units/s with burst 40; mutations cost one unit and reads cost 0.1. Backend
limits additionally apply. Overflow connections may be closed without an
operation. JSON must be an object; no HTTP batches, query parameters, chunked
bodies, cookie auth or cross-origin browser calls. `Expect: 100-continue` is
accepted only after authentication and body-length checks.
Swagger/docs assets are public local documentation; API data/control needs the
token. No CDN or online validator is used.

Named-pipe and MCP clients continue to work. See the
[API reference](api-reference.md) and [HTTP contract](../spec/16-local-http-api.md).
