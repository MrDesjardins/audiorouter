# Owner decisions 2026-11-27: API and MCP flexibility

## Objective and authority

On 2026-11-27 the owner answered the open A decisions: "A14, A1: Yes, A2: Yes,
A4: Unclear, I do have on this PC Cable-A and Cable-B but we need AudioRouter to
be working for all PC which may have different virtual cables. A12, A13,
A15-A18, fine. The idea is to have as much flexibility for people with API and
MCP." "Fine" accepts each default. Earlier in the same request the owner also
asked for the UI port, the 0.0.33 candidate and nightly upkeep (current-plan
priorities 3–5). No driver work, PR, release or publication is authorized by
this plan; a PR still needs explicit approval.

## Requirements

AUTO-06/10/15/18, R2-1a, R2-4, SEC-02, API-07, ARCH-08, NFR-03, UI-17, L10N-01.

## Decisions

| ID | Decision | Done in |
| --- | --- | --- |
| A1 | Option (a): per-client recording opt-in, off by default, set in the window | This branch; decision record in `docs/spec/13-security.md` and AUTO-18 |
| A2 | `sessions.rename` with the default MCP tool `rename_session` (Editor) | This branch |
| A4 | Open. Not CABLE-B specific: AudioRouter must work with whatever virtual cables a PC has. An agent investigates and proposes | Investigation below; owner chooses |
| A12, A13, A15–A18 | Defaults kept | Recorded in the human-gated table |
| A14 | Site and guides use the app's words ("input device", "output device") | Separate site branch |

A3 (bus lists stay expert-only) was not answered. The owner's flexibility goal
may argue for reversing it; ask before changing it.

## Tasks and status

1. Done: record the answers in [human-gated tasks](human-gated-tasks.md).
2. Done: A2. `sessions.rename` is a task-shaped edit (`simple.rs`). It trims the
   name, refuses empty names, names with control characters and names over 120
   characters, and commits a new revision through plan/commit, so undo works and
   audio is not interrupted. The current name returns `unchanged: true` without a
   revision. It is GraphWrite, so Editor and Operator can rename. Wired into
   `API_METHODS`, the dispatcher, the parameter allowlist, schemas, contracts, the
   retry contract cases, the API reference and `site/assets/api-methods.json`.
3. Done: A1. The backend part covers storage, `grant_for_client`, the
   `clients.authorize` parameter and the `clients.list` field. The four recording
   tools join the default MCP list, and the MCP server instructions and recipe 3
   now say they need the opt-in. The window's Connected clients panel gains the
   box and a "may record" tag, in three languages, and the window's MCP
   permission help, the Cowork guide and the site MCP page say so too.
4. Done: full local checks (below) and the panel in three themes. The first
   screenshots showed the toggle and the Authorize button on one line, so the
   note now sits under the toggle and the button has its own row.
5. Site wording (A14) goes on its own branch.
6. A4 investigation, then ask the owner.
7. 0.0.33 candidate: once #199, the UI port and this branch are on `main`, or
   from a local integration if the owner wants to test first.

## Validation

- `sessions_rename_saves_a_revision_through_dispatch_and_respects_roles`
  (dispatcher: trim, replay, unchanged, refusals, Observer denied, Editor
  allowed).
- `rename_session_tool_renames_the_open_session_and_undo_restores_it` (default
  MCP tool through `mcp_tool_call`, undo, Observer refused).
- `client_recording_opt_in_is_off_by_default_and_round_trips` (storage).
- `recording_opt_in_is_per_client_off_by_default_and_cleared_by_revocation`
  (dispatcher, restart, role change, revocation, self-grant refused).
- `real_mcp_pipe_honors_the_recording_opt_in_and_its_live_removal` (real backend
  process, real MCP sidecar with default tools, live allow, disallow and revoke).
- `lists known clients and authorizes a new one` (UI: off by default, the exact
  request sent, the tag, reset after authorization).
- `cargo fmt` (both), strict Clippy (workspace and `src-tauri`), control, CLI,
  storage and transport tests, `src-tauri` tests, contract drift, docs validation,
  UI typecheck, lint, format and unit tests, and screenshots of the panel in the
  dark, light and high-contrast themes.

## Risks and rollback

A client given the opt-in can start recordings without the user pressing
Record. That is the owner's explicit choice; it is per client, off by default
and visible as "may record". The flag lives in `control_settings`, so an older
build ignores it (fail-closed). Rollback: revert this branch. Stored
`clientRecording:*` rows are then ignored, and `sessions.rename` disappears from
discovery.

## Evidence

2026-11-27, Windows 11 dev PC, Rust 1.96.0: both `cargo fmt -- --check` clean;
Clippy `-D warnings` clean (workspace and `src-tauri`); every workspace test
target passed, `src-tauri` 186 passed; contract drift (157 methods) and docs
validation (326 files) passed; UI lint, format, i18n check and typecheck clean;
UI unit tests 1204/1204; `ui/e2e/clients-recording-opt-in.pw.ts` 3/3 (dark,
light, high contrast; screenshots looked at). Not run: the built shell exe with
an MCP client given the opt-in (the real-pipe test covers the backend and
sidecar).
