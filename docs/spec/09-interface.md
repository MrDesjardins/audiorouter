# 09 — Interface, onboarding, and accessibility

Milestone ownership: M05 visual editor; M07 concurrent/API-driven changes; M08 usability/accessibility evidence. M01 establishes schemas consumed here.

## Layout and interaction model

**UI-16 — Visual groups.** Tools offers a rounded rectangular Group annotation
behind nodes and wires. Properties edits its name, background color, opacity
(default 5%; integer slider from 1–100%), and caption font size (default 18 px,
adjustable from 12–48 px). Drag the caption or background to move it; resize using its border.
Its interior must not block nodes or wires. Groups have no audio ports or
routing effect and do not require Save/Stop. Persist per-session beside local canvas layout; they are not
included in exported audio session files in this initial version.

Groups offer a **Move contents with group** toggle beside Lock position and
size. New groups enable it; existing annotations without that saved property
retain the previous rectangle-only movement. A tool or nested group belongs
to its smallest fully enclosing group, with stable group ID as the tie-breaker
for equal overlapping candidates. Partial overlap does not count. Membership
is frozen for a drag. A carrying group translates all owned descendants once,
preserving their relative positions; inner toggles govern dragging that inner
group itself. Resizing a frame does not scale or move contents. Locked frames
remain fixed, including during ancestor movement: carrying an ancestor with a
locked descendant is blocked until unlocked or carrying is disabled. Tools
inside a locked annotation remain individually editable.

Select multiple tools and/or groups with modifier-click or marquee selection
to show layout controls in Properties. Provide left/center/right and
top/middle/bottom alignment, matching smallest/largest widths and heights, and
horizontal/vertical equal edge spacing. Show conventional icons beside the
visible action labels. Multi-selection also exposes Lock and Move contents
for selected groups only, with mixed checkbox states; applying an option
updates every explicitly selected group, including unlocking locked groups,
without changing geometry. Selected carrying ancestors count as
one movement unit; sizing changes only explicitly selected rectangles. Locked
frames stay fixed and participate as alignment/distribution anchors. Equal
spacing keeps endpoints fixed and distributes between locked anchors; reject
insufficient room without moving anything. Tool sizes preserve usable minimum
dimensions. Positions, tool sizes and group settings persist per session as
local presentation preferences and never change audio topology. Provide local
Undo layout/Redo layout for geometry edits and connector-side choices; restore
only existing items and respect their current locks. Later layout edits or
undo retire the temporary Undo arrange shortcut. Layout history resets when
switching sessions. Invalid saved connector preferences fall back to defaults;
optional presentation storage failures never interrupt editing.

Layout history is bounded to 20 entries per direction. No-op commands,
including repeating an unchanged Arrange, neither add an undo step nor clear
redo. Undo arrange uses the same history path and permits Redo layout; it
does not maintain a second geometry snapshot. History never changes current
group lock/carry settings or resurrects deleted graph objects.

Decode local position, size and connector records into their allowed fields;
foreign saved fields must not override geometry identity. Match backend entity
ID bounds (128 UTF-8 bytes). Reject local presentation JSON above 1,048,576
characters before parsing; a rejected record uses default presentation and
does not change the graph. Oversized writes leave prior persisted data intact.
Unavailable local-storage access, including a denied browser getter, leaves
the in-memory editor usable.

The API tab displays the optional localhost HTTP listener URL/port, start/stop,
the "Who can connect" choice and network URL (HTTP-09),
explicit token reveal/copy and a link to its local Swagger documentation.
See [HTTP acceptance](16-local-http-api.md).

The primary window gives the left-to-right canvas the available space and puts a clearly selected, tabbed workbench on the right. Its tabs provide addable input/processing/output tools, selected-node properties, session actions, MCP setup/activity, and diagnostics. Session selection and revision actions live in the Session tab; do not reserve a permanent session rail or duplicate the same actions in a tall header. Tool cards use an icon, title, and short description; unavailable capabilities say why. Keep start/stop in the top bar, alongside the live run state. Global microphone privacy mute remains available through Properties and its shortcut; omit the ambiguous "Mute mic" header action and the tray mute toggle. Do not add a separate Route status panel: the fixed top bar already shows run state and a second summary has repeatedly destabilized workspace height. At narrow widths, panels may stack without covering the graph. Detailed controls remain grouped under the selected workbench tab; advanced groups use progressive disclosure instead of one unbounded panel wall.

The MCP tab explains local assistant setup using the installed CLI path, database, named pipe, and an explicitly authorized client ID. It guides least-privilege access and shows a bounded stream of incoming tool-call names, safe argument summaries, and outcomes. Never record audio/media payloads, credentials, or hidden model reasoning. A Diagnostics view exposes recent UI failures and graph node/edge/revision checkpoints; the native backend log records RPC method, outcome, and a privacy-safe state summary in a bounded local file. No diagnostic writer runs in the realtime audio callback.

The canvas renders backend nodes, ports, edges, and statuses. It does not infer connections from proximity. While idle, show blue sending/start handles on output ports of every source and tool; hide orange receiving handles on input ports. Starting at a sending handle hides sending handles and reveals receiving handles. Color communicates the gesture role; accessible labels retain the actual output/input port roles. On drop, normalize either gesture to the canonical output-to-input edge and let the backend decide whether it is valid. Restore idle handles when the gesture ends or is cancelled. A keyboard connection dialog offers the same graph action. Template layout is automatic; layout changes never rewire audio. Application-capture nodes show whether the process is connected, closed, reconnecting, ambiguous, unsupported, or not prepared. Process presence does not imply signal; only fresh audio meters animate the route.

Device choices show channel layout before the endpoint name so similarly named stereo and multichannel endpoints are easy to distinguish. Keep distinct endpoint IDs selectable; never merge or suppress them based only on similar friendly names. A graph revision conflict tells the user the save was rejected, that the local draft remains, and that the latest saved graph must be reviewed before creating a new plan; it must not suggest retrying the same stale plan.

Remember the last selected session as a local workspace preference and restore
it when it remains available; restoring selection never starts audio. Clicking
the session name in the top-left header opens the Session workbench tab, with
the same action available from the keyboard. The header run state uses plain
text and a status dot, without a button-shaped background or border. Avoid a
second persistent sentence repeating that audio is running. Messages and
unfed-route warnings appear in a nonmodal bottom-right popover, outside the
workspace flow, with no empty reserved strip. Keep dismiss/recovery actions
accessible and long messages readable without moving the canvas. Canvas port guidance and connection-state samples live in a
"Canvas legend" disclosure that starts collapsed and expands on user action.
Clicking a canvas node or selecting one in the graph list opens Properties immediately.
Adding a tool may select the new node while keeping Tools open for repeated
additions.

Enabled and Bypass controls state their effective semantics: off inputs/sinks
are silent, off effects use their dry bypass, and Mute silences a connected
route. Changing these flags during playback saves and applies the planned
flag change immediately without an explicit Save or Stop; unrelated unsaved
names, parameters and wiring remain in the local draft. Retain prepared
streams and workers. Off native devices
contribute silence while remaining open until Stop, allowing live re-enable.
Display the changed state only after native activation is acknowledged;
otherwise explain that the playing route still uses its previous settings.
Timing does not show active
processing values for off/bypassed nodes or a stopped session.

Independent native paths publish actual source, prepared tool and mapped
destination meters in `system.diagnostics.nodeTelemetry`, using the same
meter fields as a single route. Shared plugin members expose the processed
group output at their graph boundaries; these meters do not claim separate
internal plugin measurements. A running worker or timing entry alone must
never animate a connection as though it carries sound.

## Requirements

Current VB-Cable-first onboarding selects and verifies existing VB-Cable,
Voicemeeter, physical WASAPI, or other installed virtual endpoints through the
shared API, then guides third-party application selection. It must not imply
that AudioRouter creates or installs a virtual device. The managed-bus
provisioning language in UI-01 remains a deferred signed-driver-profile path;
missing managed-driver capability is reported explicitly rather than treated
as a failed current-profile setup.

- **UI-01 — First run.** Explain source → effects → destinations with a small working example. Check backend/device readiness, offer the Gaming + Discord template, select mic/headphones and existing supported endpoints in the current VB-Cable-first profile, and guide third-party app selections. Managed virtual-bus provisioning remains a deferred signed-driver-profile option. Include a meter/test step for each destination. A Test Signal node provides in-place Play and Stop controls for its connected session route. An Audio File input imports bounded WAV/MP3 media, exposes only Play and Stop on its compact node, and keeps Pause/Resume, file selection, and looping in its inspector. It follows ordinary graph plan/commit plus session lifecycle. Mic monitoring begins muted; recording begins unarmed.
- **UI-02 — Canvas.** Support add/search, drag, connect/disconnect, rename, duplicate, delete, multi-select, zoom/pan, fit, and tidy layout. Provide undo/redo for graph edits using backend revisions. Node deletion is immediately undoable without a confirmation dialog and works with the Delete key while the canvas is focused. A dragged preview is visually provisional until saved. Persist positions separately from audio state. Copying a canvas node copies its kind, name, flags, and configuration into an app-local clipboard; pasting creates a new node with no graph edges or side-chain node reference. The clipboard survives switching sessions in the same app window. Offer Copy on a node's context menu, Paste on the canvas context menu, and Ctrl+C/Ctrl+V when a node is selected; pasted nodes join the current draft and use ordinary plan/commit validation.
- **UI-03 — Connections.** Label ports by role and channels; distinguish audio and sidechain ports. While idle, show blue sending/start handles on every node's output ports and hide orange receiving/input handles. During a sender-start drag, show only receiving handles until drop/cancel. Tools and Mixers use the same affordance as microphones and other sources. Create the same canonical output-to-input graph edge regardless of drag direction; keep keyboard connection available. Every output port may feed multiple independent downstream inputs regardless of source or tool kind; an ordinary destination input accepts one incoming edge. A Mixer input accepts multiple explicit source edges and its output may feed one or more destinations. When a second source is connected to an already-fed ordinary destination input, insert a draft Mixer before that destination and connect both the existing and new sources to it; preserve the existing route and make the topology change undoable. When a Physical Output is already fed directly by a source that also feeds the requested Mixer, route the output through that existing Mixer and remove the now-redundant direct edge so one signal is not heard twice. Mark the selected path without dimming unrelated active routes or making selection look like audio activity. Pause, remove, and insert controls have distinct icons and explanatory tooltips. Show invalid target reasons before drop where known, then display backend rejection if state changed. Insert-a-mixer and remove-and-reconnect are explicit previewable operations.
- **UI-04 — State.** Distinguish running/stopped, enabled/bypassed/muted, missing/faulted, and recording/paused using text/icons as well as color. Application-capture sources additionally show process lifecycle: connected, app closed, reconnecting, ambiguous, unsupported, and not prepared. Animate flow only when relevant activity exists; a static connected line means configured connectivity. Silent-but-valid is different from disconnected. An open but silent process must not imply audio activity. Provide reduced-motion mode.
- **UI-05 — Inspector.** Offer sliders plus precise text entry, unit labels, reset, presets, pre/post meters, and an effect of change summary. Rejected values remain an editable draft with a reason; do not display them as active. Expensive rebuild operations show progress and preserve last committed values on failure. Audio File settings include descriptive source state, a WAV/MP3 picker, and a loop toggle that remains editable while the session is running; loop changes use the live parameter-save path. Decoding is backend-owned and bounded. Temporary voice takes use an already-connected Recorder node on a running route and become expiring backend media after the temporary WAV and library row are removed; Sound Check is limited to 10 seconds (UI-24). The local shell may request `Record`; it does not gain capture-device or endpoint-administration authority. Voice recording controls may not capture through renderer/browser APIs.
- **UI-06 — Routing explanation.** Selecting an output shows “Receives audio from” with all source paths, processing, muted/bypassed sections, and latency estimates. `Voice Chat` must visibly exclude desktop/call-return in the reference template. Explanations come from `routes.inspect` and remain available headlessly.
- **UI-07 — Live editing.** Parameter changes use bounded/coalesced requests during dragging and one final committed target. Topology edits use backend plan/commit; the UI presents one Save route action and requests a second confirmation only for warnings. Local UI optimism never overrides a backend conflict. When another client edits, merge nonconflicting presentation state and render the new authoritative graph. Show who changed what using client identity, not guessed person names.
- **UI-08 — Safety and errors.** Feedback/duplicate warnings identify actual paths and remedies. Missing devices remain on the canvas. Offer rebind, retry, inspect, or stop according to backend capability. No hidden fallback to a new microphone. An emergency privacy mute affects physical capture contributions immediately through the backend; clearing it never starts stopped sessions or recorders.
- **UI-09 — Sessions/presets.** Create, duplicate, rename, export/import, start/stop, and designate startup sessions. Imports open stopped with unresolved bindings highlighted. Include templates for UC-01, processed mic, app recording, and mix-minus. Presets expand into inspectable ordinary nodes. Header Play/Stop starts or stops the visible canvas route; it does not start a stopped Test Signal or Audio File source. Test Signal Play/Stop controls only that tone through the authorized source transport and may first start a stopped route. A routed, enabled Test Signal Play remains actionable when endpoint readiness is missing so the backend can report the specific start failure. Audio playback still requires the session's exact native endpoint preparation.
- **UI-10 — Background controls.** Tray controls list session states, recording states, and open/quit actions. Omit the global mute toggle; privacy mute remains available through Properties and its shortcut. “Close window” keeps audio running; “Quit and stop audio” explicitly stops the backend after recorder finalization. Offer optional always-on-top compact meters and global shortcuts, with conflict/rebinding support. Shortcuts dispatch API actions.
- **UI-11 — Accessibility.** All essential actions work without dragging or a mouse. Provide a structured list/tree view of routes, descriptive accessible names, logical focus order, focus restoration after dialogs, high-contrast/light/dark modes, 200% zoom, and non-color status cues. Target WCAG 2.2 AA applicable criteria plus Windows Narrator testing; record the exact audit checklist in M05.
- **UI-12 — Responsiveness.** Remain usable at 1280×720 and 100–200% Windows scaling; panels may collapse. Long names wrap/truncate with accessible full labels. Virtualize large lists/canvas where needed and throttle meters. No web audio engine or processing in the renderer; browser microphone access is unnecessary.
- **UI-13 — API availability.** If backend disconnects, display a reconnecting/offline state and retain the last snapshot as stale. Disable dependent mutations, do not queue unbounded audio edits, and resync snapshot/revision on reconnect. Closing/reopening a plugin editor does not affect its audio.
- **UI-14 — Recorder UI.** Expose record/arm/pause/split/stop, destination, disk error, duration, and file library actions. Clearly separate deleting a node, removing a library entry, and recycling a recording file. Do not bury recording activity when the session sidebar changes.
- **UI-15 — Signal flow visualization.** On active connected routes, show sound traveling in the edge direction between nodes, including from application-capture and microphone inputs through processing to destinations. Represent recent signal level with a clearly visible, smoothly varying stroke width (bounded by a design-system minimum and maximum), with a restrained moving highlight or pulse; do not use a single 1 px line as the only activity cue. Animate only when fresh backend meter/telemetry reports signal on that route, and keep configured-but-silent, stopped, muted, bypassed, stale, disconnected, and faulted states visually distinct. Base width on bounded peak/RMS meter values using smoothing and hold/decay so ordinary audio is legible without flicker; clamp values and never imply activity from a connection alone. Flow direction follows graph topology. The effect remains legible at supported zoom levels and high contrast, has a reduced-motion/static equivalent, and exposes equivalent text/meter values to assistive technology. It is presentation only: no browser audio processing, inferred routing, or per-frame backend polling; consume the existing bounded meter/event path and respect NFR-08 freshness/rate limits.
- **UI-17 — Stable layout (UX rule).** Nothing may shift, bounce or push other content up and down because live data, measurements, status or hints change. Content that comes and goes while the user watches (suggestions, live status, pills, warnings, readings) occupies a reserved slot of fixed or minimum size: render the slot always and swap its contents, with a waiting/placeholder text and a disabled action when there is nothing yet. Keep the last good value instead of hiding a box when an estimate briefly becomes unavailable. Fix the width of changing numeric labels (tabular digits, minimum width). Global notifications may instead float outside layout without reserving an empty slot; showing or dismissing them must not move the workspace. Layout may change only in response to a deliberate user action such as opening a section, switching a tab, or resizing. Regress it by sampling the position of content below the changing area during live updates and requiring it to stay put.
- **UI-18 — Version and new-version notice.** The window title and the header show the running version (for example "AudioRouter 0.0.8"). Unless the user turns it off in Advanced → Device access and version checks → New versions (default on, per-user preference), the app reads the public GitHub releases list of this repository at most once a day (cached; a cached answer for an older app version is ignored after an upgrade). When a published, non-draft release with a higher `vMAJOR.MINOR.PATCH` tag exists (prereleases included), the header line shows "<version> available"; it opens that release page in the default browser through a shell command that accepts only this repository's `…/releases/tag/vX.Y.Z` address. The request sends no identifier or audio information; offline, rate-limited or failed checks show nothing and never block the app. The notice stays on the header line (UI-17). Unit tests and automated browsers never contact GitHub (an injected list is used). User-authorized 2026-10-03; see PROD-06.

Stack simultaneous warning/action notices in the bottom-right notification
popover without reserving workspace height. Neither the canvas nor sidebar may
cover their action or dismiss buttons. Long sidebar content (including Timing) scrolls within the workspace
without pushing the canvas down. Connection guidance explains the sending
blue output to receiving orange input gesture; an occupied-input notice must
distinguish replacing that input from sending the tool's output elsewhere.

The Advanced EQ property editor presents the backend `parametricEq@1` node as a
logarithmic frequency graph with up to sixteen selectable points. Dragging a
point changes its frequency and applicable gain; precise frequency, gain,
filter type, and Q controls remain keyboard accessible. The response curve
comes from `processors.response`, not an independent renderer DSP model.

The filter selector also offers Band pass and All pass (DSP-02 completion).
For these types, frequency and Q remain editable,
gain and pass slope are inapplicable, and graph dragging changes frequency
only. Explain that Band pass keeps a frequency band and All pass changes
phase without changing level. An All pass point sits on the 0 dB line.
Verify selection, precise entry, drag behavior, and save/reload using the
shared backend types, plus dark, light, and high-contrast visual checks.

The bell-shaped peaking filter is labelled **Peaking/Band** in the editor, distinct
from Band pass; its stored identifier stays `peaking` for compatibility.
An active-point selector offers point number, frequency and filter name
without changing any parameter. Small pointer motion while selecting a
graph point must not count as a drag.

Undo and Redo appear beside Save/Play in the top bar with icons, accessible
names and shortcut hints. Ctrl+Z undoes and Ctrl+Y redoes route edits,
including tool values. Keep bounded local history across the UI's own Save
and live autosave; restore content against the current backend revision.
Clear history on session changes or external graph replacement; a new edit
clears redo. Ordinary text fields retain native text undo; numeric tool
fields use route undo and immediately display the restored value. Undo of
a stopped saved route makes an editable draft to Save; parameter-only undo
while playing follows the same live-apply path as other parameter changes.

- **UI-19 — Focused settings and local themes.** Replace Setup with Status (six readiness checks and Audio devices on this PC), Virtual Cables (cable status, management and third-party app routing guidance), and UI Options (theme and connection animation preferences). Move audio-device consent preferences and version checks into an Advanced disclosure. Remove the header theme selector. Provide up to 24 editable custom palettes, each with a unique bounded name, a dark/light base and fixed color keys; users create, select and delete them (built-in themes cannot be deleted, and deleting asks first). Import adds a palette rather than replacing one. Import/export version-1 AudioRouter theme JSON files, at most 32 KB, with six-digit hex colors only; reject invalid files without altering the active palette. Export appearance alone and persist locally. Preserve dark/light/high-contrast modes and the global form style. Recording’s empty library state belongs inside its library card. Local MCP setup guides assistant choice, connection name, explicit client permissions, app-specific setup and a read-only connection check; retain standard stdio terminology only where an app asks for its transport.
- **UI-20 — Visual tool guides.** Provide a dedicated public static page for every addable built-in tool, source, destination, VST node and visual Group. Each guide explains its purpose, signal behavior, every exposed configuration field (bounds/defaults or honest dynamic-selection limits), actions and concrete examples. Include accessible conceptual signal illustrations with explicit Play/Pause/Replay/seek, keyboard support, static reduced-motion state and offscreen pause. EQ explains all eight shapes including all-pass phase; input channels show stereo/mono/left/right. Never represent illustrative diagrams as measured DSP or live audio. Link visibly from tool cards and Properties; native opening accepts known tool kinds only. Repository desktop builds may open the generated local page for review; installed builds open the canonical public guide. Site publication remains a separate authorized operation.
- **UI-21 — Voice level target zone.** The Meter inspector shades a voice target zone (peaks −18 to −6 dBFS) on its scale and shows one level verdict in a fixed-height line: waiting for sound (meter inactive, no peak, or peak below −50 dBFS), too quiet (below −18), in the zone (−18 to −6), loud (above −6 and below −1) and too loud (−1 dBFS or more). Recorded clipping keeps its own existing status line. The verdict follows the loudest channel's displayed held peak, so it does not flicker with each telemetry update, and is labelled as a guide for voice. It changes no audio or saved data.
- **UI-22 — Hidden devices.** Audio devices on this PC offers a "Show in pickers" switch for each active device. Hidden devices are left out of the Input Device and Output Device pickers on this PC only; the node's own saved device is always listed, and the picker shows a disabled line with the number of hidden devices and where to show them again. The preference is local (bounded to 256 endpoint IDs); unreadable or invalid storage hides nothing. Hiding never changes sessions, bindings, Windows or the backend.
- **UI-23 — Application source picker and groups.** In Signal Flow, let the user add one running application or select an existing `.exe` while it is closed. Keep the current exact-instance selector for a running-app choice. A closed-app `.exe` choice stores its executable name and full path with an `allVerifiedInstances` selector, never a PID; capture remains silent until a verified process is available and the route is started. Let the user select multiple running applications and/or executable paths to create separate Application Capture nodes feeding one ordinary Mixer. The nodes and Mixer remain independently editable and use normal graph validation, save and undo. Do not start applications or change their Windows output. Explain that selecting a parent app and one of its helpers may duplicate its process-tree audio until CAP-10 overlap detection reports it.

**UI-24 - Sound Check.** Audio File Properties may record a voice sample from an already-connected Recorder node while its route is running. Limit the sample to 10 seconds, with a user stop action, and import it only through the existing bounded temporary-WAV path; the resulting media expires after 24 hours and is not added to the recording library. Explain that the Audio File source must be planned and committed before playback. Play uses ordinary backend source transport through the committed downstream graph and its effects. Do not open or select a microphone, capture through browser APIs, or silently alter a running or saved route.

## Defaults that reduce work

Tool-card guide access is a compact question-mark target inside the card's
visual boundary, with an accessible tool-specific name. Help opens the guide
without adding the node; the add action and help are separate sibling controls.
Properties reserves the measured height of every wrapped workbench tab row
plus spacing, so its Selected node heading never covers the bottom tabs.

Each guide teaches a concrete setup sequence, expected result and first fault
check. Relevant sound examples show named input/output waveforms and reference
listening through one shared transport. Multiple-input examples use distinct
sources; Duck's key is control-only. Recorder replays only the completed example
interval. Example meters derive from the played samples before monitor gain.
Synthetic simplifications are stated per tool; device selection, arbitrary
plugins and visual groups do not imply an audible browser demonstration.

Desktop startup detects an existing same-user, same-Windows-session AudioRouter
instance before opening the database or contacting its backend. Show “There is
already an instance running. Please close it.” Offer Cancel and an explicit
force-close-and-continue action that explains audio stops and unsaved work or
unfinished recordings may be lost. Cancel leaves the old app untouched. Never
infer another process from an already-prepared audio worker error or terminate
unverified applications. Windows denial gives manual-close guidance.

Logs provides Open logs folder and Copy folder path. The fixed app-owned folder
opens in Explorer; support instructions identify current/previous shell, backend
and discovery JSONL files and ask for reproduction steps/time. Missing clipboard
or Explorer access provides a useful fallback; browser previews explain that
local log access requires the installed desktop app (UI-13, SEC-10).
Logs also offers Copy support bundle: the shell writes one new ZIP to the logs
folder (manifest with app/build/Windows version, the last 2,000 lines of the
backend, shell and MCP activity logs, and the client diagnostics, each line
path-filtered) and selects it in Explorer; nothing is uploaded. A Verbose
logging switch calls `diagnostics.setVerbose` and shows the time left in a
fixed-width status that never moves the panel (UI-17); it expires after one
hour. Failed requests add a client diagnostics row with the method, error
category and request ID (P2-3).

Desktop startup detects an existing same-user, same-Windows-session AudioRouter
instance before opening the database or contacting its backend. Show “There is
already an instance running. Please close it.” Offer Cancel and an explicit
force-close-and-continue action that explains audio stops and unsaved work or
unfinished recordings may be lost. Cancel leaves the old app untouched. Never
infer another process from an already-prepared audio worker error or terminate
unverified applications. Windows denial gives manual-close guidance.

Logs provides Open logs folder and Copy folder path. The fixed app-owned folder
opens in Explorer; support instructions identify current/previous shell, backend
and discovery JSONL files and ask for reproduction steps/time. Missing clipboard
or Explorer access provides a useful fallback; browser previews explain that
local log access requires the installed desktop app (UI-13, SEC-10).
Logs also offers Copy support bundle: the shell writes one new ZIP to the logs
folder (manifest with app/build/Windows version, the last 2,000 lines of the
backend, shell and MCP activity logs, and the client diagnostics, each line
path-filtered) and selects it in Explorer; nothing is uploaded. A Verbose
logging switch calls `diagnostics.setVerbose` and shows the time left in a
fixed-width status that never moves the panel (UI-17); it expires after one
hour. Failed requests add a client diagnostics row with the method, error
category and request ID (P2-3).

The primary graph remains a canvas; it has no List view switch. Advanced
provides expandable Keyboard graph controls with node selection and the same
connection/topology actions for keyboard and screen-reader users (UI-03/11).
Network Properties shows a static two-computer audio direction diagram,
configured IP and matching UDP port; configuration alone never implies signal.

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

While a connected backend's initial snapshot is pending, show loading status
and withhold the editable graph. Failure offers reconnect rather than editable
demonstration data. Numeric properties honor the backend's advertised step and
reject values between valid steps. Every enum field has an accessible caption.

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

Switching node selection must remove the preceding tool's editor and timing
controls, including while telemetry refreshes. EQ uses its frequency-response
editor, Meter uses its signal-level inspector, and Compressor/Gate use their
respective dynamics views. Identity footers must not share sibling React keys
with live editors; parameter-editor hooks run unconditionally across tool kinds.

Node Properties ends with a muted, selectable full Node ID and an accessible
copy button titled "Copy node ID for API integrations". The Session tab shows
the selected Session ID with the same controls. Copy announces a brief success
or manual-copy fallback. Identity display/copy remains available while offline;
selection changes clear feedback and long IDs wrap within the sidebar.

Meter is an insertable pass-through with matching input/output channels and no
sample or latency change. Legacy input-only nodes remain readable; Properties
offers an explicit output upgrade in the draft. Its detailed inspector shows
large per-channel dBFS RMS bars, current sample peak, held sample peak, headroom,
clipped sample count/duration and observed clipping share. Statistics come from
the backend; meters.reset requires SessionControl and affects only the selected
prepared Meter, not audio, graph revisions or other meters. Values never wrap
as their digit count changes. Explain sample peak versus true peak/LUFS and
per-channel clipped sample time versus continuous-event duration.

Arrange uses saved/draft connections rather than insertion order: sources
on the left, downstream tools/outputs on the right, separate disconnected
component lanes, measured card spacing and branch-order crossing reduction.
Disabled edges remain part of layout so audio toggles do not scatter cards.
Arrange fits the graph in view and never changes graph/audio. Undo arrange
restores the preceding positions for 15 seconds.

Workbench help uses one collapsed question-mark disclosure style across tabs,
with context-specific guidance. Tools labels it "How to build a route".
The Properties node name becomes its own editor on click or keyboard activation;
show a pencil/underline on hover or focus, use the global field style while
editing, commit a nonempty trimmed name on Enter/blur, and cancel with Escape.
Do not duplicate the name in a separate permanent textbox.

The inspector omits the "No unsaved changes" line. The header Save action is
enabled and visually emphasized for unsaved drafts. Mark affected canvas nodes
(including changed wiring endpoints) with "Save required" for explicit saves.
Retain the existing 400 ms debounced parameter-only autosave during playback;
identify pending autosave/saving distinctly, clear indicators only on an
authoritative save, and retain Save required after a rejected save. Do not
expose parameter values or secrets in these indicators.

The Properties header shows the node state beside its name: Off, Bypass,
Ready (enabled while stopped), Active, Disconnected, or failed plugin state.
Draft changes remain identified separately; Active is not proof of input signal.
Inactive processors do not present stale analysis as live or offer noise
learning. Tool descriptions avoid partial approximate latency claims; Properties
directs users to Timing for playback delay and processing measurements.

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
