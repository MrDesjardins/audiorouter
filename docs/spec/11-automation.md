# 11 — CLI, MCP, and external assistant workflows

Milestone ownership: M01 usable CLI/discovery; every feature milestone adds commands; M07 complete MCP and concurrency/security parity.

## Requirements

- **AUTO-01 — CLI coverage.** Ship `audiorouter` as an x64 Windows executable. Every public application API method must be callable with typed convenience commands or `audiorouter api call`. The generic path is first-class, validates input, and is documented; it is not an escape hatch around permissions.
- **AUTO-02 — Machine output.** `--json` emits one complete JSON object for one-shot operations and JSON Lines for `watch`. Stdout contains results/events only; diagnostics go to stderr. No ANSI colors or interactive prompts in machine mode. Errors include stable code, message, operation ID, and recovery hints. Support Unicode paths and PowerShell quoting via JSON files/stdin rather than nested escaped arguments.
- **AUTO-03 — Discovery/help.** `audiorouter help`, `schema`, `status`, `devices list`, `virtual-devices list`, `apps list`, `nodes types`, and `api methods` work without audio running. Schemas expose defaults, ranges, enums, permissions, failure policy, and dynamic capabilities. Supply executable examples and a locally generated reference at M01.
- **AUTO-04 — Plan/apply.** CLI can save and inspect a plan, apply it with base revision/idempotency key, and query operation outcome. `--dry-run` means no side effects. Convenience commands such as `node set` use the same backend plan/commit flow. `--yes` may acknowledge listed warnings for an already authorized action; it cannot grant scopes, install unsigned drivers, or bypass stale-plan checks.
- **AUTO-05 — Headless lifecycle.** Provide session start/stop/status/export/import, privacy mute, recorder commands, startup configuration, and event watching. Noninteractive calls fail with actionable information if a local permission grant or elevation is needed. They do not hang indefinitely waiting for invisible dialogs.
- **AUTO-06 — MCP adapter.** Ship `audiorouter mcp serve` over stdio. It translates discoverable tools/resources to the same backend API and uses its enrolled client identity. It owns no graph, audio state, or private store. Support both modern stateless MCP requests (per-request protocol metadata and `server/discover`) and legacy initialize-handshake clients; pin supported protocol versions and keep MCP versioning independent of AudioRouter API versioning. Every MCP version uses the same enrolled identity, grants, backend authorization and audit path.
- **AUTO-07 — Tool design.** Offer focused read tools (`describe_capabilities`, `list_devices`, `list_applications`, `get_session`, `inspect_routes`, `get_operation`) and write tools (`plan_graph_change`, `apply_graph_change`, `control_session`, `control_recorder`, `plan_virtual_device_change`, `apply_virtual_device_change`). Also expose a schema-validated `call_api` tool for full permitted coverage. Mark read-only/destructive/idempotent annotations accurately but enforce security in backend code.
- **AUTO-08 — Discoverable resources.** MCP exposes readable current capabilities, node schemas, session snapshots, redacted diagnostics, and workflow instructions as resources where supported. No default tool streams raw microphone data to an LLM. Long-running operations return IDs for polling; audio telemetry is summarized and rate-limited.
- **AUTO-09 — Concurrency.** Clients must read revision, plan, inspect consequences, commit, then confirm active status. On conflict re-read and create a new plan. Never blindly retry a stale diff against an unrelated revision. UI updates consume backend events and show the same outcome.
- **AUTO-10 — Permission consistency.** No special unlimited “AI” authority. CLI/MCP grants distinguish reads, graph writes, session control, capture, recording paths, plugin scanning, and device administration. Existing scoped authorization may cover repeated ordinary edits; every edit need not display a new approval dialog. Destructive/privileged actions still need their explicit capability and concrete targets.
- **AUTO-11 — Intent ambiguity.** An assistant must resolve device/session identity and exact destination before changing a route. It may use templates/defaults for ordinary parameters, but must not guess an ambiguous microphone, silently include extra audio, or enable recording in response to a mere inspection request. Prompts and tool descriptions state these constraints without claiming they prevent all LLM mistakes.
- **AUTO-12 — Reproducibility.** UC-10 shall run as a checked-in PowerShell acceptance script using fixture identifiers discovered at runtime. UI/CLI/MCP fixtures produce equivalent canonical graphs and equivalent errors. Tests use explicit grants and exercise denied scope as well as success.
- **AUTO-13 — Assistant setup and activity.** The desktop MCP tab documents the vendor-neutral local stdio MCP server for clients that can launch a local command, including Codex, Claude Code and supported local Cowork deployments, using the installed CLI, exact database/pipe configuration, and an explicitly enrolled client identity. It recommends observer scope first and explains when editor scope is required. Local Cowork uses a plugin ZIP exported on explicit user action, a distinct Observer client, and a bounded real-CLI capability preflight. Preserve existing grants, export configuration only, and show local availability rather than claiming a connected Cowork session. Cloud Cowork tasks cannot launch this local command; they require a separately configured remote MCP endpoint. A bounded local activity view identifies received tool calls and outcomes using redacted summaries. It never exposes audio/media contents, credentials, or model reasoning and does not create an alternate authorization path.
- **AUTO-14 — Diagnostics.** Desktop diagnostics expose recent client-side exceptions and bounded graph checkpoints. The shell records RPC method, outcome, timestamp, and privacy-safe state summary to a size-bounded local log. Secrets, raw params, and audio are excluded. Realtime audio callbacks never log.
- **AUTO-15 — Task-shaped operations (2026-10-01, user request).** Besides the one-to-one method resources, the backend offers intention-level methods (`sessions.play`/`togglePlay`/`summary`, `safety.togglePrivacyMute`, `nodes.catalog`/`set`/`toggle`/`add`/`remove`, `connections.add`/`remove`, `meters.levels`, `recorders.startRecording`/`stopRecording`). They default to the active session, accept node IDs or exact names (ambiguity is refused with candidates), use the same plan/commit/activation path and grants as the UI (`sessions.play` needs device administration like `nativePaths.prepare`), and return actionable messages. HTTP exposes them through HTTP-04; MCP exposes one plain-language tool each plus `get_recipes`, `list_sessions`, `open_session` and `create_session`, generating a missing idempotency key. Every operation has success, failure and denied-scope tests. Node-targeted runtime methods (`soundboard.trigger`/`stop`/`status`, `audioSources.transport`, `timeShift.transport`, `meters.reset`, `plugins.resume`) follow the same rules: `sessionId` is optional and `nodeId` accepts an ID or an exact name, resolved in the backend. MCP's default tools also include `list_sounds`, `trigger_sound`, `stop_sounds`, `get_soundboard_status`, `control_audio_file`, `control_time_shift`, `reset_meter`, `resume_plugin` (2026-10-31, U6), `undo_last_change`, `duplicate_session`, `delete_session`, `list_presets`, `get_status` and `get_virtual_cable_status` (2026-10-31, owner request).
- **AUTO-16 — ChatGPT connection (2026-10-11, user request).** OpenAI Secure MCP Tunnel is an optional OpenAI-specific transport for ChatGPT to reach AudioRouter's local stdio MCP server. It is not the MCP server or protocol and cannot be presented as a Cowork connection. AudioRouter manages and supervises the official local tunnel client. Keep the same client enrollment, grants, backend validation, and redacted audit as other MCP clients; the tunnel grants no additional AudioRouter authority. Explain that tunnel readiness is separate from adding the connector to ChatGPT and enabling it in a conversation. Explain that the default Observer grant permits reads only; session and graph creation require the owner to grant Editor access. Store runtime credentials with current-user Windows protection; never put them in command arguments, logs, the UI, or support bundles. Connecting requires the user's OpenAI Platform authorization and explicit acknowledgement of the remote access path; no tunnel or credential is created silently. Do not start it at sign-in unless the user separately opts in. Exclude audio/media payloads.
- **AUTO-17 — Generic remote MCP (future; user direction 2026-10-11).** Provide a standards-based remote MCP transport usable by cloud MCP clients including ChatGPT and Cowork without making an OpenAI-only tunnel a protocol dependency. Preserve AUTO-06's stdio transport for local clients. Remote access must use Streamable HTTP (or a separately justified supported MCP transport), strong client authentication/OAuth, explicit enrollment and scoped backend grants, bounded request handling, redacted audit, and a secure outbound relay or hosted endpoint for a PC behind NAT. Never expose the existing REST API or an unauthenticated/public MCP listener as a shortcut. This requirement remains future work until the remote authentication, relay/hosting, privacy, operations, and revocation design is reviewed and separately scheduled.
- **AUTO-18 — REST and MCP parity (owner rule, 2026-10-31).** A feature is not finished until the REST API and the default MCP tools can use it. REST reaches every `API_METHODS` entry by construction (HTTP-04) except the documented desktop-only methods. Every public API method is reached by a default MCP tool or resource, or is listed with its reason in the CLI test exclusion table (`MCP_EXCLUDED_METHODS`); the test `every_api_method_is_reached_by_a_default_mcp_tool_or_excluded` runs every default tool and fails on an unreached, unlisted method. `nodes.catalog` describes the settings of every node kind through an exhaustive match, so an assistant can configure each tool. A user-visible feature also gets a `get_recipes` entry when an assistant would not find the steps from tool descriptions alone. MCP role guidance: Observer reads; Editor builds and changes sessions; Operator also controls running audio; starting audio needs device administration, which MCP grants never receive.

## Command design examples

AUTO-16 connection qualification includes a bounded read-only preflight of
the actual packaged CLI: modern discovery, legacy initialization, tool listing
and a backend capability read must pass before starting the tunnel. A stale CLI,
missing enrollment or unavailable backend produces an actionable local error.
AUTO-06 must accept legacy request metadata (progress tokens and client
extensions) without treating it as modern protocol negotiation; explicit
stateless protocol metadata and server discovery select the modern envelope.
Tunnel readiness remains separate from ChatGPT plugin acceptance. On Disconnect
or shell exit, stop both the poller and its MCP descendants, including exits
that do not run Rust destructors; another poller must not survive and compete
for requests on the same tunnel.

These are intended commands, not installed executables. M01 creates real help and validates examples; app selection inside Discord/OBS remains external.

```powershell
audiorouter status --json
audiorouter schema --json
audiorouter devices list --json
audiorouter apps list --json
audiorouter session create C:\path\gaming-session.json --database C:\path\audiorouter.sqlite --idempotency-key create-gaming --json
audiorouter graph plan <session-id> --base-revision 0 --file C:\path\gaming-graph.json --output C:\path\gaming.plan.json --database C:\path\audiorouter.sqlite --json
audiorouter graph apply C:\path\gaming.plan.json --idempotency-key apply-gaming --database C:\path\audiorouter.sqlite --json
audiorouter session start <session-id> --database C:\path\audiorouter.sqlite --idempotency-key start-gaming --json
audiorouter routes inspect <session-id> <voice-node-id> --database C:\path\audiorouter.sqlite --json
audiorouter watch <session-id> --database C:\path\audiorouter.sqlite --after 0 --limit 100 --category graph.committed --json
audiorouter export <session-id> --database C:\path\audiorouter.sqlite --json
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
