//! Persistent output ownership, independent of banners and input selection.
//!
//! A record keeps the exact last published bytes. This permits rebuilds while
//! refusing to replace an authored or subsequently edited file. Records are
//! private siblings, so directory scans never treat them as project inputs.

use super::*;
use ttc::ownership::{owned_output, record, record_path};

#[derive(Clone, Copy)]
pub(super) enum OutputOwner<'a> {
    Source(&'a Path),
    Support(StdModule),
}

fn source_identity(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| normalized_absolute(path))
}

fn support_identity(module: StdModule) -> String {
    format!("@tt/std/{}", module.file_name())
}

fn owns(record: &serde_json::Value, owner: OutputOwner) -> bool {
    match owner {
        OutputOwner::Source(source) => record["source"].as_str().is_some_and(|recorded| {
            source_identity(Path::new(recorded)) == source_identity(source)
        }),
        OutputOwner::Support(module) => {
            let identity = support_identity(module);
            record["support"].as_str() == Some(identity.as_str())
                || record["source"].as_str() == normalized_absolute(Path::new(&identity)).to_str()
        }
    }
}

pub(super) fn check_output_owner(output: &Path, owner: OutputOwner) -> Result<(), String> {
    if let OutputOwner::Source(source) = owner
        && same_file(output, source)
    {
        return Err(format!(
            "ttc: {}: output would overwrite the input — pass -o <dir>",
            output.display()
        ));
    }
    if !output.exists() {
        return Ok(());
    }
    if owned_output(output) && record(output).is_some_and(|record| owns(&record, owner)) {
        return Ok(());
    }
    Err(format!(
        "ttc: {}: output is not owned by this input or has been edited; refusing to overwrite it — choose an empty output directory",
        output.display()
    ))
}

pub(super) fn write_owned_output(
    output: &Path,
    owner: OutputOwner,
    code: &str,
) -> Result<(), String> {
    check_output_owner(output, owner)?;
    write_output(output, code)?;
    let record = match owner {
        OutputOwner::Source(source) => {
            serde_json::json!({ "version": 1, "source": source_identity(source), "content": code })
        }
        OutputOwner::Support(module) => {
            serde_json::json!({ "version": 1, "support": support_identity(module), "content": code })
        }
    };
    write_output(&record_path(output), &record.to_string())
}
