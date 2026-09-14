# VDEV-12 bridge failure event evidence

Date: 2026-09-14

The control plane now advertises `virtualBridge.failed` in its discoverable
state-category catalog. Native capture-sink, render-source, and duplex
heartbeat containment publishes this event only after detaching the affected
binding, closing its lease, and deactivating the portable bridge. The affected
bus identifier is carried as the bounded operation identity; native HRESULTs,
paths, and private diagnostics remain in the structured supervisor response
and are not retained in the event stream.

Verification on Windows workspace `C:\code\audiorouter`:

- `cargo test -q -p audiorouter-control --locked -- --test-threads=1` — 150
  passed, 2 guarded live tests ignored.
- `cargo clippy -q -p audiorouter-control --locked --all-targets -- -D warnings` — passed.
- `npm.cmd --prefix ui run test -- --run` — 183 passed.
- `npm.cmd --prefix ui run typecheck` — passed.
- `tests\acceptance\docs.ps1` — 52 Markdown files and 190 local links passed.

No endpoint, default-device, driver, or other machine audio configuration was
changed. Loaded-driver heartbeat failure and production-signing validation
remain open native gates.
