# 11 — CLI, MCP, and external assistant workflows

Milestone ownership: M01 usable CLI/discovery; every feature milestone adds commands; M07 complete MCP and concurrency/security parity.

## Requirements

- **AUTO-01 — CLI coverage.** Ship `audiorouter` as an x64 Windows executable. Every public application API method must be callable with typed convenience commands or `audiorouter api call`. The generic path is first-class, validates input, and is documented; it is not an escape hatch around permissions.
- **AUTO-02 — Machine output.** `--json` emits one complete JSON object for one-shot operations and JSON Lines for `watch`. Stdout contains results/events only; diagnostics go to stderr. No ANSI colors or interactive prompts in machine mode. Errors include stable code, message, operation ID, and recovery hints. Support Unicode paths and PowerShell quoting via JSON files/stdin rather than nested escaped arguments.
- **AUTO-03 — Discovery/help.** `audiorouter help`, `schema`, `status`, `devices list`, `apps list`, `nodes types`, and `api methods` work without audio running. Schemas expose defaults, ranges, enums, permissions, failure policy, and dynamic capabilities. Supply executable examples and a locally generated reference at M01.
- **AUTO-04 — Plan/apply.** CLI can save and inspect a plan, apply it with base revision/idempotency key, and query operation outcome. `--dry-run` means no side effects. Convenience commands such as `node set` use the same backend plan/commit flow. `--yes` may acknowledge listed warnings for an already authorized action; it cannot grant scopes, install unsigned drivers, or bypass stale-plan checks.
- **AUTO-05 — Headless lifecycle.** Provide session start/stop/status/export/import, privacy mute, recorder commands, startup configuration, and event watching. Noninteractive calls fail with actionable information if a local permission grant or elevation is needed. They do not hang indefinitely waiting for invisible dialogs.
- **AUTO-06 — MCP adapter.** Ship `audiorouter mcp serve` over stdio. It translates discoverable tools/resources to the same backend API and uses its enrolled client identity. It owns no graph, audio state, or private store. MCP protocol negotiation uses a pinned supported SDK/protocol version chosen in M07; it is independent of AudioRouter API versioning.
- **AUTO-07 — Tool design.** Offer focused read tools (`describe_capabilities`, `list_devices`, `list_applications`, `get_session`, `inspect_routes`, `get_operation`) and write tools (`plan_graph_change`, `apply_graph_change`, `control_session`, `control_recorder`, `plan_virtual_device_change`, `apply_virtual_device_change`). Also expose a schema-validated `call_api` tool for full permitted coverage. Mark read-only/destructive/idempotent annotations accurately but enforce security in backend code.
- **AUTO-08 — Discoverable resources.** MCP exposes readable current capabilities, node schemas, session snapshots, redacted diagnostics, and workflow instructions as resources where supported. No default tool streams raw microphone data to an LLM. Long-running operations return IDs for polling; audio telemetry is summarized and rate-limited.
- **AUTO-09 — Concurrency.** Clients must read revision, plan, inspect consequences, commit, then confirm active status. On conflict re-read and create a new plan. Never blindly retry a stale diff against an unrelated revision. UI updates consume backend events and show the same outcome.
- **AUTO-10 — Permission consistency.** No special unlimited “AI” authority. CLI/MCP grants distinguish reads, graph writes, session control, capture, recording paths, plugin scanning, and device administration. Existing scoped authorization may cover repeated ordinary edits; every edit need not display a new approval dialog. Destructive/privileged actions still need their explicit capability and concrete targets.
- **AUTO-11 — Intent ambiguity.** An assistant must resolve device/session identity and exact destination before changing a route. It may use templates/defaults for ordinary parameters, but must not guess an ambiguous microphone, silently include extra audio, or enable recording in response to a mere inspection request. Prompts and tool descriptions state these constraints without claiming they prevent all LLM mistakes.
- **AUTO-12 — Reproducibility.** UC-10 shall run as a checked-in PowerShell acceptance script using fixture identifiers discovered at runtime. UI/CLI/MCP fixtures produce equivalent canonical graphs and equivalent errors. Tests use explicit grants and exercise denied scope as well as success.

## Command design examples

These are intended commands, not installed executables. M01 creates real help and validates examples; app selection inside Discord/OBS remains external.

```powershell
audiorouter status --json
audiorouter schema --json
audiorouter devices list --json
audiorouter apps list --json
audiorouter session create --name "Gaming" --json
audiorouter graph plan --session <session-id> --base-revision 0 --file .\gaming-graph.json --json
audiorouter graph apply --plan <plan-id> --idempotency-key <unique-key> --json
audiorouter session start --id <session-id> --idempotency-key <unique-key> --json
audiorouter routes inspect --session <session-id> --destination <voice-node-id> --json
audiorouter watch --session <session-id> --events state,meters --json
audiorouter session export --id <session-id> --output .\gaming.audiorouter --json
```

Separate device provisioning happens before graph planning if buses do not exist. Commands requiring paths accept `--file` and stdin with explicit length limits. Do not require credentials in command arguments or include them in command examples.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Requested operation completed, or an explicitly asynchronous request was accepted with an operation ID |
| 2 | Invalid command/input/schema |
| 3 | Backend unavailable/version mismatch |
| 4 | Permission or required local authorization missing |
| 5 | Revision/resource conflict or expired plan |
| 6 | Unavailable/unsupported device, plugin, or capability |
| 7 | Runtime/operation failure |
| 8 | Timeout with possibly continuing operation; query its ID |
| 130 | User cancellation |

Default mutating CLI commands wait for completion up to a documented timeout. `--no-wait` returns an accepted status with operation ID; it must not label the requested change complete. `watch` is interruptible and does not stop audio when canceled.

## Assistant example: “Stop sending desktop audio to Discord”

Read active sessions and use `routes.inspect` for the Voice Chat sink. Identify desktop contributions and any shared upstream mixer. Plan the smallest graph change that removes the desktop contribution only at that destination; splitting a shared mixer may be necessary. Preview impacts on headphones and recording. Commit against the observed revision and inspect the resulting sources. If the call app is using a different Windows input than the modeled bus, report that uncertainty and guide selection; do not claim an in-graph edit controls an unrelated endpoint.

## Assistant example: “Reduce the hum in my microphone”

Inspect the mic chain and available EQ schema. Without audio analysis, do not assert a measured hum frequency. Offer a configurable 50/60 Hz notch preset or apply the user's specified frequency through a planned EQ change. An optional future analysis feature would require explicit capture authorization and its own scope; baseline automation works from configuration/telemetry only.
