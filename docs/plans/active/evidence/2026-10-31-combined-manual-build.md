# Combined review follow-ups: manual-test snapshot

Source: `385532c732150d59950887c4d8f187b83c10eb14` on
`fix/control-source-kinds`, merging Soundboard `7276ab5e` with control source
classification and both EQ guide branches. Windows, Rust 1.96.0, Node 22.22.3.
Requirements: GRAPH-12/15, UI-17/20/25, SND-12/13/14/17/19, DIST-01/06.

## Artifact

Local folder: `C:\code\audiorouter\target\local-test-combined-20261031`.
Run `audiorouter-shell.exe` from that folder after closing the existing app.
It includes matching CLI/plugin worker, unchanged native VST3/tunnel runtimes,
the newly packed Stream Deck plugin and the bundled sound library.
`AudioRouter_0.0.25_x64-setup.exe` is also retained there; it was not installed.
Version remains 0.0.25; this is an unsigned local snapshot, not a release.

SHA-256:

- Shell: `a2ff3fccf7d5ed722cd4c4737df1acb419278577faddcaf8dcac567e1a6c92bb`.
- Installer: `2e7995cb039a60bc734fb0d0d2dd219300d0f00918d4d66507b7c72ae1ee6859`.
- Every staged file is recorded in the folder's `checksums.json`.

Build commands: `npm.cmd --prefix ui run build`,
`cargo build -j 4 --release --locked -p audiorouter-cli -p audiorouter-plugin-host`,
`cargo build -j 4 --manifest-path src-tauri/Cargo.toml --release --locked --features custom-protocol`,
`npm.cmd --prefix tools/streamdeck run pack`, and
`ui/node_modules/.bin/tauri.cmd bundle --no-sign --no-binary-patching --ci --bundles nsis --config src-tauri/tauri.app-only.release.conf.json`.
No portability claim: these binaries used the existing local runtime settings.
A separate owner-requested static-runtime build is tracked in the active plan.

## Verification

Combined checks passed: control 303 tests (11 guarded ignored), engine library
178 (1 ignored), source combinations 5, processor combinations 4, deterministic
audio 33, and the other engine suites; shell 85 (4 ignored); workspace and shell
Clippy with all targets/features and warnings denied; both Rust format checks;
UI 773 tests/84 files, production build/typecheck, lint and format check;
Stream Deck 35 tests and pack/build/typecheck; shared policy/docs/site checks;
family/guard tests 16; guide coverage 251 properties, zero pending; media budgets
150 clips; installer resource/hook check. Logs: `target/combined-*.log`.

Used the installed NVIDIA-bundled 7-Zip to list and extract the actual NSIS
payload without executing it. All 105 FLACs matched their library SHA-256s;
the manifest and notices were present. Extracted shell/CLI/plugin worker hashes
matched the built inputs. This tests package contents, not installation.

Ran that extracted shell at Medium/non-admin integrity on a disposable database,
private backend pipe and WebView2 profile. Native `soundLibrary.list` found all
105 sounds, a category-only slot appeared, and a synthetic HTML5 file drop
uploaded a WAV through the real backend and added its canvas/Properties slot.
Inspected the screenshot. Zero observed page errors; owned test processes were
cleaned up. Retained [result](combined-manual-build/native-result.json),
[token](combined-manual-build/token.json) and
[screenshot](combined-manual-build/native-after-drop.png).

Reproduction harness: `target/combined-shell-smoke.mjs <absolute staged folder>`
through `tools/quality/standard-user-process.ps1`; final wrapper arguments are
`target/combined-smoke-args.json`. Final report:
`target/combined-shell-smoke-1793470858753/result.json`.

The first attempts used a wrong Node path and a connection count above the CLI's
500 limit; corrected the harness. Later attempts established the external CLI
operator correctly lacks DeviceAdministration, while the default control pipe
was occupied. The ordinary-backend probe therefore launched no shell. Final
private-pipe scope excludes playback. Recording/plugin inventory permission
denials in its backend log are expected for this operator grant; no upload
failure or uncaught UI error occurred. The final locator was scoped to the card
because canvas and Properties both expose the same Play label.

Tool notices: Vite's 625 kB chunk; third-party zod PURE annotations/cycle;
Node's experimental SQLite API in the disposable harness. Initial Stream Deck
packing could not fetch Elgato schemas in the restricted sandbox; the normal
network-enabled retry validated and packed successfully. No dependency change.

## Remaining manual checks

With the existing app deliberately closed, run this shell and test the owner's
saved microphone + Soundboard Mixer route: Play/Stop, source meter and connection
animation; Audio File and Test Signal source animation; drop a real Explorer WAV
on a free/assigned slot; picker search/filter/preview; change/remove/mode; Stream
Deck captions, clear-title fallback and hold/release. Keep the prior build for
rollback. No installer execution, fresh-machine test, live continuity or attended
Stream Deck qualification is claimed. No PR, tag or release was created.
