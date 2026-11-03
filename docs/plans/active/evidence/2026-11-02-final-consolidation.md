# Final consolidation and local build — 2026-11-02

Owner authorized all tonight's work into main, final review, remaining fixes,
plan cleanup, a PR and a local build. Branch: `feature/ux-improvements`.
Source snapshot `e6b9e3ba420065e84c996be023cea3764a446c18` contains setup-check,
all five UX children, final review repairs, browser qualification and earlier
review head `b8cc974f`. The 62-worktree audit and additional unattached-branch
audit are recorded separately. Historical unmatched worktrees were preserved.

## Final checks

Windows 11 Home 10.0.26300, Rust 1.96.0 MSVC, Node 22.22.3.

- Parent UI full suite: 818 tests / 92 files passed, 2-worker ceiling and no
  file parallelism. Log: `target/ux-combined-final-tests.log`.
- Parent compiled-asset Edge: 24 scenarios passed with one worker, three themes,
  1280x720 and 640x360, including forward Tab exiting Properties. Final clean
  exit 0: `target/ux-final-combined-browser-clean.log`. Removed inherited
  NO_COLOR for that process to avoid PowerShell treating its conflict warning
  as a native error. An earlier run also passed assertions but exited 1 for
  that warning; it is not represented as the clean run.
- Parent format/check, zero-warning ESLint, typecheck/all and UI build passed.
  Parent contract drift: 138 methods/37 kinds/20 processors/22 event categories.
  Documentation acceptance passed (235 files/1,357 links before this record).
- Integration worktree full Rust: 1,275 workspace tests and 86 shell tests,
  source/tool combinations, plugin fixtures, both strict Clippy scopes, format,
  guide catalog, release fixtures and MCP parity passed. See
  [integration evidence](../ux-earlier-review-integration.md) for ignored gates.
- Parent rebuilt release CLI, Rust plugin worker, native VST3 worker and
  custom-protocol shell after building production UI. VST3 AGain acceptance
  passed: single-stream, auxiliary bus, async graph staging, restart/quarantine,
  state restoration, repeated-quanta timing and transformed output. No device
  stream or machine audio configuration was changed.
- Parent packaged smoke passed: `target/ux-consolidated-packaged-smoke/result.json`.
  Medium integrity (8192), non-admin; cold usable UI 1,264 ms, warm 717 ms,
  save 86 ms; two graceful quits. Native bridge remained device-access denied
  before/after restart. This is disposable-profile startup/save/restart evidence,
  not live audio, latency, recorder finalization or owner-session evidence.

## Local artifact

Run `src-tauri/target/release/audiorouter-shell.exe` from this checkout after
closing any other AudioRouter instance. Matching CLI and both plugin workers
are beside it. Do not separate the executable from its companions and sound
library. This is a local test build, not a published release or installer.

SHA-256:

The original pre-CI-repair build hashes are retained below. The current local
build was refreshed for `inout 0.2.5` at source `c4161cae` and passed the same
disposable packaged smoke (`target/ux-inout025-packaged-smoke/result.json`).
Current hashes: shell `92DBD1FB13F7FBCF00177F8F75B34C7035C19490736246932BB72D24FA13B636`,
CLI `2626986E9E63632999D8766D18C82E3188E9EECDF8B5904C05F1412FDDA23169`,
Rust plugin worker `B73EECC958A177832B01E486E936E2C562E1417C78789A9CAF467AD436359B89`.
Native VST3 worker is unchanged. All matching companions are staged beside
the executable. PR CI remains required before main integration.

```text
audiorouter-shell.exe A511BB7081A0EBA7D9B8DE857246FBFD75223F97A8F56996C9CF12BDF0D81B71
audiorouter-cli.exe 32D3CAFF8A7BE00031ED0DBF2AF2B83944C2A942C974E06114D91CA4FFFA7E17
audiorouter-plugin-worker.exe B3E3586D94FC6F3387C5F3214CF059B80D2202FB7EA71D67EBC3797ECC445969
audiorouter-vst3-worker.exe 4737877A323222548C7E751DA00841B0EA026F501544A3CE7EE70F93A3AA754B
```

Shell timestamp 21:51:31 local follows `ui/dist/index.html` at 21:49:25.
CLI source/staged hashes match. Existing owner data was not used for smoke.
Tomorrow: back up the existing session before testing; check Status, open a
finding, cancel a session switch, then deliberately play the intended route.
Paired network protocol v3 requires updating both peers together. Keep the
previous compatible build/configuration backup for rollback; do not assume
older builds accept newly saved settings or newer network packets.

## Remaining limits

Narrator, actual Windows 200% scaling, five fresh participants, native export
round-trip, recorder failure/finalization, owner audio/plugin/device recovery,
long-run endurance and full browser suite remain open. Small viewport
notifications can cover content until dismissed. Vite's 522.60 kB main-chunk
advisory remains; no budget or lint threshold was raised. Four imported raw
continuity logs have trailing blank lines; retained unchanged as historical
evidence, while source/document edits pass whitespace checks.

These are explicit M05/M08 qualification limits, not a release-ready claim.
Next: required PR CI/annotations, merge authorized consolidation when clean,
then owner tests this build tomorrow. No driver install or publication.
