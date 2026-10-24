# Active plan: OBS and Discord as a premium experience (2026-10-23)

Owner request, 2026-10-23: "I like all these proposals. We just need to make
sure it is smartly positioned inside the UI. Think of a nice UX/UI plan and
let's do this goal of making OBS and Discord a premium experience."

- Authorized: stages D1, O1, D2 and O2 of
  [OBS and Discord setup help](../future/obs-discord-integration.md), under
  DEC-22.
- Not authorized: X1, Discord RPC. It needs an application approval from
  Discord, which only the owner can request.
- No PR is opened without the owner's approval (AGENTS.md).
- Compiles and Edge runs wait while the owner is in a game or call (see
  the no-heavy-builds note in memory).

## UX principles

1. **One home, three surfaces.** All app setup lives in one new sidebar tab,
   **Apps**. It also shows up where the user already looks: a glance strip
   in the top bar, and a "used by" line on device nodes and their
   Properties. Nothing appears as a pop-up while audio runs.
2. **Say what is true, then what to do.**
   - Every status names the real device and the app's real setting, for
     example "Discord is using *CABLE In 16ch* as its speaker; AudioRouter
     expects *CABLE Input*".
   - Every problem has one primary action: *Show me how* (exact menu path
     plus a Copy device name button), or, for OBS, *Fix in OBS*.
3. **Never surprise.**
   - Reading is automatic. Writing to OBS needs a preview, a confirmation
     and a *Restore*.
   - AudioRouter never changes Discord, Windows defaults or app files.
4. **Glanceable while streaming.** Status uses icon, colour *and* a word:
   ✓ Ready, ! Check, – Not running. Colour is never the only signal.
   Slots keep a fixed size (UI-17): an app going from Ready to Check never
   moves the layout.
5. **It ticks itself.** In the setup checklist, an item turns green when
   Windows reports the app on the right device (D1). The user changes
   Discord, comes back, and sees the tick, with no Refresh button to find.

## Surfaces

### 1. Top bar: app strip (D1)

```
[● Audio running]  [Discord ✓] [OBS !]   Undo Redo  Save  Play
```

- **Which apps:** Discord and OBS, plus any app the session captures or
  feeds. At most 3 chips, then "+2".
- **Rules:**
  - Shown only when the session names at least one such app, so a plain
    route has no strip. It appears only after an explicit change: a route
    edit or the first time an app is detected. It never comes and goes
    with polling.
  - Each chip has a fixed width and a monogram (D, OBS), as icon
    extraction is a later polish item.
  - The state word is in the accessible name, and a tooltip gives the
    one-line reason.
  - Click opens **Apps**, focused on that app's card.

### 2. Apps tab (D1, O1, O2)

- **Position:** the tab sits right after **Status**, with the label "Apps".
  The Virtual Cables tab stays; the Apps tab links to it whenever a cable
  is missing.
- **Order:** one card per app, Discord first, OBS second, then other
  detected recorders and games.

Discord card:

```
Discord                                      ✓ Ready
 Microphone   CABLE-A Output   ✓ carries your processed voice
 Speaker      CABLE Input      ✓ AudioRouter captures Discord from here
 Recommended in Discord › Voice & Video:
   Noise suppression: None      (AudioRouter's Denoise already runs)
   Automatic gain control: Off  (it fights the Compressor)
   Echo cancellation: Off       (use headphones)
 [Show me how]
```

- When the app is not running: "Discord is not running. Start it to check
  its devices." The card keeps its size.
- When a device is wrong: an amber row with "expected …", plus *Show me how*
  (numbered steps, *Copy device name*).

OBS card states, each keeping its size:

- **Not installed:** the card is hidden unless the session targets OBS.
- **Installed, closed:** "Last saved OBS setup" shows the same table,
  read-only, from the active scene collection file, with the note "OBS is
  closed".
- **Running, not connected:** *Connect to OBS* opens three inline steps:
  1. In OBS: Tools › WebSocket Server Settings › Enable.
  2. Tick Enable WebSocket server, then OK.
  3. Connect. AudioRouter reads the password from OBS's settings; a
     password field appears only if OBS refuses it.

  The note says the password stays in memory and is never stored or shown
  (revised 2026-10-23, see Progress).
- **Connected:** shows "What OBS hears":

```
 Source        Device                 Hears                 Track
 Mic/Aux       CABLE-A Output         Voice                 1 2
 Desktop Audio Default (Focusrite)    Voice  Game  Discord  1 2   ! voice twice
```

Problems are listed under the table as cards. Each states the cause, why
it matters, and offers **Fix in OBS** (connected) or **Show me how**.

**Fix in OBS** dialog (O2):

- A before/after table, with one checkbox per change, for example "Desktop
  Audio: Default → CABLE Input (game + Discord, no voice)".
- **Apply** and **Cancel**.
- After applying: a toast "Changed 2 OBS sources. Restore", and the card
  keeps a *Restore previous OBS settings* link until the next fix or
  session change.
- Optional extra, unticked by default: "Separate tracks for editing: voice
  on 2, game on 3, Discord on 4".

### 3. Device nodes and Properties (D1)

- **Canvas:** Physical Input and Physical Output nodes get one quiet footer
  line, "Used by Discord · OBS". It is reserved only on device nodes,
  truncated with a tooltip, and outlined amber when an app is on a sibling
  device it should not use. It never changes the node size.
- **Properties:** a section "Apps using this device" lists each app with
  its role: *records from it*, *plays to it*, or *plays to or records it*
  (the loopback ambiguity).

### 4. Set up for streaming (D2)

- **Entry points:** the first-run guide, Session › *New from goal*, and the
  Apps tab when there is no session yet.
- **Step 1, goals:** checkbox cards — Clean mic in Discord, Stream with
  OBS, Record clips, Hear game and Discord in headphones.
- **Step 2, devices:** the microphone and headphones are preselected. The
  cables are assigned automatically with plain names ("Voice cable:
  CABLE-A"). A missing cable links to Virtual Cables.
- **Step 3, review:** a read-only mini route. Saving creates an ordinary
  session that the user can edit and undo.
- **Step 4, set up your apps:** a live checklist driven by D1, with the
  same *Show me how* steps. It ends with "Everything is set ✓".

## Backend and API (owned by the backend, so CLI, HTTP and MCP share them)

- **`applications.endpointUsage` (D1):** for each active endpoint, the
  processes with a session there, with fields: endpoint ID, direction, PID,
  executable name, state, display name.
  - Bounded: 64 endpoints × 64 sessions.
  - Refreshed at most every 3 s and only off the audio thread.
  - Grant: the same read scope as `applications.list`.
- **`apps.expectations` (D1/D2):** derived from the session graph. For each
  app the session names (Application Capture of Discord, a template's app
  roles), the expected microphone and speaker endpoints and what each
  cable carries (voice, game, Discord). It is computed from graph paths,
  and its "voice" tag comes from protected microphone sources.
- **`obs.status` / `obs.connect` / `obs.inspect` / `obs.preview` /
  `obs.apply` / `obs.restore` (O1/O2):**
  - A websocket v5 client in the backend, on `127.0.0.1:4455` only, with a
    password read from OBS's settings on Connect (or pasted) and kept in
    memory only (revised 2026-10-23; no DPAPI copy).
  - Fallback to a bounded, read-only scene-collection file when OBS is
    closed.
  - `obs.apply` needs a new `IntegrationWrite` grant, which the shell
    holds after the user connects, and a `previewId` returned by
    `obs.preview`.

## Order of work

| Step | Content | Gate |
| --- | --- | --- |
| 1 | D1 backend: per-endpoint sessions in `windows-audio` (extend `enumerate_application_audio`), the control method, schema, contracts, CLI/MCP exposure, tests with a fake inventory | Rust tests; guarded Windows check against the scratch script |
| 2 | D1 UI: Apps tab with the Discord card, top-bar strip, device-node footer and Properties section | Vitest; Edge in three themes; layout-stability sampling (UI-17) |
| 3 | D2: goal templates, the guided flow, the live checklist | Vitest per goal combination; CAP-10 clean graphs; Edge |
| 4 | O1: scene-collection reader, websocket client (read only), the "What OBS hears" analysis | Fixtures from OBS 32 (redacted); fake v5 server tests |
| 5 | O2: preview, apply and restore, tracks option, `IntegrationWrite` grant, spec and SEC updates | Fake-server tests; one attended check on a copied OBS profile |

Each step is a separate branch and PR (with approval), with docs, specs and
contracts updated in the same change.

## Validation matrix

- **Unit:** endpoint usage parsing and bounds, expectation derivation per
  topology (the owner's 3-cable route, a minimal route, no cable), OBS file
  parsing (valid, malformed, oversized, older schema), websocket
  authentication and refusal paths.
- **Windows (guarded, owner's PC):** endpoint usage matches the scratch
  script with Discord, Siege and Outplayed running. Record app versions.
- **UI:** Edge on the route harness with simulated usage. Check all states
  of each card in dark, light and high-contrast. The top strip and the
  node footers keep fixed offsets during state changes (UI-17). Keyboard
  only.
- **Owner acceptance:** the strip and cards are right on the live setup.
  For O2, "Fix in OBS" then "Restore" on a copied OBS profile.

## Risks and rollback

- The loopback ambiguity can make a correct setup look wrong. Show "plays
  to or records" and never mark that state as an error.
- Polling cost: endpoint walks are cheap, but they run off the audio thread
  and back off to every 3 s.
- Writing OBS settings is the only outward change. It needs preview,
  confirmation, restore and a separate grant.
- Rollback: each step is additive; revert its PR. No session format
  migration.

## Next action

Compile and test D1, O1 and O2 (list under Progress, "Waiting") once the
owner is not in a game, then the attended OBS check on a copied profile.

## Progress

- 2026-10-23, decision: D1 extends `apps.list` with a required
  `audioEndpoints` array of `{deviceId, direction, active}`, instead of
  adding a new method. It is one call with the existing read grant, and
  the UI already lists applications.
- Implemented, not yet compiled or run:
  - `windows-audio` records each session's endpoint ID. `GetId` is freed
    through `take_endpoint_id`. Bounded to 64 endpoints per process and 512
    bytes per ID.
  - The output schema, contract, CLI and catalog tests, and the API
    reference are updated.
  - UI: `ui/src/appUsage.ts` (device usage, cable pairing, the Discord
    check), with tests built on the owner's topology, including the
    2026-10-03 "CABLE In 16ch" failure.
  - UI: the Apps tab (`AppsPanel.tsx`), the top-bar strip
    (`AppStatusStrip.tsx`) and `useApplicationUsage.ts` (refresh on focus,
    10 s poll only while Apps is open and visible, last good list kept on
    failure). Both rows always render, and the badge and chip widths are
    fixed (UI-17).
  - Specs: UI-26 (Apps) and CAP-15 (endpoint session inventory). Quickstart
    has a "Checking Discord's devices (Apps tab)" section. The docs check
    passes.
- Review fixes, 2026-10-23, not yet run:
  - The check judges the matching microphone when Discord records from two
    devices at once.
  - Every row keeps its "Show me how" line, so a row turning from ready to
    check does not grow (UI-17).
  - "Copied" returns to "Copy device name" after 2 s.
- UX review, 2026-10-23 (owner request: a new user, then an experienced one):
  - Fixed: a cable input routed to another cable was taken for the voice,
    so the check could suggest the wrong cable. Only non-cable inputs count
    as microphones now.
  - Fixed: a session with no voice cable was a dead end ("nothing to
    compare"). The row now offers "Set up a virtual cable", which opens
    the Virtual Cables tab.
  - Fixed: with several voice cables, the message names all of them.
  - Open, for D2: a new user still has to build the mic → cable route by
    hand. "Set up for Discord" (one undoable route plus the Discord steps)
    is the real answer.
  - Open: the Apps tab is the 6th of 13 sidebar tabs, and the strip chip
    only appears once the session involves Discord. Consider a first-run
    hint when Discord is running and the session has no voice cable.
  - Open: "plays to it or records it" stays vague, because Windows cannot
    separate a loopback recorder from a player.
- Website: `site/apps.html` (Discord and OBS), linked from the header,
  footer and homepage. OBS is marked "in development"; the page states that
  the Apps tab comes after 0.0.21.
- Waiting: compiling and testing are on hold while the owner plays
  (no-heavy-builds rule).
- Next, in order:
  1. `cargo fmt` (both) and Clippy (workspace and `src-tauri`,
     `--all-features`).
  2. `cargo test -p audiorouter-windows-audio -p audiorouter-control
     -p audiorouter-cli`.
  3. UI `npm ci`, typecheck, lint and Vitest; contracts typecheck and drift.
  4. Edge screenshots of Apps in the three themes, with layout-offset
     sampling while the check changes.
  5. Time the `apps.list` handler on the live PC and watch
     `audioService.lateGaps`; compare its endpoints with the read-only
     session script.
  6. Then the device-node "Used by" footers (rest of D1).
- O1 and O2, 2026-10-23 (owner: "Yes do both please"), implemented, not
  yet compiled or run:
  - Decision change: no DPAPI. On Connect, the backend reads the WebSocket
    password from OBS's own `obs-websocket/config.json` for this Windows
    user, or takes one the user pastes. It lives only in the OBS thread's
    memory, is never journaled (`obs.connect` skips the idempotency
    journal), logged or returned. Storing a second copy would add a secret
    to protect and nothing for the user. Recorded in DEC-22 and SEC-15.
  - Decision: a new `integrationWrite` scope for connect, disconnect,
    preview, apply and restore. The desktop shell holds it; Observer,
    Editor and Operator (MCP, CLI) do not. The HTTP adapter forwards with
    the shell grant, so it refuses the OBS write methods with 403.
  - Backend: `crates/control/src/obs.rs`. A background thread
    `audiorouter-obs` owns all socket and file I/O, so the control thread
    never waits on OBS; methods read a snapshot or queue a command.
    obs-websocket v5 client on `127.0.0.1` (port from OBS's settings),
    read of the saved scene collection while OBS is closed, preview (at
    most 16 changes, 300 s, tied to the inspection), apply and restore
    that leave alone any source changed in OBS since, and automatic
    reconnect after Connect. Tests in `obs_tests.rs` use a fake v5 server
    and never read the developer's OBS files.
  - API: seven `obs.*` methods in the domain catalog (124 → 131),
    schemas, grants, contracts and the API reference.
  - UI: `obsUsage.ts` (what each source hears, the three problems, fixes
    only while connected), `useObsStatus.ts`, `ObsCard.tsx` (connect
    steps, password field only on refusal, Fix in OBS dialog, Restore,
    Disconnect), the OBS strip chip when the session has a voice cable.
    Tests: `obsUsage.test.ts` (owner's topology), `ObsCard.test.tsx`.
  - Specs: UI-26 OBS card, SEC-03/SEC-15, the API method table, DEC-22
    revision, quickstart "Checking and fixing OBS Studio".
  - Not done, deliberately: the separate-tracks option and the toast from
    the original sketch (the card's note line shows the outcome instead).
    The site page keeps OBS "in development" until a release ships it.
  - Waiting (owner playing): `cargo fmt` is clean. Still to run: Clippy
    (workspace and `src-tauri`), `cargo test -p audiorouter-control
    -p audiorouter-domain -p audiorouter-cli` and the `src-tauri` HTTP
    tests, UI typecheck, lint and Vitest, contracts typecheck and drift,
    Edge screenshots of the OBS card in three themes with offset sampling,
    then one attended Fix and Restore on a copied OBS profile.
