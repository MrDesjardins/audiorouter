# Use the local HTTP API

Open **API** in the desktop sidebar, choose a port (default **17891**), then
**Start API**. The displayed address is `http://127.0.0.1:17891` by default.
It accepts calls only from this PC. The API starts stopped each time you launch
AudioRouter; no extra service, driver or machine configuration is needed.

Choose **Open Swagger documentation** for the locally bundled interactive
reference at `/docs`. The API tab shows the saved token even while stopped.
Choose **Copy API token** to use it with your own apps or Swagger's
**Authorize** dialog. Swagger authorization remains in memory. The token is saved
encrypted for your Windows account in `%LOCALAPPDATA%\AudioRouter\api-token.dpapi`.
**Stop API** closes the listener; starting again or restarting AudioRouter reuses
the same token. **Generate new token → Replace API token** explicitly replaces it
and invalidates the old one. Update integrations after replacement. If the saved
credential cannot be decrypted, startup fails until you explicitly replace it.
Treat the token like access to the desktop
controls. Existing backend scopes still apply, including capture/recording roots
and device-administration permissions.

## One call per intention (StreamDeck, scripts, assistants)

These methods need no IDs, revisions or graph documents. `sessionId` defaults
to the session open in AudioRouter, and nodes can be named exactly as on the
canvas (case does not matter). Every change is applied live and saved, like an
edit in the window.

| Want to… | Call |
| --- | --- |
| Play or stop (one key) | `POST /api/v1/sessions/togglePlay` `{"idempotencyKey":"…"}` |
| Mute or unmute the mic (one key) | `POST /api/v1/safety/togglePrivacyMute` |
| Switch an Input Switch between A and B | `POST /api/v1/nodes/toggle` `{"node":"Input Switch","target":"selected",…}` |
| Bypass or enable a tool | `POST /api/v1/nodes/toggle` `{"node":"Voice Gate","target":"bypass",…}` |
| Set the game volume | `POST /api/v1/nodes/set` `{"node":"Game volume","parameters":{"percent":60},…}` |
| Duck the game more | `POST /api/v1/nodes/set` `{"node":"Duck game","parameters":{"amountDb":12},…}` |
| Start or stop recording | `POST /api/v1/recorders/startRecording` / `stopRecording` `{"nodeId":"Podcast recorder",…}` |
| Show levels on a display | `POST /api/v1/meters/levels` `{}` |
| Describe the setup | `POST /api/v1/sessions/summary` `{}` |

Each mutating call needs a fresh `idempotencyKey` (any unique text). A
StreamDeck "web request" or "API" action can send these with the bearer token
header. Mistakes return HTTP 400 with a sentence naming the problem, for
example the node names that exist. Play needs the device-administration
permission, like `nativePaths.prepare`.

AI assistants use the same operations through MCP: `get_recipes`,
`get_session_summary`, `add_tool`, `change_settings`, `toggle_setting`,
`connect_nodes`, `play`, `toggle_mic_mute`, `start_recording` and others.
They take names and generate the idempotency key automatically.

## Build a request in the app

The API tab has a **Build a request** section. Choose whether the request
follows the session selected in AudioRouter or always targets one session,
then a tool and one setting (Enabled, Bypass, a parameter, a Mixer input's
volume by its source's name, or a Duck's trigger node by name). It shows the
exact `POST /api/v1/nodes/set` request as a JSON body, curl or PowerShell,
with `<your API token>` where your token goes. Choosing changes nothing;
**Send now** applies it once so you can hear the result. Send a new
`idempotencyKey` for each event from your integration; reuse it only when
retrying the same event.

## External app integration: game menu and match states

For an independent executable example, see the
[Stats.cc Siege service](../../examples/integrations/stats-cc-siege/README.md).
It keeps preparation at 50%, action at 100%, and Discord at 100%, using the
Mixer's direct upstream node IDs.

An external app on this PC can control the active session through the existing
REST API. No VST code or in-process extension is needed. Start the API in the
window, then use its bearer token. `sessions.summary` lists tools and their
IDs/names/settings; `nodes.catalog` lists supported parameter names, types,
ranges and defaults. Duck's `keyNodeId` is currently a special reference
setting, absent from its numeric catalog entries; select its ID from the
session's nodes. Omit `sessionId` to follow the active editing session,
or include it to pin an integration to a particular session.

For the menu/match example, put a Volume tool named **Game volume** on the
game branch before its Mixer. Keep the Discord branch at 100%. Send these
bodies to `POST /api/v1/nodes/set` with `Content-Type: application/json` and
`Authorization: Bearer <token>`:

```json
{"node":"Game volume","parameters":{"percent":50},"idempotencyKey":"menu-event-001"}
```

```json
{"node":"Game volume","parameters":{"percent":100},"idempotencyKey":"match-event-002"}
```

Use a unique key per event; retries of that event reuse its key. Explicit
values make duplicate or repeated menu notifications harmless. Prefer
`nodes.set` to toggles for game-state events. Percent controls sample amplitude;
50% is approximately -6.02 dB, not a guarantee of half perceived loudness.

A Mixer can instead change both inputs in one validated edit. Its input keys
use the IDs of the nodes directly feeding it (which may be EQ/Volume nodes,
not the original capture sources):

```json
{"node":"Game and Discord","parameters":{"inputVolume:game":50,"inputVolume:discord":100},"idempotencyKey":"menu-mixer-003"}
```

At match start, set both keys to 100. The IDs above are examples; obtain the
actual direct-input IDs from the session summary. Changing two separate
Volume tools takes two calls; it is not an atomic multi-node edit.

Other supported intentions on the selected tool:

| Intention | Fields in a `nodes.set` body (plus node and idempotencyKey) |
| --- | --- |
| Enable / disable | `"enabled":true` / `false` |
| Bypass / process | `"bypass":true` / `false` |
| Set Duck attenuation | `"parameters":{"amountDb":6.02}` |
| Tune Duck response | `"parameters":{"thresholdDb":-35,"attackMs":20,"holdMs":300,"releaseMs":500}` |
| Choose Duck trigger | `"parameters":{"keyNodeId":"mic"}` (actual node ID) |

Duck follows the trigger's audio level; a menu event is not an audio trigger.
Use Volume/Mixer for a fixed menu level, or configure Duck for voice-triggered
attenuation. Disabled processors pass dry; disabled sources/sinks suppress
their contribution. Re-enabling a source excluded from preparation can require
Stop/Play; a live edit does not implicitly open an unprepared microphone.

These calls validate and persist graph changes, rather than being an ephemeral
per-frame automation stream. Inspect returned activation state for a playing
session; some changes require preparation. Limit calls to state transitions,
serialize updates, handle errors/conflicts, and restore explicitly chosen levels
on exit. A game must provide an integration/event source itself; AudioRouter
does not infer menu/match state. Current HTTP access is localhost only.

The next proposed UI task is an integration request builder: active/pinned
session selector, tool selector, property selector from the backend schema,
validated value, and copyable request. It is not implemented yet. Existing
Swagger and name-based `nodes.set` are the current entry points.

## Calls and graph changes

Read resources: `GET /api/v1/status`, `/api/v1/sessions`,
`/api/v1/sessions/active`, `/api/v1/capabilities`. Select the editing session
with `PUT /api/v1/sessions/active`, providing `sessionId` and a new
`idempotencyKey`. This changes the session shown by the open UI; it does not
start audio. Audio lifecycle remains
`POST /api/v1/sessions/start` and `/stop`.
For a saved multi-path route, first call `POST /api/v1/nativePaths/prepare`
with `{"sessionId":"patrick-main-session"}`. This opens stopped audio clients
and prepares plugins using the exact devices saved on the nodes; it requires
DeviceAdministration permission. Then call `POST /api/v1/sessions/start` with
the same `sessionId` and a fresh `idempotencyKey`. Desktop Play performs this
preparation before Start; calling Start alone does not prepare devices.
Both `session/start` and `sessions/start` are supported aliases.
If preparation fails, correct the reported device, application, plugin or
permission problem before retrying; do not substitute a default microphone.
Every backend method also has an action resource: replace the dot with a slash,
for example `graph.plan` → `POST /api/v1/graph/plan`. Send its discovery-defined
parameters as a plain JSON object and receive the plain result, without a
JSON-RPC envelope. Use `Content-Type: application/json` and
`Authorization: Bearer <token>` for API calls. The generated `/openapi.json`
contains exact schemas for 119 of the 121 backend methods, four GET aliases,
and the active session PUT alias. Two methods are exceptions, because they are
the user's own decisions; only the AudioRouter window may call them, and this
adapter answers 403:
- `recordings.setRoot` approves where recordings are written;
- `devices.setAccess` allows AudioRouter to open audio devices on Play.

`recordings.getRoot` and `devices.getAccess` read them. Once the user has
allowed device access in the window, `sessions.play` and `sessions.togglePlay`
work from this API too.

PowerShell example (enter the token interactively, never put it in arguments):

```powershell
$apiUrl = 'http://127.0.0.1:17891'
$apiSecret = Read-Host 'API token'
$apiHeaders = @{ Authorization = "Bearer $apiSecret" }
Invoke-RestMethod "$apiUrl/api/v1/status" -Headers $apiHeaders
$sessions = Invoke-RestMethod "$apiUrl/api/v1/sessions" -Headers $apiHeaders
# Read or select the session shown in the editor; this does not start audio.
$active = Invoke-RestMethod "$apiUrl/api/v1/sessions/active" -Headers $apiHeaders
Invoke-RestMethod "$apiUrl/api/v1/sessions/active" -Method Put `
  -Headers $apiHeaders -ContentType 'application/json' `
  -Body (@{ sessionId=$sessions.items[0].id; idempotencyKey=[guid]::NewGuid().ToString() } | ConvertTo-Json)
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
