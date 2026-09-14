# VDEV-12 bridge failure event evidence

Date: 2026-09-14

The control plane now advertises `virtualBridge.failed` and
`virtualBridge.expired` in its discoverable state-category catalog. Native
capture-sink, render-source, and duplex
heartbeat containment publishes this event only after detaching the affected
binding, closing its lease, and deactivating the portable bridge. The affected
bus identifier is carried as the bounded operation identity; native HRESULTs,
paths, and private diagnostics remain in the structured supervisor response
and are not retained in the event stream.

The injected-tick control-thread lease sweep returns exact stable IDs for
bridges that expired, silenced, and drained, then publishes one
`virtualBridge.expired` event per affected bus. Active bridges produce no event.
The UI consumes both bridge categories and presents the bounded bus-scoped
failure/expiry message through its existing accessible status channel.
The shared TypeScript contract now constrains `StateEvent.category` and event
subscription filters to the same 17 categories advertised by Rust discovery.
The same parity check now includes the emitted `devices.changed` endpoint
inventory event and `recovery.safeModeCleared`, for 19 categories total; the UI
subscribes to both so endpoint and recovery changes refresh the authoritative
snapshot.
The `events.subscribe` JSON schema also exposes this exact 19-value enum,
preventing schema-driven clients from requesting undiscoverable categories.

Verification on Windows workspace `C:\code\audiorouter`:

- `cargo test -q -p audiorouter-control --locked -- --test-threads=1` — 150
  passed, 2 guarded live tests ignored.
- `cargo clippy -q -p audiorouter-control --locked --all-targets -- -D warnings` — passed.
- `npm.cmd --prefix ui run test -- --run` — 184 passed.
- `npm.cmd --prefix ui run typecheck` — passed.
- `npm.cmd --prefix contracts run check:drift` — 71 methods, 19 node kinds, 7
  processors, and 19 event categories aligned.
- `npm.cmd --prefix contracts run typecheck` — passed.
- `cargo test -q -p audiorouter-control --locked -- --test-threads=1` — 150
  passed, 2 guarded live tests ignored, including the schema equality regression.
- `tests\acceptance\docs.ps1` — 53 Markdown files and 192 local links passed.

No endpoint, default-device, driver, or other machine audio configuration was
changed. Loaded-driver heartbeat failure and production-signing validation
remain open native gates.
