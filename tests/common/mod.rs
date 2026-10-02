#![allow(dead_code)] // each suite uses the part of this it needs

pub mod baseline;
pub mod cases;
pub mod matrix;
pub mod typescript_cases;

#[path = "../../src/test_workspace.rs"]
mod workspace;

#[allow(unused_imports)]
pub use workspace::Workspace;

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Whether this repository has a TypeScript for a suite to run against:
/// the one `package.json` pins, installed by `npm ci` ([`typescript`]).
///
/// Every suite that needs a checker asks this one question, so a checkout
/// is either configured for all of them or for none.
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

/// Whether the pinned TypeScript is installed with its API client — what
/// ttc drives for `--check-types`.
pub fn toolchain_installed() -> bool {
    typescript().is_some_and(|dir| dir.join("dist/api/sync/api.js").exists())
}

/// The repository's `node_modules/typescript`, when it is installed.
///
/// A project resolves TypeScript from `node_modules` walking upwards
/// (`src/typescript/toolchain.rs`), and every project a suite creates lives
/// under this repository, so this is the install ttc finds for them — and
/// `package.json` is what says which version it must be. An install of any
/// other version fails the suite instead of answering for a TypeScript the
/// repository does not pin: `npm ci` is the fix.
pub fn typescript() -> Option<PathBuf> {
    static INSTALL: OnceLock<Option<PathBuf>> = OnceLock::new();
    INSTALL
        .get_or_init(|| {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let dir = root.join("node_modules/typescript");
            let installed = fs_json(&dir.join("package.json"))?;
            let pinned = fs_json(&root.join("package.json"))
                .expect("the repository's package.json is readable JSON");
            let pinned = pinned["devDependencies"]["typescript"]
                .as_str()
                .expect("package.json pins devDependencies.typescript");
            let installed = installed["version"].as_str().unwrap_or("<none>");
            assert_eq!(
                installed, pinned,
                "node_modules/typescript is {installed} but package.json pins \
                 {pinned} — run `npm ci` at the repository root"
            );
            Some(dir)
        })
        .clone()
}

fn fs_json(path: &std::path::Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(serde_json::from_str(&text).expect("an installed package.json is JSON"))
}

/// Whether `node` and the pinned TypeScript's `tsc` can run. Absent, a
/// suite skips — unless `TTC_REQUIRE_TSGO` is set, which makes it a failure.
pub fn tsc_available() -> bool {
    let node = Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    let ready = node && typescript().is_some_and(|dir| dir.join("lib/tsc.js").exists());
    assert!(
        ready || !toolchain_required(),
        "TTC_REQUIRE_TSGO is set but node or the pinned TypeScript's tsc is \
         missing — install Node.js and run `npm ci` at the repository root"
    );
    ready
}

/// The pinned TypeScript's `tsc` — the package's own bin entry, run by
/// `node` so no `PATH` lookup can substitute another version. Guard the
/// case with [`tsc_available`] first.
pub fn tsc() -> Command {
    let entry = typescript()
        .map(|dir| dir.join("lib/tsc.js"))
        .expect("guard the case with common::tsc_available()");
    let mut command = Command::new("node");
    command.arg(entry);
    command
}
