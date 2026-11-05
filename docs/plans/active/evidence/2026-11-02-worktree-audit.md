# Worktree audit — 2026-11-02

## Scope and method

ENG-04; M05/M08 traceability for UX-01–06, UI-27 and AUTO-18. Audit child
branch: `t3/worktree-audit-plan-cleanup`, initially clean at `bf99cf97`.
Parent owns integration, review, PR/merge and native build. This child changes
only Markdown in its assigned worktree. No other worktree, profile, device,
process, branch or remote was changed. No fetch was performed: comparisons
use cached `origin/main` `f2b1c0b108f6139e18dd8ceebf8b2d941a37e53d` and fixed
parent baseline `bf99cf975334088a4a3e0a790cc42b148792dbfb`.

Windows PowerShell audit commands: `git worktree list --porcelain`, then for
every registered path `git -C <path> status --short`, `branch --show-current`,
`rev-parse HEAD`, `log -1 --format='%cs %s'`,
`rev-list --left-right --count <base>...HEAD`, and `cherry <base> HEAD` for
both bases. Also read non-merge branch logs `bf99cf97..HEAD` for every
worktree with commits ahead of parent. Set `GIT_OPTIONAL_LOCKS=0` to avoid
optional index writes. Initial
cross-worktree reads hit Git's ownership guard; repeated successfully with
per-command `-c safe.directory=<exact-path>`, without persistent configuration
changes. Invalid first-pass results were discarded. Read branch logs and
`range-diff b04a6b4b^..b04a6b4b 7b2bd937^..7b2bd937` for the UX exception.

The final inventory has **62 worktrees**. This is a point-in-time read while
other agents may work: recheck the parent and qualification/review worktrees
before integration. Clean means no short-status entries, including untracked
files reported by Git; ignored build artifacts are outside this audit.

Ancestry and patch identity are different evidence. `cherry +` means an exact
patch was not found, not that its behavior is absent: squash merges and later
changes often differ. Old unmatched branches require a separate semantic review
if revived; do not merge them into tonight's work merely because they are ahead.

## Tonight's six source workstreams

All six are already represented in parent `bf99cf97`, which contains cached
main and is 13 commits ahead (12 non-merge patches). Integrate the combined
parent branch after its current qualification; do not cherry-pick these again.

1. Setup check: `1460f6d1`, `960f9d51`, `de96803d`, `fb38dd96` are ancestors
   of parent. Covers UI-27/17/19 and AUTO-18, including diagnostic lifecycle,
   multi-route feedback and stopped wording. Original `feature/setup-check`
   no longer has a registered worktree; the work lives in the parent history.
2. UX-01 session safety: child `b04a6b4b`, integrated as `7b2bd937`.
   `git cherry` reports a difference because the child's testing-library import
   was already supplied by UX-02. Range-diff shows that import hunk is the only
   patch difference. Parent subsequently adds cross-session Cancel coverage.
3. UX-02 setup actions: `47bd90cc` → `6ae1d8a4`, patch-equivalent in parent.
4. UX-03 saved/preview/export clarity: `2aa66f02` → `67517f83`, equivalent.
   Parent `cd25f2e5` retains Audio running/stopped plus verified Draft preview.
5. UX-04 onboarding prototype and UX-06 recovery wording/checklist:
   `39108ed9` → `c920fed7`, equivalent. Measurements and participant acceptance
   remain open; this is not completed onboarding qualification.
6. UX-05 Workbench keyboard accessibility: `326382d0` → `9831785f`, equivalent.
   The child has an untracked cache directory; leave it in place.

Planning `096a1132`, integration regression `cd25f2e5`, authorization
`b7b5601d` and main merge `bf99cf97` complete the parent's 13-commit delta.
The final request supersedes gaming-time parent restrictions, without granting
this audit child native execution or PR/push/merge authority.

## Other dated work and integration disposition

Cached main already includes the 2026-11-02 marketing record/PR #105
(`fd4d7b51`, merge `dab85acf`), Rollup #101 (`c888fbe0`), Cargo ZIP #103
(`f2a78ca1`) and rusqlite #104 (`f2b1c0b1`). Install-action #102 is `046ff9ea`
(its commit date says 2026-11-03); it is also already in cached main and parent.
The retained `dependabot-104` checkout's fix `8120def4` and update history are
historical inputs to the squashed #104 result, not an additional tonight UX
slice. Do not merge that detached checkout again solely on `cherry +`.

`code-stats` is already ancestral to main. `code-stats-data` is a separate
generated-data branch with unrelated history; never merge it into product main.
The October branches below are historical snapshots or unmatched older series,
not newly authorized integration work. Preserve all of them. Their outstanding
native/release gates remain as recorded in the active plan, not closed by this
inventory. No additional source integration is recommended from this audit.

## Protected uncommitted work

Preserve without staging, reverting, deleting or copying into parent:

- `C:/ar018`, `C:/ar019`, and `.worktrees/release-0.0.22`,
  `release-0.0.23`, `release-final-0.0.24`, `release-final-0.0.25` beneath
  `C:/code/audiorouter`: modified
  `tools/streamdeck/com.mrdesjardins.audiorouter.sdPlugin/manifest.json`.
- `.worktrees/agents-lessons`: modified `.github/workflows/quality.yml`
  (one insertion and one deletion).
- `C:/Users/miste/AppData/Local/Temp/audiorouter-pr48-ts7`: modified
  `contracts/src/index.ts` (four insertions and one deletion).
- `C:/Users/miste/.t3/worktrees/audiorouter/t3-e68085ca`: untracked
  `ui/.cache-ux-accessibility/`.

These may be user or agent work; provenance and intent were not inferred.

## Complete inventory

Notation: `M=b/a`, `P=b/a` are behind/ahead commit counts relative to cached
main and fixed parent. `CM=+/−`, `CP=+/−` count non-merge patches reported by
`git cherry`. Paths beginning `W/` expand to `C:/code/audiorouter/.worktrees/`;
`T/` to `C:/Users/miste/.t3/worktrees/audiorouter/`; `TMP/` to
`C:/Users/miste/AppData/Local/Temp/`. Branch `detached` means no branch name.
Status is clean unless explicitly marked dirty/untracked below. All rows were
read successfully on the corrected pass.

```text
Path | HEAD | branch | M | P | CM | CP | last commit date / disposition
C:/code/audiorouter | bf99cf975334 | feature/ux-improvements | 0/13 | 0/0 | 12/0 | 0/0 | 11-02 parent, qualify combined branch
C:/ar018 | 54b948f18884 | detached | 240/0 | 253/0 | 0/0 | 0/0 | 10-17 historical, dirty manifest
C:/ar019 | 8190fd3233ca | detached | 231/0 | 244/0 | 0/0 | 0/0 | 10-18 historical, dirty manifest
W/agents-lessons | 011b0bd87acb | docs/agents-lessons-split | 16/1 | 29/1 | 1/0 | 1/0 | 10-31 old unmatched docs, dirty workflow
W/apps-inventory | 998a2fb90d2d | feature/apps-inventory-thread | 130/4 | 143/4 | 2/0 | 2/0 | 10-28 old unmatched series
W/audio-file-media | 06a0f3038e7f | fix/audio-file-media | 156/1 | 169/1 | 1/0 | 1/0 | 10-24 old unmatched patch
W/card-overflow | f8bfe317f25f | fix/canvas-card-overflow | 156/1 | 169/1 | 1/0 | 1/0 | 10-24 old unmatched patch
W/code-stats | c6024031905c | feature/code-stats | 7/0 | 20/0 | 0/0 | 0/0 | 11-01 ancestor
W/code-stats-data | a9ba24054972 | code-stats-data | 4752/1 | 4765/1 | 1/0 | 1/0 | 11-01 separate generated data
W/dependabot-104 | 8120def495a7 | detached | 2/3 | 15/3 | 2/0 | 2/0 | 11-02 #104 inputs, already squashed into main
W/device-used-by | c55732db6304 | feature/device-used-by | 156/9 | 169/9 | 9/0 | 9/0 | 10-24 old unmatched series
W/discord-setup | 5a21c38ba602 | feature/discord-setup | 156/1 | 169/1 | 1/0 | 1/0 | 10-24 old unmatched patch
W/future-plans | 568480219982 | feature/soundboard | 76/0 | 89/0 | 0/0 | 0/0 | 10-29 ancestor
W/obs-discord | 47488db95ed1 | docs/obs-discord-integration | 161/22 | 174/22 | 22/0 | 22/0 | 10-24 old unmatched series
W/output-switch | f224856e9789 | feature/output-switch | 167/2 | 180/2 | 2/0 | 2/0 | 10-22 old unmatched series
W/pr74-repair | b0d6d5093ad3 | detached | 147/0 | 160/0 | 0/0 | 0/0 | 10-26 ancestor
W/pr79-repair | 726b23991646 | deps/minor-major-no-patch | 143/0 | 156/0 | 0/0 | 0/0 | 10-26 ancestor
W/pr81-repair | 068b395a4378 | main | 12/0 | 25/0 | 0/0 | 0/0 | 10-31 stale local main, ancestor
W/qualification-v0.0.23 | 117379911351 | detached | 155/0 | 168/0 | 0/0 | 0/0 | 10-24 ancestor
W/quality-q1 | 2fe905ae875b | quality/q1-verification | 162/3 | 175/3 | 3/0 | 3/0 | 10-23 old unmatched series
W/quality-recovery-install-inventory | 050a47246b2c | quality/recovery-install-inventory | 128/0 | 141/0 | 0/0 | 0/0 | 10-28 ancestor
W/release-0.0.21 | 40882907d7c9 | fix/review-followups-0.0.21 | 163/12 | 176/12 | 12/0 | 12/0 | 10-23 old unmatched series
W/release-0.0.22 | 097c0354be1c | detached | 159/0 | 172/0 | 0/0 | 0/0 | 10-24 ancestor, dirty manifest
W/release-0.0.23 | 117379911351 | detached | 155/0 | 168/0 | 0/0 | 0/0 | 10-24 ancestor, dirty manifest
W/release-0.0.24 | 44573455badf | release/0.0.24 | 135/0 | 148/0 | 0/0 | 0/0 | 10-27 ancestor
W/release-0.0.25 | a8bff560a5d0 | docs/release-0.0.25-record | 67/1 | 80/1 | 1/0 | 1/0 | 10-30 old unmatched record
W/release-final-0.0.24 | 358e03e3bf7f | detached | 130/0 | 143/0 | 0/0 | 0/0 | 10-27 ancestor, dirty manifest
W/release-final-0.0.25 | c9a6b2726654 | detached | 67/0 | 80/0 | 0/0 | 0/0 | 10-30 ancestor, dirty manifest
W/release-record-023 | a2dd3bb40823 | docs/release-0.0.23-record | 155/1 | 168/1 | 1/0 | 1/0 | 10-24 old unmatched record
W/release-records | 43d8104df7d4 | docs/release-0.0.24-0.0.25-records | 58/0 | 71/0 | 0/0 | 0/0 | 10-31 ancestor
W/session-state-row | eef71aaa1661 | fix/remove-session-state-row | 156/1 | 169/1 | 1/0 | 1/0 | 10-24 old unmatched patch
W/sidebar-video | b547b8696bb2 | fix/sidebar-video | 129/0 | 142/0 | 0/0 | 0/0 | 10-28 ancestor
W/sleep-resume | 706e3d872da7 | security/codeql-source-review | 138/0 | 151/0 | 0/0 | 0/0 | 10-26 ancestor
W/streamdeck-advisories | a3aca7b44b19 | fix/streamdeck-build-advisories | 129/0 | 142/0 | 0/0 | 0/0 | 10-28 ancestor
W/tool-guide-property-videos | 7a2b1f84679e | feature/tool-guide-property-videos | 47/0 | 60/0 | 0/0 | 0/0 | 10-31 ancestor
W/try-discord | d1e27929ca77 | feature/apps-devices-0.0.23 | 156/26 | 169/26 | 17/0 | 17/0 | 10-24 old unmatched combination
W/ui-zod-advisory | c5a643a8f888 | fix/ui-zod-advisory | 124/0 | 137/0 | 0/0 | 0/0 | 10-28 ancestor
W/vc-assurance | 7580d5bcbfbb | docs/virtual-cable-assurance | 120/0 | 133/0 | 0/0 | 0/0 | 10-28 ancestor
W/vst3-plugin-windows | 4efdb90f9be9 | feature/vst3-plugin-windows | 167/7 | 180/7 | 6/0 | 6/0 | 10-22 old unmatched series
W/wave-link-app-sources | 278bd7bf832c | feature/wave-link-app-sources | 169/5 | 182/5 | 5/0 | 5/0 | 10-22 old unmatched series
W/wave-link-sound-check | 9079af73a900 | feature/wave-link-sound-check | 168/4 | 181/4 | 4/0 | 4/0 | 10-22 old unmatched series
W/website-copy | 554d91aeb4b2 | docs/website-copy-review | 131/0 | 144/0 | 0/0 | 0/0 | 10-27 ancestor
C:/code/audiorouter/target/release-source-v0.0.3 | 8cb52dd17a0f | release/0.0.3 | 744/0 | 757/0 | 0/0 | 0/0 | 10-02 ancestor
C:/code/audiorouter-marketing | 253fd345bfb6 | marketing/vibecoding-post | 60/0 | 73/0 | 0/0 | 0/0 | 10-30 ancestor
T/docs-guide-graphic-eq-bands | c34336773a34 | docs/guide-graphic-eq-bands | 36/0 | 49/0 | 0/0 | 0/0 | 10-31 ancestor
T/docs-guide-parametric-eq-bands | 048d14441351 | docs/guide-parametric-eq-bands | 33/0 | 46/0 | 0/0 | 0/0 | 10-31 ancestor
T/feature-soundboard-followups | 7276ab5e7050 | feature/soundboard-followups | 29/0 | 42/0 | 0/0 | 0/0 | 10-31 ancestor
T/t3-05b375a2 | 2aa66f02deda | t3/clarify-saved-preview-audio-state | 3/5 | 12/1 | 5/0 | 0/1 | 11-02 integrated UX-03
T/t3-37408fd1 | bf99cf975334 | t3/worktree-audit-plan-cleanup | 0/13 | 0/0 | 12/0 | 0/0 | 11-02 this audit, initially clean
T/t3-3b193c8a | 47bd90cc5dee | t3/setup-tool-actions | 3/5 | 12/1 | 5/0 | 0/1 | 11-02 integrated UX-02
T/t3-7add24d5 | bf99cf975334 | t3/ux-final-qualification | 0/13 | 0/0 | 12/0 | 0/0 | 11-02 parent qualification worktree
T/t3-86b5d54b | bf99cf975334 | t3/review-ux-setup-qualification | 0/13 | 0/0 | 12/0 | 0/0 | 11-02 parent review worktree
T/t3-8fa93ad9 | b04a6b4bdee1 | t3/session-switch-confirmation | 3/5 | 12/1 | 5/0 | 1/0 | 11-02 integrated UX-01, range-diff reviewed
T/t3-a0070852 | 39108ed94b5e | t3/onboarding-recovery | 3/5 | 12/1 | 5/0 | 0/1 | 11-02 integrated UX-04/06
T/t3-e68085ca | 326382d0dbd3 | t3/workbench-tab-accessibility | 3/5 | 12/1 | 5/0 | 0/1 | 11-02 integrated UX-05, untracked cache
TMP/audiorouter-pr46-update | 7be9e6c0a97b | detached | 205/6 | 218/6 | 5/0 | 5/0 | 10-20 old unmatched series
TMP/audiorouter-pr48-ts7 | 983c09bdae6a | detached | 206/2 | 219/2 | 1/0 | 1/0 | 10-21 old unmatched series, dirty contracts
TMP/audiorouter-q4-graph-fault | 61c101d945a1 | quality/q4-graph-commit-fault | 192/0 | 205/0 | 0/0 | 0/0 | 10-21 ancestor
TMP/audiorouter-quality-layout | 3b79fa17cafb | quality/production-smoke | 187/0 | 200/0 | 0/0 | 0/0 | 10-21 ancestor
TMP/audiorouter-quality-skip | fd2b4c65ca50 | quality/docs-skip-evidence | 194/1 | 207/1 | 1/0 | 1/0 | 10-21 old unmatched evidence
TMP/audiorouter-ts7-main | 3138690580e9 | quality/regression-gates | 193/0 | 206/0 | 0/0 | 0/0 | 10-21 ancestor
C:/wlp | 955fa63fbd41 | feature/wave-link-parity | 170/1 | 183/1 | 0/1 | 0/1 | 10-22 patch-equivalent historical slice
```

## Qualification and next action

This reconciliation changes execution status, not requirements or evidence.
Parent must still complete combined UI/contracts/docs checks, final review and
three-theme/200% inspection, then build the UI before the embedded release shell
and stage matching CLI with hashes/timestamps. Native export round-trip,
recording finalization/audio continuity, Narrator/keyboard journeys, five fresh
participants (four within ten minutes, all identify Discord sources), and UX-06
startup/panel/recovery measurements remain unfulfilled wherever no separate
evidence exists. Existing installer/hardware/signing gates are not waived.

Read-only inventory and branch comparisons succeeded; this child did not run
product tests or native qualification. Documentation acceptance and whitespace
checks passed on Windows: `powershell -NoProfile -ExecutionPolicy Bypass -File
.\tests\acceptance\docs.ps1` (final run: 230 Markdown files, 1,206 local links) and
`git diff --check` (exit 0). The complete changed Markdown diff was inspected.
Rollback: revert this docs-only
commit; leave all audited worktrees intact. Next task: parent integrates this
documentation commit and resolves final qualification/review findings, then
performs its authorized PR/main consolidation and isolated local build.
