# WP-10 virtual-cable UI implementation — 2026-10-11

## Objective and scope

Implement the virtual-cable status, Setup controls, cable binding, and route
creation UI from specification 17 §7.4. Requirement IDs: VCAB-02, VCAB-05,
VCAB-25, VCAB-29, VDEV-10, SEC-08, UI-17.

This archive closes the host implementation scope only. It does not close the
attended app/VM gate, install a driver, or claim endpoint or audio qualification.
The remaining gate is tracked in the
[active virtual-cable plan](../active/virtual-cable.md).

## Implementation

- Added cable status polling through a useSyncExternalStore store. The UI keeps
  the last known status during brief polling failures and clears it on
  disconnect.
- Added Cable input/output entries to both tool surfaces. They are available
  only when cables are installed. New input nodes start on Cable A; output
  nodes start on Cable B. The Properties picker preserves a node's saved cable
  even if Windows no longer lists it.
- Added Setup → AudioRouter cables with status, driver version, cable rows,
  route counters in reserved slots, and a Cable A → Cable B route action. This
  is the pass-through action; the app has no user-facing template picker.
- Added shell-only elevated helper actions and typed host bridge operations
  for install, update, repair, remove, cable count, cable names and low-latency
  configuration. They are not backend RPC methods.
- Added a pre-reduction session-impact confirmation, device format display and
  a reserved 48 kHz float-format hint. The low-latency switch reflects QUERY
  state and requests 128 frames when enabled or 480 when disabled.
- Updated first-run wording to say AudioRouter never installs audio drivers
  on its own.

## Decisions

- Kept status read-only through the backend; privileged lifecycle operations
  remain in the desktop shell and helper.
- Put pass-through creation in Setup because the app has no reachable template
  picker. This does not add a template identifier to the library.
- Preserved UI-17 layout by reserving the status, hint, action and telemetry
  slots.

## Verification

The former WP-10 execution entry recorded these host and browser results; they
are preserved here:

- UI typecheck, format and lint passed; lint reported zero errors and 25
  existing warnings.
- UI unit suite: 522 passed. Focused cable panel/host suite: 34 passed.
- Edge cable UI suite: 4 passed across dark, light and high-contrast themes,
  including format-hint visibility and layout-shift sampling.
- Full browser suite for slice 1: 236 passed, 9 skipped, 0 failed.
- Helper, control and shell checks, Clippy and Rust formatting passed as
  recorded in the WP-10 execution entry.

These checks ran on the host or browser harness. They did not exercise real
UAC prompts, installed-app helper discovery, real status refresh after an
operation, or live driver settings.

## Remaining acceptance gate

WP-11 must provide the adjacent helper and a production-signed four-file
driver package. Then verify the installed app's status, install/update/repair/
remove flows, cable count/name operations, and low-latency state in the
disposable VM. Repeat the three-theme layout check against the integrated app.
Do not install or remove a driver on the development host.

## Rollback and handoff

Rollback is limited to reverting the WP-10 implementation commits; no session
format migration was introduced. Keep the implementation in place while the
signed package and attended VM gate are pending. Resume at the WP-11 package
prerequisite in the active virtual-cable plan.
