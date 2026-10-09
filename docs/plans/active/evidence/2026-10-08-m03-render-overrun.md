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

## 2026-10-08 10-minute run — capture underrun

- Guest archive: `C:\VMs\ar-share\evidence-20261008-215641.zip`
- SHA-256: `95EABDA6F2491954742A1AD3A511B711878411EB48D5ECCCB6EA906A68A561AD`
- The harness completed all 600 seconds and exited 0, but the clean-run gate
  failed. Capture-sink underrun reached 2,208 frames (46 ms at 48 kHz), first
  visible at elapsed 67.193 seconds, and remained at that value. Other driver
  error counters and harness render sequence gaps stayed zero.
- Both workers had MMCSS Pro Audio, 1 ms timer resolution, and no priority
  fallback. Maximum capture/render poll gaps were 64,984/65,410 μs. The
  control heartbeat, progress output and WAV append maxima were 648, 652 and
  11,519 μs. The maximum poll-gap timestamps are not recorded, so their exact
  overlap with the underrun cannot be proven. The similarly sized gap in both
  workers is consistent with a transient guest scheduling pause; this remains
  an inference, not a confirmed root cause.
- Render source produced 7,652 blocks (3,672,960 frames, 76.52 seconds) over
  the 600-second run, with progress showing long intervals without new render
  blocks. The Cable A Input stimulus was intermittent rather than continuous.

Do not count this as the clean 10-minute gate and do not proceed to stall or
8-channel tests. Next: repeat the 600-second run only with a known file looped
continuously to Cable A Input, keep the VM running in the foreground, and avoid
other heavy host work. If capture underruns recur with both workers scheduled
normally, investigate the VM's scheduling and the capture queue before making
any driver or acceptance-threshold change.

## 2026-10-08 600-second retry — render startup race

- Guest archive: `C:\VMs\ar-share\evidence-20261008-220557.zip`
- SHA-256: `5D525C838C2ED2F9E513C644EBB2F6182C90764D3196FF7039492C30BA71319F`
- The harness failed at startup, about 17 ms after the timer began. The render
  reader observed sequence 2 when it expected 1, reported one 480-frame
  overrun, read zero render blocks, and wrote a zero-frame WAV. The capture
  counters were zero. The render poll gap at detection was 1,466 μs and its
  maximum was 2,165 μs. This run does not qualify the 600-second gate.
- The harness activated the render and capture leases before waking its render
  worker. Windows could publish and overwrite the first slot before the reader
  began polling, even though its measured poll cadence was timely.
- Fix: set the run start and wake the render reader before activating either
  lease; wait until it has polled the empty mapping, then activate render and
  capture. A focused worker regression publishes the first block immediately
  after the empty poll and verifies it is consumed without a sequence gap.
- Host checks on 2026-10-08: focused example tests 14/14 passed; workspace
  Clippy and `src-tauri` Clippy passed with `-D warnings`; both formatting
  checks and `git diff --check` passed. The static-CRT tool's `--help` path
  exited with the expected code 64. These checks do not establish guest timing.
- A new tool-only bundle is staged at
  `C:\VMs\ar-share\diagnostics-20261008-render-startup`. It replaces only
  `m03_bridge_tone.exe` and carries the current VM check/retry scripts. The
  metadata requires the existing driver SHA-256
  `692C013CF9727985FE804F4020388108D1A568ECF8FC0D99B9B525771E39D646`;
  no driver files are included.

Next: run the short 30-second retry in the VM with Cable B Output Listen
enabled and actively play a looping sound into Cable A Input as prompted by
the script. Collect and review its archive. Retry the 600-second run only
after the short run passes; keep the VM in the foreground and loop the audio
stimulus continuously. The previous 10-minute run still has its independent
capture underrun and remains unqualified.

## 2026-10-08 short retry — post-start render gap

- Guest archive: `C:\VMs\ar-share\evidence-20261008-221304.zip`
- SHA-256: `D231DE404A80BA2BDA9A9E1BE2C234449765C412C1DAFC1C18C2706F8E1255A3`
- The new startup ordering was active (`render worker polling before lease
  activation`). At 996 ms the reader had consumed through sequence 2, then saw
  sequence 4 instead of 3. The driver reported one 480-frame overrun; the WAV
  contained two blocks. Capture counters stayed zero. The poll gap at the
  detection was 2,022 μs and the maximum was 6,249 μs. The run fails the
  unchanged zero-error gate.
- This run used the brief Windows Test sound. It does not establish whether
  the skipped block came from an audio callback publishing quanta in a burst
  or a delayed reader poll; the current trace cannot distinguish those causes.
- The render worker now immediately polls again after successfully consuming a
  block, instead of sleeping 1 ms first. Empty polls still sleep to avoid a
  busy loop. The retry script now requests a looping Media Player source on
  Cable A Input for the full 30 seconds, so the guest check exercises sustained
  render traffic rather than a brief startup sample.

Next: rerun the updated 30-second bundle with the audio file looping into
Cable A Input for the entire check. Do not start the 600-second test until the
short run passes; keep the original zero-counter criterion.

## 2026-10-08 sustained 30-second retry — pass

- Guest archive: `C:\VMs\ar-share\evidence-20261008-221819.zip`
- SHA-256: `35D39E181CED721350470CA4E4B5B9EE98948541DB4569072B1ED17CA78403F4`
- The updated retry passed all four checks. Capture and render counters and
  harness sequence gaps were zero. Both workers reported MMCSS Pro Audio,
  1 ms timer resolution, and no priority fallback.
- Capture wrote 3,003 blocks and render recorded 3,000 blocks (1,440,000
  frames, 30 seconds). Maximum capture/render poll gaps were 6,099/6,079 μs;
  heartbeat, progress output, lease close, and WAV append maxima were
  151/210/21/9,061 μs.
- This clears the short-run gate only. It does not qualify the 600-second run;
  the earlier 10-minute capture underrun remains unresolved.

Next: with Cable B Output Listen enabled, loop a known audio file to Cable A
Input continuously for the entire 600-second run. In the VM, run
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\ar\vm-checks.ps1 -Step tone -ToneSeconds 600`,
then collect with
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\ar\vm-checks.ps1 -Step collect`.
Review the new archive before any stall or 8-channel check.

## 2026-10-08 600-second retry — immediate render startup overrun

- Guest archive: `C:\VMs\ar-share\evidence-20261008-221952.zip`
- SHA-256: `BA8A096043E41D8CEB56472886403FD788BC711065C81BB18DCA2726DF5DD074`
- The archive's `tone.txt` shows this run did **not** reach 564 seconds. It
  failed 18 ms after the timer started, before the first progress report, with
  `previous=0`, `expected=1`, `observed=2`, one 480-frame render overrun, and
  zero recorded render blocks. Capture counters remained zero. The maximum
  render poll gap was 2,564 μs. The startup-poller message was present.
- This differs from the passing short retry, whose script waited for the user
  to start playback after its five-second countdown. If playback was already
  active when the 600-second command opened its render lease, the first two
  publications could have arrived before the reader acknowledged sequence 1;
  the archive does not record playback state, so this is a hypothesis, not a
  confirmed cause.

Superseded 2026-10-09: the user confirmed playback was already running. This
is valid, and pausing it is not the repair. The [source review and driver
repair](2026-10-09-m03-render-publication-review.md) address batched publication,
OPEN ordering and lease lifetime. No further VM retry until host review and
candidate preparation are complete; the zero-counter gate is unchanged.
