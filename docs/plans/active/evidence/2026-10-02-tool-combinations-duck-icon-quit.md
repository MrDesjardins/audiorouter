# Tool combination suite, Duck icon and window Quit

Date: 2026-10-02. Windows 11 x64, MSVC, Node 22, Edge Playwright harness.
User requests: a combination test suite (every tool, 4–6-tool chains,
Enabled/Bypass patterns), a duck icon for the Duck tool, and a window Quit
button that closes the UI and backend without the tray.

## Combination suite (`crates/engine/tests/tool_combinations.rs`)

- 20 built-in chain tools (including Duck in level and Siege-round modes).
- 60 deterministic chains: lengths 4, 5 and 6, each a stride-3 rotation from
  every start tool, so each tool is in at least 6 chains and at 4 or more
  positions including first (asserted by
  `every_tool_appears_in_many_chains_and_positions`).
- 5 layouts: single path, fan-out to two outputs, one Mixer with a second
  source, a chain split around two connected Mixers (the user's topology), and
  two independent paths.
- 6 flag patterns: all active, all bypassed, all off, alternating bypass,
  alternating off, mixed (one bypassed, one off).
- 1 800 combinations through `compile_native_paths_with_plugins_and_audio`
  (the desktop multi-path compiler). Each must compile, process finite audio,
  keep the active route's sources/outputs and be accepted by
  `RealtimeMixerFanout::replace_paths` (the live toggle path). All bypassed and
  all off must equal the dry source sum to 1e-5. A sampled set recompiled twice
  must produce identical audio.
- Mutation check: removing today's Duck bypass fix makes the suite fail at
  `TimeShift → Compressor → Delay → Duck`, all bypassed, `UnsupportedPath`.
  Restored afterwards.
- Supporting API: read-only `CompiledPathSet::paths()`.
- Runtime: 19.9 s in a debug build.

## Duck icon and Quit

- Duck tool card: monochrome duck SVG in the icon colour (was `⤓`, shared
  with Compressor); card text mentions the Siege-round trigger.
- Window Quit: top-bar button, desktop shell only, two-step confirm (5 s),
  then the shell command `quit_app` runs `system.quit` with a fresh
  idempotency key (same finalization as tray "Quit and stop audio") and exits;
  a refusal keeps audio running and shows the reason.
- Screenshots in all three themes:
  [duck dark](screenshots/2026-10-02-duck-icon-quit/duck-icon-dark.png),
  [light](screenshots/2026-10-02-duck-icon-quit/duck-icon-light.png),
  [high contrast](screenshots/2026-10-02-duck-icon-quit/duck-icon-high-contrast.png);
  [quit dark](screenshots/2026-10-02-duck-icon-quit/quit-armed-dark.png),
  [light](screenshots/2026-10-02-duck-icon-quit/quit-armed-light.png),
  [high contrast](screenshots/2026-10-02-duck-icon-quit/quit-armed-high-contrast.png).
  The first armed style (green text on pink) had poor contrast and was
  replaced with white on dark red.

## Checks (Windows)

| Check | Result |
| --- | --- |
| Engine library + integration | 157 + 33 deterministic + 3 combinations + 7 routes passed |
| Control library | 227 passed, 9 ignored |
| Shell | 44 passed, 1 ignored |
| UI `npm test` / typecheck | 46 files, 450 tests passed |
| Playwright `duck-icon`, `duck-siege-round`, `spatial-audio` | 15 passed |
| Contracts drift | passed |

Review build `target/reviews/duck-quit-combos-20261002/` (shell SHA-256
`5b3bf0ebd2ea093f775a31c91f329dec7a13d29fdb828da94b9a89e9aa366357`),
`AUDIOROUTER_BUILD_ID=review-duck-quit-combos-20261002`; UI 20:58, shell 21:00.
The real window Quit (process exit) is an attended check; the harness only
proves the button calls `quit_app` after confirmation.
