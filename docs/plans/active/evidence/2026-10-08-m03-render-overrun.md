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
