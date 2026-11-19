# VST branch app-only release candidate (2026-11-18)

Unsigned, app-only artifacts prepared from a clean tree for the release agent.
Not a signed or published release; any later source change needs a new run.

- Source: `d6c400c7b` on `work/vst-compatibility` (clean working tree,
  checked by the script). This replaces the first candidate built from
  `8232e5ed3` (installer `4723c66d…1861`), which predates the plain refused
  preset message.
- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File
  tools/release/prepare-artifacts.ps1 -OutputDirectory
  target/release-candidate-2026-11-18b -AppOnly`, with the pinned VST3 SDK
  (`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`) installed by
  `tools/m06-vst3-sdk/install.ps1`. Rust 1.96.0, `+crt-static`.
- Output: `release-manifest.json`, `SHA256SUMS.txt`, SBOMs, notices and the
  Stream Deck plugin in `target/release-candidate-2026-11-18b` (not
  committed).

## Artifacts

| File | SHA-256 | Bytes |
| --- | --- | --- |
| AudioRouter_0.0.30_x64-setup.exe | `d76278cb838f824a53d899c1091ea79571fef9748224dd3d218169a7ef68814f` | 24,233,424 |
| com.mrdesjardins.audiorouter.streamDeckPlugin | `7781721c73ffce166afcfd3f5b0d903a2d324434e41a74c2b22b9857933aa4ae` | 93,317 |

## Binaries inside the installer

The release manifest hashes the installer, not the files inside it. These
are the binaries the installer packaged (`src-tauri/target/release`):

| Binary | SHA-256 | Bytes |
| --- | --- | --- |
| audiorouter-shell.exe | `4e264e01c0b74fd762dd3a2bfee1cfaf26e36c96389d281cfd67544ffa4c2eb0` | 27,458,048 |
| audiorouter-cli.exe | `46d1d7d82af73a09c8cbd10c0ec974516b334a4e74f1a4a74fa9cab1f95356d9` | 18,042,368 |
| audiorouter-plugin-worker.exe (VST2 x64) | `44fe4d106b58e65af1e64ba52721120cae8f2157629433be462137d564059319` | 1,409,024 |
| audiorouter-plugin-worker-x86.exe (VST2 x86) | `95c4a78a3e9d44206a4140280f2cea66599dc0021e2977bbd19132c7588c6b0a` | 1,179,136 |
| audiorouter-vst3-worker.exe (VST3 x64) | `a1ccd146fb389ab88a02e1639296b1e28bec266962b944eb3ff9f8e0779ed6cb` | 503,808 |
| audiorouter-vst3-worker-x86.exe (VST3 x86) | `58e53b4ee9eb3d1376569ee47a5317a721d6dc7e9dbcc4f09e30a7b50888ad6c` | 417,792 |

## Checks

- The shell (20:12:57) is newer than `ui/dist/index.html` (20:10:42); the
  build ran the UI build first.
- The shell contains the plain refused-preset message ("; the plugin keeps
  its previous settings") from `d6c400c7b`.
- `fresh_install_shell` against the built shell, with no other AudioRouter
  running and no developer variables: passed; the backend answered
  `status.get` 62 ms after launch and a raw caller was denied
  `DeviceAdministration`.
- Not run: installing the NSIS package (the 2026-11-15 smoke covered 0.0.29),
  signing, hosted CI, and the owner checks in
  [vst-owner-checks.md](../vst-owner-checks.md).
