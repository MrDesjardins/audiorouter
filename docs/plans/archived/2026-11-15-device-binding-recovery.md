# Device binding recovery and setup-field focus — 2026-11-15

> **Archived 2026-11-16 — merged in PR #154. Physical reboot/re-enumeration and attended audio/Narrator gates remain open.**

## Objective and requirements

Resolve two owner-reported issues: saved physical-input IDs that no longer
resolve although the microphone is present, and Setup Check navigation that
opens Properties without directing the user to the endpoint selector.
Requirements: CAP-02/11/12, STATE-03, UI-05/08/11/17/27, ENG-02/04; M02/M05/M07.

## Decisions

- Keep endpoint ID as primary identity. Friendly names are recovery hints from
  the bounded per-user cache and Windows' inactive endpoint inventory; they are
  not caller-editable graph identity. A legacy ID already forgotten by Windows
  and never cached has no name-based recovery and remains manual.
- When a pinned physical-input ID is absent, a replacement candidate is
  eligible only if exactly one active capture endpoint has a case-insensitive,
  exact friendly-name match. Never use substring/fuzzy matching, defaults,
  channel heuristics, first-match selection, or a render endpoint.
- Owner decision (2026-11-15): automatically rebind a stopped saved session
  when exactly one active capture endpoint has the case-insensitive exact name
  recovered from the bounded cache or Windows' inactive inventory for the
  missing ID. Persist through graph plan/commit with a commit-time stopped-
  session precondition before any device is opened. Keep the route stopped until
  deliberate Play; do not edit a running/attached session or dirty UI draft.
- No match, multiple same-name matches, direction mismatch, missing name
  metadata, or stale/ambiguous inventory retains the current missing-device
  finding and asks the user to choose manually. No fallback microphone is
  opened.
- Open affected tool selects the node, opens Properties, scrolls the Input
  device endpoint control into view, focuses it, and briefly highlights it.
  Navigation never opens the native dropdown menu, changes the endpoint, skips
  Save, or starts audio.
- Old sessions that have only an opaque endpoint ID have no reliable friendly
  name to recover; they remain manual until the user reselects the endpoint.

## Ordered work

1. Update CAP-02, UI-08/UI-27, UC-06, graph binding metadata, and delivery
   traceability with unique-name automatic rebind and manual ambiguity fallback.
2. Store bounded last-observed endpoint names in the local per-user cache when
   devices are enumerated and when a user confirms an endpoint selection.
3. Have device setup checks identify a unique active capture candidate by exact
   case-insensitive name, while preserving the `deviceMissing` result. Ambiguity
   remains unresolved.
4. Commit the exact replacement ID for eligible stopped sessions through the
   normal graph transaction before any audio open. Keep Open affected tool,
   scroll/focus/highlight, and manual repair for unresolved findings.
5. Add regression tests for unique match, zero/multiple matches, case-only
   differences, wrong direction, inactive Windows metadata, missing legacy
   metadata, active-session commit refusal, focus/scroll/highlight and no audio
   start.
6. Run focused backend/UI checks, formatting, lint, typecheck, documentation
   validation and diff review. Record attended Windows device/reboot checks as
   remaining if not run.

## Validation and evidence

- Backend setup-check tests prove candidate selection is unique, exact-name,
  case-insensitive, active and capture-direction only; the setup check remains
  read-only. Automatic reconciliation commits only stopped saved sessions.
- UI integration tests use the real setup-finding action and assert the selector
  focus/scroll target and manual fallback. A ten-session integration fixture
  verifies four stopped saved routes sharing the missing exact ID are committed
  to the unique replacement, with no Start/Stop calls and no changes to the six
  unrelated routes. Visual browser checks verify the focus indicator and
  temporary highlight in dark, light and high-contrast.
- Windows reboot/re-enumeration acceptance is distinct from portable matching
  tests. Reconciliation never opens a candidate; native open is deferred until
  subsequent Play.

## Risks and rollback

Friendly names are user-editable and may collide. Restricting recovery to one
exact match, stopped sessions and a normal graph commit prevents opening an
unreviewed endpoint or changing a live route. Normal authorization and revision
conflict rules remain authoritative. Roll back the scoped change; exact IDs
remain supported and ambiguous/uncached sessions remain manually repairable.

## Progress

### Latest-main integration (2026-11-15, Windows)

- Branch `fix/device-binding-recovery` starts at fetched `origin/main`
  `01cc24a14cda3785a7ea984b5ed8ac13bb5b8fd1`. Transferred only recovery
  changes from the original checkout. Resolved release-notes/current-plan
  conflicts by preserving main's merged theme documentation and adding the
  recovery notes. Original checkout and unrelated edits remain intact.
- Repeated control library tests: 423 passed, 13 explicitly ignored.
  Workspace and shell strict all-target/all-feature Clippy pass. Shell's
  first check required the missing `ui/dist`; production build supplied it
  and the repeated check passed. Both Rust format checks pass.
- UI focused regression tests: 22 passed. Bundled Edge UX qualification:
  six passed across dark/light/high-contrast and two viewport sizes.
  UI typecheck:all, lint, format/check, contracts drift (148 methods), docs
  acceptance (269 files / 1,491 links), and diff checks pass. Default MCP
  API reachability regression passes.
- The unrestricted full UI run had five failures (four timeouts and one
  recording-refresh assertion) while builds and browser tests ran concurrently.
  Logging/lifecycle rerun with one worker: 29 passed. Full two-worker rerun:
  122 files / 1,086 tests passed in 92.48 seconds. No timeouts or acceptance
  assertions were relaxed.
- Production UI build passes with a 634 kB main-chunk size warning. Retain
  this warning for this repair: splitting the existing application bundle
  changes unrelated loading behavior and belongs to separate performance work.
- Remaining gates: physical microphone reboot/re-enumeration, attended shell
  audio, Narrator, and refreshed Properties guide recording. This PR is not
  release or completed tool-documentation qualification. Next: open the
  owner-requested PR after local integration checks, then qualify those gates.

- Read the relevant product, workflow, architecture, graph, capture, interface,
  API, persistence, delivery, M05/M07 and setup-navigation specifications.
- Confirmed setup navigation currently focuses the Properties tab only. The
  physical-input editor's accessible selector is `Input device endpoint`.
- Confirmed saved physical-input nodes contain an exact `endpointId`; the
  implementation uses a bounded local cache and checks Windows' inactive
  endpoint inventory to map an old ID that is still enumerated. If the old ID
  has left Windows and was not cached, manual selection remains necessary.
- Implemented: bounded local endpoint-name cache; exact, unique,
  case-insensitive capture matching; Setup Check retains `deviceMissing`, names
  the proposal, and stays read-only; eligible stopped saved sessions receive a
  normal graph plan/commit update before any endpoint open; ambiguous/uncached
  sessions remain manually repairable; Open affected tool focuses, scrolls and
  highlights the Input device endpoint selector.
- Final local verification after the ten-session fixture expansion: control
  library passed 423 tests / 13 explicit ignores; strict control Clippy passed.
  UI passed 122 files / 1,086 tests; UI typecheck/lint/format passed. Contract
  drift and docs acceptance passed (269 Markdown files / 1,491 links). Both Rust
  format checks and `git diff --check` passed. Six bundled Edge UX scenarios
  passed across dark/light/high-contrast at 1280×720 and 640×360. A broad
  windows-audio run once had an unrelated paired-tone timing flake under parallel
  load; its isolated rerun passed with zero discontinuities.
- Remaining unverified: real Windows reboot/re-enumeration with a microphone,
  attended native audio and shell testing, Narrator. If Windows has already
  removed a legacy ID and AudioRouter never cached its name, name-based recovery
  is impossible and manual selection is required.
