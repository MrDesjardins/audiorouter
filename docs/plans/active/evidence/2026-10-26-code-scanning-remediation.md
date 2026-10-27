# Code-scanning remediation evidence — 2026-10-26

## Current scan and alert inventory

Default branch `main` is `8ecf9f91912aaf4988ca1b3b345c2cbb229adef8`
(PR #73 merge). `gh api repos/MrDesjardins/audiorouter/code-scanning/alerts`
reported 38 open findings before this work:

| Rule | Count | Locations / initial finding |
| --- | ---: | --- |
| `rust/hard-coded-cryptographic-value` | 12 | OBS and network authentication values in test-only modules |
| `rust/cleartext-logging` | 22 | Session-ID flows through tests/examples, API/resource results and persistence paths |
| `rust/weak-sensitive-data-hashing` | 1 | OBS WebSocket v5 challenge-response implementation |
| `js/incomplete-multi-character-sanitization` | 1 | Markdown heading slug lookup |
| `js/bad-code-sanitization` | 1 | Browser test expression built from a closed guide-kind label choice |
| `cpp/memset-may-be-deleted` | 1 | Self-test intentionally poisons ignored appended bytes |

Analysis records identify the original findings on `11737991135112c7085f6b45a88770f36287fd04`; the CodeQL scan after #73 merged uses `8ecf9f91912aaf4988ca1b3b345c2cbb229adef8`. The only alert action taken was dismissing alert #27 (`obs_tests.rs` test fixture) as `used in tests`, with a comment that it is compiled only under `cfg(test)` and is not a production credential. The remaining open count is 37.

## Source-to-sink findings

- The 12 literal values are located in `crates/control/src/obs_tests.rs`,
  `crates/protocol/src/network_audio.rs`'s `#[cfg(test)]` module and
  `crates/windows-audio/src/network_audio.rs`'s test module. They are fixture
  credentials/keys, not runtime defaults. Alert #27 was dismissed as test-only;
  the other 11 remain open pending an approved path to clear test-only results.
- `crates/cli/src/lib.rs::mcp_tool_activity_record` serializes only tool name,
  allowed argument field names, outcome, safe error category and correlation
  id. Its existing test `mcp_activity_records_tool_identity_and_safe_argument_names_only`
  includes `session-private` in a request and asserts that value is absent from
  the serialized record; `sessionId` remains only as a field name.
- `crates/domain/src/lib.rs` `RetainedEvent` stores `session_id` in the bounded
  in-memory state-event ring for API filtering; this is not a file log.
- Control operation journal records are persisted to the local application
  database to support idempotent replay. This is durable user state, not the
  default diagnostic/support log. API and CLI command results intentionally
  return resource IDs to the requesting client. The VM example prints only
  its fixed `vm-cable-route` fixture ID; recording and CLI locations are test
  assertions. No password, pairing key, audio payload or file content was
  identified in these flows.
- `EntityId::new` accepts an arbitrary string. Although resource IDs are not
  authentication secrets and SEC-10 requires affected-resource audit context,
  an ID may be user chosen. Follow-up source review found no production
  diagnostic logger emitting raw session IDs: the flagged paths are test
  fixtures/assertions, API/resource results, in-memory events, or the local
  idempotency journal. These are not default diagnostic/support logs. The API
  returns IDs to the requesting client by contract; the local database stores
  them as required resource identity and replay state. No blanket redaction is
  warranted because it would reduce debugging value or break those contracts.
  Keep the 22 findings open until the new CodeQL run confirms the exact alert
  locations and rule behavior; if any instance resolves to a real log sink,
  preserve correlation with a stable session-scoped fingerprint and retain
  the raw ID only in the required API/database paths.
- `crates/control/src/obs.rs` follows OBS WebSocket v5's mandatory challenge
  response formula. SEC-15 states the password is memory-only, never logged or
  stored. Do not change the formula or misclassify it as password storage.
- `tools/docs/validate.mjs::slugify` returns a Markdown anchor used in Set/Map
  matching; it does not create HTML. `tools/site/firefox-guide-audio.mjs`
  selected one of two literal button labels from `guide.kind` and JSON-escaped
  it before evaluating a browser expression. The code now uses one of two
  static expressions, avoiding interpolation entirely.
- `tools/m03-scheduler-trace/main.cpp::SelfTest` writes 0xff to a test fixture's
  unused appended bytes to prove the decoder ignores extensions; it does not
  erase secrets. It now uses `std::fill_n` with an explanatory comment instead
  of `memset`.

## Automatic review boundary

A bulk GitHub API action to dismiss alerts #4–26 and #28–38 was rejected by
automatic approval review because it would dismiss 22 production
cleartext-logging alerts without proving that each session identifier is
non-sensitive. Do not retry the same action by splitting it into batches. The
privacy question remains open; no logging or storage contract was weakened.

## Local changes and verification

The working branch is `security/code-scanning-remediation`, based on the
merged main SHA above. It contains only two narrowly scoped tooling changes so
far: remove dynamic JavaScript source interpolation from the Firefox guide
test and make the C++ decoder fixture's nonzero test data explicit. No audio,
OBS, network-pairing, API or persistence behavior changed. Local syntax and
format checks passed: `node --check tools/site/firefox-guide-audio.mjs` and the
UI Prettier check for that file. Documentation validation passed 200 Markdown
files / 1,099 local links; `git diff --check` passed. No C++ compiler is
available in this shell, so the modified C++ file was not compiled locally. No
tests were added or run. Hosted CodeQL re-analysis requires a pushed branch/PR;
opening a PR still needs the owner's explicit approval under `AGENTS.md`.

## Next action

Inspect the complete diff, commit and push the focused branch, then request the
owner's approval before opening a PR. Resolve with the owner whether arbitrary
session IDs are not secrets, and no production diagnostic log sink was found.
Keep the 22 alerts open pending new-head CodeQL evidence; do not redact API or
database resource IDs. If a real diagnostic sink is identified, use a stable
session-scoped fingerprint to retain log correlation.

## Follow-up inventory — 2026-10-27

After PRs #74 and #79 merged, GitHub's open-alert API returned 35 findings:
22 `rust/cleartext-logging`, 11 `rust/hard-coded-cryptographic-value`, one
`rust/weak-sensitive-data-hashing`, and one
`js/incomplete-multi-character-sanitization`. The prior browser-expression and
C++ self-test findings are absent from this open inventory. This is a snapshot,
not a dismissal or a conclusion that all remaining findings are false positives.

The 22 logging findings report nine locations in the M03 example, five in
recording tests, one CLI test, one CLI entry point, four domain locations,
one plugin location and one persistence location. Initial comparison shows
that a reported line alone is insufficient to establish a sink: alert #9's
message includes paths in persistence and storage while its primary location
points into the domain file. Inspect the individual data-flow paths and the
source revision before classifying each finding. No alert state was changed.

Next: obtain and inspect the individual CodeQL data-flow paths, map each to
its current source and sink, and record an evidence-backed disposition under
SEC-10/13/15. Keep API and database identities intact and retain the earlier
automatic-review restriction on bulk dismissal. Rollback: discard this
follow-up evidence; no runtime behavior has changed.

## Current-main SARIF data-flow review — 2026-10-27

Downloaded the GitHub CodeQL Rust analysis `2033284868`, category
`/language:rust`, for main commit `26de4a4f1af13577cd644f333f0a47b2b613afb9`.
The SARIF contains 22 cleartext-logging result locations and four paths for
most locations. It contains findings even when an alert has been dismissed;
alert state must be taken from the alert API separately. Source lines below
refer to this exact revision, not an earlier scan.

- Alerts #4–9 terminate at six collection expressions:
  `persistence.rs:530` (`operation_order`), `plugins.rs:594`
  (`plugin_inventory_order`), and `domain/src/lib.rs:2620`, `:2892`,
  `:3355`, `:710` (`events`, crash timestamps, committed-key order, buses).
  Each result has four paths beginning at `load_active_session_id` or
  `validate_session_id`; the trace propagates through a whole `ControlPlane`
  receiver into fields of other objects. These reported sinks are in-memory
  collections, not log-file writes. The collections still warrant their own
  retention/privacy review; the SARIF path alone does not show a session ID
  reaches any of those particular fields.
- Alert #16 ends at `crates/cli/src/main.rs:22`, `println!("{output}")`.
  The four SARIF paths converge through a generic dispatcher and claim to reach
  `presets_command` (`cli/src/lib.rs:317–324`), which issues the fixed
  `presets.list` request; the scanner merges return values from unrelated
  dispatcher arms. CLI stdout is a caller-requested result and can be
  redirected to a file; it is not the automatic diagnostic logger. Do not
  infer that session IDs are secret-free, or that arbitrary CLI output is
  never retained. Resource identities in requested API results are contractual.
- Alert #10 is an assertion in `cli/src/lib.rs:4557`; #11–15 are assertions or
  failure messages in `control/src/recording_tests.rs:282,399,403,409,494`.
  Those test fixtures may print values when tests fail, but are not shipped
  diagnostic logging paths. Alert #17–25 are the VM cable example at lines
  83,178,182,190,209,234,248,260,262; that tool constructs an in-memory
  session with fixed ID `vm-cable-route` and prints test responses or
  diagnostics. Its user-supplied endpoint arguments remain distinct from the
  flagged session-ID flows; this analysis does not certify all example output.
- The current JavaScript analysis `2033263113` locates the sanitization alert
  at `tools/docs/validate.mjs:18–19`: `slugify` creates a heading lookup key;
  its result is used for `Set` membership in Markdown link validation, not
  inserted into HTML. Its regex is not a general-purpose HTML sanitizer.
- The Rust report flags `obs.rs:700`: SHA-256 participates in the fixed OBS
  v5 challenge-response protocol, not a password storage scheme (SEC-15).
  Other reported hard-coded credentials are within `obs_tests.rs` or the
  `#[cfg(test)]` modules of the protocol and Windows audio network files.
  Verify the current GitHub state per alert before proposing a disposition.

This is a review of reported flows, not proof that every possible diagnostic
path is safe. No alert was dismissed, and no bulk dismissal was retried after
the earlier automatic-review rejection. Next: inspect the remaining fixture
locations and any separate real diagnostics paths, then record individually
supported decisions. The Rust and JavaScript SARIF reports can be retrieved
read-only from the GitHub analysis IDs above.

## Confirmed shell diagnostic exposure — 2026-10-27

The earlier assertion that no production diagnostic logger emits raw session
IDs was incorrect. `src-tauri/src/main.rs` writes request `sessionId` directly
into `shell.jsonl` when it consists of permitted ASCII characters; user-chosen
IDs can contain private information even when they meet that filter. Support
bundles currently copy old shell-log lines after path-only redaction, so they
can include those historical IDs. This is independent of the reported CodeQL
logging paths. Scope: SEC-10; reproduction is a shell RPC with an ordinary
user-chosen alphanumeric ID, and the affected versions include the current
0.0.23 development build.

Decision and ordered work: replace new raw shell-log IDs with process-scoped
keyed fingerprints for correlation, preserving API identity and request IDs.
Remove legacy `sessionId` from shell log records at support-bundle export;
reject malformed shell log lines in the bundle rather than leaking their raw
contents. Preserve old local logs untouched. Add focused tests for stable and
distinct fingerprints, raw ID absence, and legacy/malformed bundle records;
run shell tests, formatting and Clippy, documentation validation and inspect
the diff. Hardware is not needed. Revert the narrow shell logging and bundle
changes to roll back; old logs remain on disk. Next: implement and validate
without interrupting the owner's running test build.

Validation on Windows 2026-10-27: shell regression tests passed for the
logged session reference, raw-ID absence, and legacy/malformed bundle lines;
`cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` passed
85 tests with 4 ignored. Workspace and shell Clippy with all targets/features,
`--locked --offline` and `-D warnings` passed. Both `cargo fmt --check`
commands, `node tools/docs/validate.mjs` and `git diff --check` passed.
Three earlier focused workspace test commands were cancelled during compilation
to avoid competing with the owner's live audio test; they did not pass or fail.
The later shell validation ran with one below-normal-priority build worker and
did not open the owner's audio route. No CodeQL alert disposition was changed.
Next: review the final diff, commit this isolated fix, and obtain a new-head CI
scan. The existing raw local log files persist until rotation or user removal;
export from the fixed build strips the legacy top-level field.
