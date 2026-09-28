#![allow(dead_code)] // each suite uses the part of this it needs

#[path = "../../src/test_workspace.rs"]
mod workspace;

#[allow(unused_imports)]
pub use workspace::Workspace;

use std::path::{Path, PathBuf};

/// Whether this repository has a TypeScript for a suite to run against,
/// resolved the way ttc resolves it: `node_modules` from here upwards.
///
/// Two suites need this and used to each answer it: the backend cases,
/// which cannot run without a checker, and the emit fixtures, whose
/// artifact carries the annotations a checker infers. One answer keeps the
/// two from disagreeing about whether a checkout is configured.
pub fn toolchain() -> bool {
    if toolchain_installed() {
        return true;
    }
    // A caller that asked for no skipping gets an error, not a pass.
    assert!(
        !toolchain_required(),
        "TTC_REQUIRE_TSGO is set but this repository has no TypeScript \
         installed — run `npm ci` at the repository root"
    );
    false
}

/// True when the caller has declared that a toolchain must be present.
pub fn toolchain_required() -> bool {
    std::env::var_os("TTC_REQUIRE_TSGO").is_some_and(|v| !v.is_empty() && v != "0")
}

/// The API client of an installed TypeScript, searched for the way
/// `toolchain.rs` searches — a guard that mirrors only part of the
/// compiler's rules reports "no toolchain" where the compiler finds one
/// (TASK-217).
pub fn toolchain_installed() -> bool {
    const CLIENTS: [&str; 2] = ["typescript", "@typescript/native-preview"];
    let mut dir = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    while let Some(current) = dir {
        for client in CLIENTS {
            if current
                .join("node_modules")
                .join(client)
                .join("dist/api/sync/api.js")
                .exists()
            {
                return true;
            }
        }
        dir = current.parent().map(Path::to_path_buf);
    }
    false
}
