//! Which files on disk are ttc's own outputs.
//!
//! Every file a build writes has a private sibling record,
//! `.<name>.ttc-output.json`, holding the exact bytes last published. A file
//! is ttc's output while its record says so and its content is still those
//! bytes; an edited file is the user's. The build driver consults this before
//! overwriting a file or taking one as an input, and the typed engine before
//! admitting one to the TypeScript program, as `tsc` leaves its own outputs
//! out of a default `include`.

use std::fs;
use std::path::{Path, PathBuf};

/// The record that says who published `output`.
pub fn record_path(output: &Path) -> PathBuf {
    output.with_file_name(format!(
        ".{}.ttc-output.json",
        output.file_name().unwrap_or_default().to_string_lossy()
    ))
}

/// The parsed record of `output`, when there is a readable one.
pub fn record(output: &Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&fs::read(record_path(output)).ok()?).ok()
}

/// Whether `output` is a file ttc published and nobody has edited since.
pub fn owned_output(output: &Path) -> bool {
    let Some(record) = record(output) else {
        return false;
    };
    record["version"] == 1
        && record["content"].as_str().is_some_and(|expected| {
            fs::read_to_string(output).is_ok_and(|actual| actual == expected)
        })
}
