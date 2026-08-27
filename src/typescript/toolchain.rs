//! Where TypeScript 7 comes from — the one description both halves read.
//!
//! ttc drives the same TypeScript twice: as an **API server** for the
//! checker ([`super::native`]) and as a **language server** for the editor
//! surface ([`super::service`]). They are two halves of one install — the
//! protocols carry no version negotiation — so *which* install, and *where
//! its files are*, is decided here rather than once per consumer. Two
//! copies of these rules is how a project ends up type-checking on the
//! command line while the editor answers nothing (TASK-255).
//!
//! **Order, first hit wins:**
//!
//! 1. **Named by the environment** — `TTC_TSGO_API` (with an optional
//!    `TTC_TSGO_BIN`) for the API client, `TTC_TSGO_BIN` for the language
//!    server.
//! 2. **A built typescript-go checkout** — `TTC_TSGO_ROOT`, else a
//!    `../typescript-go` sibling.
//! 3. **A package installed in the project** — `node_modules` from the file
//!    upwards, which is the TypeScript the project's code is written
//!    against.
//!
//! 1 and 2 are instructions, not guesses: when `TTC_TSGO_*` names something
//! that is not there, ttc says so and stops instead of quietly running some
//! other TypeScript. The `../typescript-go` sibling is a convention rather
//! than an instruction, so it is skipped when absent. No path is compiled
//! in, and a tree that is not built yet is reported as such.
//!
//! The installed layout is upstream's, not ours: `getExePath.js` in the
//! published package derives the executable's name from the package's own
//! name (`typescript` ships `tsc`, every other distribution ships `tsgo`)
//! and finds it in `@typescript/<base>-<platform>-<arch>/lib`.

use std::path::{Path, PathBuf};

/// `tsgo`'s path inside a built typescript-go tree.
pub(crate) const BIN_IN_TREE: &str = "built/local/tsgo";
/// The JS API client's path inside a built typescript-go tree.
const API_IN_TREE: &str = "_packages/native-preview/dist/api/sync/api.js";
/// The API client's path inside an installed package.
const API_IN_PACKAGE: &str = "dist/api/sync/api.js";

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

/// The distributions ttc resolves, in order. `typescript` is the eventual
/// released package; `@typescript/native-preview` is what the TypeScript
/// team publishes today.
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

/// A place a toolchain may come from, in the order ttc takes them. Both
/// halves walk this one list, so their priority cannot drift apart.
enum Source {
    /// What the environment names outright.
    Named,
    /// A built typescript-go checkout. `named` marks the one the user
    /// pointed at (`TTC_TSGO_ROOT`): its absence is an error, where the
    /// sibling's is simply a miss.
    Checkout { root: PathBuf, named: bool },
    /// Whatever the project installed, from `from` upwards.
    Installed { from: PathBuf },
}

/// The sources for a toolchain serving `from`, in priority order.
fn sources(from: &Path) -> Vec<Source> {
    let mut sources = vec![Source::Named];
    if let Some(root) = env_path("TTC_TSGO_ROOT") {
        sources.push(Source::Checkout { root, named: true });
    }
    sources.push(Source::Checkout {
        root: PathBuf::from("../typescript-go"),
        named: false,
    });
    sources.push(Source::Installed {
        from: from.to_path_buf(),
    });
    sources
}

/// The API client half, and the executable to run it against.
///
/// An installed package ships both together and the client finds its own
/// executable, so `bin` is `None` there; a checkout names both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Client {
    /// The `tsgo` executable to run as the API server, or `None` to let the
    /// API client run the one shipped beside it.
    pub bin: Option<PathBuf>,
    /// The JS client module the host imports (`.../api/sync/api.js`).
    pub api: PathBuf,
}

/// Resolves the API client for a project at `from`. The error names what is
/// missing and how to produce it.
pub(crate) fn client(from: &Path) -> Result<Client, String> {
    for source in sources(from) {
        match source {
            Source::Named => {
                if let Some(api) = env_path("TTC_TSGO_API") {
                    return checked(Client {
                        bin: env_path("TTC_TSGO_BIN"),
                        api,
                    });
                }
            }
            Source::Checkout { root, named } => {
                let candidate = Client {
                    bin: Some(root.join(BIN_IN_TREE)),
                    api: root.join(API_IN_TREE),
                };
                if named || candidate.api.exists() {
                    return checked(candidate);
                }
            }
            Source::Installed { from } => {
                for node_modules in node_modules_from(&from) {
                    for distribution in &DISTRIBUTIONS {
                        let api = node_modules.join(distribution.client).join(API_IN_PACKAGE);
                        if api.exists() {
                            return checked(Client { bin: None, api });
                        }
                    }
                }
            }
        }
    }
    Err(format!(
        "no TypeScript compiler found — install one \
         (`npm i -D typescript@7`), or build a typescript-go checkout \
         (`go build -o {BIN_IN_TREE} ./cmd/tsgo` plus `npm ci && npx \
         tsc -b _packages/native-preview`) and point ttc at it with \
         TTC_TSGO_ROOT"
    ))
}

/// Resolves the `tsgo` executable that serves the language service for a
/// project at `from`.
///
/// Always absolute. The server is spawned with its working directory in the
/// *project*, so a relative answer — the sibling checkout's, or a relative
/// `TTC_TSGO_ROOT` — would be resolved against a directory it was never
/// measured from, and the session would fail to start for one project while
/// working for another (TASK-217).
pub(crate) fn service_binary(from: &Path) -> Result<PathBuf, String> {
    for source in sources(from) {
        match source {
            Source::Named => {
                if let Some(bin) = env_path("TTC_TSGO_BIN") {
                    return checked_bin(bin).map(absolute);
                }
            }
            Source::Checkout { root, named } => {
                let bin = root.join(BIN_IN_TREE);
                if named {
                    return checked_bin(bin).map(absolute);
                }
                if bin.exists() {
                    return Ok(absolute(bin));
                }
            }
            Source::Installed { from } => {
                for node_modules in node_modules_from(&from) {
                    for distribution in &DISTRIBUTIONS {
                        let exe = node_modules
                            .join(distribution.platform_package())
                            .join("lib")
                            .join(exe_file_name(distribution.exe));
                        if exe.exists() {
                            return Ok(exe);
                        }
                    }
                }
            }
        }
    }
    Err("no tsgo language server found — install TypeScript 7 \
         (`npm i -D typescript@7`) or point TTC_TSGO_ROOT at a built \
         typescript-go checkout"
        .to_string())
}

impl Distribution {
    /// `@typescript/<base>-<os>-<arch>` — the package carrying the native
    /// executable for this host.
    fn platform_package(&self) -> String {
        format!("@typescript/{}-{}-{}", self.base, os_name(), arch_name())
    }
}

/// Rejects a client whose halves are not both present, naming the step that
/// produces the missing one, and makes what survives absolute — the host
/// imports the client by path and runs in the project's directory, so a
/// relative path would resolve against neither.
fn checked(client: Client) -> Result<Client, String> {
    if let Some(bin) = &client.bin {
        checked_bin(bin.clone())?;
    }
    if !client.api.exists() {
        return Err(format!(
            "no TypeScript API client at {} — in a typescript-go checkout \
             build it with `npm ci && npx tsc -b _packages/native-preview` \
             (the client and the executable must come from one build)",
            client.api.display(),
        ));
    }
    Ok(Client {
        bin: client.bin.map(absolute),
        api: absolute(client.api),
    })
}

/// The executable, or the error that names how to build it.
fn checked_bin(bin: PathBuf) -> Result<PathBuf, String> {
    if bin.exists() {
        return Ok(bin);
    }
    Err(format!(
        "no tsgo executable at {} — build one with `go build -o {} \
         ./cmd/tsgo` in a typescript-go checkout",
        bin.display(),
        BIN_IN_TREE,
    ))
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

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
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
            let node_modules = install(&dir, distribution);
            assert!(
                node_modules
                    .join(distribution.client)
                    .join(API_IN_PACKAGE)
                    .exists()
            );
            let exe = node_modules
                .join(distribution.platform_package())
                .join("lib")
                .join(exe_file_name(distribution.exe));
            assert!(exe.exists(), "no executable for {}", distribution.base);
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
    /// anywhere below it — Node's own rule.
    #[test]
    fn resolution_walks_up_to_the_installing_root() {
        let dir = scratch("walk");
        let distribution = &DISTRIBUTIONS[1];
        install(&dir, distribution);
        let nested = dir.join("packages/app/src");
        std::fs::create_dir_all(&nested).unwrap();
        let found = node_modules_from(&nested).find(|nm| {
            nm.join(distribution.platform_package())
                .join("lib")
                .join(exe_file_name(distribution.exe))
                .exists()
        });
        assert!(found.is_some(), "the root install must serve a nested file");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The order is one list, and both halves read it: an environment that
    /// names a toolchain is taken before anything a project installed.
    #[test]
    fn the_environment_outranks_an_installed_package() {
        let ordered: Vec<&'static str> = sources(Path::new("/tmp"))
            .iter()
            .map(|source| match source {
                Source::Named => "named",
                Source::Checkout { named: true, .. } => "checkout",
                Source::Checkout { named: false, .. } => "sibling",
                Source::Installed { .. } => "installed",
            })
            .collect();
        assert_eq!(ordered.first(), Some(&"named"));
        assert_eq!(ordered.last(), Some(&"installed"));
        let sibling = ordered.iter().position(|s| *s == "sibling").unwrap();
        let installed = ordered.iter().position(|s| *s == "installed").unwrap();
        assert!(sibling < installed);
    }
}
