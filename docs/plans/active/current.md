# Active plan — v0.0.5 draft, final first-run check

Updated 2026-10-02. All tonight's source changes and version tag are pushed.
Verified installer and GitHub prerelease draft are prepared. Not published yet.

## Objective, requirements and decisions

Finish the requested 0.0.5 experimental release. Includes built-in Siege-round
Duck, connected-Mixer Duck bypass repair, 1,800 tool combinations, Advanced EQ
input spectrum, Duck slider/icon/widget and desktop Quit. Requirements DSP-19,
GRAPH-08/14/15, AUTO-15, UI-05/11/12/13, ARCH-04, SEC-01/10, DIST-01–08.
No VST implementation, user data, API token or device-format changes.

User still uses AudioRouter and requested preparation first. Leave shell PID
61752 and Siege PID 49376 alone. No second desktop shell. Final closure request
is pending after concrete assets are ready; Siege may stay open for first-run.

## Where things stand

Source/tag: 6e8148fcfcb6b4b85c47d3e007d8149b17ca2ae0 / v0.0.5.
[Release evidence](evidence/2026-10-02-release-0.0.5.md) records commands,
failures, checksums, source identity, build and downloaded-asset verification.

| Gate | Result |
| --- | --- |
| Locked workspace and shell | Pass; native/attended opt-in fixtures ignored; shell 44/1 ignored |
| UI/build/contracts/example | 453 UI tests pass after initial load-related timeout; build/typecheck/contracts and 11 example tests pass |
| Production Edge / visuals | 40 pass; three-theme EQ/Duck/Quit screenshots inspected |
| Package | Unsigned installer/shell/CLI/worker/examples/UI/SBOM/notices/hashes verified locally and after draft download |
| Exact packaged first run | Pending normal app closure; earlier version passes do not qualify 0.0.5 |
| Native / full M08 | Real-match Duck, native Quit, combined continuity/path-change/spatial listening, installer/hardware/accessibility/signing remain open and disclosed |

Installer: target/releases/v0.0.5/AudioRouter_0.0.5_x64-setup.exe.
GitHub draft is tagged v0.0.5 and explicitly prerelease. Previous v0.0.4 remains
published. Runtime game snapshots and private configurations are excluded.

## Ordered remaining tasks

1. Once user quits AudioRouter normally, confirm no instance; run fresh_install_shell
   on this exact packaged shell, new database/default pipe/no developer grant.
2. Record result; if passing, publish the already-authorized experimental release
   and verify published state/downloaded hashes. If failing, repair and rebuild.
3. Commit/push final evidence and archive this preparation plan. Keep open M08
   gates in a maintenance plan; do not represent this as completed M08.

## Risks and rollback

Undocumented Stats.cc protocol may change; its feed setup remains explicit.
Unknown feed state releases Duck. Independent-clock drift/queue latency and
unqualified native scenarios remain limits. Keep v0.0.4 and compatible backups;
no migration or driver install. Never overwrite published assets.

Exact next action: await normal AudioRouter closure for exact-executable first-run.
Prior execution history: [superseded preparation](../archived/2026-10-02-release-0.0.5-preparation.md).
