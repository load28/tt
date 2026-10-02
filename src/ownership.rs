//! Which files on disk are ttc's own outputs.
//!
//! Every file a build writes has a private sibling record,
//! `.<name>.ttc-output.json`, holding the exact bytes last published (and,
//! while a publication is in flight, the bytes it replaces). A file
//! is ttc's output while its record says so and its content is still those
//! bytes; an edited file is the user's. The build driver consults this before
//! overwriting a file or taking one as an input, and the typed engine before
//! admitting one to the TypeScript program, as `tsc` leaves its own outputs
//! out of a default `include`.

use std::fs;
use std::path::{Path, PathBuf};

/// The record that says who published `output`.
pub fn record_path(output: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(output.file_name().unwrap_or_default());
    name.push(".ttc-output.json");
    output.with_file_name(name)
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
        && fs::read_to_string(output).is_ok_and(|actual| {
            ["content", "replaced"]
                .iter()
                .any(|key| record[*key].as_str() == Some(actual.as_str()))
        })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    #[test]
    fn a_record_names_its_output_byte_for_byte() {
        let first = record_path(Path::new(OsStr::from_bytes(b"out/a\xff.ts")));
        let second = record_path(Path::new(OsStr::from_bytes(b"out/a\xfe.ts")));
        assert_eq!(
            first.file_name().unwrap().as_bytes(),
            b".a\xff.ts.ttc-output.json"
        );
        assert_ne!(first, second);
    }
}
