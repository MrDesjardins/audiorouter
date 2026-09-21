# VST plugin hosting completion plan

Status: implementation complete, archived 2026-09-20. This sub-plan closes
out UI-reachable VST add/remove/manage, canvas visual identity for VST vs.
native nodes, isolated-worker health visibility, mixed physical/application
multi-input mixing, and quick-insert into an existing connection. It does not
close any attended Windows evidence gate; those remain tracked in
[the active plan](../active/current.md).

## Objective

Make VST2/VST3 plugin hosting a complete, discoverable feature of the canvas
editor rather than backend-complete-but-buried functionality: a plugin must
be easy to find, add, remove, and tell apart from a native built-in tool, and
its isolated-worker health must be visible instead of silent. Multiple
capture sources (physical microphones and application-capture processes)
must be combinable into one mixer bus alongside a plugin, matching the
VoiceMeeter-style bus model this project already committed to.

## Scope and decisions

- **Add/remove reuses the existing scan panel**, rather than building a
  second one. The existing `PluginScanPanel` (already implemented, but only
  reachable from the background/advanced panel behind the "Full workspace"
  toggle) is embedded unmodified in a new always-reachable shelf-triggered
  picker dialog, plus its original location.
- **Multi-input routing uses the existing shared-bus mixer/fan-out feature**,
  not a new per-input-to-per-output routing matrix. Offered explicitly as a
  three-way choice to the user (shared bus, true independent per-input
  routing, or single-chain-only); the user chose the shared-bus extension as
  the tractable, already-tested foundation. True independent routing was
  identified as a much larger, multi-day realtime-graph-compiler change and
  was explicitly declined.
- **A disabled plugin node is a safe placeholder**, matching the engine's own
  fail-closed policy (PLUG-05/07-processing.md): a disabled `plugin` node
  compiles to a dry pass-through with no stage inserted, so inserting one
  into a live connection never silently mutes or breaks the signal.
- **Isolated-worker health is published from the worker's own dedicated
  thread via plain atomics**, not by exposing `SupervisedWorkerProcess`
  itself outward. `PluginRuntimeBridge` fully owns and moves its worker into
  a background thread at construction, so no other thread can safely poll it
  directly; the thread already calls `worker.process()` every quantum and now
  also publishes `worker.state()`/`worker.failure_diagnostic()` into new
  atomics after each call.
- **Application-capture identity is re-supplied and revalidated explicitly**
  at every multi-input prepare call, mirroring the existing single-source
  `nativeApplications.prepare` contract, rather than trusting a session's
  persisted node parameters alone.

## Implementation

1. **Shelf-reachable add/remove/manage** (`ui/src/App.tsx`,
   `ui/src/SessionFlowCanvas.tsx`): a "Plugin (VST2/VST3)" shelf button opens
   a modal containing a new `LoadedPluginsPanel` (lists every `plugin` node
   currently in the draft with Select/Unload) above the existing
   `PluginScanPanel`. `PluginNodeInspector` gained an explicit "Unload plugin
   from draft" button. A `PluginScanPanel` mounted twice at once (background
   panel + modal) previously collided on a hardcoded heading `id`; it now
   takes a `headingId` prop.
2. **Canvas visual clarity** (`ui/src/SessionFlowCanvas.tsx`,
   `ui/src/styles.css`): every node card now shows a colored family badge —
   Input (cyan), Native (gold), VST (new purple `--vst-accent`), or Output
   (green) — in addition to the existing signal-flow border color, plus a
   humanized kind label (a plugin shows its exact loaded format, "VST3
   Plugin"/"VST2 Plugin", instead of the generic word "plugin").
3. **Mixed multi-input sources** (`crates/windows-audio`, `crates/control`,
   `contracts/src/index.ts`, `crates/cli`, `ui/src/App.tsx`): a new
   `MultiInputCaptureSource` enum (`Physical(SharedCapture)` /
   `ApplicationLoopback(ProcessLoopbackCapture)`) unifies both under the
   `AudioCaptureSource`/`EndpointLifecycle` traits both already implemented,
   so the realtime pump/mixer needed no changes. `prepare_native_multi_input_worker`
   now takes `&[NativeMultiInputSourceBinding]` instead of a physical-only
   endpoint list, validating each binding against the compiler's source-node
   order and the enabled node's actual kind/identity.
   `nativeMultiInputs.prepare`'s wire params changed from `captureEndpointIds`
   to a `sources` array of `{kind:"physical",...}`/`{kind:"application",...}`
   entries. The `NativeMultiInputPanel` UI gained a second picker for enabled
   application-capture nodes already committed to the session.
4. **Isolated-worker health visibility** (`crates/plugin-host`,
   `crates/engine`, `crates/control`, UI): `RealtimePluginProcessor::health()`
   (default `Unknown`, so existing test doubles need no changes) plus
   `PluginWorkerState`/`PluginWorkerHealth` types live in `engine` (not
   `plugin-host`, which depends on `engine` for this trait — the reverse
   dependency would cycle). `RuntimeGraph`/`RuntimeProcessor` gained
   `plugin_health_for_node`, mirroring the existing
   `processor_telemetry_for_node`/`meter_snapshot_for_node` pattern.
   `native_node_telemetry()` now includes a `plugin: {state, failureCount} |
   null` field per node. This only covers the single "native endpoint
   worker" telemetry path (physical/duplex); the separate multi-input worker
   has never had node telemetry wired at all, a pre-existing gap not closed
   here. The UI shows an alert only when a worker is actually
   failed/quarantined (canvas node, "Loaded plugins" list); the plugin's own
   inspector always shows current state and failure count.
5. **Quick-insert into an existing connection** (`ui/src/draft.ts`,
   `ui/src/SessionFlowCanvas.tsx`, `ui/src/GraphList.tsx`, `ui/src/App.tsx`):
   `insertDraftPluginProcessor` splices a scanned plugin directly into an
   edge, the same edge-splice shape `insertDraftProcessor` already used for
   built-in processors. `onOpenPluginPicker` now optionally carries an edge
   id end to end (canvas edge menu's new "VST plugin…" action, the list
   view's new "Insert VST plugin" button); the existing picker modal runs in
   an "insert into this connection" mode (different heading/wording) when
   opened with an edge id, reusing every existing scan/add/loaded-plugins UI.

## Defects found and fixed during this work

1. The plugin picker's open-focus effect used a generic
   `querySelector("input, button")`, which happened to focus the dialog's
   "Close" button instead of the directory input because Close sits earlier
   in DOM order. Fixed to prefer the first `input`, falling back to the first
   `button` only when no input exists.
2. Rendering `PluginScanPanel` in two places at once (background panel +
   modal) would have collided on a hardcoded heading `id`, breaking
   `aria-labelledby` resolution for whichever instance lost. Fixed via the
   `headingId` prop before it shipped (found by reasoning about dual-mount
   implications, not by a failing test).
3. **A dispatch-blocking bug, found during a full post-implementation
   review.** `validate_method_params` — a top-level allow-list gate that
   rejects any JSON-RPC request field name it doesn't recognize for a given
   method, running before dispatch — still listed
   `nativeMultiInputs.prepare`'s field as `captureEndpointIds` after the
   field was renamed to `sources`. Every real call to this method (UI or
   CLI) would have been rejected outright with `"unknown parameter: sources"`
   before ever reaching the handler, for both the new mixed-source path and
   the pre-existing physical-only path. The field name was duplicated in
   four places in `crates/control/src/lib.rs` (the allow-list, the
   `describe` input schema, the `describe` result schema, and a plain
   description string); only the two closest to the changed code were
   updated at the time, and every test called
   `ControlPlane::prepare_native_multi_input_worker` directly, bypassing
   `validate_method_params` and `dispatch()` entirely. This is the second
   time in this project's history a bug survived full green-test-suite
   status by living in a layer no test exercised (the first was an earlier
   CSS-`display:none` hidden-panels defect) — passing tests confirm the
   paths tests take, not the ones a real client uses. Fixed all four stale
   spots and added
   `native_multi_inputs_prepare_param_allowlist_matches_the_real_request_shape`,
   a fast, non-Windows-gated test that calls the validator directly with the
   real request shape (asserts it passes) and the old stale shape (asserts
   it is now rejected) — a template for any future JSON-RPC field rename.

## Validation

- `cargo build --workspace --all-targets`, `cargo clippy --workspace
  --all-targets` (zero warnings), `cargo test --workspace` (every crate,
  lib+integration+doc tests) — all green throughout, and re-verified clean
  after the allow-list fix. Final per-crate counts: control 179 (was 177),
  engine 119 (was 118), plugin-host 71 (was 70), windows-audio 88, cli 36,
  all others unchanged.
- `npm run typecheck` and the full Vitest suite in `ui/` — 282/282 passing
  (was 273 at the start of this plan's scope).
- `node tools/contracts/check-drift.mjs` — 86 methods, 20 node kinds, 7
  processors, 20 event categories match across UI/CLI/Rust throughout.
- `node tools/docs/validate.mjs` — clean.
- No attended Windows hardware/browser verification was performed or
  claimed; the Chrome browser automation extension was unavailable for the
  entire duration of this plan's work (checked twice, both times
  disconnected), so all verification is portable dev-loop evidence only.

## Known limits

- The multi-input worker's node telemetry (meter/processor/plugin health) is
  not wired at all — a pre-existing gap, unrelated to this plan's scope, not
  closed here. Only the default single "native endpoint worker" path reports
  `nodeTelemetry`.
- True independent per-input-to-per-output routing (each source mapped to
  its own distinct destination, not summed into one shared bus) remains
  unimplemented by explicit user choice; it would require a materially
  larger change to the realtime graph compiler than the shared-bus model
  used here.
- Native VST editor GUI windows remain out of scope, unchanged by this plan;
  confirmed absent at both the internal worker-protocol layer
  (`EditorLifecycle`/`EditorParentAuthorization`/`EditorDescriptor` exist but
  are never exposed through the public API) and the UI, consistent with the
  project's documented native-editor boundary. The inspector's parameter
  controls are the supported control surface instead.
- No attended verification (Windows shell, Narrator, real hardware capture)
  was possible during this plan; all evidence above is portable.

## Handoff

This plan's scope — shelf-reachable plugin add/remove/manage, canvas VST
visual identity, mixed physical/application multi-input mixing, isolated-worker
health visibility, and edge quick-insert — is complete and archived here.
Rollback: revert the commits covering this plan's scope; no persisted session
format, backend contract outside the documented `nativeMultiInputs.prepare`
field rename, or default audio behavior changes. Any future work on the
declined "true independent per-input routing" option, or on wiring the
multi-input worker's node telemetry, should start a new plan rather than
reopening this one.
