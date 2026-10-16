# Tool-guide animation sources

The documentation uses locally hosted, finite native Web Animations API (WAAPI) illustrations in `site/assets/guide-animation.js`. WAAPI is a HyperFrames supported runtime adapter. The public site needs no animation package, remote service, audio permission or video download. This directory does not modify the separate launch film.

## Embed contract

Load `/assets/guide-animation.js` before the page's guide initializer. Give each illustration a fixed-width host; the visual reserves a 760:340 aspect ratio, then caption and controls.

```html
<div id="signal-example"></div>
<script src="/assets/guide-animation.js"></script>
<script>
  const illustration = AudioRouterGuideAnimation.mount(document.getElementById("signal-example"), {
    type: "dynamics", title: "Compressor: quieter peaks",
    description: "Above the threshold, a 4:1 ratio reduces the increase in level. This static comparison is illustrative; attack and release are explained separately.",
    parameters: { thresholdDb: -8, ratio: 4 }
  });
  // Call from labeled native controls, never from app telemetry.
  illustration.setParameters({ ratio: 8 });
  // play(), pause(), replay(), seek(seconds), destroy() are also available.
</script>
```

Visual types: waveform (neutral input/output), gain, dynamics, gate, limiter, duck, eq (or spectrum), delay, pitch, cleanup, mixer, switch, recording, buffer, routing, plugin.

Parameters: gain/duck `gainDb`; dynamics `thresholdDb` and `ratio`; gate/limiter `thresholdDb`; EQ `gainDb`, `frequencyHz`, `q`; delay `delayMs`; pitch `semitones`. Illustrative inputs are bounded by the visual runtime; these bounds are not the backend configuration contract. Routing families use labeled path diagrams rather than a waveform. Explain every other configuration in adjacent tool-specific property text and examples. Do not describe the illustration as a real DSP preview.

## Accessibility and performance

Nothing starts automatically, including for ordinary motion preferences. Native Play/Pause, Replay and Seek controls support keyboard use; SVG title/description and visible captions expose the meaning without animation or color alone. Reduced motion starts at the informative static frame; deliberate Play remains available. Animations pause when the diagram leaves view or the page becomes hidden. Every native animation is finite (eight seconds), paused initially, uses transform/opacity with `fill: both`, and can seek without a separate render clock. It has no audio playback.

## Optional future video export

For a separate approved export, author one standalone HyperFrames root (no template) with `data-composition-id`, `data-width="760"`, `data-height="340"`, `data-duration="8"`; mount only the SVG, caption and finite WAAPI animations synchronously, omitting website controls. The WAAPI adapter seeks `document.getAnimations()` directly. Do not call Play or use animation completion callbacks to change visual content. Use the HyperFrames entry/motion-graphics/core/animation/CLI skills and the installed version's full check before preview. Final export needs FFmpeg and preview approval. No export project, rendered video or publication is included in this source-only change.

Local prerequisites observed 2026-10-14: Node 22.22.3, Edge and Chrome installed; cached HyperFrames 0.8.107, launch-film GSAP 3.14.2 present; FFmpeg/ffprobe not found on PATH.
