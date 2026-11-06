# Security review repairs — 2026-11-05

## Owner-approved PR isolation

The owner subsequently requested a PR containing all security fixes while the
other T3 Code thread continues UI work. The security patch and its two code
files/two evidence files were copied to `fix/security-boundaries`, based on
`origin/main` at `0a5d7677`, in `C:\code\audiorouter-security-fixes`. The shared
checkout/index were preserved. Its mixed active plan was not copied; this
branch records its own security-only execution entry. No UI source, recordings,
guide assets, unrelated release metadata or UI plan changes are included.
The only UI package changes are the minimatch security override and lockfile.
The earlier mixed-checkout failures below are historical; isolated results
are recorded here after validation. The owner authorizes PR creation, not merge.

Isolated branch validation passed on Windows with Rust 1.96.0/Node 22.22.3:
`cargo test -p audiorouter-control -p audiorouter-cli -p audiorouter-transport
--locked` (control 337, CLI 48, stdio integration 5, security integration 1,
transport 37, portable continuity 1; hardware/attended fixtures remain ignored),
shell tests (87 passed, 4 ignored), both strict all-target/all-feature Clippy
runs, Rust formatting and checks, contract typecheck/drift, documentation
validation (242 files/1,414 links), and diff whitespace. Logs are retained in
the worktree's ignored `target/security-pr-tests.log` and Clippy logs.

The broader run exposed two older stdio fixtures using an unenrolled in-memory
backend despite enrolling their sidecar database. Both now use the enrolled
database at the backend; all five stdio integration tests pass. No permission
fallback or relaxed production grant was added to make the fixtures pass.

Isolated UI `npm ci --ignore-scripts`, format, all-files typecheck, lint,
823 Vitest tests and the normal production build passed. No UI source/e2e
files changed. Full npm audit reports zero vulnerabilities; both Rust advisory
checks passed. Warnings retained with scope recorded: the unchanged main
`App.startupOrder.test.tsx:58` mock emits two `disconnected.obsStatus` errors
while its assertions pass (same source as main, left for the parallel UI task);
Vite's 529.14 kB chunk warning concerns bundle layout and is not suppressed;
five unused root advisory exceptions belong to the shared shell policy.
These warnings do not establish new security exposure and remain explicit
qualification/maintenance work. Hosted CI, cross-account/logon testing and
attended packaged/native-dialog qualification remain separate evidence gates.

## Scope and decisions

Owner requested all findings in the [security review](2026-11-05-security-review.md)
be fixed. Work is local and uncommitted on `release/0.0.27`, based on
`0a5d7677c20167b740d70b7fc08f597b5cfc8a1e`, with concurrent UI/site work preserved.
Requirements: SEC-01–04/09/12/14, HTTP-01/03/06, AUTO-18, ENG-02/04.
No PR, release, driver action, owner-route changes or deployment is included.
The installed/release binaries have not been replaced.

## Implemented repairs

- **MCP authority:** `crates/cli/src/lib.rs` wraps forwarded requests with a
  credential for its configured enrollment identity. Credentials are random,
  stored with current-user DPAPI and bound to the client ID by their protected
  storage location. `crates/transport/src/client_credentials.rs` bounds reads,
  refuses reparse storage paths, publishes without replacing an existing file,
  and fails closed on corruption. Credentials are stripped before dispatcher
  and diagnostic handling. The backend resolves the current enrollment on every
  call; the standalone MCP mode also refreshes its grant for every message.
  Storage reads take precedence over cached enrollments, so live revocation
  and role changes are enforced. Only the hosting process can use its desktop
  grant. Legacy raw external pipe calls are read-only, even when their Windows
  user is enrolled as Operator. Same-user malicious code able to recover DPAPI
  credentials remains outside the documented isolation boundary.
- **Pipe identity:** native clients query the server process before sending
  payloads and require the same Windows user SID and authentication/logon ID.
  The server applies the same check to clients. A first-instance pipe guard is
  connected to the server itself and retained throughout the server lifetime;
  the four request instances cannot leave a squatting gap. The reservation is
  released before the singleton mutex. This authenticates the OS user/logon,
  not a particular executable against hostile code under that same user.
- **Exports:** added desktop-only `fileExport`, separate from graph editing,
  and `sessions.approveExport`. The existing native Save dialog forwards an
  approval after its path/overwrite consent. Approval covers one exact local
  destination for five minutes and is consumed on use. Backend checks refuse
  traversal, UNC/device paths, mapped network drives and reparse ancestors;
  replacement needs explicit approval and an unchanged destination fingerprint.
  Staging no longer deletes a predictable file before writing. HTTP refuses
  approval and bundle export; both methods are explicitly excluded from default
  MCP tools because the window owns file consent. Scripts retain JSON export
  via `sessions.export`. Approval disappears on backend restart.
- **Contracts/docs:** 139 API methods, schemas, typed contracts, HTTP/OpenAPI
  exclusions, API reference, privacy/security docs and public API catalog agree.
  The site generator derives desktop-only exclusions from the HTTP adapter.
  Its API-only mode preserves unrelated tool catalogs and guide work. No audio
  tool, Properties control or visual styling changed.
- **Development dependency:** pin the minimatch override to 11.0.1 to address
  GHSA-g3hv-75wg-m249. The package supports Node 18, 20 and >=22, compatible
  with local/CI Node 22.22.3. ESLint exercises the updated matcher successfully.

## Windows evidence

Pinned Rust 1.96.0, Windows host, Node 22.22.3. Tests use disposable databases,
random pipes and test processes; no physical audio devices or owner instance.

- `cargo test -p audiorouter-cli --locked --test security_boundaries`: passed.
  Starts the freshly built CLI's real backend and MCP stdio entry points.
  Default-tool Observer mutation is denied. Editor/Observer cannot use
  desktop-only methods; Operator's allowed workflow succeeds; revocation
  affects an already-running sidecar. Raw same-user mutation and a forged
  credential are denied. Child profile/credential/log paths are isolated.
- `cargo test -p audiorouter-control --locked --lib`: 337 passed, 11 ignored.
  After final export/role refinements, `authorization::tests::` (25) and
  `sessions::tests::` (17) passed again. Export regression exercises the
  authorized dispatcher, exact/unapproved paths, consumed/expired consent,
  overwrites, a destination changed after consent, UNC/device rejection and
  an actual Windows directory junction. Removing that fixture junction
  preserves its real target.
- `cargo test -p audiorouter-cli -p audiorouter-transport --locked --lib`:
  CLI 48 passed; transport initially 36, then all 37 passed after adding DPAPI
  credential/corruption coverage. First-instance tests reject an existing
  squatter, keep the name reserved between requests, and prove release on
  shutdown. Identity checks accept the current process and reject an
  unqueryable PID. Existing bounded/concurrent pipe tests still pass.
- `cargo test --manifest-path src-tauri/Cargo.toml --locked`: final 87 passed,
  4 ignored. Includes HTTP 403 for approval/export, OpenAPI exclusion/parity,
  and the native Save-result approval helper followed by a real pipe export
  and denial of reuse. No native dialog was opened for this test.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  and the same command for `--manifest-path src-tauri/Cargo.toml`: passed.
  The first run identified a test-module placement diagnostic, which was fixed.
- `cargo fmt --all`, `cargo fmt --manifest-path src-tauri/Cargo.toml`, and both
  `-- --check` runs passed. A transient Windows file-mapping error on the first
  shell formatter attempt cleared on retry; no unrelated Rust files changed.
- `cargo check -p audiorouter-cli --locked` and shell `cargo check`: passed.
  Lockfile changes only add the direct dependencies used by these repairs.
- Contracts typecheck/drift passed: 139 methods, 37 node kinds, 20 processors,
  22 event categories. API catalog regeneration and tool-guide catalog check
  passed (221 public parameter descriptors). Documentation and diff whitespace
  validation passed.
- `npm ci --ignore-scripts` and UI lint passed with the updated matcher;
  `npm audit --json` reported zero vulnerabilities, including dev dependencies.
  UI application `npm run typecheck` and direct Vite production build passed.
  The normal build's all-files typecheck is not green (see below).
- Rust advisory checks passed for both Windows dependency graphs. Five root
  `advisory-not-detected` warnings remain intentionally documented: the shared
  policy's dated unic exceptions apply to the shell graph, not the root graph.
  No vulnerability advisory was suppressed or newly ignored.

## Remaining checks and limits

The concurrent UI work has independent failures and was not edited:

- `npm --prefix ui run typecheck:all` fails at
  `ui/e2e/ui-round-20-fixes.pw.ts:105`: a fixture assigns `endpointId: undefined`
  to a scalar-only parameter record. The normal `npm run build` includes this
  check; the separate application typecheck and Vite build do not waive it.
- Full UI Vitest: 878 passed, 1 failed. `App.lifecycle.test.tsx:479` expects
  “Select the speaker or headphone device” while the current UI says
  “Choose the speaker or headphones”. `App.startupOrder.test.tsx` also emitted
  `disconnected.obsStatus is not a function` from its test double. These were
  observed in the shared checkout with the security dependency update; no
  clean-baseline comparison was attempted while that work remained active.
- Vite reports a 541.59 kB application chunk warning; no bundle-limit change
  or suppression was made as part of the security repair.

No cross-account/logon exploit was run: another authenticated Windows account
was unavailable. The real first-instance and current-token tests are not a
substitute for that gate. No attended Save dialog, packaged WebView2 smoke,
driver audit, hostile plugin sandbox qualification, remote-relay penetration
test or full fuzz campaign was run. The owner instance was preserved; the
default-pipe packaged smoke is unsuitable while it is running. These repairs
do not close all M07/M08 release/security gates or certify the whole application.

## Compatibility, rollback and next action

Update the shell and MCP sidecar together. Older raw sidecars lose mutation
authority; they must not fall back to the desktop grant. Exports formerly
attempted through HTTP or a graph-edit grant now need the native Save workflow.
There is no session/database migration; new credential files are current-user
protected and reveal no token in configuration/arguments.

Rollback is a scoped source revert and compatible prior app/configuration pair;
doing so reintroduces the reviewed flaws. Keep remote/restricted MCP disconnected
on an older build. Next task: reconcile the independent UI checks, then qualify
a matching packaged shell/sidecar and attended native Save workflow on an
isolated Windows instance before release. Cross-account/logon testing remains
an explicit security gate. No PR should be opened without owner approval.
