//! What every suite that touches the file system needs: a directory that
//! cleans up after itself, and a name that cannot collide with a previous
//! run's.
//!
//! 1. The disk fills. A container's writable space is a fixed allowance,
//!    and a suite that leaks a directory per case spends it on nothing.
//! 2. **A name can be reused.** `pid_max` is 32,768 on Linux by default,
//!    so a run started an hour later can hold a pid an earlier run held —
//!    and then `tt-test-<pid>-7` is a directory another run's files are
//!    still in. Which case lands on which number depends on thread
//!    scheduling, so *which* case sees the collision is different every
//!    time, which is what an intermittent failure looks like from outside
//!    (docs/tasks/TASK-222).
//!
//! A per-process nonce closes the second, and `Drop` closes the first.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static SEQ: AtomicUsize = AtomicUsize::new(0);
static RUNS: Mutex<()> = Mutex::new(());

fn nonce() -> u128 {
    static NONCE: OnceLock<u128> = OnceLock::new();
    *NONCE.get_or_init(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or_default()
    })
}

/// A temporary directory that removes itself when the test ends.
pub struct Workspace {
    path: PathBuf,
}

impl Workspace {
    /// A fresh directory named for `tag`, the process, and the case.
    pub fn new(tag: &str) -> Workspace {
        Workspace::under(&std::env::temp_dir(), tag)
    }

    /// The same, with `sub` created inside it.
    pub fn with_subdir(tag: &str, sub: &str) -> Workspace {
        let workspace = Workspace::new(tag);
        std::fs::create_dir_all(workspace.path.join(sub)).expect("a writable temporary directory");
        workspace
    }

    /// A workspace under the repository's `target/`.
    ///
    /// For a case that needs the project to have **dependencies**: a
    /// TypeScript project resolves its compiler from `node_modules` walking
    /// upwards (`src/typescript/toolchain.rs`), so a project in the system
    /// temp directory has none, while one under the repository inherits the
    /// repository's — exactly as a package of a monorepo inherits its
    /// root's. The naming, and therefore the collision argument above, is
    /// unchanged.
    pub fn in_repo(tag: &str) -> Workspace {
        Workspace::under(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("target/tt-tests"),
            tag,
        )
    }

    /// The same, with `sub` created inside it.
    pub fn in_repo_with_subdir(tag: &str, sub: &str) -> Workspace {
        let workspace = Workspace::in_repo(tag);
        std::fs::create_dir_all(workspace.path.join(sub)).expect("a writable temporary directory");
        workspace
    }

    fn under(base: &Path, tag: &str) -> Workspace {
        let run = base.join(format!("tt-test-run-{}-{}", std::process::id(), nonce()));
        let path = run.join(format!("{tag}-{}", SEQ.fetch_add(1, Ordering::SeqCst)));
        let _runs = RUNS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::fs::create_dir_all(&path).expect("a writable temporary directory");
        Workspace { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gives a child whose exit cannot finalize instrumentation its own
    /// disposable coverage profile. LLVM instrumentation is inherited by
    /// subprocesses, but a process killed by the test or ended from a
    /// non-unwinding error path cannot contribute a complete aggregate file.
    /// Normally exiting children keep the inherited path and remain measured.
    pub fn isolate_unfinalized_child_profile(&self, command: &mut std::process::Command) {
        if std::env::var_os("LLVM_PROFILE_FILE").is_some() {
            command.env(
                "LLVM_PROFILE_FILE",
                self.path.join("unfinalized-child-%p-%m.profraw"),
            );
        }
    }
}

impl std::ops::Deref for Workspace {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Workspace {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

// So a workspace can be a command's argument or working directory without
// the caller reaching past it.
impl AsRef<std::ffi::OsStr> for Workspace {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.path.as_os_str()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
        let _runs = RUNS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(run) = self.path.parent() {
            let _ = std::fs::remove_dir(run);
        }
    }
}
