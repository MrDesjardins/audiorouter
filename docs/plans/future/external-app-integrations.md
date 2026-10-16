# External app integration extensions

**Delivered:** the request builder (HTTP-08) is implemented as
[HTTP-08](../../spec/16-local-http-api.md): session follow/pin, tool and
setting choice from `nodes.catalog`, Mixer inputs by upstream name, Duck
trigger by node name (new `"reference": "node"` catalog marker; Duck
`trigger` and phase settings now in the catalog), validated values, JSON/curl/
PowerShell output with a token placeholder, and an explicit Send.
Evidence: `ui/src/requestBuilder.test.ts`, `ui/e2e/api-request-builder.pw.ts`
(real backend, three themes). Still open from this plan: temporary overrides,
scenes/batches of several tools, integration-specific permissions, and the
"affected output / preparation required" hint; each needs a decision first.

This backlog covers only follow-up extensions. No new feature is authorized
merely by being listed here.
Requirements: AUTO-15, API-01/07/09/12, HTTP-01/04/05, GRAPH-05/06/08, UI-04.

## Existing support

REST already offers active-session selection, `sessions.summary`,
`nodes.catalog`, `nodes.set` and `nodes.toggle`. Tools are addressed by ID or
unique name; omitted sessionId follows the active editing session. Parameter
edits and flags share backend graph validation, persistence and activation.
The [HTTP guide](../../operations/local-http-api.md#external-app-integration-game-menu-and-match-states)
contains concrete requests. VST hosting is outside this work.

Discovery gap (resolved 2026-10-03 by the `"reference": "node"` marker and the builder): Duck's numeric parameters are advertised by
`nodes.catalog`, but `keyNodeId` is described only in prose and the dedicated
Duck editor. A general property picker needs a node-reference descriptor and
choices from the selected graph. Mixer input parameters are a family keyed by
upstream ID; the builder must expand them using actual connected nodes.

Menu event: game Volume/Mixer input 50%, Discord 100%. Match event: both 100%.
Duck is audio-level-triggered, with adjustable amount/threshold/timing and a
node-ID trigger. A menu-state event should set Volume/Mixer rather than pretend
to be microphone activity. Current edits are persisted; 50% means amplitude.

## Proposed experience

API tab: choose Follow active session or Pin session, then tool by visible
name with kind/ID disambiguation, then Enabled, Bypass or a schema-backed
property and value. Provide direct Mixer input selection by upstream name and
Duck trigger selection by node name, converting to IDs in the generated body.
Show units/ranges, affected output and whether preparation is required.
Generate URL, method, JSON and header instructions; keep tokens out of copied
examples by default. An explicit Send action shows the actual backend outcome.
No action on merely selecting a tool or generating a request.

## Use cases and acceptance

| Use case | Acceptance |
| --- | --- |
| Menu/match game levels | 50/100 then 100/100; Discord unaffected; inspect returned activation and effective samples on a prepared synthetic route |
| Enable/disable or bypass a tool | Explicit state setting, visible UI refresh, prepared-device limits and protected failure policy retained |
| Voice ducking tuning | Trigger resolves to ID; amount, threshold and timing use the catalog; external events never simulate audio |
| StreamDeck/assistant control | Same methods, name ambiguity rejected, retries use one event key |
| Session switching | Following active resolves each event; pinned integrations retain the intended session; missing tool fails clearly |
| Integration exit/crash | Decide restore policy and persisted versus temporary state explicitly; no hidden timeout/reset |
| Several tools per event | Mixer input changes atomic in one edit; separate node calls not atomic; batch/scenes require a separate contract decision |

Prerequisites: API enabled locally with token/authorized grant; tools already
present and prepared where live changes require it; game supplies events via
its own supported integration. No game hooks, memory reads, remote listener,
in-process extension execution or automatic detection is implied.

Still open: temporary overrides, atomic scenes/batches, integration-specific
permissions, and the affected-output/preparation hint. Decide state-restoration
and batch atomicity before implementation; approve any new grant scope first.
No release or arbitrary-game compatibility claim follows from the request
builder.
