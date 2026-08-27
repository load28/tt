//! Where TypeScript 7 comes from: **the package the project installed**.
//!
//! ttc drives the same TypeScript twice — as an API server for the checker
//! ([`super::native`]) and as a language server for the editor surface
//! ([`super::service`]). They are two halves of one install, so both are
//! resolved here, from one description, and there is exactly one place they
//! can come from: `node_modules`, walked upwards from the project.
//!
//! **There is deliberately no environment variable and no checkout path.**
//! TypeScript 7 publishes the native executable *and* the API client in its
//! npm packages, so a second way to name a toolchain would only be a second
//! way for the editor and the command line to disagree about which
//! TypeScript a project uses — which is what having one used to cause
//! (TASK-255, TASK-256). A project pins its TypeScript the way it pins
//! every other dependency, and everything that reads this module inherits
//! that one answer.
//!
//! The layout is upstream's, not ours: `getExePath.js` in the published
//! package derives the executable's name from the package's own name
//! (`typescript` ships `tsc`, every other distribution ships `tsgo`) and
//! finds it in `@typescript/<base>-<platform>-<arch>/lib`.

use std::path::{Path, PathBuf};

/// The API client's path inside its package.
const API_IN_PACKAGE: &str = "dist/api/sync/api.js";

/// How to install what is missing — the one sentence every error here ends
/// with, so the fix never depends on which half reported it.
const INSTALL: &str = "install it in this project (`npm i -D typescript@7`)";

/// One npm distribution of TypeScript 7.
struct Distribution {
    /// The package carrying the JS API client.
    client: &'static str,
    /// The base name the per-platform package is derived from:
    /// `@typescript/<base>-<os>-<arch>`.
    base: &'static str,
    /// The executable's name inside that package's `lib/` (no extension).
    exe: &'static str,
}

/// The distributions ttc resolves, in order. `typescript` is the released
/// package; `@typescript/native-preview` is the preview channel it grew out
/// of, still resolved for a project that has not moved off it.
const DISTRIBUTIONS: [Distribution; 2] = [
    Distribution {
        client: "typescript",
        base: "typescript",
        exe: "tsc",
    },
    Distribution {
        client: "@typescript/native-preview",
        base: "native-preview",
        exe: "tsgo",
    },
];

/// The API client half — the JS module the host imports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Client {
    /// The client module's path (`.../dist/api/sync/api.js`). Absolute: the
    /// host imports it by path and runs in the project's directory, so a
    /// relative path would resolve against neither. The client finds the
    /// executable shipped beside it on its own.
    pub api: PathBuf,
}

/// Resolves the API client for a project at `from`.
pub(crate) fn client(from: &Path) -> Result<Client, String> {
    for node_modules in node_modules_from(from) {
        for distribution in &DISTRIBUTIONS {
            let api = node_modules.join(distribution.client).join(API_IN_PACKAGE);
            if api.exists() {
                return Ok(Client { api: absolute(api) });
            }
        }
    }
    Err(format!("no TypeScript compiler found — {INSTALL}"))
}

/// Resolves the executable that serves the language service for a project
/// at `from`.
///
/// Always absolute. The server is spawned with its working directory in the
/// *project*, so a relative answer would be resolved against a directory it
/// was never measured from, and the session would fail to start for one
/// project while working for another (TASK-217).
pub(crate) fn service_binary(from: &Path) -> Result<PathBuf, String> {
    for node_modules in node_modules_from(from) {
        for distribution in &DISTRIBUTIONS {
            let exe = node_modules
                .join(distribution.platform_package())
                .join("lib")
                .join(exe_file_name(distribution.exe));
            if exe.exists() {
                return Ok(absolute(exe));
            }
        }
    }
    Err(format!("no TypeScript language server found — {INSTALL}"))
}

impl Distribution {
    /// `@typescript/<base>-<os>-<arch>` — the package carrying the native
    /// executable for this host.
    fn platform_package(&self) -> String {
        format!("@typescript/{}-{}-{}", self.base, os_name(), arch_name())
    }
}

/// The executable's file name on this platform.
fn exe_file_name(bin: &str) -> String {
    if cfg!(windows) {
        format!("{bin}.exe")
    } else {
        bin.to_string()
    }
}

/// Every `node_modules` directory from `from` upwards, nearest first — how
/// Node itself resolves a package, and therefore how a project's own
/// TypeScript is found rather than some other project's.
fn node_modules_from(from: &Path) -> impl Iterator<Item = PathBuf> {
    let mut dir = Some(from.canonicalize().unwrap_or_else(|_| from.to_path_buf()));
    std::iter::from_fn(move || {
        let current = dir.take()?;
        let node_modules = current.join("node_modules");
        dir = current.parent().map(Path::to_path_buf);
        Some(node_modules)
    })
}

/// The path as a process started elsewhere will see it.
fn absolute(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

fn arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `node_modules` holding one distribution's published layout: the API
    /// client in the client package, the executable in the platform package.
    fn install(dir: &Path, distribution: &Distribution) -> PathBuf {
        let node_modules = dir.join("node_modules");
        let api = node_modules.join(distribution.client).join(API_IN_PACKAGE);
        std::fs::create_dir_all(api.parent().unwrap()).unwrap();
        std::fs::write(&api, "").unwrap();
        let exe = node_modules
            .join(distribution.platform_package())
            .join("lib")
            .join(exe_file_name(distribution.exe));
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, "").unwrap();
        node_modules
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tt-toolchain-{name}"));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Both halves of a published install are found — the checker's API
    /// client *and* the language server's executable. Finding only one is
    /// the failure this module exists to prevent: the command line then
    /// type-checks while the editor answers nothing (TASK-255).
    #[test]
    fn both_halves_of_every_published_distribution_resolve() {
        for distribution in &DISTRIBUTIONS {
            let dir = scratch(distribution.base);
            install(&dir, distribution);
            assert!(client(&dir).is_ok(), "no client for {}", distribution.base);
            let exe = service_binary(&dir)
                .unwrap_or_else(|e| panic!("no executable for {}: {e}", distribution.base));
            assert_eq!(
                exe.file_name().unwrap(),
                exe_file_name(distribution.exe).as_str()
            );
            std::fs::remove_dir_all(&dir).ok();
        }
    }

    /// Upstream's rule (`getExePath.js`): the `typescript` package ships
    /// `tsc`, every other distribution ships `tsgo`. The preview package
    /// named `tsc` here is what silenced the editor.
    #[test]
    fn executable_name_follows_the_package_name() {
        for distribution in &DISTRIBUTIONS {
            let expected = if distribution.base == "typescript" {
                "tsc"
            } else {
                "tsgo"
            };
            assert_eq!(distribution.exe, expected);
        }
    }

    /// A package installed at the workspace root serves a file nested
    /// anywhere below it — Node's own rule, and what makes one install at a
    /// monorepo root enough.
    #[test]
    fn resolution_walks_up_to_the_installing_root() {
        let dir = scratch("walk");
        install(&dir, &DISTRIBUTIONS[0]);
        let nested = dir.join("packages/app/src");
        std::fs::create_dir_all(&nested).unwrap();
        assert!(client(&nested).is_ok());
        assert!(service_binary(&nested).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Both halves fail the same way, and the message names the one fix —
    /// there is no second place a toolchain could have come from.
    #[test]
    fn a_project_without_typescript_is_told_how_to_install_it() {
        let dir = scratch("empty");
        for message in [client(&dir).unwrap_err(), service_binary(&dir).unwrap_err()] {
            assert!(message.contains(INSTALL), "unhelpful message: {message}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
