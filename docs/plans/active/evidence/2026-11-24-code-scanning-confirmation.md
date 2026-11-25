# Code-scanning confirmation — 2026-11-24

Read-only verification for [code-scanning remediation](../code-scanning-remediation.md).
No alert was dismissed, reopened or edited; no code changed for this record.

## Open alerts

```
gh api 'repos/MrDesjardins/audiorouter/code-scanning/alerts?state=open&per_page=100'
```

Returned **0 open alerts** on 2026-11-25T02:25Z (the query ran after midnight UTC).

`state=all` returned 39 alerts total across the alert history:

| State | Count | Rules |
| --- | ---: | --- |
| open | 0 | – |
| fixed | 2 | `cpp/memset-may-be-deleted` (#1), `js/bad-code-sanitization` (#3) |
| dismissed | 37 | 25 `rust/cleartext-logging`, 11 `rust/hard-coded-cryptographic-value`, 1 `rust/weak-sensitive-data-hashing`, 1 `js/incomplete-multi-character-sanitization` |
| **total** | **39** | |

This matches the plan's initial 38-alert baseline plus alert #27 (dismissed
first): the open count went 38 → 37 → 35 (after #74/#79) → 0. Nothing is open
now.

## Dispositions (from the alert API, not inferred)

| Alerts | Rule | Dismissed reason | Comment |
| --- | --- | --- | --- |
| #2 | `js/incomplete-multi-character-sanitization` (`tools/docs/validate.mjs:18`) | mitigated | "Fixed" |
| #4–#25 | `rust/cleartext-logging` (persistence, plugins, domain, cli, recording tests, M03 example) | mitigated | "Fixed" |
| #26 | `rust/weak-sensitive-data-hashing` (`crates/control/src/obs.rs:700`) | mitigated | "Fixed" |
| #27 | `rust/hard-coded-cryptographic-value` (`crates/control/src/obs_tests.rs:11`) | used in tests | "Test-only OBS fixture password … compiled only under `cfg(test)`; not a production credential or runtime value." |
| #28–#38 | `rust/hard-coded-cryptographic-value` (`obs_tests.rs`, protocol and windows-audio `#[cfg(test)]` network modules) | mitigated | "Fixed" |
| #39 | `rust/cleartext-logging` (`crates/control/examples/m03_cable_session.rs:309`) | false positive | "The value is an opaque local session ID (hardcoded `vm-cable-route` in this developer-only VM example), not a credential. Flagged because helpers are named session_id." |

#1 (`cpp/memset-may-be-deleted`) and #3 (`js/bad-code-sanitization`) are `fixed`:
their code changes were merged, so GitHub marks them fixed without a dismissal.
The earlier review record is
[2026-10-26-code-scanning-remediation.md](2026-10-26-code-scanning-remediation.md);
it also records the corrected shell-diagnostic exposure (raw user-chosen
session IDs in `shell.jsonl`) that was not one of the reported CodeQL flows.

## Latest CodeQL analyses

```
gh api 'repos/MrDesjardins/audiorouter/code-scanning/analyses?ref=refs/heads/main&per_page=20'
```

At 2026-11-25T02:25Z the default branch was `8e35e3ca` (#188, merged
2026-11-25T02:19Z). Complete analyses for that commit:

| Language | Commit | Results in SARIF | Rules |
| --- | --- | ---: | ---: |
| actions | `8e35e3ca` | 0 | 98 |
| c-cpp | `8e35e3ca` | 0 | 120 |
| csharp | `8e35e3ca` | 0 | 120 |
| javascript-typescript | `8e35e3ca` | 1 | 200 |
| python | `8e35e3ca` | 0 | 121 |
| rust | `8e35e3ca` | in progress at the query time | – |

The rust analysis for `8e35e3ca` was still running; the last complete rust
analysis at `53e1de80` reported 13 results, all of which correspond to
dismissed alerts (a SARIF result is not an open alert). The
`javascript-typescript` result at `8e35e3ca` is the one dismissed
`js/incomplete-multi-character-sanitization` alert.

## Check annotations on main

The latest **completed** main run at record time was push run
[45496604253](https://github.com/MrDesjardins/audiorouter/actions/runs/45496604253)
on `53e1de80`: 23 check runs, all success or intentionally skipped, and every
check's `output.annotations_count` was 0. That includes the six CodeQL `Analyze`
checks.

The current main push run
[45532180917](https://github.com/MrDesjardins/audiorouter/actions/runs/45532180917)
on `8e35e3ca` was still in progress. Its completed checks had 0 annotations; the
browser shards, Windows checks and the rust CodeQL analysis were not complete,
so this record does not claim that run passed.

## What is not claimed

- Not a new CodeQL run: the analyses are the ones GitHub already produced.
- Not a guarantee that a future diff cannot introduce a new finding; only that
  the current default-branch alert set is empty.
- The dismissed "cleartext-logging" alerts cover API/database resource identity
  and test/example output, not just logs. The source review and the corrected
  shell log redaction are in the earlier evidence file; do not re-dismiss a new
  finding because these are closed.

Rollback: delete this evidence file; no state changed.
