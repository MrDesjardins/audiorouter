# GLib 0.18.5 soundness backport

Source: the complete published crates.io `glib` 0.18.5 package, retaining
its MIT LICENSE and COPYRIGHT. Cargo cache marker `.cargo-ok` is omitted.

Upstream fix: https://github.com/gtk-rs/gtk-rs-core/pull/1343
Advisory: https://rustsec.org/advisories/RUSTSEC-2024-0429.html

Changes: `VariantStrIter::impl_get` passes `&mut p` for the mutable C
out-argument instead of writing through `&p`; adds a safety explanation
and a regression covering all five affected iterator methods. A standalone
workspace table allows the regression to run separately from the Windows app.
The shell's `[patch.crates-io]` applies this copy to every transitive consumer.
No dependency upgrade or advisory suppression is involved.

On Linux with GLib development libraries installed, run:

```sh
cargo test --manifest-path vendor/glib/Cargo.toml --locked --release --lib string_iterator_out_pointer_regression
```

Optimizations matter: they exposed the original immutable out-pointer UB.
CI executes this regression on Linux; Windows does not compile GTK/GLib.
The added Cargo.lock pins the standalone regression dependencies.
The portable dependency guard also checks actual Cargo resolution and the
exact out-pointer invariant. Removing either fix must fail that guard.

The retained version can still trigger version-based security scanners.
Do not dismiss the alert as an upstream upgrade. Remove this patch and vendor
copy once Tauri's full GTK dependency chain accepts a fixed GLib release
(>=0.20), then regenerate the lockfile and rerun the dependency checks.
