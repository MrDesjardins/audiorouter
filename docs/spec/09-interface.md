# 09 — Interface, onboarding, and accessibility

Milestone ownership: M05 visual editor; M07 concurrent/API-driven changes; M08 usability/accessibility evidence. M01 establishes schemas consumed here.

## Layout and interaction model

The primary window has a session sidebar, a central left-to-right canvas, a searchable source/effect/output library, a selected-node inspector, and a compact status strip. Start/stop, input privacy mute, and recording status remain visible. Basic mode exposes useful defaults; channel matrices, raw plugin parameters, and timing details live in advanced inspectors.

The canvas renders backend nodes, ports, edges, and statuses. It does not infer connections from proximity. Dragging from an output port to an input produces a proposed connection; the backend decides whether it is valid. A keyboard connection dialog offers the same action. Template layout is automatic; layout changes never rewire audio.

## Requirements

- **UI-01 — First run.** Explain source → effects → destinations with a small working example. Check backend/driver/device readiness, offer the Gaming + Discord template, select mic/headphones, provision required virtual buses through the shared API, and guide third-party app selections. Include a meter/test step for each destination. Mic monitoring begins muted; recording begins unarmed.
- **UI-02 — Canvas.** Support add/search, drag, connect/disconnect, rename, duplicate, delete, multi-select, zoom/pan, fit, and tidy layout. Provide undo/redo for graph edits using backend revisions. A dragged preview is visually provisional until committed. Persist positions separately from audio state.
- **UI-03 — Connections.** Label ports by role and channels; distinguish audio and sidechain ports. Highlight the full upstream/downstream path on selection. Show invalid target reasons before drop where known, then display backend rejection if state changed. Insert-a-mixer and remove-and-reconnect are explicit previewable operations.
- **UI-04 — State.** Distinguish running/stopped, enabled/bypassed/muted, missing/faulted, and recording/paused using text/icons as well as color. Animate flow only when relevant activity exists; a static connected line means configured connectivity. Silent-but-valid is different from disconnected. Provide reduced-motion mode.
- **UI-05 — Inspector.** Offer sliders plus precise text entry, unit labels, reset, presets, pre/post meters, and an effect of change summary. Rejected values remain an editable draft with a reason; do not display them as active. Expensive rebuild operations show progress and preserve last committed values on failure.
- **UI-06 — Routing explanation.** Selecting an output shows “Receives audio from” with all source paths, processing, muted/bypassed sections, and latency estimates. `Voice Chat` must visibly exclude desktop/call-return in the reference template. Explanations come from `routes.inspect` and remain available headlessly.
- **UI-07 — Live editing.** Parameter changes use bounded/coalesced requests during dragging and one final committed target. Topology edits use plan/commit; local UI optimism never overrides a backend conflict. When another client edits, merge nonconflicting presentation state and render the new authoritative graph. Show who changed what using client identity, not guessed person names.
- **UI-08 — Safety and errors.** Feedback/duplicate warnings identify actual paths and remedies. Missing devices remain on the canvas. Offer rebind, retry, inspect, or stop according to backend capability. No hidden fallback to a new microphone. An emergency privacy mute affects physical capture contributions immediately through the backend; clearing it never starts stopped sessions or recorders.
- **UI-09 — Sessions/presets.** Create, duplicate, rename, export/import, start/stop, and designate startup sessions. Imports open stopped with unresolved bindings highlighted. Include templates for UC-01, processed mic, app recording, and mix-minus. Presets expand into inspectable ordinary nodes.
- **UI-10 — Background controls.** Tray controls list session states, mic privacy mute, recording states, and open/quit actions. “Close window” keeps audio running; “Quit and stop audio” explicitly stops the backend after recorder finalization. Offer optional always-on-top compact meters and global shortcuts, with conflict/rebinding support. Shortcuts dispatch API actions.
- **UI-11 — Accessibility.** All essential actions work without dragging or a mouse. Provide a structured list/tree view of routes, descriptive accessible names, logical focus order, focus restoration after dialogs, high-contrast/light/dark modes, 200% zoom, and non-color status cues. Target WCAG 2.2 AA applicable criteria plus Windows Narrator testing; record the exact audit checklist in M05.
- **UI-12 — Responsiveness.** Remain usable at 1280×720 and 100–200% Windows scaling; panels may collapse. Long names wrap/truncate with accessible full labels. Virtualize large lists/canvas where needed and throttle meters. No web audio engine or processing in the renderer; browser microphone access is unnecessary.
- **UI-13 — API availability.** If backend disconnects, display a reconnecting/offline state and retain the last snapshot as stale. Disable dependent mutations, do not queue unbounded audio edits, and resync snapshot/revision on reconnect. Closing/reopening a plugin editor does not affect its audio.
- **UI-14 — Recorder UI.** Expose record/arm/pause/split/stop, destination, disk error, duration, and file library actions. Clearly separate deleting a node, removing a library entry, and recycling a recording file. Do not bury recording activity when the session sidebar changes.

## Defaults that reduce work

Give nodes descriptive names such as `USB mic`, `Voice EQ`, and `To Discord`. Position sources on the left and sinks on the right. Offer mono-mic to stereo mapping automatically as an explicit edge matrix. Use preconfigured conservative voice presets and show their purpose. Do not make the user select a sample rate or buffer period during routine onboarding; show negotiated values under diagnostics.

Device selection should show both a familiar label and a disambiguator such as USB interface/role. The user can audition input levels before starting a route, but any microphone test is an explicit capture action with visible state. Setup persists incomplete drafts without activating them.

## Verification

Test keyboard-only completion of UC-01, Narrator discovery of ports/connections, high contrast, reduced motion, 200% scaling, error recovery, and external CLI edits while an inspector is open. Record route-comprehension observations in addition to task duration. UI automation may use fake devices; actual sound, driver lifecycle, and third-party app setup need Windows integration evidence.
