# Q5.1 first-pass, retry and duration metrics in the nightly trend

Date: 2026-11-24. Requirements: ENG-02/03/04, NFR-04/05/06/08/11/14. Scope:
the nightly trend table only; no product behavior, endpoint, driver, installer
or hardware change.

## Why

A retried green test was reported as a pass. The Edge config already sets
`retries: 1` and `failOnFlakyTests: true` in CI, so a flaky test fails the job,
but the trend table had no first-pass or retry column and the Playwright report
was not retained. A run whose first attempt failed and whose retry passed could
therefore be invisible in the durable history, and `stats.unexpected` alone
reports zero for it.

## What changed

- `ui/playwright.ci.config.ts`: when `PLAYWRIGHT_JSON_OUTPUT_NAME` is set, the
  CI config adds Playwright's `json` reporter beside `list`. The development
  config, retry count, `failOnFlakyTests` and every assertion are unchanged.
- `.github/workflows/ci.yml`: each browser shard creates a report directory,
  writes `audiorouter-playwright-json/report-<shard>.json`, and uploads it as
  `playwright-report-<shard>` even when the shard fails. Traces remain
  failure-only.
- `.github/workflows/quality.yml`: the nightly now builds the same browser
  fixtures once (`browser-prepare`), runs the same four Edge shards
  (`browser`), and summarizes their reports (`browser-trend`, which writes
  `metrics/browser.env` even when a shard failed). The trend job downloads
  `metrics-*`, appends the new columns, and links the run.
- `tools/quality/test-trend.mjs`: reads Playwright JSON reports and counts
  `total`, first-pass failures (an attempt 0 with status
  failed/timedOut/interrupted), retried tests, flaky tests (passed only after a
  retry), final failures, skips and the longest shard's duration. A test marked
  `test.fail()` (`expectedStatus: "failed"`) and a skip are not first-pass
  failures.
- `tools/quality/summary.mjs` and `quality-history-chart.mjs`: the history gains
  `browser_total`, `browser_first_pass_failures`, `browser_retried_tests`,
  `browser_flaky`, `browser_failed`, `browser_skipped`, `browser_duration_s`
  and `run_url`. A cached or published CSV with the old 10-column header is
  migrated in place with blanks instead of being rejected or dropping the new
  values; an unknown column is refused.
- Tests: `tools/quality/test-trend.test.mjs` (13), `tools/quality/summary.test.mjs`
  (3), `quality-history-chart.test.mjs` (8; older-header and round-trip cases)
  and new `policy.test.mjs` assertions pinning the workflow wiring.

The trend's `commit` remains the short source SHA of the measured run;
`run_url` resolves the exact run and its shard reports.

## Validation against retained passing, retry-passing and failing runs

A real Playwright 1.65.0 run on Edge (Windows, Node 22.22.3, `CI=true`,
`retries: 1`) used three retained probe tests: one pass, one that fails only on
attempt 0, and one that fails both attempts. Its JSON report is the shape the
parser reads. The tool reported:

| Metric | Value |
| --- | ---: |
| Report files | 1 |
| Tests | 3 |
| Passed first try | 1 |
| First-pass failures (green retry included) | 2 |
| Retried tests | 2 |
| Flaky (passed only after a retry) | 1 |
| Failed after every attempt | 1 |
| Skipped | 0 |

This is the required distinction: the flaky test is in `first_pass_failures`
while `browser_failed` counts only the test that never passed. The probe files
were temporary (Playwright's own output and config under `ui/.q5probe`, since
deleted); their real report shape is retained as compact fixtures under
`tools/quality/testdata/playwright/`: `passing.json`, `retry-passing.json`,
`failing.json`, `expected-failure.json` (an expected failure plus a skip) and
`nested.json`. The fixtures keep the fields the parser reads, not the full
report, so they stay readable and small.

The retained GitHub runs I could inspect had no Playwright JSON artifact (the
reporter was list-only), so hosted passing/retry/failing reports do not exist
to replay. The parser is therefore qualified on the real local report and the
retained fixtures, not on an old hosted artifact. The first nightly after this
change is the first hosted evidence.

## Review fix: all four shard reports are required

A partial download (for example 3 of 4) would silently report fewer tests and
fewer first-pass failures. `test-trend.mjs` now accepts `--expect-shards`; the
nightly passes `--expect-shards 1,2,3,4`. Every expected `report-<n>.json` must
be present and parseable with a `suites` array and `stats` object. A missing,
truncated or structurally empty report is treated as missing: the tool lists
the missing shard (and any unreadable file), writes **no** `browser.env`, and
exits 0 so the trend row stays blank instead of under-reporting. The trend
workflow no longer special-cases "zero files".

Regression tests (4 new, 13 in `test-trend.test.mjs`): 3 of 4 present (no env,
names shard 4), all 4 present (total 9), truncated JSON for shard 2 (names
shard 2 as unreadable), and a structurally empty `{}` report (missing). Proof
the guard is load-bearing: without `--expect-shards` the same 3-file input is
accepted and writes `browser_total=7`; the partial-set test asserts no env file
and fails without the guard.

## Checks run (this worktree, Windows 11, Node 22.22.3)

| Command | Result |
| --- | --- |
| `node --test tools/quality/test-trend.test.mjs tools/quality/summary.test.mjs tools/quality/quality-history-chart.test.mjs` | 24 tests passed |
| `node tools/quality/verify.mjs --checks policy` | passed (107 tests, 6.1 s) |
| `node tools/quality/test-trend.mjs playwright <real probe report>` | counts above |
| `node tools/quality/summary.mjs history <legacy 10-column CSV> ...new columns` | old row migrated with blanks, new row complete, table printed |
| `npm --prefix ui run typecheck:all` | passed |
| `npm --prefix ui run lint` | passed (0 warnings) |
| `npm --prefix ui run format:check` | passed |
| `actionlint 1.7.12` over all workflows | exit 0 |
| `node tools/docs/validate.mjs` | passed (see plan entry for the count) |
| `git diff --check` | passed |

## Not run

- The hosted nightly and CI workflows (this branch is not pushed, and the rules
  forbid triggering a workflow). The first scheduled run after merge is the
  first hosted evidence for the new jobs; the PowerShell summary step was
  exercised locally only through the same `node` command it invokes.
- The full Edge suite through `playwright.ci.config.ts` with the JSON env: that
  needs the built `e2e_backend` and `target/browser-acceptance-ui`, several
  minutes, and it does not change the one-line reporter override. The focused
  probe run used the same JSON reporter shape.
- Rust and UI unit-test durations. The trend records the Edge suite's longest
  shard only; job-level durations are visible in each Actions run. The reports
  say so in the appended note.
- Open regression age and escaped-defect counts. There is no tracked dataset
  for either (the repository had 0 open issues at 2026-11-24), so they are not
  columns. Escaped defects stay in the plan narrative (for example the keybinding
  and pipe-guard defects in Q5.3); the trend says they are not measured rather
  than reporting zero.
- Timings on a hosted runner.

## Cost and rollback

The nightly quality workflow now runs the full Edge suite a second time (CI's
`schedule` already does). Each shard took 700 s in run 45496604253, so the four
parallel shards add about 12 minutes to the nightly. If the owner prefers one
nightly Edge run, the `ci.yml` schedule can be dropped later; that is a
scope/cost decision, not a change made here.

Rollback: remove the two `browser`/`browser-prepare`/`browser-trend` jobs, the
new trend columns and note, the JSON reporter override, the report directory and
upload steps, `tools/quality/test-trend.mjs` and its test/fixtures, and the
migration in `summary.mjs`/`quality-history-chart.mjs`. Old published rows stay
readable because the parser accepts the 10-column header.
