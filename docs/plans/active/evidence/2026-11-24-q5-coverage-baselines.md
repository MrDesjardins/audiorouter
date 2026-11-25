# Q5.2 nightly coverage baselines before any floor

Date: 2026-11-24. Requirements: ENG-02/03/04, NFR-04/05/06/08/11/14. Scope:
measurement only. No floor, threshold, gate, workflow or product behavior was
changed.

## Collection

Read-only GitHub artifacts, not a re-run and not local coverage:

```
gh run list --workflow quality.yml --limit 8 --json databaseId,conclusion,headSha,createdAt,event
gh run download <run> --repo MrDesjardins/audiorouter --name coverage --dir <dir>
```

Each `coverage` artifact holds `_temp/coverage/rust-workspace.json`,
`rust-shell.json` (cargo-llvm-cov `--summary-only`) and
`ui/.../coverage/coverage-summary.json` (Vitest v8). Per-crate totals use the
same crate grouping as `tools/quality/summary.mjs` (`crates/<name>/…`), so the
numbers here match the nightly step summary.

## Runs used

| Run | Commit | Date (UTC) | Trigger | Conclusion |
| --- | --- | --- | --- | --- |
| [45406044566](https://github.com/MrDesjardins/audiorouter/actions/runs/45406044566) | `0fb41911` | 2026-11-24 | schedule | success |
| [44896903266](https://github.com/MrDesjardins/audiorouter/actions/runs/44896903266) | `a4e92bb0` | 2026-11-21 | schedule | success |
| [44729124895](https://github.com/MrDesjardins/audiorouter/actions/runs/44729124895) | `a4e92bb0` | 2026-11-20 | schedule | success |
| [44516582590](https://github.com/MrDesjardins/audiorouter/actions/runs/44516582590) | `33cdf58d` | 2026-11-19 | schedule | success |
| [44296751803](https://github.com/MrDesjardins/audiorouter/actions/runs/44296751803) | `33cdf58d` | 2026-11-18 | schedule | success |

Five nightly rows cover three distinct source commits. Two pairs are the same
commit on consecutive nights, so they show runner/jitter repetition, not source
variation. The failed nightlies [45175932055](https://github.com/MrDesjardins/audiorouter/actions/runs/45175932055)
(2026-11-23, failed in the keybinding test) and
[45017043920](https://github.com/MrDesjardins/audiorouter/actions/runs/45017043920)
(2026-11-22, coverage job failed on `LogFilesPanel`) were **not** used: the
latter has a `coverage` artifact but no `metrics-coverage` summary, so it is not
a complete measurement. A failed run is not a baseline row.

## Baseline (range over the five runs)

| Module | Lines % | Functions % | Lines counted | Lowest run |
| --- | ---: | ---: | ---: | --- |
| control | 69.1–70.0 | 58.4–59.5 | 30 311–30 948 | 44896903266 |
| cli | 87.1–87.8 | 61.6–63.9 | 6 165–6 590 | 44896903266 |
| windows-audio | 71.7 (flat) | 67.8 (flat) | 13 373 | all |
| driver-helper | 67.6 (flat) | 66.3 (flat) | 1 745 | all |
| transport | 90.4 (flat) | 84.5–85.1 | 2 279–2 388 | all |
| domain (portable) | 92.2 (flat) | 96.4–96.5 | 4 080–4 118 | all |
| dsp (portable) | 95.6 (flat) | 94.4 (flat) | 5 978 | all |
| engine (portable) | 94.3–94.4 | 93.4 (flat) | 16 512–16 544 | 44516582590 |
| plugin-host (portable) | 62.3–63.8 | 59.6–60.8 | 11 573–11 889 | 45406044566 |
| protocol (portable) | 96.0–96.1 | 94.4–94.5 | 1 072–1 094 | 44896903266 |
| recording (portable) | 90.8 (flat) | 88.3 (flat) | 3 665 | all |
| storage (portable) | 91.7 (flat) | 90.9 (flat) | 6 889 | all |
| desktop shell | 58.3–59.4 | not reported | ~9 300 (varies) | 44896903266 |
| UI lines (Vitest) | 72.62–73.57 | 69.19–69.88 | – | 44296751803 |
| UI statements | 70.98–71.85 | – | – | 44296751803 |
| UI branches | 67.92–68.40 | – | – | 44296751803 |

The lowest-coverage modules are the desktop shell (58–59 %), `plugin-host`
(62–64 %), `driver-helper` (67.6 %), `control` (69–70 %), `windows-audio`
(71.7 %) and the UI branches (68 %). They are the first candidates for a
ratchet, not current gates.

## Explicitly no floor yet

Five nightly rows over three commits, with two duplicated commits, is too little
data to set a floor. In particular:

- The metric moves with source volume (for example `control` grew from 30 311
  to 30 948 lines while coverage moved 0.9 points), so a floor today would
  encode the 2026-11-24 test set, not a stable property.
- A floor set at the minimum would let a real drop of 0.1–1.3 points through
  on the noisiest modules; a floor at the maximum would fail the next legitimate
  change.
- Line coverage is not sequence or negative-path evidence. The Q2, Q4.1
  (contract matrix), Q4.2 (29 fault-interleaving tests plus 5 000-seed soak) and
  Q4.3 (bounded parser/compiler fuzzing) slices provide the branch/invariant
  evidence for changed critical modules; the owner should require that evidence
  for a change to `control`, `engine`, `storage`, `recording`,
  `windows-audio` or the shell regardless of the line percentage.

Recommended next measurement: keep collecting at least 14 nightly rows with at
least 7 distinct commits and record the per-commit variance, then propose
per-module floors for the stable flat modules (`dsp`, `domain`, `storage`,
`recording`, `protocol`, `windows-audio`, `driver-helper`) first. That is a
future decision, not authorized here.

## What this is not

- Not a local coverage run. Local Windows, Linux or hosted results differ, and
  this file reports the hosted nightly artifacts only.
- Not branch coverage for Rust (cargo-llvm-cov's summary is line, function and
  region; the JSON has region counts) and not the UI branch percentages for each
  file. Per-file and per-region data exist in the artifacts and HTML report for
  a targeted review.
- Not evidence that a critical module's failure paths are tested. See the Q2 and
  Q4 evidence above.

Rollback: delete this evidence file; no code, workflow or threshold changed.
