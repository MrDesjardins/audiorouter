# Privacy and permissions guide

AudioRouter is designed for local, offline operation. The current repository
does not upload audio, send recordings to an LLM, or change Windows privacy
settings. Its optional HTTP control listener binds only to 127.0.0.1 and starts
stopped. Its bearer token is saved encrypted for the current Windows account
with DPAPI and retained until explicit regeneration in API settings. Stop closes
the listener without replacing the credential. Guarded Windows user-mode audio
routing through explicitly selected existing VB-Cable, Voicemeeter, and
physical WASAPI endpoints is qualified; AudioRouter-managed virtual-device
provisioning remains unavailable.

## What is protected

- The local control surface uses an owner-only Windows named-pipe boundary and
  same-user identity checks in the native transport.
- Method-level grants are enforced by the control dispatcher, independently of
  UI validation or MCP tool descriptions.
- Graph plans require a current revision and an idempotency key before a
  mutation is committed.
- Recording roots, plugin state, bundle staging, and backup destinations are
  bounded, absolute, canonicalized, and protected against traversal and
  reparse-point escapes where the current operation requires them.
- Plugin discovery reads bounded binary metadata without loading or executing
  plugin code. The worker protocol has process/heartbeat/failure boundaries,
  but it is not a complete OS filesystem/network sandbox.

## Permission scopes

The important scopes are deliberately separate:

- `config.read` — inspect capabilities, sessions, routes, and diagnostics.
- `graph.write` — plan and commit configuration graph changes.
- `session.control` — start and stop an approved session.
- `audio.capture` — authorize capture-related operations and privacy mute.
- `recording.write` / `recording.manage` — create or manage recording output
  and library entries.
- `pluginScan` — inspect explicitly selected plugin files.
- `deviceAdministration` — plan/apply managed virtual-device desired state.
- `startup.write` — authorize startup desired-state changes; native OS
registration is performed only by the explicit desktop shell command.

The enrolled local desktop shell also has the `recording.write` (`Record`)
scope for recording actions the user explicitly starts in an approved root.
This grant does not include `audio.capture`. It includes
`deviceAdministration` only after you answer **Allow** to "Allow AudioRouter
to use your audio devices?", which appears the first time you press Play.
That answer is stored on this computer, and **Setup → Audio device access**
withdraws it. Only the AudioRouter window can give it: the localhost HTTP API
refuses `devices.setAccess`, and CLI and MCP clients neither inherit this
local-shell grant nor gain from your answer. The developer variable
`AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` is no longer needed.

A generic read grant cannot elevate itself to another scope. Revoked or
unknown clients are denied before method dispatch. Imported bundles do not
install drivers, execute plugins, arm recorders, or register startup.

API discovery serializes the conceptual `startup.write` scope as
`startupWrite`. The portable control plane stores the desired preference and
reports native registration as shell-owned; it does not write the OS registry
itself.

## Audio privacy boundary

The process-local privacy latch silences physical-capture contributions inside
AudioRouter and remains durable across the tested control restart path. It does
not disable another Windows application's direct microphone access. Existing
user-mode native routes must report their exact endpoint/process binding and
fail closed when preparation or identity validation fails; managed driver
endpoints remain unavailable and must not be represented as healthy zeros.

AudioRouter sends audio over the network only through a **Network Send** node
that the user added and addressed, and only while its session is prepared.
The stream is unencrypted and goes to exactly the IP address entered. It is
meant for a trusted local network, such as a gaming PC feeding a streaming PC.
Privacy mute silences it like any other output. A **Network Receive** node
listens on its UDP port only while its session is prepared, and plays audio
only from the sender address entered. Other datagrams are counted and
ignored, and received data is never treated as anything but audio samples
(SEC-13).

Give both nodes the same **Pairing key** to authenticate the stream: the
receiver then plays only packets tagged with that key (HMAC-SHA256) and drops
replayed ones. Without a key, a device on the same network can fake the
sender's address and inject audio, which may reach OBS or Discord. Pairing
does not encrypt: others on the network can still listen. The key is stored
with the session and in exported bundles, and is never written to logs,
diagnostics or the inspector's change summary.

Do not grant capture or recording scope to an automation client unless its
requested action and approved file roots are understood. Review the concrete
method, session, destination, and path before approving a mutating operation.

## Diagnostics and support

In AudioRouter, choose **Logs → Copy support bundle**. It saves one
`audiorouter-support-<time>.zip` in the logs folder and selects it in File
Explorer. The ZIP holds a `manifest.json` (app version, build, Windows
version, file list), the last 2,000 lines of `backend.jsonl`, `shell.jsonl`
and `mcp-activity.jsonl`, and the window's client diagnostics. Every line
passes through a path filter (`C:\…`, `\\server\…`, `/home/…` become
`<path>`) before it enters the ZIP. Nothing is uploaded; attach the file
yourself. **Open logs folder** opens File Explorer on the folder instead, and
**Copy folder path** copies its location.

To trace one action across the logs, use its request ID. The window sends a
short `requestId` (8 characters such as `K7Q2M9XD`) with every request; the
tray makes one per action, the localhost HTTP adapter one per HTTP request
(or uses a valid `X-Request-Id` from the caller), and the MCP server one per
tool call. The same ID appears as `requestId` in `shell.jsonl`,
`backend.jsonl` and `mcp-activity.jsonl`, and a failed request adds a client
diagnostics row such as `RPC failed: sessions.play (permissionDenied) [req
K7Q2M9XD]`. IDs are 1 to 32 characters from `A-Z a-z 0-9 -`; anything else is
dropped, never logged.

For a hard-to-catch problem, turn on **Logs → Verbose logging** before you
reproduce it. For one hour at most (the status shows the time left; it then
switches itself off) the backend and shell also log successful routine reads
(such as the 20 Hz diagnostics poll) and each request's `durationMs`. Verbose
records carry `"verbose": true`. They still contain no parameters, file paths
or audio. Verbose mode fills the 5 MiB logs faster, so switch it off when
done. The switch is the `diagnostics.setVerbose` method (`sessionControl`).

Before reproducing an issue, note the local time and the action you take.
Afterwards, copy the JSONL files from `%LOCALAPPDATA%\AudioRouter\logs`
(or `%TEMP%\AudioRouter\logs` when LOCALAPPDATA is unavailable):

- `shell.jsonl` and `shell.previous.jsonl`: desktop requests and outcomes.
- `backend.jsonl` and `backend.previous.jsonl`: backend requests and outcomes.
- `discovery.jsonl` and `discovery.previous.jsonl`: failed device reads,
  including endpoints skipped because they disappeared during enumeration.
- `mcp-activity.jsonl`: assistant tool calls (tool name, safe argument
  names, outcome, request ID).

Each file rotates at 5 MiB and keeps one previous file. Collect all three
types when the input/output list is empty. The diagnostics review build adds
`version`, `buildId`, `processId`, and `timeUnixMs`. Failure `detail` includes
the stable category, fixed operation name, numeric and hexadecimal HRESULT,
retryability, and category-based guidance when available. Discovery records
also include capture/render direction, the temporary enumeration index, and
whether the endpoint was skipped or enumeration failed. Indices are not
persistent device identities. Successful device-list records contain counts,
not device names. Successful event subscription polls are omitted; errors
remain visible. Missing context appears as null rather than a guessed value.

For example, `0xE000020B` with `inventory.openPropertyStore` and
`skippedDisappearedEndpoint` identifies a disappeared endpoint's metadata
read; the matching device-list success counts show whether discovery recovered.
Provide the files plus reproduction steps, Windows version, and approximate
failure time. Do not include the session database or audio unless requested.

Diagnostics are metadata-only and redact ordinary sensitive path/identity
details. Support captures must not include microphone samples, recordings,
tokens, or private signing material. Preserve the exact error code and
operation ID when reporting a failure, along with the active-plan revision and
the relevant sanitized evidence.

See the [headless runbook](headless-runbook.md) for safe commands and the
[security specification](../spec/13-security.md) for the full threat model.
