//! Writing to stdout, and what happens when the reader goes away.
//!
//! `println!` panics when the write fails, and this binary's panic hook
//! reports an internal compiler error, points at the issue tracker and
//! exits 101 (`crate::ice`). A closed pipe is not that: `ttc --help | head`
//! and a plugin that stops reading `ttc -p` are ordinary usage, and nothing
//! about the compiler went wrong. Printing goes through here so that case
//! ends the run quietly instead.
//!
//! Rust ignores `SIGPIPE`, which is why the write returns an error rather
//! than killing the process the way `cat` is killed; restoring the default
//! disposition needs `unsafe`, which this crate forbids. Handling the error
//! is the same outcome by a route the compiler is allowed to take.

use std::io::{self, Write};

/// Prints `text` and a newline.
pub(super) fn line(text: &str) {
    write(text, true);
}

/// Prints `text` with no trailing newline.
pub(super) fn text(text: &str) {
    write(text, false);
}

fn write(body: &str, newline: bool) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let wrote = out.write_all(body.as_bytes()).and_then(|()| {
        if newline {
            out.write_all(b"\n")
        } else {
            Ok(())
        }
    });
    let Err(error) = wrote else {
        return;
    };
    if error.kind() == io::ErrorKind::BrokenPipe {
        // The reader stopped reading. There is nothing left to say, and
        // nothing to report: this is how a shell pipeline ends.
        std::process::exit(0);
    }
    // A full disk or a closed descriptor is a real failure of this run, and
    // it is the run that failed, not the compiler.
    eprintln!("ttc: cannot write to stdout: {error}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_write_that_succeeds_says_nothing() {
        // The interesting paths end the process, so what is pinned here is
        // that the ordinary one does not: `line` and `text` return.
        line("");
        text("");
    }
}
