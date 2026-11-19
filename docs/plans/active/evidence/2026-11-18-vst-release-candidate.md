# VST branch app-only release candidate (2026-11-18)

Unsigned, app-only artifacts prepared from a clean tree for the release agent.
Not a signed or published release; any later source change needs a new run.

- Source: `8232e5ed3bf70886de2b7a9e130954f98084f5e2` on `work/vst-compatibility`
  (clean working tree, checked by the script).
- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File
  tools/release/prepare-artifacts.ps1 -OutputDirectory
  target/release-candidate-2026-11-18 -AppOnly`, after installing the pinned
  VST3 SDK (`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`) with
  `tools/m06-vst3-sdk/install.ps1`. Rust 1.96.0, `+crt-static`.
- Output: `release-manifest.json`, `SHA256SUMS.txt`, SBOMs, notices and the
  Stream Deck plugin in `target/release-candidate-2026-11-18` (not committed).

## Artifacts

| File | SHA-256 | Bytes |
| --- | --- | --- |
| AudioRouter_0.0.30_x64-setup.exe | `4723c66db65f86d425ca419966351098d833d83cfbec1bbade6ff3f5adf41861` | 24,226,486 |
| com.mrdesjardins.audiorouter.streamDeckPlugin | `79dcdc6539d6bfdfe7274222ea921b7fc2f8e78988e37c7a157775c8560430af` | 93,317 |

## Binaries inside the installer

The release manifest hashes the installer, not the files inside it. These
are the binaries the installer packaged (`src-tauri/target/release`; the
CLI and workers are byte-identical to the `target/release` builds):

| Binary | SHA-256 | Bytes |
| --- | --- | --- |
| audiorouter-shell.exe | `58776be8aa5bf0432133b64b8697d8a0a2a4f686cdb8cb25d041443961d53429` | 27,457,536 |
| audiorouter-cli.exe | `4eee2162325328c17d992d4b2c98854b29fbd2e4997b790db2ce1af72c36f015` | 18,042,368 |
| audiorouter-plugin-worker.exe (VST2 x64) | `33f3862da5e66fe825a3b1f7d972c6e206dff3d87a1d73a85443fe0e7db68406` | 1,410,560 |
| audiorouter-plugin-worker-x86.exe (VST2 x86) | `f3b4deaa7441a1ca3fa92082bcdec04655b447fdd82390e99bec632aa5ff5f54` | 1,179,136 |
| audiorouter-vst3-worker.exe (VST3 x64) | `84dbcf8c681d67f7f39cbb1605d35cf9644aec26c378c6bbdf78116467f066f9` | 503,808 |
| audiorouter-vst3-worker-x86.exe (VST3 x86) | `e88e5738bdc5e0358c5c5ae2016e4cd1524d4fcecc9ccc7d6c0352e29249b4cc` | 417,792 |

## Checks

- The shell (20:01:09) is newer than `ui/dist/index.html` (19:58:36); the
  build ran the UI build first.
- Both VST2 workers contain the `presetRejected` answer from `9b141ba73`.
- `fresh_install_shell` against the built shell, with no other AudioRouter
  running and no developer variables: passed; the backend answered
  `status.get` 65 ms after launch and a raw caller was denied
  `DeviceAdministration`.
- Not run: installing the NSIS package (the 2026-11-15 smoke covered 0.0.29),
  signing, hosted CI, live-ear checks.
