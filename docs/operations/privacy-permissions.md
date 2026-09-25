# Privacy and permissions guide

AudioRouter is designed for local, offline operation. The current repository
does not upload audio, send recordings to an LLM, expose an HTTP control
listener, or change Windows privacy settings. Guarded Windows user-mode audio
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
This grant does not include `audio.capture` or `deviceAdministration`; opening
or preparing an input endpoint remains separately authorized. CLI and MCP
clients do not inherit this local-shell grant.

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

Do not grant capture or recording scope to an automation client unless its
requested action and approved file roots are understood. Review the concrete
method, session, destination, and path before approving a mutating operation.

## Diagnostics and support

Diagnostics are metadata-only and redact ordinary sensitive path/identity
details. Support captures must not include microphone samples, recordings,
tokens, or private signing material. Preserve the exact error code and
operation ID when reporting a failure, along with the active-plan revision and
the relevant sanitized evidence.

See the [headless runbook](headless-runbook.md) for safe commands and the
[security specification](../spec/13-security.md) for the full threat model.
