# MCP setup and connection UX — active execution plan

Status: implementation in progress on `feature/mcp-setup-ux`, isolated from the
owner's dirty checkout. This plan covers a local desktop UX improvement only;
the existing MCP protocol, transports, authorization semantics, and tool
catalog remain the compatibility baseline.

## Objective

Make the MCP setup experience easier to understand and diagnose without making
AudioRouter's authority broader or changing a working connection. A first-time
user should be able to answer, from the MCP tab:

1. Which connection path applies to the assistant they use?
2. Which exact enrolled client identity will receive requests?
3. What can that client do with its current role?
4. Has the local connection completed a safe read-only check, and if not, what
   should the user do next?

The result must continue to distinguish local stdio, local Cowork, ChatGPT's
OpenAI-specific tunnel, and unavailable cloud Cowork access. It must never
claim that an external assistant accepted a connector merely because the local
CLI or tunnel preflight succeeded.

## Requirements and acceptance scenarios

Primary requirements:

- **AUTO-06:** preserve local stdio MCP, modern stateless discovery, legacy
  initialize clients, enrolled identity, backend authorization, and audit.
- **AUTO-07/08:** preserve task-oriented default tools, resources, annotations,
  bounded telemetry, and no raw audio/media exposure.
- **AUTO-10/11:** explain least privilege and ambiguity boundaries without
  granting or inferring authority from prose or UI annotations.
- **AUTO-13:** improve assistant setup/activity guidance, local Cowork export,
  Observer-first advice, and bounded redacted activity.
- **AUTO-15:** keep task-shaped tools and actionable errors usable.
- **AUTO-16 / DEC-20 / DEC-21:** keep the OpenAI tunnel optional and clearly
  provider-specific; tunnel readiness remains separate from ChatGPT connector
  acceptance; no generic remote endpoint is introduced.
- **AUTO-18:** no API method, contract, NodeKind, or MCP tool is added by this
  UX slice; parity remains protected by the existing reachability test.
- **SEC-02/03/04/10/14:** retain explicit enrollment, scoped grants, backend
  enforcement, redacted audit, credential protection, and remote transport
  boundaries.
- **UI-17/UI-19:** reserve changing status/message space and keep the setup
  usable in dark, light, and high-contrast themes.

Acceptance scenarios:

- A user chooses local assistant, local Cowork, or ChatGPT and sees only the
  relevant next step and an accurate boundary statement.
- The local assistant setup displays the exact client ID used by generated
  configuration and its current role, or a clear “not authorized yet” state.
- Observer, Editor, and Operator are explained using concrete capabilities;
  the UI states that none of these MCP roles grants device administration or
  permits pressing Play through MCP.
- The user can reach the existing client authorization controls for that exact
  client without a hidden grant change.
- A successful local read-only check is distinguishable from “waiting for the
  external assistant to call” and from “external connector not yet accepted.”
- Known denied activity (permission, unavailable, stale/revoked, conflict,
  invalid input) has a useful next action while retaining redaction.
- Existing ChatGPT tunnel setup, local Cowork export, generated Codex/Claude
  configuration, tool counts, modern/legacy protocol behavior, and backend
  scopes continue to work.

## Baseline evidence

- `ui/src/Workbench.tsx` currently presents ChatGPT, Cowork, and local assistant
  sections in one long MCP page. Client permissions are rendered separately by
  `ui/src/AdvancedPanels.tsx` and passed into the page as an opaque React node.
- The generated local command includes the exact CLI, database, pipe, and
  client ID. Copy actions are already hidden when the installed CLI is absent.
- `ClientsPanel` currently lists client IDs and roles, but gives no role
  description and does not identify the selected setup client's current state.
- MCP activity already exposes tool name, safe argument field names, outcome,
  client ID, and bounded error kind. It intentionally excludes argument values,
  credentials, paths, and audio/media data.
- Existing UI E2E coverage exercises disconnected/connected tunnel states,
  missing CLI, local command copy, Cowork export visibility, three themes, and
  redacted activity. Existing stdio coverage exercises modern and legacy
  protocol behavior and denied writes.

## Scope

Included:

1. A guided connection-path selector or equivalent progressive disclosure on
   the MCP page, preserving direct access to all supported setup paths.
2. Role explanations and exact-client status, using existing client-list data
   and authorization controls; no grant is created or changed implicitly.
3. A read-only local verification affordance/state model based on existing
   setup information and bounded activity, with language that does not promise
   external-client acceptance.
4. Actionable, privacy-safe display of known MCP failure categories.
5. Focused component/unit and Playwright regressions, three-theme visual review,
   and updated MCP-facing operational documentation where wording changes.

Excluded:

- MCP wire protocol or supported-version changes.
- New API methods, permissions, roles, contracts, or default tools.
- Generic remote MCP, hosted relay, OAuth, LAN listener, or cloud Cowork.
- OpenAI tunnel credential handling, startup policy, or runtime replacement.
- Audio routing, graph behavior, driver work, or live device access.
- Automatic enrollment, role escalation, or automatic Play authorization.

## Decisions

1. **Keep the working transports.** UX labels and progressive disclosure may
   change, but generated stdio commands, the tunnel runtime, Cowork ZIP format,
   client IDs, and protocol envelopes remain compatible.
2. **One exact identity.** The setup client ID remains the identity used in the
   command/configuration. Its role is read from the existing client list. If it
   is absent or revoked, the UI explains that authorization is needed; it does
   not repair the state automatically.
3. **Role language is capability language.** Observer means read-only;
   Editor means saved graph/session edits; Operator adds controls for already
   running audio. Device administration, microphone consent, and Play remain
   desktop-only boundaries.
4. **Verification means local evidence.** The UI can show CLI availability,
   setup configuration, and observed redacted activity. It cannot assert that
   ChatGPT/Cowork accepted a connector or that an assistant will use a tool
   until activity is actually observed.
5. **No layout shift.** Selector changes swap reserved content slots; tunnel and
   local verification messages retain their existing stable-slot behavior.

## Ordered implementation tasks

1. Add this plan and capture the clean isolated branch baseline.
2. Trace the existing Workbench/App/backend data flow and client-list loading;
   choose the smallest data path that exposes exact-client status without a new
   backend/API contract.
3. Implement role descriptions and a reusable exact-client status presentation.
   Preserve the existing `ClientsPanel` authorization/revocation actions.
4. Implement progressive connection-path guidance. Ensure the selected path
   does not alter tunnel state, client grants, generated command values, or
   external URLs.
5. Add a read-only local check/status presentation. Exercise success, waiting,
   missing CLI/backend, unrecognized client, revoked client, and permission
   denial through the real UI entry path or existing fixture adapter.
6. Update user-facing MCP runbook/example wording if the new flow changes the
   documented sequence. Keep the remote/cloud boundary explicit.
7. Add focused unit tests and extend the existing MCP Playwright fixture for
   all three themes, fixed-height/status-slot behavior, exact client role,
   denied action guidance, and the external-acceptance distinction.
8. Run formatting, lint, typecheck, focused tests, MCP stdio regression, docs
   acceptance, and visual browser checks. Inspect the complete diff and record
   limitations here.

## Validation matrix

| Area | Check | Evidence target |
| --- | --- | --- |
| UI logic | MCP Workbench/Vitest focused tests | setup path, role text, status/error states |
| Real UI entry point | Existing MCP Playwright tests | clicks, exact setup text, client role, activity |
| Themes/layout | Edge screenshots dark/light/high contrast | no layout shift or clipped controls |
| MCP compatibility | `cargo test -p audiorouter-cli --test mcp_stdio --locked` | modern/legacy, tool catalog, denial/redaction |
| API/MCP parity | existing CLI parity test if source catalog touched | no drift or explicit unchanged result |
| UI quality | `npm --prefix ui run format:check`, lint, typecheck, tests | zero formatter/lint/type errors |
| Documentation | `tests/acceptance/docs.ps1` | local links/headings/fences |
| Diff hygiene | `git diff --check`, full diff/status review | only scoped branch files |
| Windows/native | not required for this UI-only slice | no audio/device claim; state limitation |

## Prerequisites

- Windows checkout with the pinned Node/Rust toolchains.
- Existing UI dependencies or a locked `npm ci` if needed.
- No audio endpoint, driver, external credential, OpenAI account, Cowork
  account, live owner profile, or running shell is required.
- Browser screenshots use deterministic fixtures and are UI evidence only.

## Risks and mitigations

- **Existing user edits conflict:** worktree is isolated from the dirty checkout;
  do not cherry-pick or rewrite unrelated files.
- **Role status becomes stale:** label it as last observed and provide Refresh;
  never infer a grant from the typed client ID.
- **External client is confused with local readiness:** use separate status
  labels and explicit “waiting for assistant activity” wording.
- **Permission UX accidentally changes authority:** reuse existing list,
  authorize, and revoke actions; no new backend grant path.
- **MCP page becomes taller or shifts:** keep status/action slots reserved and
  verify geometry across themes and sidebar widths.
- **Protocol regression hidden by UI tests:** run the existing real-binary stdio
  integration test unchanged.

## Rollback

Revert only the scoped MCP UX plan/source/test/documentation files on this
branch. No database migration, grant migration, protocol version change, API
contract change, credential change, or audio state change is expected. The
existing MCP setup and transport remain recoverable by removing the UX changes.

## Implementation progress

- Added a path selector for On this PC, Local Cowork, and ChatGPT. Each choice
  reveals only its applicable setup surface; the cloud-Cowork boundary remains
  explicit.
- Added a two-second client-list refresh alongside existing MCP activity. The
  local setup displays the exact configured client ID as authorized Observer,
  Editor, Operator, revoked, or not authorized.
- Added role capability explanations and an explicit reminder that MCP roles do
  not grant device administration or Play.
- Added bounded local-check wording and known redacted error guidance for
  permission, unavailable, conflict, invalid, not-found, rate-limit, and
  unsupported outcomes.
- Passed the selected client ID into the existing Client permissions panel as an
  initial value without changing authorization behavior.
- Added Workbench tests for path switching, role text, revoked state, and
  permission-denial guidance.
- Added Playwright coverage for the three connection paths, exact client role
  prefill, stable Cowork/ChatGPT status slots, missing CLI guidance, and
  dark/light/high-contrast captures. Existing Cowork export coverage now
  selects the Local Cowork path and checks new, Editor, and revoked identities.

## Current next action

Validation is complete for the available portable and fixture-backed gates. The
remaining limitation is that the browser run uses the development disposable
harness; no packaged shell, live owner profile, external ChatGPT connector, or
native audio/device qualification was performed.

## Validation record

Windows checkout, branch `feature/mcp-setup-ux`, 2026-11-04:

- `npm --prefix ui run format:check` passed.
- `npm --prefix ui run typecheck:all` passed.
- `npm --prefix ui run lint` passed with zero warnings.
- Focused Workbench/status tests passed: 10 tests.
- Full UI suite passed: 93 files, 823 tests.
- Development Playwright MCP path scenario passed: 1 test, including local,
  Local Cowork, ChatGPT, exact-client permission targeting, stable status
  geometry, and three theme captures.
- Development Playwright Cowork MCP suite passed: 3 tests for new, Editor, and
  revoked client states.
- `cargo test -p audiorouter-cli --test mcp_stdio --locked --no-fail-fast`
  passed: 5 tests, including modern/legacy stdio interoperability, Cowork
  identity, and denied Observer writes.
- `tests/acceptance/docs.ps1` passed: 239 Markdown files and 1,410 local links.
- `git diff --check` passed after removing generated `node_modules/` and
  `test-results/` artifacts.

The CI Playwright configuration was not used because its prebuilt
`target/browser-acceptance-ui` assets are absent in this isolated worktree;
the development harness supplied the browser evidence instead. No API,
contract, authorization semantic, MCP protocol, transport, credential, or
native audio changes were made, so contract-drift and native/audio gates were
not required for this slice.
