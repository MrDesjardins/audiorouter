# Contributing to AudioRouter

Thank you for helping. AudioRouter is licensed under the
[GNU General Public License v3.0 only](LICENSE), and its copyright holder also
offers commercial licenses. To keep both possible, every contribution must be
made under the terms below.

## Contribution terms

By submitting a contribution (code, documentation, assets or other material)
to this repository, you agree that:

1. The contribution is your own original work, or you have the right to
   submit it under these terms.
2. You license the contribution to everyone under `GPL-3.0-only`.
3. You also grant Patrick Desjardins, the copyright holder of AudioRouter, a
   perpetual, worldwide, non-exclusive, royalty-free, irrevocable license to
   use, modify, sublicense and distribute the contribution under any license
   terms, including commercial licenses.
4. You keep the copyright in your contribution.

If you cannot agree to these terms, please open an issue describing the change
instead of submitting it.

## Before you start

Development agents and contributors read [AGENTS.md](AGENTS.md) and the
[active plan](docs/plans/active/current.md) first. Keep changes focused, add
tests for behavior changes, and run the relevant checks before submitting.

## Formatting

CI fails on unformatted Rust code. `rust-toolchain.toml` pins the toolchain
CI uses, so your local `rustfmt` gives the same result. Before committing:

```powershell
cargo fmt --all
cargo fmt --manifest-path src-tauri/Cargo.toml
```

To have Git refuse unformatted commits, enable the checked-in hook once per
clone: `git config core.hooksPath .githooks`. Claude Code sessions format
each Rust file they edit automatically (`.claude/settings.json`).
