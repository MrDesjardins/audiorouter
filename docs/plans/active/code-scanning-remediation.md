# Code-scanning alert remediation — 2026-10-26

## Objective and baseline

Reduce the open CodeQL alerts on AudioRouter's default branch by correcting
real exposure and closing only findings whose code and data flow prove they are
not vulnerabilities. Do not blanket-exclude queries or folders. Current main is
`8ecf9f91912aaf4988ca1b3b345c2cbb229adef8`, merged PR #73. The initial
GitHub snapshot had 38 open alerts (12 `rust/hard-coded-cryptographic-value`, 22
`rust/cleartext-logging`, one `rust/weak-sensitive-data-hashing`, one
`js/incomplete-multi-character-sanitization`, one `js/bad-code-sanitization`,
and one `cpp/memset-may-be-deleted`). Alert #27 is now dismissed as
`used in tests`; 37 alerts remain open. Alert ids and exact locations were
read from `gh api repos/MrDesjardins/audiorouter/code-scanning/alerts` on this
date.

## Requirement coverage

- SEC-10: keep logs limited to useful audit metadata and redact sensitive data.
- SEC-13: network pairing credentials never appear in logs or diagnostics;
  authenticated packets retain the specified HMAC-SHA256 protocol.
- SEC-15: OBS credentials remain in memory only; the documented OBS v5
  challenge response must stay interoperable.
- Quality/CI: preserve all CodeQL languages and default queries unless an
  alert is proved test-only or a rule false positive; no broad query/path
  exclusions.

## Decisions

- Do not change the OBS v5 challenge algorithm: SEC-15 and OBS interoperability
  require its challenge-response formula; it is not password storage.
- Treat hard-coded values in `#[cfg(test)]` fixtures as test-only only after
  confirming no production inclusion or reuse. Record their specific paths and
  dismiss them as “used in tests” rather than weakening the query globally.
- Review every cleartext-logging result against its actual source, sink and
  data. `EntityId::new` accepts arbitrary strings up to later boundary checks,
  so IDs can contain user-chosen data. Do not dismiss session-ID alerts merely
  because IDs are normally opaque. Determine whether each sink is a log,
  API response, in-memory event, or the local idempotency journal. Preserve
  operation replay and event-filtering contracts.
- Replace the fixed browser-test label interpolation with a static expression
  selected from the closed `guide.kind` enum. Keep the markdown slug utility's
  HTML-free lookup-only purpose and the scheduler decoder's poisoned-extension
  self-test explicit; do not add general CodeQL suppressions.

## Ordered work

1. Read each alert's source/sink evidence and inspect current code. Identify
   which logger or persisted diagnostic path actually receives session ids.
2. Fix any production log/diagnostic exposure without removing required
   resource correlation or changing API response values. Add only focused
   regression coverage for an actual behavior change.
3. Apply the small browser expression hardening and address the C++ test
   warning only if a safe, intent-preserving code change makes it clearer.
4. Dismiss verified test-only and rule-mismatch alerts individually with
   precise rationale; never dismiss a real exposure as a false positive.
5. Run the applicable format, static analysis, focused tests and CodeQL PR
   analysis. Inspect every remaining alert and the complete diff.
6. Record current-head evidence, commit and push the remediation branch. Do
   not open a PR until the owner approves the reviewed branch and changes.

## Validation matrix

| Area | Evidence required |
| --- | --- |
| Security contract | SEC-10/13/15 behavior unchanged except approved redaction |
| Rust | Pinned Windows CI format, tests and strict Clippy; no local Linux result presented as Windows evidence |
| JavaScript | Node syntax and formatter; focused fixture run if code changes |
| C++ | Windows CI compile/self-test if source changes |
| CodeQL | New-head scan for all six enabled languages; review alert count and each remaining instance |
| Privacy | Confirm no pairing key, OBS password, personal path, or raw sensitive payload enters logs/support bundles |

## Risks and rollback

Over-redaction can break event filtering, idempotent replay or troubleshooting;
under-redaction can expose user-controlled identifiers in retained logs.
Preserve SEC-10 audit usefulness and SEC-13/15 secret boundaries. Roll back
only the narrowly scoped code and documentation changes; retain alert
dismissal rationales and run evidence. No release, driver, permission, or
security-setting changes are in scope.

## Evidence and next action

The source-to-sink review and a disposition boundary are in
`evidence/2026-10-26-code-scanning-remediation.md`. GitHub automatic review
rejected one bulk dismissal attempt because it combined 22 session-ID findings
without proving every identifier non-sensitive. Do not repeat that action by
splitting it into smaller batches. Continue with source fixes and safe
verification; if a disposition decision still requires owner judgment, stop
before changing alert state and ask the owner.
