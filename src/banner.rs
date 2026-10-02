//! The `// @generated` banner at the top of a file ttc writes: a compiled
//! module and a declaration sidecar put theirs in the same place.

/// Where a generated banner went, so a source map can shift only the lines
/// that actually moved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BannerPlacement {
    /// Lines the banner added.
    pub lines: usize,
    /// The generated line it was written at. Lines before it did not move.
    pub at_line: usize,
}

/// Writes a banner into `code` at the first position the file allows.
///
/// A `#!` line and a byte-order mark are only themselves when they come
/// first: a comment above either one turns a runnable script into a parse
/// error and leaves a stray U+FEFF mid-file. Everything else about the top
/// of a file — a license comment, a blank line, a directive prologue such as
/// `"use client"` — a comment may precede, because a comment is not a
/// statement and does not end a prologue.
pub fn write_banner(code: &mut String, banner: &str) -> BannerPlacement {
    let line_map = crate::lines::LineMap::ecma(code);
    let mut at = line_map.line_start(0).unwrap_or(0);
    let mut at_line = 0;
    let mut lines = 1;
    let mut prefix_newline = false;
    if code[at..].starts_with("#!") {
        at = line_map.line_end(0).unwrap_or(code.len());
        // The banner starts the line after the shebang, which stays put.
        at_line = 1;
        if line_map.len() == 1 {
            // A shebang that runs to the end of the file: the banner
            // needs a line of its own to sit on.
            prefix_newline = true;
            lines += 1;
        }
    }
    let mut written = String::with_capacity(code.len() + banner.len() + 1);
    written.push_str(&code[..at]);
    if prefix_newline {
        written.push_str(crate::line_ending(code));
    }
    written.push_str(banner);
    written.push_str(&code[at..]);
    *code = written;
    BannerPlacement { lines, at_line }
}
