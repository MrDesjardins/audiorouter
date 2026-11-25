# Localization layout evidence: 200 % zoom in three themes, 2026-11-24

Requirement: L10N-07 (allow longer labels and wrapping at 200 % zoom in dark,
light and high contrast), UI-17. Branch `plans/ui-localization`, local commits
only.

## What ran

Windows 11, 2026-11-24. Vite development server on port 4291 (temporary config
copy, deleted afterwards), Playwright with Edge (`channel: "msedge"`), one
worker. A 720 × 500 viewport is the 1440 × 1000 layout at 200 % (CSS pixels
halve), and the pages were captured in dark, light and high contrast, in
English and in the development pseudolocale `en-XA`:

```text
npx playwright test --config=playwright.zoom.config.ts --workers=1 ui-language-zoom.pw.ts
Running 6 tests using 1 worker
ok 1 en     ... in dark
ok 2 en     ... in light
ok 3 en     ... in high-contrast
ok 4 en-XA  ... in dark
ok 5 en-XA  ... in light
ok 6 en-XA  ... in high-contrast
6 passed (10.0s)
```

The committed spec is `ui/e2e/ui-language-zoom.pw.ts`. It selects Appearance in
Settings, switches the theme, optionally selects `en-XA`, asserts the document
`lang`, checks that `scrollWidth` does not exceed `clientWidth` on both the
Settings page and the route workbench, and saves a full-page screenshot per
case. The pseudo cases skip in a production browser build, where `en-XA` is not
bundled.

## Files

- `appearance-en-{dark,light,high-contrast}.png` — Settings → Appearance at
  200 %.
- `route-en-{dark,light,high-contrast}.png` — the route after leaving Settings.
- `appearance-en-XA-…png`, `route-en-XA-…png` — the same with the expanded
  pseudolocale.

## Result and limits

Every case passed: no horizontal page overflow and the Appearance heading,
language select, theme select, Back to the route button and Settings tab list
stay visible. This is a development browser run. It does not cover the packaged
WebView2 shell, Windows text scaling, a real translated catalog beyond the
existing French and Simplified Chinese files, or RTL (not declared supported).
`en-XA` is a layout probe, not translation-quality evidence. Screenshots are
written by the spec to Playwright's `testInfo.outputPath` and copied here from
that run output.