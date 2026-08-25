//! What every suite that touches the file system needs: a directory that
//! cleans up after itself, and a name that cannot collide with a previous
//! run's.
//!
//! Before this, three suites each built their own `tt-<tag>-<pid>-<seq>`
//! and left cleanup to the caller, who mostly forgot: 5,600 directories
//! were sitting in `/tmp` on the machine this was written on. Two things
//! follow from that, and the second is the reason this module exists.
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
//! A per-process nonce closes the second, and `Drop` closes the first —
//! except when the test failed, where the directory is what a person needs
//! to look at, so it is kept and its path is printed.

#![allow(dead_code)] // each suite uses the part of this it needs

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

static SEQ: AtomicUsize = AtomicUsize::new(0);

/// A value this process will not share with any other, so a recycled pid
/// cannot land on a directory that is still in use.
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
        let path = std::env::temp_dir().join(format!(
            "tt-{tag}-{}-{}-{}",
            std::process::id(),
            nonce(),
            SEQ.fetch_add(1, Ordering::SeqCst),
        ));
        std::fs::create_dir_all(&path).expect("a writable temporary directory");
        Workspace { path }
    }

    /// The same, with `sub` created inside it.
    pub fn with_subdir(tag: &str, sub: &str) -> Workspace {
        let workspace = Workspace::new(tag);
        std::fs::create_dir_all(workspace.path.join(sub)).expect("a writable temporary directory");
        workspace
    }

    pub fn path(&self) -> &Path {
        &self.path
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
        if std::thread::panicking() {
            // The failure is the point; the files are the evidence.
            eprintln!("workspace kept for inspection: {}", self.path.display());
            return;
        }
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
