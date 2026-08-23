//! Capture the exact Cargo build target and compiled target features for the
//! performance benchmark's configuration report. Cargo exposes
//! `CARGO_CFG_TARGET_FEATURE` to build scripts, but not directly to the
//! benchmark at runtime, so these values are forwarded as compile-time
//! environment variables. This avoids a hand-maintained CPU-feature list and
//! helps ensure baseline and candidate measurements use comparable binaries.

use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_FEATURE");
    println!("cargo:rerun-if-env-changed=RUSTFLAGS");

    let target = env::var("TARGET").expect("Cargo must set TARGET for build scripts");
    let feature_list = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let mut features: Vec<_> = feature_list
        .split(',')
        .filter(|feature| !feature.is_empty())
        .collect();
    features.sort_unstable();
    features.dedup();

    println!("cargo:rustc-env=CLI_BUILD_TARGET={target}");
    println!(
        "cargo:rustc-env=CLI_BUILD_TARGET_FEATURES={}",
        features.join(",")
    );
}
