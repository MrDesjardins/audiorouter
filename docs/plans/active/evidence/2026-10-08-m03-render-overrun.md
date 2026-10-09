# M03 isolated tone render overrun — 2026-10-08

## Evidence

- Guest archive: `C:\VMs\ar-share\evidence-20261008-212725.zip`
- SHA-256: `3486CB5300BB013C45DAEB5856C19DEDFC89BBF53EB0456C46953D22744511D6`
- Guest run: `20261008-212630-tone`, driver `0.1.0.0`, protocol `1.1`.
- Requested run length: 600 seconds. The clean-run tone check terminated after
  about 29 seconds when it detected a render sequence gap; this is not a
  10-minute qualification run.

## Result

The render-source driver counter reported `overrun_frames: 480`; the harness
also detected one skipped sequence. The maximum render worker poll gap was
11,186 μs, against a 10 ms period at 48 kHz / 480 frames. Capture counters
remained zero and the maximum capture poll gap was 10,985 μs. Maximum
heartbeat, progress-output and lease-close durations were 284, 536 and 24 μs;
WAV append and finish took 631 and 712 μs. These measurements make a long
control or disk operation an unlikely explanation for this particular missed
block. They do not prove the exact scheduler event that caused it.

The render read count did not advance across several progress intervals,
indicating no newly published render blocks during those intervals; it later
resumed. The run still ended on a single sequence gap and remains a failure
under the unchanged zero-error criterion. No driver counter or acceptance
threshold was relaxed.

## Harness follow-up

The audio workers previously used `THREAD_PRIORITY_HIGHEST` without MMCSS.
They now request Windows `Pro Audio` MMCSS and retain the thread-local guard
until each worker exits. `THREAD_PRIORITY_HIGHEST` remains only as an explicit
fallback when MMCSS is unavailable. Startup output identifies the worker,
COM, MMCSS, timer and fallback capabilities. A render-gap error now includes
the previous, expected and observed sequences, skipped count, elapsed time,
and current/max poll gap. Acceptance counters and thresholds are unchanged.

Host validation on 2026-10-08: `cargo fmt --all` and the `src-tauri` format
check passed; the focused tone example had 13/13 tests pass; the existing
Windows service-thread setup/release test passed; workspace and `src-tauri`
Clippy passed with `-D warnings`. These checks do not establish guest timing.

The clean-source build was committed and pushed as `1c94edfd` on `main`. The
release executable launched its `--help` path as expected (exit 64) without
opening the bridge device. The tool-only bundle is
`C:\VMs\ar-share\diagnostics-20261008-mmcss-render`; executable SHA-256 is
`7E422B60B7C6DA52840716BCEA218867B00EA0ACE194C967B5C3ED560BD5CE89`. It
contains the tone executable and VM retry/check scripts, not a driver. The
bundle verifies the installed driver against SHA-256
`692C013CF9727985FE804F4020388108D1A568ECF8FC0D99B9B525771E39D646`.

Next: in the VM run
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File Z:\diagnostics-20261008-mmcss-render\retry-isolated-tone.ps1`.
Keep Cable B Output Listen enabled and stimulate Cable A Input as directed by
the script. Review its collected 30-second archive before retrying the
10-minute test. A clean 10-minute VM run remains required; MMCSS support is
not itself proof of continuity.

## MMCSS 30-second retry — teardown race found

- Guest archive: `C:\VMs\ar-share\evidence-20261008-213523.zip`
- SHA-256: `DE46106097E7E101F42DD723DEE070453836A4488A9A6B59B44A154C779ADB04`
- Both workers reported `MMCSS_Pro_Audio=true`, with no fallback.
- Render and capture driver error counters and harness sequence gaps were all
  zero. Maximum render/capture poll gaps were 6,220/5,880 μs.
- The tool still exited with `render mapping: SampleSizeMismatch` at shutdown.

Code review found a race in the exit condition. Lease deactivation clears the
driver mapping header before the main thread sets the worker stop flag. The
render worker therefore could observe the intentional header clear while
`stop=false` and report it as a runtime error. The capture writer had the same
window. Both workers now accept `SampleSizeMismatch` only after a separate
retirement signal is set before lease deactivation. All other mapping errors
remain fatal. The focused tone suite passes 13/13; workspace and `src-tauri`
Clippy pass with `-D warnings`; both formatting checks pass. Guest retry is
still pending. Do not begin the 10-minute gate until the short check passes.

The retirement-race fix was committed and pushed as `8007f466` on `main`. A
fresh static-CRT tone executable passed its non-device `--help` launch. The
tool-only retry bundle is staged at
`C:\VMs\ar-share\diagnostics-20261008-retirement-fix`; executable SHA-256 is
`0E73F90326633723665209D6971D051E36FD1FF9E516EE8EC4C9A477EAAD6EF8`. It
verifies the existing guest driver hash
`692C013CF9727985FE804F4020388108D1A568ECF8FC0D99B9B525771E39D646` and
contains no driver files.

Next, in Administrator PowerShell in the VM, run
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File Z:\diagnostics-20261008-retirement-fix\retry-isolated-tone.ps1`.
That script performs the 30-second check, collects evidence, and copies the
archive to the shared folder. Review it before the 10-minute run.

## 2026-10-08 retry with no Cable A Input playback

- Guest archive: `C:\VMs\ar-share\evidence-20261008-213942.zip`
- SHA-256: `6ACA1F67B51DE1399EDA38B22A7D6F3426138F06F950359B6E8911963D392D8B`
- Both workers reported `MMCSS_Pro_Audio=true`; driver counters and harness
  sequence gaps were zero. Render poll gap max was 6,750 μs.
- No Cable A Input audio was played, so `render-source blocks read: 0`; the
  tool correctly failed with `no render audio recorded`.

This is not driver continuity evidence because the test stimulus was absent.
The retry script now pauses for readiness, gives a five-second countdown, and
states explicitly that Cable A Input's Test button must be clicked during the
30-second run. The runbook also clarifies that playback starts before the
command finishes. The updated tool-only bundle is staged at
`C:\VMs\ar-share\diagnostics-20261008-tone-ready`; the exact guest command is
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File Z:\diagnostics-20261008-tone-ready\retry-isolated-tone.ps1`.
Press Enter when ready, then click Cable A Input Test as soon as the tone run
starts. Review the short result before the 10-minute gate.

## 2026-10-08 clean 30-second retry

- Guest archive: `C:\VMs\ar-share\evidence-20261008-214459.zip`
- SHA-256: `8B8A609736851222A7CFA11729EC6D1B990A1C8092B937B034171A5EB9165A2A`
- The tone step passed all four checks. Capture and render driver error
  counters were zero; the harness reported zero sequence gaps.
- Capture wrote 3,003 blocks. Render recorded 506 blocks (242,880 frames,
  about 5.06 seconds) during the 30-second run. Maximum capture/render pump
  gaps were 5,928/5,948 μs; maximum heartbeat, progress output and lease
  close were 1,112/267/37 μs.

This clears the short-run gate, but the Cable A Input test stimulus was brief.
For the 10-minute run, route a known audio file to Cable A Input and keep it
playing continuously for the full 600 seconds. Do not use a single short Test
sample as the long-run stimulus. In the VM run
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\ar\vm-checks.ps1 -Step tone -ToneSeconds 600`,
then collect the evidence with `powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\ar\vm-checks.ps1 -Step collect`.
Keep Cable B Output Listen enabled as before. Review the collected trace before
the stall or 8-channel checks.
