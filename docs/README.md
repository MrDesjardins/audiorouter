# Documentation map

Status: implementation baseline with partial portable foundations, updated 2026-09-06. Requirements describe intended behavior unless the active plan/evidence explicitly records implementation and verification. Numeric budgets remain acceptance targets until measured. Native audio, driver, signing, packaging, and hardware gates are not implied by portable tests.

## How to read the specification

Read 01–04 first, then the feature areas needed for a milestone. `MUST`/`shall` denotes a required release condition at the assigned milestone. `SHOULD` denotes a documented preference that can be varied with evidence. `Future` is excluded from v1. A requirement ID identifies the entire numbered item, including its subordinate conditions. Examples are illustrative unless explicitly identified as acceptance fixtures.

| File | Responsibility | Delivery owner |
| --- | --- | --- |
| [01 Product](spec/01-product.md) | Outcomes, scope, platform, release tiers | M00–M08 |
| [02 Workflows](spec/02-workflows.md) | Golden routes, exclusions, acceptance stories | M03–M08 |
| [03 Architecture](spec/03-architecture.md) | Processes, realtime boundary, stack, ownership | M00–M02 |
| [04 Graph](spec/04-graph.md) | Domain entities, channels, bypass, atomic edits | M01–M02 |
| [05 Windows capture](spec/05-windows-capture.md) | Hardware, apps, loopback, rebinding | M00/M02/M03 |
| [06 Virtual devices](spec/06-virtual-devices.md) | Endpoint model, driver, persistence, pass-through | M00/M03/M08 |
| [07 Processing](spec/07-processing.md) | Built-in DSP, pitch, VST hosting, failure policy | M04/M06 |
| [08 Recording](spec/08-recording.md) | Files, branches, formats, recovery, library | M04 |
| [09 Interface](spec/09-interface.md) | Canvas, onboarding, accessibility, status | M05 |
| [10 API](spec/10-api.md) | Discovery, transactions, events, error contracts | M01/M07 |
| [11 CLI and MCP](spec/11-automation.md) | Headless parity, tools, automation examples | M01/M07 |
| [12 Persistence](spec/12-persistence.md) | Sessions, migrations, startup, crash recovery | M01/M07 |
| [13 Security](spec/13-security.md) | Local trust, consent, plugin and driver boundaries | M01/M03/M06/M07 |
| [14 Quality](spec/14-quality.md) | Measurable budgets, test matrix, release evidence | M00–M08 |
| [15 Delivery](spec/15-delivery.md) | Sequence, traceability, decisions, risks, sources | M00–M08 |

## Plans versus specifications

- [Active](plans/active/current.md): execution state, evidence, and next steps. Start here when resuming work.
- [Archived](plans/archived/README.md): completed or superseded execution plans, including what actually happened.
- [Future](plans/future/README.md): deliberately deferred features and conditions for reconsidering them.
- [Milestones](spec/15-delivery.md#milestone-sequence): stable implementation contracts. They do not move when execution plans are archived.

The repository-level [AGENTS.md](../AGENTS.md) governs the entire development lifecycle and records evidence-backed lessons. Do not use session transcripts as a substitute for updating these documents.

## Starting implementation

To continue implementation, read `AGENTS.md`, the relevant milestone, and the [active plan](plans/active/current.md). Report measured Windows evidence separately from portable or simulated evidence. Do not infer driver installation, signing, or machine audio changes from a successful compile or unit test.
