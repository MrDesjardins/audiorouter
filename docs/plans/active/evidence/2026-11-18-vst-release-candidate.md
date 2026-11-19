# VST branch app-only release candidate (2026-11-18)

Unsigned, app-only artifacts prepared from a clean tree for the release agent.
Not a signed or published release; any later source change needs a new run.

- Source: `39e5e32ff` on `work/vst-compatibility` (clean working tree,
  checked by the script), built 2026-11-19. It replaces the candidates built
  from `8232e5ed3` (installer `4723c66d…1861`) and `d6c400c7b` (installer
  `d76278cb…814f`), which predate the plain refused-preset message and the
  sidechain output-queue fix (`14aa4af12`).
- Command: `powershell -NoProfile -ExecutionPolicy Bypass -File
  tools/release/prepare-artifacts.ps1 -OutputDirectory
  target/release-candidate-2026-11-19 -AppOnly`, with the pinned VST3 SDK
  (`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`) installed by
  `tools/m06-vst3-sdk/install.ps1`. Rust 1.96.0, `+crt-static`.
- Output: `release-manifest.json`, `SHA256SUMS.txt`, SBOMs, notices and the
  Stream Deck plugin in `target/release-candidate-2026-11-19` (not
  committed).

## Artifacts

| File | SHA-256 | Bytes |
| --- | --- | --- |
| AudioRouter_0.0.30_x64-setup.exe | `54f40f629fdd9bfedad111f4424921cd8b34164d082711cc7db8020d484b2204` | 24,232,033 |
| com.mrdesjardins.audiorouter.streamDeckPlugin | `e59d832d46f671b70d5536404564881fc2f9a349448c425a985749fe6cebeef4` | 93,317 |

## Binaries inside the installer

The release manifest hashes the installer, not the files inside it. These
are the binaries the installer packaged (`src-tauri/target/release`):

| Binary | SHA-256 | Bytes |
| --- | --- | --- |
| audiorouter-shell.exe | `fff7f31dab6c03384775d46db5335a6d237b37e8f6d5c7ca72110d875e01bfaa` | 27,458,048 |
| audiorouter-cli.exe | `4a10220cded53dffe378e97ffaeaa9a2dd9e1fba6521d0dc29b9d5ba53c09371` | 18,042,880 |
| audiorouter-plugin-worker.exe (VST2 x64) | `85515addea6bab6c00e2dc150772b5f58f980245892c55349bdb2db10a53f6e7` | 1,409,024 |
| audiorouter-plugin-worker-x86.exe (VST2 x86) | `f9b5f0f29e7ee31af04db070ac93761664d795c417579b5565665ed1495e4477` | 1,179,136 |
| audiorouter-vst3-worker.exe (VST3 x64) | `43de3df1320c10b064ac8b6b431be267b1b88b45d214e47f45ef6a76309f75e1` | 503,808 |
| audiorouter-vst3-worker-x86.exe (VST3 x86) | `aeb84426a33844e9010ca6c699d87270854186ec84568646b79bcebc5bdc0841` | 417,792 |

## Checks

- The shell (23:30:01) is newer than `ui/dist/index.html` (23:27:55); the
  build ran the UI build first.
- The shell contains the plain refused-preset message ("; the plugin keeps
  its previous settings") from `d6c400c7b`; the source includes the output-queue fix from
  `14aa4af12`.
- `fresh_install_shell` against the built shell, with no other AudioRouter
  running and no developer variables: passed; the backend answered
  `status.get` 61 ms after launch and a raw caller was denied
  `DeviceAdministration`.
- Not run: installing the NSIS package (the 2026-11-15 smoke covered 0.0.29),
  signing, the main CI workflow (it runs on the pull request), and the owner
  checks in [vst-owner-checks.md](../vst-owner-checks.md).
