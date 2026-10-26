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
