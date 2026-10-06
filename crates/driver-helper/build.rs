// The helper changes the driver store and creates a root device, so Windows
// must show a UAC prompt when it starts (VDEV-08, 17 §6). Only the binary
// gets the manifest: unit-test executables stay runnable without elevation.
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bins=/MANIFESTUAC:level='requireAdministrator' uiAccess='false'"
        );
    }
    println!("cargo:rerun-if-changed=build.rs");
}
