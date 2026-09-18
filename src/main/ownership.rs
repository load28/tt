//! Persistent output ownership, independent of banners and input selection.
//!
//! A record keeps the exact last published bytes. This permits rebuilds while
//! refusing to replace an authored or subsequently edited file. Records are
//! private siblings, so directory scans never treat them as project inputs.

use super::*;

fn record_path(output: &Path) -> PathBuf {
    output.with_file_name(format!(
        ".{}.ttc-output.json",
        output.file_name().unwrap_or_default().to_string_lossy()
    ))
}

fn record(output: &Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&fs::read(record_path(output)).ok()?).ok()
}

pub(super) fn owned_output(output: &Path) -> bool {
    let Some(record) = record(output) else {
        return false;
    };
    record["version"] == 1
        && record["content"].as_str().is_some_and(|expected| {
            fs::read_to_string(output).is_ok_and(|actual| actual == expected)
        })
}

pub(super) fn check_output_owner(output: &Path, source: &Path) -> Result<(), String> {
    if same_file(output, source) {
        return Err(format!(
            "ttc: {}: output would overwrite the input — pass -o <dir>",
            output.display()
        ));
    }
    if !output.exists() {
        return Ok(());
    }
    let owner = normalized_absolute(source);
    if owned_output(output)
        && record(output).is_some_and(|record| record["source"].as_str() == owner.to_str())
    {
        return Ok(());
    }
    Err(format!(
        "ttc: {}: output is not owned by this input or has been edited; refusing to overwrite it — choose an empty output directory",
        output.display()
    ))
}

pub(super) fn write_owned_output(output: &Path, source: &Path, code: &str) -> Result<(), String> {
    check_output_owner(output, source)?;
    write_output(output, code)?;
    let record =
        serde_json::json!({ "version": 1, "source": normalized_absolute(source), "content": code });
    write_output(&record_path(output), &record.to_string())
}
