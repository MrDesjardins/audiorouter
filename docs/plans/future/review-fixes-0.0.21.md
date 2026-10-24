# Code review follow-ups after 0.0.21 — 2026-10-23

A quick review of every change from `v0.0.20` to the 0.0.21 release branch
(plugin windows, Output Switch, Sound Check, app sources and groups, hidden
devices, the dedicated audio thread, canvas layout) found nine issues. Five
were fixed before the release. On 2026-10-23 the owner asked for every
remaining item to be fixed; all are resolved on the branch
`fix/review-followups-0.0.21` (see the active plan for checks).

## Fixed in 0.0.21

| Finding | Fix | Regression |
| --- | --- | --- |
| The plugin bridge sent its slider list with every quantum, so a knob turned in a VST3 editor snapped back within ~3 ms whenever the node had saved slider values | Sliders are sent only when their list changes (`changed_parameters`) | `slider_events_are_sent_once_per_change_so_editor_edits_survive` |
| An Output Switch side feeding a node that mixes several feeds either leaked while unselected or muted the other feeds | Refused in 0.0.21; supported since R1 below | see R1 |
| A one-output route through an Output Switch was sent to the single-endpoint worker, which rejects the switch | `needsNativePaths` selects the multi-path worker for any connected switch | `uses the multi-path worker for a route through an Output Switch` |
| The start-up latency query ended the plugin worker when the plugin could not report a delay | A worker `Failure` answer counts as no delay; pipe errors still end the worker | existing latency tests |
| Two doc comments were attached to the wrong functions (`load_plugin_state_asset`, `continuity_counts`) | Moved back | — |

## Resolved after 0.0.21

### R1 — Mix an Output Switch side with other feeds — fixed

Each used side of an Output Switch is now its own gated chain
(`OutputProcessorChain::gate`) in both multi-path compilers, so a Mixer sums
the gated side with its other feeds. A node keeps, as sink gates, only the
gates every one of its feeds passes through. Regression
`output_switch_side_mixes_with_other_feeds_and_gates_tools_after_it` (both
switch positions, Discord level unchanged); it fails with the side gate
disabled. The break-before-make fade is the existing
`output_switch_routes_one_side_and_breaks_before_making_the_other_live`.

### R2 — Meters and tools after the unselected side saw audio — fixed

Same change: the gate is applied where the side leaves the switch, so a
Meter, Duck key or plugin after the unselected side sees silence. The sink
still re-applies its gates, so a tail or plugin noise after the side stays
silent too. Regressions: the R1 test (Meter on side B) and
`output_switch_gates_a_meter_on_the_unselected_side_of_a_single_chain`
(fails with the side gate disabled).

### R3 — Recorder taps while the control thread is blocked — not a defect

Every production file recorder (WAV, FLAC, MP3, one-click and node
recordings) is created by `create_file_recorder_with_config`, which wraps it
in `ThreadedRecorderWorker`: its own thread drains the queue every ≤ 2 ms.
The control thread's `drain_pending` only reads progress and failures. New
regression `the_recorder_thread_writes_audio_while_the_control_thread_never_drains`
feeds ten times the queue capacity with no control-thread drain and parses
the WAV.

### R4 — Unreachable "Replace input connection" code — removed

The state, handler, both buttons and the Workbench prop are gone. Decision:
a backwards drag needs no explanation, because the canvas only connects an
output handle to an input handle; a second source on an occupied input adds
a Mixer as before.

### R5 — Duplicated COM file-dialog code — merged

`src-tauri/src/session_file_dialog.rs` has one thread/COM/dialog path driven
by a `DialogRequest` (title, filter, default extension, file name).

### R6 — Site comparison wording for VST2 — restored and improved

`site/compare.html` again says AudioRouter loads x64 VST2 and VST3 effects
and opens their editors live, names UADx, and states that the eight free
UADx effects were tested on 2026-10-22. VST2 support is on for every x64
VST2 effect on Windows (`PluginCompatibility::SupportedVst2X64Gated` is a
name, not a switch). The Wave Link "Third-party effects" row now marks
AudioRouter stronger (VST2 and VST3 against VST3 only).

### R7 — Edge suite flakes on hosted runners — fixed or contained

From the failed first attempts' logs:

- `session-workflows` (Delete after restart): a **product bug**. Deleting a
  session while the session list was still loading after start-up left the
  window on the deleted session. Delete now asks the backend for the
  remaining sessions (`deleteSession` in `ui/src/App.tsx`).
- `canvas-layout-review` "new connections keep both chosen sides": the drag
  started before the removed connection left the draft; the test now waits
  for the removal.
- `compact-workspace` "debounced live autosave": polling missed the 400 ms
  autosave state during a runner stall; the test records every state with a
  MutationObserver instead.
- `canvas-layout` fixture: the first page load exceeded the 5 s expect
  timeout; CI now uses 10 s.
- `plugin-inspector`: 60 s budget (#63).
- CI retries a failed Edge test once; the list reporter names it "flaky", so
  a flake no longer fails the job but stays visible.
