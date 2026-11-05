# Dependabot 7: GLib iterator backport

Requirements: ENG-02, ENG-04, ENG-05. Base: main `068b395a`.
Branch: `fix/dependabot-7-glib-iterator`. Owner requested a branch and PR.

## Cause and bounded repair

[Dependabot 7](https://github.com/MrDesjardins/audiorouter/security/dependabot/7)
reports [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).
The GLib 0.18.5 string iterator writes through an immutable reference passed
to variadic C FFI. Optimized compilation can discard that write, leaving a
null pointer for CStr::from_ptr.

`cargo tree --manifest-path src-tauri/Cargo.toml --target all -i glib --locked`
shows Tauri 2.11.6 / GTK 0.18.2 requiring GLib 0.18.5. The corresponding
Windows-target command prints no GLib consumers. Registry lookup confirms
there is no 0.18.6 release; the advisory's fixed >=0.20 cannot satisfy GTK's
0.18 requirement. This change backports
[upstream PR 1343](https://github.com/gtk-rs/gtk-rs-core/pull/1343).

Vendor provenance: published crates.io package checksum
`233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`,
upstream commit `42b9caf98e03ded086362d9653ca58fe94dc8658`. Hash comparison
against the cached published package finds changes only in Cargo.toml
(standalone workspace) and src/variant_iter.rs (fix, safety comment, test).
MIT license/copyright retained; cache marker omitted; PATCH.md and a locked
standalone test dependency graph added. The application lockfile changes
only the GLib source/checksum to use the local Cargo patch.

## Reproducible validation (2026-11-04)

Windows host, pinned Rust 1.96.0:

- `cargo test --manifest-path src-tauri/Cargo.toml --locked`: 85 passed,
  4 ignored explicit attended/isolated fixtures.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings`: passed.
- `cargo fmt --all` and `cargo fmt --manifest-path src-tauri/Cargo.toml`,
  followed by both corresponding `-- --check` commands: passed, no unrelated
  formatting changes. Edited vendor Rust file formatted separately.
- `node --test tools/quality/glib-backport.test.mjs tools/quality/policy.test.mjs tools/quality/verify.test.mjs`:
  70 passed. Guard checks all-target metadata resolves the single GLib
  package to vendor/glib and rejects the original immutable pointer code.
- `cargo deny --manifest-path src-tauri/Cargo.toml check advisories licenses sources bans`:
  exit 0; all four policies pass. Duplicate-version warnings remain under
  the explicit `multiple-versions = "warn"` policy; this change introduces
  no additional package versions. Do not relax or suppress the advisory.
- `node tools/docs/validate.mjs`: passed (223 Markdown files, 1,181 links).
- `git diff --check`: passed for authored changes. The staged import reports
  one blank line at EOF in vendor/glib/LICENSE:18; retained byte-for-byte
  from the published package with the rest of the third-party license.

WSL Ubuntu, pinned Rust 1.96.0, GLib 2.80.0 (installed libglib2.0-dev):

```sh
CARGO_TARGET_DIR=/tmp/audiorouter-glib-backport-target cargo test --manifest-path vendor/glib/Cargo.toml --locked --release --lib variant_iter::tests
```

All 12 iterator tests pass. Temporarily reverting the two pointer edits
and running the focused `string_iterator_out_pointer_regression` produces
SIGSEGV (signal 11); restoring the fix passes again. This is real optimized
GLib execution on Linux, separate from the Windows shell evidence.

The imported crate emits 34 compiler warnings: unnecessary parentheses and
hidden elided lifetimes. Retained to keep the vendored source auditable
against the published package; unrelated third-party modernization is not
part of this soundness fix. Strict Windows product Clippy remains clean.
Raw local WSL logs: /tmp/audiorouter-glib-{original,fixed}.txt. CI repeats
the focused optimized regression with the standalone lockfile.

## Limits, rollback, next action

No GUI/audio/driver/installer acceptance was run; this changes no Windows
runtime dependency, API, control or session format. Hosted CI remains
required before merge. This is a source backport retaining upstream version
0.18.5, so version-based Dependabot scanning may still report the advisory;
do not claim automatic closure or dismiss it as an upstream upgrade.

Rollback: revert the PR, restoring the prior registry resolution; no data
migration. Remove the vendor copy/patch after Tauri's GTK dependency chain
accepts a fixed GLib release, then regenerate the lockfile and revalidate.
Next: open the authorized PR and review its CI results before merging.
