# Simplified Chinese rendered audit, rerun 2026-11-24

Requirement: L10N-01/04/07 (app-owned text comes from the catalog; no English
prose decides behaviour or leaks into the Chinese interface). Opt-in fixture
`ui/e2e/chinese-ui-audit.pw.ts`; it never opens vendor plugin code or a real
audio device.

## What ran

Windows 11, 2026-11-24. Vite development server on port 4291 (temporary config
copy, deleted afterwards), Playwright with Edge, one worker:

```text
$env:AUDIOROUTER_CHINESE_AUDIT="1"
npx playwright test --config=playwright.zoom.config.ts --workers=1 chinese-ui-audit.pw.ts
1 passed (24.5s)
```

The fixture selects Simplified Chinese through the app, builds one route with
every built-in tool kind plus fixture plug-in and instrument nodes, and captures
each tool inspector, the daily workbench tabs, the two extra Status views and
every Settings page. It records a screenshot per scroll position and every
Latin-script line of visible text and accessible attribute.

Result: 59 panels/views in `inventory.json`, 105 screenshots in the run folder
(`target/chinese-ui-audit`). `workspace.png`, `tool-gain-1.png`,
`settings-uiOptions-1.png` and `page-recording-1.png` are copied here.

## What the inventory found, and what was fixed

The previous run left tool display names written in English inside translated
Chinese sentences. Fixed on 2026-11-24:

- `PluginPanels.tsx` passed the literal `"VST plugin"`/`"Instrument"` to the
  guide link, so the Chinese accessible label read
  `打开VST plugin指南`. New keys `g3Plug.guidePlugin`/`guideInstrument`; the
  regression test in `PluginScanPanel.test.tsx` asserts `打开VST 插件指南`.
- Six Chinese messages embedded `Input Device`, `Output Device`,
  `Test Signal` or `Denoise` instead of the localized tool names:
  `devices.help`, `setupWorkbench.otherAppsHelp`,
  `deviceTroubleshooting.bindingHelp`, `firstRun.step1`,
  `settings.troubleshoot.manualHelp`, `h1.recNoiseValue`.

After the fixes the inventory has one remaining match for those names,
`speechDenoise-1`, which is the fixture node ID, not visible prose.

Remaining Latin-script lines are technical by design: units and measurements
(`dB`, `Hz`, `ms`, `%`, `48 kHz`, `kbps`), product and format names
(`Discord`, `OBS Studio`, `VST3`, `WAV`, `FLAC`, `MP3`, `Codex`, `Claude Code`,
`Stream Deck`, `VB-Cable`, `CABLE Input/Output`), paths and file examples,
fixture node IDs and route names, the language endonyms `English`/`Français`/
`Pseudo (expanded)`, fixed device names from the harness (`Preview Game (7.1)`,
`Preview Microphone`, `Preview Output`), the `Browser fixture:` harness notice,
and diagnostics log lines (`Graph checkpoint: …`, `Virtual cable status read
failed`). Those log lines and the harness fixture are not shipped user prose.

## Limits

This is capture coverage, not a clean-translation verdict and not a fluent
review. It does not cover live devices, third-party plugin windows, the packaged
shell, the tray or Narrator. The open translation questions are in
`../2026-11-24-fluent-review-queue.md`.