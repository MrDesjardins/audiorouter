# Q6 release and rollback scenario to evidence map

Date: 2026-11-24. Requirements: ENG-05 and the M08 release conditions in
[spec 14](../../../spec/14-quality.md) and
[release qualification](../../../operations/release-qualification.md). This is
a mapping of existing evidence, not a new install, hardware or release result.
The last published candidate is **v0.0.31** (`a4e92bb0`); v0.0.32 is not
published (see the gap table).

## Map

| # | Scenario (Q6 item) | Evidence | Status | Gap |
| --- | --- | --- | --- | --- |
| 1 | Candidate built from a clean pinned checkout, SHA/profiles/versions/sidecars/notices/hashes recorded | [v0.0.31 RQ-01](2026-11-19-release-qualification-v0.0.31.md): 15 published assets downloaded, installer SHA-256 `b94e5276…3a92` matches the manifest, `verify-artifacts.ps1` exit 0, manifest version/tag/x64/Rust 1.96.0/`includesDriver:false`/`signed:false`/`publicationReady:false`; [release qualification](../../../operations/release-qualification.md#current-verified-artifacts) records the preparation/verification commands | Evidence exists for v0.0.31 | v0.0.32 is unreleased; it needs its own clean build and manifest before any publication |
| 1 | The exact artifacts are tested, not a rebuilt exe | [v0.0.31 RQ-04](2026-11-19-release-qualification-v0.0.31.md): candidate and previous hashes verified before copy; the same bytes ran install/repair/rollback/upgrade/uninstall/reinstall; RQ-05 interactive run used files extracted from that install | Passed for v0.0.31 | The v0.0.31 published bytes do not include the two #181 fixes made after `a4e92bb0` |
| 2 | Standard-user install | RQ-04 (fresh guest `AR-ReleaseQualification-Fresh-20261121`, standard user `arqualfresh`, `-WebViewBaseline Present -ProductionRecording`), four passing runs; [installer runtime evidence](2026-10-28-installer-runtime.md) for the earlier 0.0.24 candidate | Passed for v0.0.31 (one Windows build, guest Defender exclusion deviation) | A second supported Windows build and a run without the Defender deviation |
| 2 | Upgrade | RQ-04: rollback to v0.0.29 then upgrade to v0.0.31, session and 440 Hz recording retained | Passed for v0.0.31 | v0.0.32 upgrade path not run |
| 2 | Compatible database migration / rollback | RQ-04: the installed CLI read a real session database and the generated PCM WAV hash was retained after every transition; [database recovery guide](../../../operations/database-recovery.md) documents backup/restore | Partial: data retention across the version pair is proven; no distinct schema-change migration/rollback test | A schema-version migration and a restore from a v0.0.31 database into the next candidate are still needed |
| 2 | Missing WebView2 | [RQ-05 runtime-absent result](../delegate-release-qualification.md) (search "RQ-05 (partly met"): with NAT disconnected, setup exited 2 and left no registration/files/database/shell/runtime; after reconnect the same installer bootstrapped WebView2 157.0.4341.60 and passed the lifecycle with the recording intact; aggregate deliberately `failed` (`unattributed-installer-failure`) | Partial / attribution open | No bootstrap-specific diagnostic; an attended interactive absent-runtime install has not been watched |
| 2 | Uninstall retention | RQ-04: uninstall and reinstall retained the session and recording; [RQ-05 interactive uninstall](2026-11-19-release-qualification-v0.0.31.md) left user data when "Delete application data" stayed unticked | Passed for v0.0.31 | v0.0.32 uninstall not run |
| 2 | Clean machine | The fresh disposable guest had no AudioRouter installed and a standard user; [installer runtime evidence](2026-10-28-installer-runtime.md) records why a test-configured VM is not clean-machine evidence | Partial: fresh disposable guest, one build | A clean physical machine or second supported Windows build; the Defender exclusion means installers were not scanned |
| 3 | All applicable PR/full-suite checks at the candidate SHA | `a4e92bb0` had 81 check runs: 70 success, 9 skipped, 2 failure. CI push [44680624134](https://github.com/MrDesjardins/audiorouter/actions/runs/44680624134) passed. The failures were both manual `build-draft`: [44681158510](https://github.com/MrDesjardins/audiorouter/actions/runs/44681158510) failed `bounded_backend_command_interoperates_with_authenticated_pipe_client` ("timed out waiting for a free named-pipe instance"), and [44681552819](https://github.com/MrDesjardins/audiorouter/actions/runs/44681552819) failed `GlobalKeybindsPanel.test.tsx:242` | **Not all green at the candidate SHA** | The two defects were fixed after publication in #181; a new candidate must be green at its own SHA before release |
| 3 | Scoped fault/recovery evidence | Q4.1 contract matrix, Q4.2 29 fault-interleaving tests plus the seeded scheduler, Q4.3 bounded parser/compiler fuzzing, all merged in #187 and summarized in [the plan](../application-quality.md) | Evidence exists in-tree | The live hardware fault cases (Q4.7) remain open |
| 3 | Outstanding hardware and accessibility gates | [current plan](../current.md), [release qualification known blockers](../../../operations/release-qualification.md#known-release-blockers) | Open | Tracked as human gates (below) |
| 4 | Compatible previous package kept | v0.0.29 installer SHA-256 `f2c43d11…09d8` verified and used for rollback in RQ-04; v0.0.30 installer `34143713…` is recorded as the immediate previous version | Partial | The v0.0.30 local copy was quarantined by Defender (`Trojan:Win32/Suschil!rfn`); the published digest matches but the local bytes could not be rehashed |
| 4 | Tested database backup/restore path | [database recovery guide](../../../operations/database-recovery.md); storage backup/corruption tests; RQ-04 session retention | Partial | No recorded end-to-end restore of a v0.0.31 database plus recordings into a previous compatible package on a reference machine |
| 4 | Rollback restores a compatible package/configuration pair | RQ-04 rollback to v0.0.29 passed with session and recording retained | Passed for v0.0.31 → v0.0.29 | Not run for v0.0.30 (quarantined) or v0.0.32 |
| 4 | Known issues, migration and recovery instructions published | [release notes](../../../operations/release-notes.md) and [release qualification](../../../operations/release-qualification.md) document unsigned experimental status, blockers and recovery; the v0.0.31 manifest is `publicationReady:false` with three blockers | Documentation exists | Confirm the v0.0.32 release text carries the current known issues (unsigned prompt, Defender false positive, no signed driver) before publishing |
| 5 | Opt-in preview and structured owner feedback | v0.0.31 was published as an unsigned pre-release; owner feedback exists in release evidence and the #181 defects came from owner reports | Partial | No recorded structured preview cohort/feedback loop; release is still one-owner |
| 6 | Major dependency/compiler upgrades get their own PR and full local verification | Dependabot PRs and the [2026-10-20 Node-engine lesson](../../../../AGENTS.md); CI classification runs UI, Stream Deck, contracts and release checks for dependency changes | Process exists | No rehearsal of a major compiler/toolchain upgrade (the toolchain stays pinned at 1.96.0); a future bump is its own change |

## The two post-publication defects

The v0.0.31 candidate SHA had real failures that were fixed afterwards on
`main` by #181 (`fix/keybind-selection-after-failed-delete` and
`fix/pipe-guard-claim-race`). Both are recorded in [the plan](../application-quality.md).
They do not retroactively fail the published bytes, which are immutable, but
they show why Q6 item 3 asks for checks at the candidate SHA: publishing before
those fixes is what a green test count at an older SHA would have hidden.

## Not covered by this map

- No live audio, device change, driver install, signing, publication, VM start
  or installer execution was performed for this map.
- The VM evidence paths above are `%TEMP%` directories from the original runs;
  the durable record is the evidence Markdown, not the raw folders.
- Driver/cable qualification is a separate track (DEC-18) and is not part of
  the app-only M08 gate.

## Gaps that must be closed by a person or hardware

The exact steps and pass criteria are in the human-gated task list (the manager
records it in `docs/plans/active/human-gated-tasks.md`). In short: qualify an
exact 0.0.32 candidate; run the lifecycle on a second Windows build without the
Defender exclusion and without a quarantined rollback fixture; watch one
interactive absent-WebView2 install; record an end-to-end database+recording
backup/restore on a reference machine; and decide the unsigned-publication and
signing route.
