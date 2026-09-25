# 09 — Interface, onboarding, and accessibility

Milestone ownership: M05 visual editor; M07 concurrent/API-driven changes; M08 usability/accessibility evidence. M01 establishes schemas consumed here.

## Layout and interaction model

The primary window gives the left-to-right canvas the available space and puts a clearly selected, tabbed workbench on the right. Its tabs provide addable input/processing/output tools, selected-node properties, session actions, MCP setup/activity, and diagnostics. Session selection and revision actions live in the Session tab; do not reserve a permanent session rail or duplicate the same actions in a tall header. Tool cards use an icon, title, and short description; unavailable capabilities say why. Keep start/stop, privacy mute, and recording state easy to find. At narrow widths, panels may stack without covering the graph. The status workspace pairs aligned session/privacy actions on the left with short, ordered guidance on the right. Detailed controls remain grouped under the selected workbench tab; advanced groups use progressive disclosure instead of one unbounded panel wall.

The MCP tab explains local assistant setup using the installed CLI path, database, named pipe, and an explicitly authorized client ID. It guides least-privilege access and shows a bounded stream of incoming tool-call names, safe argument summaries, and outcomes. Never record audio/media payloads, credentials, or hidden model reasoning. A Diagnostics view exposes recent UI failures and graph node/edge/revision checkpoints; the native backend log records RPC method, outcome, and a privacy-safe state summary in a bounded local file. No diagnostic writer runs in the realtime audio callback.

The canvas renders backend nodes, ports, edges, and statuses. It does not infer connections from proximity. Dragging from an output port to an input produces a proposed connection; the backend decides whether it is valid. A keyboard connection dialog offers the same action. Template layout is automatic; layout changes never rewire audio.

## Requirements

Current VB-Cable-first onboarding selects and verifies existing VB-Cable,
Voicemeeter, physical WASAPI, or other installed virtual endpoints through the
shared API, then guides third-party application selection. It must not imply
that AudioRouter creates or installs a virtual device. The managed-bus
provisioning language in UI-01 remains a deferred signed-driver-profile path;
missing managed-driver capability is reported explicitly rather than treated
as a failed current-profile setup.

- **UI-01 — First run.** Explain source → effects → destinations with a small working example. Check backend/device readiness, offer the Gaming + Discord template, select mic/headphones and existing supported endpoints in the current VB-Cable-first profile, and guide third-party app selections. Managed virtual-bus provisioning remains a deferred signed-driver-profile option. Include a meter/test step for each destination. A Test Signal node provides in-place Play and Stop controls for its connected session route. An Audio File input imports bounded WAV/MP3 media, exposes only Play and Stop on its compact node, and keeps Pause/Resume, file selection, and looping in its inspector. It follows ordinary graph plan/commit plus session lifecycle. Mic monitoring begins muted; recording begins unarmed.
- **UI-02 — Canvas.** Support add/search, drag, connect/disconnect, rename, duplicate, delete, multi-select, zoom/pan, fit, and tidy layout. Provide undo/redo for graph edits using backend revisions. Node deletion is immediately undoable without a confirmation dialog and works with the Delete key while the canvas is focused. A dragged preview is visually provisional until saved. Persist positions separately from audio state.
- **UI-03 — Connections.** Label ports by role and channels; distinguish audio and sidechain ports. Mark the selected path without dimming unrelated active routes or making selection look like audio activity. Pause, remove, and insert controls have distinct icons and explanatory tooltips. Show invalid target reasons before drop where known, then display backend rejection if state changed. Insert-a-mixer and remove-and-reconnect are explicit previewable operations.
- **UI-04 — State.** Distinguish running/stopped, enabled/bypassed/muted, missing/faulted, and recording/paused using text/icons as well as color. Animate flow only when relevant activity exists; a static connected line means configured connectivity. Silent-but-valid is different from disconnected. Provide reduced-motion mode.
- **UI-05 — Inspector.** Offer sliders plus precise text entry, unit labels, reset, presets, pre/post meters, and an effect of change summary. Rejected values remain an editable draft with a reason; do not display them as active. Expensive rebuild operations show progress and preserve last committed values on failure. Audio File settings include descriptive source state, a WAV/MP3 picker, and a loop toggle; decoding is backend-owned and bounded. Temporary voice takes use an already-connected Recorder node on a running route, stop at 120 seconds, and become expiring backend media after the temporary WAV and library row are removed. The local shell may request `Record`; it does not gain capture-device or endpoint-administration authority. Voice recording controls may not capture through renderer/browser APIs.
- **UI-06 — Routing explanation.** Selecting an output shows “Receives audio from” with all source paths, processing, muted/bypassed sections, and latency estimates. `Voice Chat` must visibly exclude desktop/call-return in the reference template. Explanations come from `routes.inspect` and remain available headlessly.
- **UI-07 — Live editing.** Parameter changes use bounded/coalesced requests during dragging and one final committed target. Topology edits use backend plan/commit; the UI presents one Save route action and requests a second confirmation only for warnings. Local UI optimism never overrides a backend conflict. When another client edits, merge nonconflicting presentation state and render the new authoritative graph. Show who changed what using client identity, not guessed person names.
- **UI-08 — Safety and errors.** Feedback/duplicate warnings identify actual paths and remedies. Missing devices remain on the canvas. Offer rebind, retry, inspect, or stop according to backend capability. No hidden fallback to a new microphone. An emergency privacy mute affects physical capture contributions immediately through the backend; clearing it never starts stopped sessions or recorders.
- **UI-09 — Sessions/presets.** Create, duplicate, rename, export/import, start/stop, and designate startup sessions. Imports open stopped with unresolved bindings highlighted. Include templates for UC-01, processed mic, app recording, and mix-minus. Presets expand into inspectable ordinary nodes. Header Play/Stop starts or stops the visible canvas route; it does not start a stopped Test Signal or Audio File source. Test Signal Play/Stop controls only that tone through the authorized source transport and may first start a stopped route. A routed, enabled Test Signal Play remains actionable when endpoint readiness is missing so the backend can report the specific start failure. Audio playback still requires the session's exact native endpoint preparation.
- **UI-10 — Background controls.** Tray controls list session states, mic privacy mute, recording states, and open/quit actions. “Close window” keeps audio running; “Quit and stop audio” explicitly stops the backend after recorder finalization. Offer optional always-on-top compact meters and global shortcuts, with conflict/rebinding support. Shortcuts dispatch API actions.
- **UI-11 — Accessibility.** All essential actions work without dragging or a mouse. Provide a structured list/tree view of routes, descriptive accessible names, logical focus order, focus restoration after dialogs, high-contrast/light/dark modes, 200% zoom, and non-color status cues. Target WCAG 2.2 AA applicable criteria plus Windows Narrator testing; record the exact audit checklist in M05.
- **UI-12 — Responsiveness.** Remain usable at 1280×720 and 100–200% Windows scaling; panels may collapse. Long names wrap/truncate with accessible full labels. Virtualize large lists/canvas where needed and throttle meters. No web audio engine or processing in the renderer; browser microphone access is unnecessary.
- **UI-13 — API availability.** If backend disconnects, display a reconnecting/offline state and retain the last snapshot as stale. Disable dependent mutations, do not queue unbounded audio edits, and resync snapshot/revision on reconnect. Closing/reopening a plugin editor does not affect its audio.
- **UI-14 — Recorder UI.** Expose record/arm/pause/split/stop, destination, disk error, duration, and file library actions. Clearly separate deleting a node, removing a library entry, and recycling a recording file. Do not bury recording activity when the session sidebar changes.
- **UI-15 — Signal flow visualization.** On active connected routes, show sound traveling in the edge direction between nodes, including from application-capture and microphone inputs through processing to destinations. Represent recent signal level with a clearly visible, smoothly varying stroke width (bounded by a design-system minimum and maximum), with a restrained moving highlight or pulse; do not use a single 1 px line as the only activity cue. Animate only when fresh backend meter/telemetry reports signal on that route, and keep configured-but-silent, stopped, muted, bypassed, stale, disconnected, and faulted states visually distinct. Base width on bounded peak/RMS meter values using smoothing and hold/decay so ordinary audio is legible without flicker; clamp values and never imply activity from a connection alone. Flow direction follows graph topology. The effect remains legible at supported zoom levels and high contrast, has a reduced-motion/static equivalent, and exposes equivalent text/meter values to assistive technology. It is presentation only: no browser audio processing, inferred routing, or per-frame backend polling; consume the existing bounded meter/event path and respect NFR-08 freshness/rate limits.

The Advanced EQ property editor presents the backend `parametricEq@1` node as a
logarithmic frequency graph with up to sixteen selectable points. Dragging a
point changes its frequency and applicable gain; precise frequency, gain,
filter type, and Q controls remain keyboard accessible. The response curve
comes from `processors.response`, not an independent renderer DSP model.

## Defaults that reduce work

Give nodes descriptive names such as `USB mic`, `Voice EQ`, and `To Discord`. Position sources on the left and sinks on the right. Offer mono-mic to stereo mapping automatically as an explicit edge matrix. Use preconfigured conservative voice presets and show their purpose. Do not make the user select a sample rate or buffer period during routine onboarding; show negotiated values under diagnostics.

Device selection should show both a familiar label and a disambiguator such as USB interface/role. The user can audition input levels before starting a route, but any microphone test is an explicit capture action with visible state. Setup persists incomplete drafts without activating them.

## Follow-up interaction slice

The next M05 usability slice adds a bounded, graph-native **Test Signal**
source and visible meters. The source must be an ordinary inspectable graph
node, remain stopped/unarmed until the user deliberately starts the session,
and expose frequency, level, and duration controls through the same validated
parameter path as other nodes. It must not use Web Audio or browser microphone
access. Destination meters must show signal presence, peak/RMS values, and
clipping/stream state so a user can confirm a route before involving a
microphone, VoiceMeeter, Discord, or another application.

VoiceMeeter and other installed virtual endpoints are compatible exploration
boundaries in the current VB-Cable-first profile. The UI should say that the
third-party application may remain open; the user only needs to stop or close
it when it owns the exact endpoint AudioRouter is explicitly preparing. The UI
must distinguish “no signal,” “not prepared,” “stopped,” and “endpoint owned by
another client.”

The editor remains transport-independent: the same versioned backend contract
must be usable by the desktop shell, CLI, and MCP. Browser access is not a
current capability because the implemented transport is an authenticated
same-user Windows named pipe; a loopback HTTP/WebSocket adapter requires a
separate origin, enrollment, authorization, rate-limit, and lifecycle gate.

The Test Signal acceptance must include: add source, set a conservative tone,
connect it to an enabled physical output, plan/commit, prepare the exact
endpoint, click Play on the Test Signal node, observe meters, click Stop, and
confirm that no endpoint default or persistent audio configuration changed.
The node controls start and stop the entire session, so the UI states this
clearly when other sources are present. An unrouted or disabled Test Signal
cannot be played from its node card. The controls do not prepare endpoints or
grant `deviceAdministration` implicitly.

The signal-flow visualization acceptance must additionally cover a live
microphone source and an application-capture source. For each, verify that
fresh signal produces a directional, level-responsive edge stroke through
connected nodes; silence decays to the configured static edge; stopped,
muted, stale, and faulted states do not suggest healthy flow; and changes to
stroke width follow meter updates without changing graph state. Verify
reduced-motion, keyboard/list access to equivalent levels and states, and
readability at 100–200% scaling and high contrast. A static/fake meter fixture
can verify rendering behavior, but live source visibility needs attended
Windows evidence.

## Session refresh and audio readiness

Session refreshes must preserve an unchanged graph's draft, selection, layout,
and action messages. Incoming newer revisions replace a clean draft; they leave
a dirty draft intact with explicit conflict guidance. Older snapshots and local
creation responses must not override a newer committed revision. Graph plans
and route inspection target the selected session, and commit success updates the
saved revision without requiring reconnect. Play may temporarily preview the
validated current draft without saving a new revision when the selected session
has a prepared native endpoint. Stop ends the preview and leaves the saved
session unchanged. Unsupported source combinations fail with an explanation;
a simulated runtime must never be presented as successful audio playback.

Canvas rendering must retain measured node dimensions and in-progress positions
across meter refreshes. Capture privacy mute must not hide measured signal flow
from Test Signal or Audio File sources; capture-only paths remain marked muted.
Native preparation guidance distinguishes control connection readiness from
audio readiness and explains the exact endpoint and authorization steps.

## Session refresh and audio readiness

Session refreshes must preserve an unchanged graph's draft, selection, layout,
and action messages. Incoming newer revisions replace a clean draft; they leave
a dirty draft intact with explicit conflict guidance. Older snapshots and local
creation responses must not override a newer committed revision. Graph plans
and route inspection target the selected session, and commit success updates the
saved revision without requiring reconnect. Play may temporarily preview the
validated current draft without saving a new revision when the selected session
has a prepared native endpoint. Stop ends the preview and leaves the saved
session unchanged. Unsupported source combinations fail with an explanation;
a simulated runtime must never be presented as successful audio playback.

Canvas rendering must retain measured node dimensions and in-progress positions
across meter refreshes. Capture privacy mute must not hide measured signal flow
from Test Signal or Audio File sources; capture-only paths remain marked muted.
Native preparation guidance distinguishes control connection readiness from
audio readiness and explains the exact endpoint and authorization steps.

## Verification

The editor presents one Save action in the main header; graph planning and
commit remain backend steps behind that action. Session management contains
switch, create, duplicate, rename, delete, undo, and revert controls without
exposing revision or plan terminology as the primary workflow. The header
shows whether audio is stopped, starting, or running, and Play failures leave
an actionable message visible even while Properties is open. Properties must
not resize the canvas or sidebar. The Physical Output property chooses the
exact render device. When all required exact device selections are present,
Play may prepare them through the authorized backend before starting; it must
not choose a substitute capture device. Devices explains any extra adapter
binding and provides manual recovery controls.

Session refreshes must preserve an unchanged graph's draft, selection, layout,
and action messages. Incoming newer revisions replace a clean draft; they leave
a dirty draft intact with explicit conflict guidance. Older snapshots and local
creation responses must not override a newer committed revision. Graph plans
and route inspection target the selected session, and commit success updates the
saved revision without requiring reconnect. Play may temporarily preview the
validated current draft without saving a new revision when the selected session
has a prepared native endpoint. Stop ends the preview and leaves the saved
session unchanged. Unsupported source combinations fail with an explanation;
a simulated runtime must never be presented as successful audio playback.

Canvas rendering must retain measured node dimensions and in-progress positions
across meter refreshes. Capture privacy mute must not hide measured signal flow
from Test Signal or Audio File sources; capture-only paths remain marked muted.
Native preparation guidance distinguishes control connection readiness from
audio readiness and explains the exact endpoint and authorization steps.

Test keyboard-only completion of UC-01, Narrator discovery of ports/connections, high contrast, reduced motion, 200% scaling, error recovery, and external CLI edits while an inspector is open. Record route-comprehension observations in addition to task duration. UI automation may use fake devices; actual sound, driver lifecycle, and third-party app setup need Windows integration evidence.
