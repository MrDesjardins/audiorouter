# M03 virtual-routing contract evidence

## 2026-09-06 — capability-contract foundation

The authoritative Rust and shared TypeScript node registries now include
`virtual-render-source@1` and `virtual-capture-sink@1`. Both advertise an
explicit unavailable capability reason: `requires M03 managed virtual driver`.
The UI library exposes both entries for discovery but keeps them disabled, so
the editor cannot imply that a virtual endpoint exists or install a driver.

The portable engine matches both kinds as device-bound placeholders. Disabled
nodes therefore retain the existing silence-boundary behavior; enabled nodes
do not create a fake bus or bypass resource validation. M03 driver installation,
bridge ownership, endpoint identity, and Windows routing remain open.

Validation: all locked Rust workspace tests and strict Clippy passed; the
contracts TypeScript check passed; UI Vitest passed 56 tests using Vite's
runner config loader. The UI production build could transform and render but
Windows returned `EPERM` while creating the configured or alternate output
directory, so no production-build pass is claimed. No driver, audio endpoint,
or machine audio configuration was changed.
