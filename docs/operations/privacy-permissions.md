# Privacy and permissions guide

AudioRouter is designed for local, offline operation. The current repository
does not upload audio, send recordings to an LLM, expose an HTTP control
listener, or change Windows privacy settings. Native audio routing is still an
unavailable capability in this development snapshot.

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
- `startup.write` — startup registration, which remains unavailable here.

A generic read grant cannot elevate itself to another scope. Revoked or
unknown clients are denied before method dispatch. Imported bundles do not
install drivers, execute plugins, arm recorders, or register startup.

## Audio privacy boundary

The process-local privacy latch silences physical-capture contributions inside
AudioRouter and remains durable across the tested control restart path. It does
not disable another Windows application's direct microphone access. Because
the native graph is not yet active, the current build must report audio as
unavailable rather than imply that the latch controls all system capture.

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
