# Active plan — current state (updated 2026-11-01)

This is a short current-state document. The detailed execution history from
0.0.13 to 0.0.26 (2026-10-07 to 2026-10-31) is in the
[maintenance execution record](../archived/2026-11-01-maintenance-0.0.13-to-0.0.26.md);
older history is indexed in [archived plans](../archived/README.md).

## Active task: code-review fixes (2026-11-01, owner request)

The owner asked to take the two code reviews out of `future/` and fix all of
their open items: [code review 2026-11-01](code-review-2026-11-01.md)
and the remaining items of [code review 2026-10-07](code-review-2026-10-07.md).
Scope, decisions, per-item tasks, validation and rollback are in the
[code-review fixes plan](code-review-fixes.md). Branch
`feature/code-review-fixes-2026-11-01`, from `review/code-review-2026-11-01`.
No pull request without the owner's approval.

Code fixes and bounded concern splits are committed, including encrypted
network pairing/restart freshness (`4ec26071`), combined root/shell notices
(`d720be19`) and UI/audio module extractions (`7abc2e1b`). The branch-specific
plugin scan exhaustiveness fix is on `feature/plugin-library-ux` (`ed1e41ec`).
Windows unit, loopback, source/tool combinations, browser/theme and guide
playback checks pass as recorded in the plan. Large-media open timing and
optimized DSP microbenchmarks are recorded there too. Live cable continuity,
large-library service-gap measurements and an attended built-app run remain
open; this is not a release qualification.

## Where things stand

- Latest published release: unsigned app-only
  [v0.0.26 prerelease](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.26)
  ([record](../archived/2026-10-31-release-0.0.26.md)). The v0.0.25 GitHub
  release is still a draft that 0.0.26 superseded
  ([record](../archived/2026-10-31-release-0.0.25.md)).
- Release metadata for 0.0.26 is on `release/0.0.26`; integrating it into main
  needs an owner-approved PR.
- Delivered and archived on 2026-11-01: the
  [Soundboard](../archived/2026-11-01-soundboard.md), the
  [Output Switch](../archived/2026-11-01-output-switch.md), the
  [Wave Link parity first round](../archived/2026-11-01-wave-link-parity-first-round.md)
  and the [0.0.24 release preparation](../archived/2026-10-28-release-0.0.24.md).

Other active plans:

| Plan | State |
| --- | --- |
| [Code-review fixes](code-review-fixes.md) | Active (this task) |
| [Virtual cable (DEC-18)](virtual-cable.md) | RC3 0.2.4 passed Stage A in the VM except Stage C items; EV certificate not bought |
| [Application quality](application-quality.md) | Q1–Q3 merged; remaining stages in the plan |
| [OBS and Discord](obs-discord.md) | D1, O1, D2, O2 authorized; see the plan for status |
| [Code-scanning remediation](code-scanning-remediation.md) | CodeQL alert triage |
| [Release signing and publication](release-signing-and-publication.md) | Parked by the owner (2026-10-03) |

## Open owner gates

These need the owner, hardware, or a separate decision. They are carried
from archived plans and are not waived.

- **0.0.26 attended checks:** normal Play/Stop on the installed build, a real
  Explorer file drop onto a Soundboard, the attended Stream Deck Sound key
  (slot setting, once/loop/hold, custom and cleared titles), and an installer
  run on a clean machine (install, upgrade, uninstall, missing WebView2).
- **Soundboard:** live audio continuity with a Soundboard in a Mixer route, a
  listening review of every library sound, and a set picker (not built).
- **Output Switch:** the owner's hardware test of a real source switching
  between two endpoints (PR #61).
- **Tool guides:** exhaustive property coverage and continuous automation
  qualification
  ([details](../archived/2026-11-01-maintenance-0.0.13-to-0.0.26.md#owner-revision-after-gate-review-2026-10-29)).
- **MCP:** ChatGPT denied-write, reconnect/revocation qualification and the
  attended Cowork import
  ([details](../archived/2026-11-01-maintenance-0.0.13-to-0.0.26.md#chatgpt-mcp-integration-plan-2026-10-11-active)).
- **Scratchy Discord voices after an overnight run** (2026-10-18): record the
  diagnostics listed in the
  [archived section](../archived/2026-11-01-maintenance-0.0.13-to-0.0.26.md#scratchy-discord-voices-after-an-overnight-run-2026-10-18-owner-report-open)
  the next time the static is heard, before restarting.
- **UAD:** a sine-continuity run with a latency-changing plugin worker
  ([U4/U6 section](../archived/2026-11-01-maintenance-0.0.13-to-0.0.26.md#uad-u4-and-u6-live-plugin-latency-and-silent-licence-failures-2026-10-31-owner-request)).
- **M08:** signing (parked), clean-machine install/upgrade/uninstall, NFR-11
  24-hour and sleep/resume endurance.

## Pull request approval preference 2026-10-22

The owner requests that no agent create or open a pull request until a human
explicitly approves the exact branch and changes. Complete local validation,
inspect the diff, and report the branch, commit and results before asking for
that approval. Authorization to implement, commit, push or run CI is not PR
approval. PR #59 was eventually reopened and merged after owner testing.

## Jev paused — 2026-10-22

The owner asked to pause Jev because it is not providing enough value for the
time it adds. The workflow now supports manual dispatch only; pull-request
CI, formatter, test and Clippy checks remain unchanged. This is reversible by
restoring the `pull_request` trigger in
`.github/workflows/jev-review.yml`. No requirement or acceptance gate was
waived.

## Jev review scope

Jev may report only correctness, safety, security, accessibility, concurrency
or performance risks that the configured compiler, formatter, linters and
static analyzers cannot detect. rustfmt and the UI formatter own formatting.
The full decision record is in the
[archived section](../archived/2026-11-01-maintenance-0.0.13-to-0.0.26.md#jev-review-scope).

## Next action

Work through the [code-review fixes plan](code-review-fixes.md) in its listed
order, then report the branch, commits and checks to the owner for PR approval.
