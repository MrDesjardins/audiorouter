# Dependabot 7: Windows applicability

Requirements: ENG-02/04/05. Branch: `fix/dependabot-7-glib-iterator`.
PR: [#115](https://github.com/MrDesjardins/audiorouter/pull/115).

[Dependabot 7](https://github.com/MrDesjardins/audiorouter/security/dependabot/7)
reports [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).
The vulnerable GLib string iterator passes an immutable reference as a
mutable C out-argument. Fixed GLib >=0.20 cannot satisfy GTK's 0.18 requirement.

## Owner decision

The owner rejected importing approximately 55,000 lines of third-party code.
The vendor copy, Cargo patch and backport-specific CI/tests are removed.
Manifests, lockfiles, product source and CI match main. This PR records
applicability only; the upstream vulnerability remains unresolved.

Cargo.lock includes dependencies across targets. Tauri uses WebView2 on
Windows; GLib belongs to its Linux GTK stack. The supported Windows app
does not compile or load GLib. Cargo's normal crates.io resolution suffices;
no local vendor copy or manually loaded GLib library is needed.

## Validation

Pinned Rust 1.96.0, Windows host:

```sh
cargo tree --manifest-path src-tauri/Cargo.toml --locked --target all -i glib
cargo tree --manifest-path src-tauri/Cargo.toml --locked --target x86_64-pc-windows-msvc -i glib
cargo check --manifest-path src-tauri/Cargo.toml --locked
node tools/docs/validate.mjs
```

Initial inspection: all-target graph contains Tauri 2.11.6 / GTK 0.18.2 /
GLib 0.18.5; Windows graph contains no GLib consumers. The withdrawn
backport experiment reproduced SIGSEGV in optimized WSL execution with
the original code and passed with the upstream fix. This is historical
evidence, not a fixed dependency in the final PR.

After removal: Windows-target inspection again found no GLib consumers;
the locked shell `cargo check` passed, documentation validation passed
(239 files / 1,408 links before adding the active-plan link), and the final
diff contains documentation only. No vendor files remain in the PR diff.

## Limits and next action

Dependabot can keep reporting the vulnerable version in Cargo.lock. No
advisory suppression or alert dismissal was made. A future Linux build
needs a compatible upstream GTK/Tauri upgrade or an explicitly approved
alternative fix. No GUI/audio/driver/installer evidence is claimed.
Rollback: revert these documentation-only changes; no data migration.
Next: review this Windows-applicability disposition and track the upstream
dependency upgrade.
