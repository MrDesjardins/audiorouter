---
name: AudioRouter — live visual sound
format: 1920x1080
colors:
  canvas: "#10131a"
  surface: "#141923"
  ink: "#e9edf5"
  muted: "#9ba8bd"
  cyan: "#25b8df"
  violet: "#a37cff"
  green: "#53c79a"
typography:
  display: { fontFamily: "Inter", weight: 900, px: 104, lineHeight: 0.96 }
  body: { fontFamily: "Inter", weight: 500, px: 30, lineHeight: 1.3 }
  label: { fontFamily: "Inter", weight: 700, px: 24, tracking: "0.08em" }
spacing:
  safe: 64px
  gap: 28px
components:
  footage: { radius: 20px, border: "2px solid #25b8df" }
  chip: { radius: 999px, foreground: "#10131a", background: "#25b8df" }
---

Brand from ui/src/styles.css: canvas, surface, ink, muted, cyan token, violet
input-port and green output-port accents; Inter is the product font.
Use system sans fallback when a local Inter file is unavailable, no network fonts.
Product footage fills most of the frame. Oversized headlines cue an outcome,
then move aside for readable inspectors. One dominant accent per shot.
Stable lower 17% reserved for concise voiceover captions.
No invented interface, performance number, glow wallpaper or decorative UI.
Static sketch sheet shows hierarchy only; no animation or encoded video.
