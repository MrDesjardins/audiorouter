# Stats.cc → AudioRouter: Siege round volume

A standalone Node service consuming Stats.cc's existing game-state feed and
calling AudioRouter's local REST API. No VST or AudioRouter implementation changes.
The inspected Stats.cc installation is **1.8.1-stable** (Electron/overlayed.gg).
Its optional state feed is an undocumented vendor interface and can change.

> **Built-in alternative:** AudioRouter builds after v0.0.4 can follow the Siege
> round without this service. Add a **Duck** on the game line and set its
> Trigger to **Siege round (Stats.cc)**; only Stats.cc and AudioRouter need to
> run. This example remains for Mixer-level control and as a REST reference.

## Behavior

| Stats.cc state | Siege Mixer input | Discord Mixer input |
| --- | --- | --- |
| Main menu / matchmaking | 30% | 100% |
| Match created / map or operator selection / planning | 30% | 100% |
| Preparation (`prep`) | 30% | 100% |
| Round action (`action`) | 100% | 100% |
| Round/match results; between rounds | 30% | 100% |
| Source unavailable or malformed | 100% recovery level | 100% |
| Normal service stop | 100% | 100% |

An unknown nonempty phase string on a valid connected match counts as another
non-action phase (30%). Malformed/missing state is a source failure (100%).
On connection the service waits for a **fresh snapshot**: Stats.cc sends no
initial snapshot. Old match files and logs are not replayed as current state.
Source silence alone is not a failure: Stats.cc updates only when state changes,
and WebSocket ping/pong detects a disconnected source.

Only `inputVolume:<game upstream ID>` and `inputVolume:<Discord upstream ID>`
on the Mixer are changed, in one validated `nodes.set` call. The EQ and Mixer
master are not adjusted. Percent is sample amplitude; 30% is about −10.46 dB.
Edits are **saved** in AudioRouter and can add graph history. Hard termination,
PC failure or unavailable API can prevent restoration; set both inputs to 100%
manually in that case. A normal stop deliberately restores 100%, not the level
that happened to be present before launch (the inspected Siege input was 80%).

## Setup

Requires Windows, Stats.cc 1.8.1, Node **22 or later**, and AudioRouter running
with the intended graph. Commands below are PowerShell:

```powershell
cd C:\code\audiorouter\examples\integrations\stats-cc-siege
npm.cmd ci
Copy-Item config.example.json config.local.json
```

Edit `config.local.json` for your graph. The defaults match the inspected setup:

- Mixer: **Siege + Discord** (`mixer-1`)
- Game input directly feeding it: **Siege Advanced EQ** (`siege-eq`)
- Discord input: **Discord.exe capture 1** (`application-capture-1`)

Names are exact, case-insensitive; an ID also works. Ambiguous names, missing or
disabled connections, wrong Mixer kind or bypass are refused. Leave `sessionId`
null to resolve the active session **once at launch**; it then stays pinned.
Set a session ID explicitly to choose another session. The real inspected
session is `patrick-main-native`. Switching the editor later does not redirect
this service to another graph. Target connections are rechecked before writes.

### Enable Stats.cc's existing feed

The automated setup works for every user with the inspected installation layout:

```powershell
npm.cmd run setup:stats
```

It creates `%APPDATA%\stats.cc\ceb44052-d616-4bdc-993c-70bae41091e9.txt`
containing the port from `statsUrl` (default **17892**). It checks installed
version, rejects an occupied port or different existing configuration, and
does not patch binaries/JavaScript, terminate processes or change firewall rules.
Close Stats.cc normally, including its tray app, then reopen it. This enables
the listener at startup; running setup twice with the same port is harmless.
This command was **not run on the user's installed Stats.cc during development**.

**Vendor listener boundary:** Stats.cc binds this feed to **all interfaces**
without authentication, and its snapshots contain player/profile information.
Keep inbound network access to this port blocked; do not approve a firewall
exception. The example connects only to `127.0.0.1` and never logs raw snapshots.
If you need strictly loopback-only vendor behavior, leave the feed disabled;
this example cannot change its binding without altering Stats.cc's implementation.

Undo the matching setting and restart Stats.cc normally:

```powershell
npm.cmd run setup:stats -- --remove
```

Removal refuses a file containing a different port. If someone configured the
same port before using this example, preserve their file instead of removing it.

### Start AudioRouter's API

Open **API → Start API → Reveal API token** in AudioRouter. Default address:
`http://127.0.0.1:17891`. The API is stopped at app launch. Builds with persistent
tokens retain the credential until you choose **Generate new token**. This service
automatically reads AudioRouter's current-user DPAPI credential privately from
`%LOCALAPPDATA%\AudioRouter\api-token.dpapi`; it is never written as plaintext.
If no saved credential exists, the service prompts without echoing it. Older
builds (including v0.0.3) regenerate on API start and need the current credential.
Never put it in configuration, source files, URLs or command arguments.
An automation may supply `AUDIOROUTER_API_TOKEN` privately through its environment.

## Run and review

```powershell
# No token or AudioRouter writes; observe redacted Stats.cc phases only.
npm.cmd run observe

# Saved token or hidden prompt; print target IDs, then exit without edits.
npm.cmd run inspect

# Validate targets and report intended changes without writing.
npm.cmd start -- --dry-run

# Live integration: automatic Mixer changes.
npm.cmd start
```

Use one mode at a time; type **quit** and press Enter, or press **Ctrl+C**, to
stop normally. No background Windows service or startup task is installed.
The service's read-only status endpoint is
`http://127.0.0.1:17893/status`; it reports phase, desired/applied levels and
source/API state, never token, players or full match data. Observe/dry-run have
no applied level and do not restore levels because they never changed them.

Watch menu → queue/selection → preparation → action → results while playing.
Verify **30 → 30 → 30 → 100 → 30** for Siege and constant **100** for Discord
in AudioRouter's Mixer properties. Every new round's preparation is quiet again.
Raw event names such as `round_ended` are not treated as a phase: the feed
broadcasts complete state and the `phase_changed` result is authoritative.

Writes are serialized and latest-state updates replace queued work. Retries
reuse the event's idempotency key. HTTP deadlines are three seconds; response
size is bounded to 4 MiB and WS messages to 1 MiB. Reconnect delay grows from
one to fifteen seconds. Authorization, invalid target and activation errors
pause writes until restart; transient server/network failures retry. If the
API token changed, restart with its current token. Graph changes can be saved
but require Stop/Play; such activation errors are reported, never called live.

## Evidence and limitations

`npm.cmd test` runs phase, targeting, API and standalone-service fixtures using
synthetic snapshots and loopback HTTP/WS servers. The real graph was inspected
read-only over the existing named pipe, validating the resolver against actual
node IDs and incoming edges. Real Stats.cc broadcast delivery, gameplay phases
and heard/native sample levels still require the attended steps above; fixtures
do not prove a live match. Stats.cc and AudioRouter's running setup were unchanged.

See the [implementation plan](../../../docs/plans/archived/2026-10-02-stats-cc-siege-integration.md)
and [HTTP guide](../../../docs/operations/local-http-api.md).
