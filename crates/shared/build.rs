//! Build script: exposes the workspace root so the game can find `assets/`
//! regardless of the working directory it is launched from.

fn main() {
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..").canonicalize().unwrap_or(manifest);
    println!(
        "cargo:rustc-env=DEEPCORE_ASSETS={}",
        root.join("assets").display()
    );
    println!("cargo:rerun-if-changed=build.rs");
}
