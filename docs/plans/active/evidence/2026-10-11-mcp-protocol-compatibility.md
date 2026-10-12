# MCP protocol compatibility repair — 2026-10-11

## Reported failure and scope

The user reported ChatGPT's custom MCP setup error for Secure MCP tunnel
`tunnel_6acc5b1bf6c8819195497c541c696b80`: “The server couldn't complete MCP
setup. Check that the endpoint supports MCP and returns valid initialization
and tool responses.” The local tunnel runtime had reported ready. The exact
account-side request trace is unavailable, so the protocol mismatch below is
the leading cause rather than a confirmed live ChatGPT diagnosis.

The 0.0.15 CLI MCP server required a legacy `initialize` handshake, rejected
2025-11-25, and did not implement `server/discover` or the stateless
2026-07-28 request envelope. The 2026-07-28 MCP specification requires
`server/discover`, per-request protocol metadata and cache hints on catalog
and resource-read results. References checked 2026-10-11:
[protocol versioning and dual-era behavior](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/versioning.mdx),
[server discovery](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/server/discover.mdx),
[modern tool response requirements](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/server/tools.mdx),
[2026-07-28 changelog](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/changelog.mdx).

## Change

The stdio adapter now supports stateless protocol 2026-07-28 and legacy
initialize clients at 2025-11-25, 2025-06-18, and 2025-03-26. Modern discovery
and results include server identity; list/read responses include bounded cache
metadata. Unsupported modern versions return `-32022` and the supported
version list. Modern requests still dispatch through the same enrolled-client
grant and backend authorization path. A regression verifies an Observer's
modern write remains denied. Legacy ping is answered after initialize;
modern ping returns method-not-found because the 2026-07-28 revision removed
that method.

Requirements updated: AUTO-06. Covered integration: AUTO-16 and SEC-14.
Changed files: `crates/cli/src/lib.rs`, `crates/cli/tests/mcp_stdio.rs`,
`docs/spec/11-automation.md`, `docs/operations/headless-runbook.md`, and
`docs/plans/active/current.md`.

## Verification

Environment: Windows x64, pinned repository Rust toolchain, working tree based
on `30e0afb6` (`Add managed ChatGPT MCP tunnel`).

- `cargo fmt --all` and `cargo fmt --manifest-path src-tauri/Cargo.toml`;
  both corresponding `-- --check` commands passed.
- `cargo test -p audiorouter-cli --test mcp_stdio mcp_stdio_client_interoperates_with_cli_process -- --exact` — passed. Covers modern discover, tools/list, tools/call, resources/list/read without initialize, unsupported version, malformed modern metadata, denied write, modern ping rejection, legacy 2025-11-25 initialize/ping/tools.
- `cargo test --release --locked -p audiorouter-cli --test mcp_stdio mcp_stdio_client_interoperates_with_cli_process -- --exact` — passed against the optimized release CLI.
- `cargo test -p audiorouter-cli --lib` — 40 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` — passed.
- `node tools/docs/validate.mjs` — passed; 153 Markdown files and 866 local links.
- `git diff --check` — passed.
- `cargo build --release --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml` — passed. Shell uses the existing `ui/dist`; no UI files changed.

The full `cargo test -p audiorouter-cli --test mcp_stdio -- --test-threads=1`
run is not green: the bounded backend pipe test again reports
`Windows("Access is denied. (0x80070005)")`, and its dependent MCP pipe proxy
test gets `null` instead of `false`. These are the same pipe-startup failures
already recorded in the active plan before this change. The modified direct
stdio MCP process test passes in both debug and release.

Release CLI: `target/release/audiorouter-cli.exe`, SHA-256
`09F78979E2EF8F875751124E50525DA02DF18BFF25E47384EC48D19DA9CEB643`.
Release shell: `src-tauri/target/release/audiorouter-shell.exe`, SHA-256
`0F4626EC42C76CDD8A8415A1BFAEBBDDA550CFEC8BB7CCB9CD651D4F0F6FFB76`.
No NSIS installer was created: `target/release/driver/audioroutervirtual.sys`
is absent, and release packaging requires the Microsoft-signed driver payload.
The rebuilt binaries do not prove ChatGPT account/workspace acceptance; retry
the custom server setup with the rebuilt installed package to close that
remaining gate.

## Rollback and next action

Rollback by reverting the adapter, test, AUTO-06, runbook and plan/evidence
changes together. Next: deliver/package the updated CLI with the signed driver
release payload, retry ChatGPT setup through the user's actual tunnel, and
record whether initialization and tool discovery succeed. Do not treat local
tunnel readiness or this process test as account-side acceptance.
