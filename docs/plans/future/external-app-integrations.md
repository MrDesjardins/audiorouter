# External app integrations and a request builder

Proposed 2026-10-02 from the user's game-menu/match example. Specification and
use-case review only; feature implementation is not authorized by this plan.
Requirements: AUTO-15, API-01/07/09/12, HTTP-01/04/05, GRAPH-05/06/08, UI-04.

## Existing support

REST already offers active-session selection, `sessions.summary`,
`nodes.catalog`, `nodes.set` and `nodes.toggle`. Tools are addressed by ID or
unique name; omitted sessionId follows the active editing session. Parameter
edits and flags share backend graph validation, persistence and activation.
The [HTTP guide](../../operations/local-http-api.md#external-app-integration-game-menu-and-match-states)
contains concrete requests. VST hosting is outside this work.

Current discovery gap: Duck's numeric parameters are advertised by
`nodes.catalog`, but `keyNodeId` is described only in prose and the dedicated
Duck editor. A general property picker needs a node-reference descriptor and
choices from the selected graph. Mixer input parameters are a family keyed by
upstream ID; the builder must expand them using actual connected nodes.

Current discovery gap: Duck's numeric parameters are advertised by
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

Ordered next task if approved: schema-backed request builder; named Mixer/Duck
selection; generated-request fixtures and real adapter tests; three-theme UI
review; prepared synthetic live menu/match test and guarded native continuity.
Decide temporary overrides, scenes/batches and integration permissions before
adding those capabilities. Rollback UI/guide changes without saved-graph
migration. No release or arbitrary-game compatibility claim.
