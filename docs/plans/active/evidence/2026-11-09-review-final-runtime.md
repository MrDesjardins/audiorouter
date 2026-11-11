# Review fixes: final runtime qualification

Windows, 2026-11-09. Harness: `tools/quality/review-fixes-smoke.mjs`.
Artifacts: `target/review-final-1794278178307/` (local, not committed).
The saved `evidence.json` contains all assertions and no error field.

The custom-protocol debug shell used isolated database, application data,
and WebView profiles. Its embedded UI was newer than `ui/dist/index.html`.
Generated Test Signal audio passed through Gain at -12 dB to an owned
loopback UDP receiver. There were no physical audio nodes or external peers.

- Processed peak: 0.063095; held dry peak: 0.251188; released peak: 0.063053.
- Refused running-session deletion retained the held comparison.
- The 30-second comparison expiry restored processing (peak 0.063042).
- Button-row geometry stayed identical after focus and scrolling settled.
- During a 1708 ms installed-plugin scan, 641 audio packets and 106 UI timer
  ticks continued. This demonstrates servicing during the scan, not a latency bound.
- The shell's HTTP listener refused a UNC plugin scan with status 403.
- Dark, light, and high-contrast Properties screenshots were visually inspected:
  readable controls and stable reserved comparison geometry.

Backend log review found one non-OK event: the deliberately refused
`sessions.delete` request, code -32602. No other backend error was recorded.
The test-owned shell was stopped; the owner's subsequently running release
shell was not touched.

## Limits and remaining checks

The Network Send route's canvas Play tone control was disabled by its existing
physical-output classification. The harness used the header Play control and
public source transport API. This evidence does not claim that canvas control
works for this route.

Two-session comparison dispatch/publication uses a controlled publication
boundary regression. The real native run above covers one generated route;
it does not prove simultaneous physical endpoint operation.

This run exposed source transport replacement during a flag-only comparison
rebuild. The repair preserves unchanged source handles and playback state;
its native regression and expanded source combination tests passed. Final
workspace/control/shell/Clippy checks remain pending for that repair. The
subsequent full engine run was cancelled when the owner began gaming and
must not be counted as passed. No builds or runtime checks resume while the
owner is gaming.

## Lightweight follow-up review

Reviewed source retention across `audio_media.rs`, `native_workers.rs`,
`status.rs`, and the engine's runtime and Mixer/path-set graph traversal.
Retention is gated by the existing flag-only publication path; ordinary
activation still compiles fresh transports. Handles are collected by owning
session before the backend replaces its handle maps. Arc replacement occurs
on the control plane before graph publication, without adding callback work.
The two-session publication seam and its storage exist only under `cfg(test)`.

`git diff --check` passed during this review. These file reads and whitespace
checks are not substitutes for the pending final compile, tests, and Clippy.
