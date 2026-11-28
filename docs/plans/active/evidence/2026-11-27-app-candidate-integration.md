# App-only candidate integration and qualification, 2026-11-27

## Scope and environment

Owner authorized integrating the approved app work and preparing the next
complete unsigned app-only candidate. No driver, PR merge, tag, publication,
installer installation or endurance soak. The owner's app, database and WebView
profile were not used.

- Windows 11 Home build 26300, WebView2 158.0.4370.45, Node 22.22.3, npm 10.9.8,
  pinned Rust/Cargo 1.96.0.
- Branch `t3/integrate-next-app-candidate`; artifact source revision
  `2323072e3c8145c4fa3e7c00221425cf2a40baf8` (clean tree at build time). This
  includes the merged #201, #203 and #205 work, the aa95a6394 spacing fix and
  current `origin/main`.
- Qualified shell SHA-256
  `0f319c482028034d2a9c15c068deda8fcf7f1c251e0f766c8aced55b7dbcf43e`
  (28,065,792 bytes), file version 0.0.33.

## Audit and integration

The prior candidate `candidate/0.0.33` (`29bbb6b74`) was inspected first and
left unchanged. Its six executables were copied to
`target/candidate-0.0.33/bytes/` before any build, with the recorded hashes:

- shell `3B6B86CD6CDEFE6803637D2D51A03C351B3D5308ECAF44192E77603BAEBCBDF0`
- CLI `3037E5D01BD78310681FCC7E787253DEFF6742C201B772D0C04D678DDA20C599`
- x64 plugin worker `120500B8D052AF48E976D0BD7DE9EEAC972DF6CDF288E6593BAC250D4C98F9A9`
- x64 VST3 worker `D081966E42EBB2AF7CF2E8006A2C53164A5A7CB5BC764FF5D7B5134FF1CA7B46`
- x86 plugin worker `5CC713FB66F697F6678013EB3C328BD6E1B0CE254D9150DFE1DA9C9932332536`
- x86 VST3 worker `E2B15392BCC875703956A7B6C92279EB3FA86C432A7568DABB67EAEDC0CFA75C`

The prior candidate already contained the recovery tree equivalent to #199
(`d912424c2`), A1/A2 recording opt-in and `sessions.rename`, the frontend
spacing/error port and the site device wording. An ancestry and diff audit found
unmerged work in `origin/feat/multichannel-output`, so the branch was merged
twice as it advanced:

1. `53afc3ce1`: A4 output widening, its spec text, version bump/release notes,
   and two harness repairs (`ca455e6db`, `c27f6cfd1`).
2. `e7783dfae`: A4 also covering the single-endpoint and application workers
   (`205892499`), A3 read-only bus lists as default MCP tools (`62551f1b2`), the
   A4 live-continuity plan results (`0145e83f2`) and the A3 release note
   (`6b8728299`).

`origin/docs/site-device-wording`, `origin/feat/mcp-rename-and-recording` and
`origin/improve/ui-accessibility-app-qualification` were ancestors of that
branch, so no duplicate merge was needed. After #201 and #203 merged, the branch
was updated with current `origin/main` (`d296f7bed`), which also brought #205,
`8d7ba0d8d`/`9a8a15254` evidence and the `aa95a6394` recording-permission spacing
fix. The final integration commit is
`2323072e3c8145c4fa3e7c00221425cf2a40baf8`, and PR #207 now targets `main`.
After #206 merged, the branch was updated again; #206 changes documentation,
specification and evidence text only, so the candidate bytes were not rebuilt.

The A4 plan text was corrected to the code actually merged: float32 devices with
2–32 channels in every worker, stereo on channels 0 and 1, every other channel
silent. The multichannel thread's live 7.1 CABLE-B continuity results (clean
multi-path and three clean single-endpoint runs; one unattributed endpoint
failure that did not recur) are merged in #206 and recorded in
[the multichannel evidence](2026-11-27-multichannel-output.md). This thread did
not rerun them.

The first ad-hoc installer command used the base `src-tauri/tauri.conf.json`,
which has bundling disabled and no resources. It produced a shell-only package
and was discarded before qualification. The final package was built through
`tools/release/prepare-artifacts.ps1 -AppOnly`, which uses
`tauri.app-only.release.conf.json` and stages the CLI, both x64 workers, both
x86 workers, the tunnel runtime, the Stream Deck package and the sound library.

## Verification

Commands ran in the integration worktree; logs are under
`target/integrate-0.0.33/`.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` and the `src-tauri` check | clean |
| workspace Clippy `--all-targets --all-features -D warnings` | clean |
| `src-tauri` Clippy, same flags | clean |
| `cargo test --workspace --locked --no-fail-fast` | 64 targets, 1,683 passed, 0 failed, 77 ignored |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 186 passed, 0 failed, 4 ignored |
| contract drift | 157 methods, 38 node kinds, 20 processors, 24 event categories |
| documentation validation | 334 Markdown files, 1,960 local links |
| UI lint / Prettier / i18n / typecheck | clean; 3 catalogs, 3,807 messages |
| `npm --prefix ui test` | 137 files, 1,210 passed, 0 failed (after the aa95 merge) |
| `ui/e2e/clients-recording-opt-in.pw.ts` on compiled Edge fixtures | 3/3 themes passed, including the new 8 px minimum-gap assertion |
| site CSP check and site tests | current; 4/4 |
| site catalog and guide checks | 133 methods/138 operations/34 kinds; 39 guides, 151 anchors, 1,350 links |
| UI hook tests / quality policy tests | 4/4; 78/78 |
| `verify.mjs --checks release-tests` | passed on the earlier merged tree; no changed input in the last merge |
| Stream Deck typecheck, tests, validation and pack | 35/35, package created |
| `prepare-artifacts.ps1 -AppOnly` | complete unsigned app-only artifacts, 0.0.33 |
| `verify-artifacts.ps1` on the manifest | passed |

One full workspace run stopped at `a_client_waits_for_a_free_instance_for_as_long_as_its_timeout`
(`busy_pipe_wait`) with "timed out waiting for a free named-pipe instance". The
same test passed 3/3 in isolation immediately afterward, and the complete
`--no-fail-fast` rerun above passed all 64 targets. No product change was made
for that timing flake; it is reported rather than hidden.

Direct-entry regressions that cover the integrated behavior passed:
`sessions_rename_saves_a_revision_through_dispatch_and_respects_roles`,
`rename_session_tool_renames_the_open_session_and_undo_restores_it`,
`client_recording_opt_in_is_off_by_default_and_round_trips`,
`recording_opt_in_is_per_client_off_by_default_and_cleared_by_revocation`,
`real_mcp_pipe_honors_the_recording_opt_in_and_its_live_removal`,
`an_observer_reads_the_virtual_bus_lists_through_default_mcp_tools`,
`an_output_to_a_7_1_device_plays_on_its_front_left_and_right`,
`an_output_to_a_mono_or_non_float_device_is_refused_with_the_play_rule`,
`stereo_plays_on_the_first_two_channels_of_any_wider_float_device`,
`widening_accepts_an_equal_width_and_refuses_partial_or_oversized_frames`,
`only_ieee_float32_mix_formats_may_be_widened` and
`scheduler_bridge_keeps_a_stereo_graph_for_a_multichannel_render_device`.

The fresh embedded UI (`ui/dist/index.html`, 06:34:25 UTC) predates the qualified
shell (06:35:14 UTC), and this build includes the aa95a6394 spacing fix. The
current Vite chunk-size and third-party `use no memo` and
Stream Deck Zod/Rollup warnings are the same retained warnings documented in the
frontend qualification; none is a failed check.

## Final artifacts

Unsigned, unpublished local app-only package:

`target/candidate-0.0.33-main/artifacts/AudioRouter_0.0.33_x64-setup.exe`,
24,562,846 bytes, SHA-256
`405ae6bf6f941d5e1c92318b1afd44d4e1cc427e12b863976dab3ec2141a7583`.
The manifest records `sourceRevision 2323072e3...`, `signed: false`,
`publicationReady: false`, and notes the pending install/upgrade/uninstall,
clean-machine and hardware blocks.

Companion binaries used by the package:

- `audiorouter-shell.exe` `0f319c482028034d2a9c15c068deda8fcf7f1c251e0f766c8aced55b7dbcf43e`
- `audiorouter-cli.exe` `852a5377523d42318f80618dd2db324bcafe534812d9b240338a6c76c50abbc8`
- `audiorouter-plugin-worker.exe` `3fe2743986c1efda6475c0458dddef8db9f002a53701b7ed51fea1c5be05bc38`
- `audiorouter-vst3-worker.exe` `a713be8af2c4a68cf32024bb3844e4e11927f0063a96a37d3593d9af82b4aa4d`
- `audiorouter-plugin-worker-x86.exe` `813e44fc3ed3b4bfc784a551fb37c36b350c9fff3ead2ab303cdd35b71605cc`
- `audiorouter-vst3-worker-x86.exe` `d602cc0ada02cffb45c029969c064aeca2c461148863efbc8683d03525451b81`

The Stream Deck package is `09313272f8b46f5b3613f84ba0f3d5e528a5fb55807b69fa590d431ecfe5a6a2`
(93,317 bytes). `SHA256SUMS.txt`, Cargo/npm SBOMs, the pinned tunnel runtime
notices and the release manifest accompany the installer.

## App qualification

- `fresh_install_shell` on the qualified shell: passed with a new disposable
  database and WebView profile, default pipe and no `AUDIOROUTER_*` variables.
  Backend answered in 90 ms; raw callers remained denied.
- Packaged-shell qualifier on the same shell: **11/11 cases passed** — baseline,
  missing binary, occupied pipe, shell exit, bridge, readiness, page error,
  cleanup fallback and the three cancellation cases.
- Final baseline: clean revision `2323072e3`, shell hash above, non-admin Medium
  integrity, device access denied, cold UI ready 1,453 ms, warm restart 1,955 ms,
  no uncaught UI errors, both owned shells exited 0, graceful cleanup.
  These are single samples, not OS-cold or p95 measurements. The embedded UI
  (06:34:25 UTC) predates the shell (06:35:14 UTC), and the rebuilt panel includes
  the aa95a6394 spacing fix.

## Human and environment gates

Not run and not claimed: attended live audio/device (F1–F6), Windows 150/200%
scaling, Narrator, native tray/dialog languages, CABLE-A continuity, attended
7.1 Play, the application worker live, a hardware check that channels 3–8 stay
silent, Defender/MOTW/trust prompt, clean machine, second Windows build,
reference-PC rollback, signing (E6) and the deferred eight-hour soak. The 7.1
continuity gate was run by the multichannel thread and merged in #206; this
thread did not rerun it. The new installer was not installed; its VM lifecycle
is a separate attended/VM gate. No public release, tag, draft or PR merge exists
from this thread.

## Next action

The owner may test the local app-only package. The frontend thread owns attended
accessibility; attended 7.1 Play and the application worker live path remain open
from #206, and publication/signing/PR merge approval remain separate decisions.