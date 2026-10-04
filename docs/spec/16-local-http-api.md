# 16 — Local HTTP API

Ownership: M07 adapter/security; M05 API tab; M08 packaging.
Decision (2026-09-29, user): HTTP alongside named-pipe JSON-RPC 2.0 and MCP,
localhost only. No remote control, second backend or new audio permissions.
Decision (2026-10-04, user, amends the above): for a local-network control
panel ([hardware control panel plan](../plans/future/hardware-control-panel.md)),
the user may also open the listener on one private or link-local address of
this PC (HTTP-09). Same token and grant; no internet exposure, pairing or
second backend.

## Contract

- **HTTP-01 — Shared authority.** Forward each HTTP operation to the existing
  backend. Its validation, grants, revisions, persistence and runtime activation
  remain authoritative. Normal frontend refresh reflects external mutations
  while preserving unsaved drafts.
- **HTTP-02 — Lifecycle.** API tab starts/stops the listener and displays its
  actual URL/port. Default 17891, explicitly bound to IPv4 127.0.0.1. Disabled
  at launch. Port collision is actionable; never bind all interfaces or attach
  to another service. HTTP-09 adds the only other listener.
- **HTTP-03 — Token.** Generate an unpredictable bearer token once and retain it
  using current-user Windows DPAPI in the local app-data directory. Opening the
  API tab shows the saved token with Copy and optional Hide, even while stopped.
  Explicit regeneration immediately shows the replacement. Never log it or put plaintext in configuration, URLs, browser
  storage or process arguments. Stop closes the listener; restart reuses the token.
  Explicit Generate new token replaces the saved credential and revokes the old
  one, with confirmation that integrations need updating. Corrupt or inaccessible
  storage fails closed until explicit regeneration. The API stays stopped at launch.
  The adapter uses the desktop backend grant; denied scopes stay denied.
- **HTTP-04 — Resources.** Every discovered method has an HTTP action resource:
  `POST /api/v1/{namespace}/{operation}`, plain JSON parameters/results, without
  a JSON-RPC envelope. Read aliases: `GET /api/v1/capabilities`,
  `GET /api/v1/sessions`, `GET /api/v1/status`. Graph plan/commit retain
  baseRevision/idempotencyKey. Errors preserve backend code/message/data and
  use HTTP 400/403/409/429/503. Durable-save activation failures remain explicit
  in successful commit results.
- **HTTP-07 — Session inventory and selection.** `GET /api/v1/sessions` and
  `GET /api/v1/sessions/active` expose every session and the UI's selected
  editing session. `PUT /api/v1/sessions/active` with `sessionId` and an
  `idempotencyKey`
  selects an existing session and updates the open UI through the backend event
  stream. Selection does not start audio; `POST /api/v1/sessions/start` remains
  a separate operation. Selection survives restart through the UI workspace
  preference and is not persisted as audio-session data.
- **HTTP-08 — Request builder.** The API tab builds one `POST
  /api/v1/nodes/set` request: follow the active session (omit `sessionId`) or
  pin one; choose a saved tool by name (kind and ID when names repeat); choose
  Enabled, Bypass or a catalog parameter. A Mixer's `inputVolume:` family
  expands to its connected inputs by upstream name, and a parameter marked
  `"reference": "node"` in `nodes.catalog` (Duck `keyNodeId`) offers node
  names and sends the node ID. Values are validated against the catalog range
  or choices. It shows the URL, the JSON body, curl and PowerShell with a
  token placeholder, never the saved token. Choosing never mutates; an
  explicit Send applies the body once through the same backend method and
  shows the result.
- **HTTP-09 — Local-network listener (opt-in).** The API tab's "Who can
  connect" offers "This PC only" (default) and each private (RFC 1918) or
  IPv4 link-local address of a connected, non-loopback adapter on this PC,
  named by adapter. The choice is locked while running and not remembered
  across launches. Choosing one adds a second listener on exactly that
  address and the same port; loopback keeps working. Public, CGNAT, VPN
  overlay and wildcard addresses are refused, and an address the PC no longer
  has fails start with an actionable message. The network listener accepts
  only loopback, private or link-local peers; Host must be exactly
  `<address>:<port>` of a listener, and a sent Origin must equal it. Token,
  grant, desktop-only methods, bounds and rate budget are shared with
  loopback. The tab shows the network URL and states that devices on the
  network with the token can control AudioRouter, that traffic is not
  encrypted, and that Windows Firewall may ask once. OpenAPI lists both
  server URLs.
- **HTTP-05 — Swagger.** `/docs` serves bundled Swagger UI, no CDN, analytics,
  remote validator or internet requirement. `/openapi.json` generates OpenAPI
  3.1 from backend discovery schemas for every method, including permissions,
  side effects and bearer security. Swagger authorization is memory-only.
- **HTTP-06 — Bounds.** Exact listener Host and same-origin browser requests;
  no wildcard CORS, cookies or token URLs. Bound headers, JSON size/depth,
  deadlines, concurrency and request rate. HTTP runs off audio threads.
  Documentation has restrictive CSP and only fixed assets. No bodies/tokens
  in diagnostics.

## Acceptance and rollback

Test session listing parity with the UI inventory, active-session GET/PUT/UI
refresh, and method/OpenAPI parity, malformed/oversized input, token/origin/Host
rejection, port collision, encrypted persistence, restart/explicit rotation and denied permissions.
For HTTP-09: refusal of non-private addresses, peer and Host/Origin rules, a
real bind on one of this PC's private addresses served beside loopback, and
the API tab choice in all three themes. A second device on the network is an
attended check. Run HTTP
plan/commit against the frontend's backend and observe its revision/parameter
in the UI. Review all three themes. These are adapter checks, not audio evidence.
Rollback: stop the listener or revert its adapter/UI; existing pipe/MCP and
session formats remain compatible.
