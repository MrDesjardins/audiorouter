# Managed ChatGPT MCP runtime — 2026-10-11

Scope: AUTO-16, SEC-14, UI-17, DEC-20. This is local implementation/build
evidence only. No OpenAI Platform resource was created and no real runtime key
was used.

## Implementation

- Added a Windows Tauri supervisor for the pinned OpenAI Secure MCP Tunnel
  runtime. It is off at app start, launches on Connect, checks its loopback
  `/readyz`, and kills/reaps the process on Disconnect or app shutdown.
- Tunnel ID and runtime key are serialized together and protected by current-
  user DPAPI. The UI's Forget action removes this local protected copy; it does
  not revoke the Platform key or remote tunnel.
- First Connect authorizes `audiorouter-chatgpt` as Observer (read-only) if it
  is not already enrolled. Existing Client permissions can change or revoke
  that local grant.
- The runtime key is passed only to the official runtime through its
  environment. The trusted AudioRouter MCP child clears inherited
  `CONTROL_PLANE_API_KEY` and `OPENAI_API_KEY` before it handles MCP requests.
  Logs and support bundles do not receive the credential.
- `tunnel-client`'s documented Windows parser strips backslashes in stdio
  command strings, so the local command uses forward-slash executable and
  database paths. AudioRouter passes its named-pipe name through a separate,
  non-secret inherited environment variable.
- Added the Connect setup panel, restricted-key disclosure and Platform links,
  stable status/message slots, Disconnect/Forget/Update actions, and the final
  ChatGPT connector handoff. The app reports local tunnel readiness, not proof
  that ChatGPT has created or authorized its connector.
- Release configuration bundles the runtime, LICENSE, NOTICE, license report
  and SPDX SBOM. Release staging pins v0.0.16, checks archive SHA-256, verifies
  GitHub attestation, validates archive paths, then stages the assets.

## Artifact and build results

OpenAI Windows runtime archive:

```text
tunnel-client-runtime-v0.0.16-windows-amd64.zip
SHA-256 639fdfe10d91c672d8f8a49386ebf39acaac4e560a88134735b1739a02ddb82d
```

The archive hash matched the pinned value in this workspace. A previous
verification run recorded successful `gh attestation verify --repo
openai/tunnel-client`; re-verifying during this build failed because the
restricted environment could not resolve GitHub and initialize the Sigstore
verifier. The release staging script correctly stopped. The already extracted
runtime artifact was retained from the earlier provenance-verified download.

Windows release outputs:

| File | SHA-256 |
| --- | --- |
| `src-tauri/target/release/audiorouter-shell.exe` | `1850703ea3165b9fd11175e6458ac8c0760c20d1ee83a2ae389b81771178abf5` |
| `src-tauri/target/release/audiorouter-cli.exe` | `df977c7c48ad5104cae0eedbcd0c55e68f5ec98460a9550337c96d38b04743cd` |
| `src-tauri/target/release/tunnel-client-runtime.exe` | `19e7970f825e0180546acba4c73ed1163a85f8d6fdd4393ff0854458a642d1b8` |

`ui/dist/index.html` references `index-DHAJf5oB.js`; the built shell contains
that bundle name. The final release build was:

```text
npm.cmd --prefix ui run build
cargo build --release --locked -p audiorouter-cli
cargo build --release --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml
```

## Checks

- `cargo fmt --all -- --check`: pass.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: pass.
- `cargo check --locked --manifest-path src-tauri/Cargo.toml`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --all-features -- -D warnings`: pass.
- UI formatting and typecheck: pass.
- UI lint: 0 errors, 25 existing exhaustive-deps warnings.
- UI production build: pass.
- `cargo build --release --locked -p audiorouter-cli`: pass.
- `cargo build --release --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml`: pass after final UI build.
- Documentation acceptance: pass (153 Markdown files, 866 local links).
- Rust/UI unit tests were not run. Real tunnel/ChatGPT account qualification was
  not possible without an authorized Platform tunnel and restricted runtime
  key.

## Open gates and limits

Follow-up defect report: the OpenAI setup links used `target="_blank"` in the
desktop WebView, which had no configured external-link opener. Replaced those
links with UI actions that invoke a Windows command restricted to three fixed
OpenAI destinations (Platform tunnels, Platform API keys, ChatGPT connectors).
The action now reports success/failure in the reserved status slot. The final
shell build includes this correction; UI formatting, typecheck, lint, Rust
formatting, and Tauri Clippy passed. No click-level/real-browser test was run.

Follow-up defect report: first ChatGPT authorization returned `invalid
idempotency key`. The UI's diagnostic operation label plus random nonce became
too long after the backend scoped it by the Windows caller SID and RPC method
for the 128-byte journal bound. UI mutation keys now contain only the random
nonce; method and caller scoping already provide separation. Client enrollment
is persisted before journal insertion, so this error may have granted Observer
even though the UI reported failure. Reopening this build and pressing Connect
reuses an existing grant; a not-yet-granted client uses the compact key.
UI format/typecheck/lint/build passed after this correction (lint remains at
25 existing warnings), and the release shell was rebuilt with the new bundle.

Follow-up usability defect (2026-10-11): the connector flow did not explain
that the default ChatGPT client grant is Observer, so a user asking ChatGPT to
create a session could reach a correctly connected MCP server and still be
denied by `sessions.create` (`GraphWrite`). Setup → MCP now states that
Observer is read-only, points to Client permissions to grant Editor for
session creation, and explains that the added connector must be selected in
the ChatGPT conversation's add-tools menu. This preserves explicit owner
authorization and the default read-only grant. UI format-check, typecheck,
lint (0 errors, 25 existing warnings), production build, CLI release build,
and shell release build passed. No live ChatGPT MCP tool was exposed to this
Codex task, so no session was created or claimed. Account-level write access
and connector use remain unqualified.

- Three-theme screenshot/layout review is still open. The Vite server started,
  but the available in-app browser timed out connecting to its loopback address;
  separate Edge/Chrome browser control was unavailable.
- No fake-client process/redaction/lifecycle tests or real Windows DPAPI test
  were run. Clippy compiled Rust test targets but did not execute tests.
- The signed-driver payload required by `prepare-artifacts.ps1` was not
  available. The app executable and its CLI/runtime sidecars were built, but
  no NSIS installer was produced.
- No authorized OpenAI Platform/ChatGPT workspace was available, so tunnel
  polling, workspace discovery, tool reads/writes, connector approval,
  credential revocation and shutdown were not exercised end to end.
- Disconnect is local only. The user must revoke/rotate the OpenAI runtime key
  and tunnel through Platform; AudioRouter does not claim remote revocation.

Next: finish the three-theme and native/fake-client qualification gates, then
produce the installer when the signed driver input and OpenAI account evidence
are available.
