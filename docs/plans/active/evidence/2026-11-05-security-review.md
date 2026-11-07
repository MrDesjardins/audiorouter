# Application security review — 2026-11-05

## Scope and result

Read-only review of the current application source, focused on SEC-01/02/03/04,
SEC-05/07/09/12/13/14 and HTTP-01/03/06. Source HEAD was
`0a5d7677c20167b740d70b7fc08f597b5cfc8a1e` on `release/0.0.27`; the checkout
contained substantial concurrent, uncommitted UI/site/documentation work, which
was preserved. This is a bounded source and local abuse review, not a security
certification or a complete penetration test. No application fixes, release,
PR, device configuration, driver installation or owner-route changes were made.

**Result: one high-severity reproduced authorization failure, one medium-severity
file-authorization finding, and one medium-severity pipe identity gap.** Current
dependency and scanning checks do not establish that the application is safe:
the authorization failure passes the existing focused tests.

## Findings

### High — MCP pipe forwarding discards the enrolled client's authority

Location: `crates/cli/src/lib.rs:3206` (`mcp_api_value`), with the fixed server
grant applied at `crates/transport/src/lib.rs:1900` and the desktop grant selected
at `src-tauri/src/main.rs:1753`.

The no-pipe MCP path calls `dispatch_authorized_for_client(request, client_id,
grant)`. The pipe path forwards only the JSON-RPC method/parameters. Neither the
enrolled MCP client identity nor its grant reaches the backend. The pipe server
identifies the OS user and dispatches every connection with its startup grant.
The normal desktop backend supplies the desktop grant, including privileges
that its comments expressly exclude from enrolled CLI/MCP clients.

An already enrolled **Observer** assistant connected to the live backend can
therefore use ordinary default mutation tools despite its read-only role. This
does not require malicious code execution on the PC: the assistant sends a
normal `tools/call` through its configured trusted sidecar. The broader desktop
grant also makes desktop-only backend checks ineffective for such forwarded
calls. HTTP has a separate denylist; the MCP pipe path does not.

Windows reproduction used the existing `target/debug/audiorouter-cli.exe`
(reports version 0.0.26), a separate review database, a random named pipe and
the real `mcp serve` stdio entry point. With `review-observer` enrolled as
Observer, default tool `create_session` returned `permissionDenied: GraphWrite`
without a pipe. The same tool with `--pipe` against an operator backend returned
`isError: false` and a new session ID. No audio was opened. The isolated backend
was terminated after the probe. An initial unsupported handshake version and
an insufficient connection count were corrected before the successful run.

Reproduction artifacts: `target/security-review-2026-11-05/probe.py` and
`target/security-review-2026-11-05/results.json` (local, ignored scratch evidence).
The CLI fixture uses an operator backend rather than the running desktop; the
source trace establishes that the desktop supplies an even broader grant.
The review did not run the exploit against the owner's instance or a remote relay.
The existing CLI binary was not rebuilt for this probe; the current source's
matching forwarding and server-grant paths were independently inspected.

Remediation: authenticate and preserve a distinct client identity end-to-end;
resolve and enforce its current grant in the backend on every request, including
revocation. Keep the desktop channel distinct. Merely checking permissions in
the sidecar leaves direct pipe requests and backend authorization incorrect.
Add a regression using the real pipe/MCP entry point: Observer mutations denied,
Editor cannot consent to devices/record/startup/OBS, revocation affects a live
sidecar, and intended desktop/HTTP authority still works.

### Medium — graph-edit permission authorizes file export outside approved roots

Location: `crates/domain/src/lib.rs:1566`,
`crates/control/src/sessions.rs:938`, and `crates/control/src/sessions.rs:26`.

`sessions.exportFile` requires only `GraphWrite`, so an Editor has permission
to invoke it. Its supplied path is checked for being absolute and having a
`.audiorouter` extension. Storage rejects existing destinations and reparse
ancestors for a new export, but does not require an owner-approved export root.
With `replace: true`, the handler explicitly replaces an existing regular
`.audiorouter` file, still without a root grant or a separate destructive action
permission. HTTP also forwards this method with the desktop grant.

Thus an editor/integration authorized to edit a graph can create session bundles
in any writable directory or replace unrelated session backups. A bundle includes
imported audio and plugin states, so a Windows UNC destination can additionally
cause private bundled media to be written to a network share if the account has
access. Path inspection itself can contact that share. No UNC traffic or external
exfiltration was attempted during this review. This is a confirmed source-level
authorization gap, not a demonstrated arbitrary-extension overwrite or code
execution vulnerability; the `.audiorouter` restriction remains effective.

Remediation: define an explicit export/file-write authority and approved roots,
require separate deliberate replacement authority, and reject network paths unless
that destination was explicitly authorized. Apply the rule in the backend shared
by REST/MCP/UI. Test Editor denial, outside-root denial, UNC denial and deliberate
desktop export/replacement success. Requirements: SEC-03/04/09.

### Medium — pipe clients do not authenticate the server; first-instance protection absent

Location: `crates/transport/src/lib.rs:1241` and
`crates/transport/src/lib.rs:1385`.

Server creation uses `PIPE_ACCESS_DUPLEX` without `FILE_FLAG_FIRST_PIPE_INSTANCE`.
Client connection returns an opened pipe handle immediately, without checking
the server's process/token identity. The server-side owner ACL and client SID
check protect the genuine server, but do not authenticate a pipe that another
process created first. The separately named server mutex coordinates genuine
instances; it does not authenticate a pre-created pipe.

A local attacker able to claim the predictable pipe name before startup can
impersonate the backend to clients, receive requests and fabricate responses;
a different account can create a permissive pipe for the intended account to
open. This fails the explicit SEC-01 anti-squatting and backend-identity contract.
No cross-account squatting experiment or impersonation payload was run, so
cross-account impact is a source-supported attack prerequisite, not measured
Windows exploit evidence. Same-user process compromise remains outside the
strong isolation promise documented in the threat model.

Remediation: coordinate first-instance creation with the multi-instance listener
and validate the connected server's expected identity before exchanging application
messages. Include Windows cross-user/logon tests and reject a squatted pipe.
Do not infer cross-user protection from same-user round-trip tests.

## Checks and positive controls

Parent reviewer supplied these Windows results for this review:

- `cargo deny --manifest-path Cargo.toml check advisories` and
  `cargo deny --manifest-path src-tauri/Cargo.toml check advisories` passed;
  `deny.toml` restricts the graph to `x86_64-pc-windows-msvc`. Workspace emitted five
  unused advisory-ignore warnings for exceptions relevant to the shell graph;
  these are configuration maintenance, not proof of runtime exposure.
- `npm audit --omit=dev --json` reported zero production vulnerabilities in
  `ui`, `contracts`, and `tools/streamdeck`. Full UI audit reported the moderate
  development-only minimatch denial-of-service advisory GHSA-g3hv-75wg-m249
  (ten affected dependency entries, one root advisory). Update the development
  dependency chain; this is not ten independent application vulnerabilities.
- GitHub `code-scanning/alerts?state=open` returned an empty list at review time.
- `cargo test -p audiorouter-control --locked authorization::tests::`: 25 passed.
- `cargo test -p audiorouter-protocol --locked network_audio::tests::`: 6 passed.
- Windows network-audio transport suite: 17 passed, including socket pairing,
  stale receiver replay rejection and clock-drift checks; no physical audio devices.

Source inspection found exact HTTP Host/Origin checks, bearer comparison,
desktop-only HTTP denials, bounded request handling, current-user DPAPI secret
storage, restrictive packaged CSP, owner-only genuine-pipe ACLs, isolated plugin
jobs and bounded authenticated network-audio parsing. These controls reduce risk
but do not compensate for the MCP authorization finding.

## Limits, immediate containment and next task

No full fuzz campaign, hostile plugin execution, kernel/driver audit, second-user
Windows test, LAN adversary exercise, signing/installer validation or remote relay
penetration test was performed. Plugin job isolation is crash containment rather
than a filesystem/network sandbox, as SEC-07 discloses. LAN HTTP deliberately
uses plaintext bearer authentication; it should only be enabled on a trusted
network. Unpaired network audio is deliberately unauthenticated plaintext.

Until the authorization fix is verified, disconnect remote MCP and avoid exposing
the live backend to assistants/clients expected to be read-only. Do not rely on
Observer/Editor roles for pipe-connected MCP. Keep the optional HTTP listener
stopped unless needed, and use trusted plugins and paired audio on trusted networks.

Next task: fix and exercise the MCP authority boundary first, then file-export
authorization and pipe server identity, with full entry-point regressions and
updated API/MCP contracts. A fix requires a separate authorized implementation
change; this report does not mark the security milestone or release gates complete.
