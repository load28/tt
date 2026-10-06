//! Persistent output ownership, independent of banners and input selection.
//!
//! A record keeps the exact last published bytes. This permits rebuilds while
//! refusing to replace an authored or subsequently edited file, or the
//! output of another input that still exists. Records are private siblings,
//! so directory scans never treat them as project inputs.

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

fn recorded_source(source: &Path) -> Result<String, String> {
    source_identity(source)
        .into_os_string()
        .into_string()
        .map_err(|_| {
            format!(
                "ttc: {}: input path is not valid UTF-8, so its output cannot record its owner — rename the input",
                source.display()
            )
        })
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

/// Whether the source input `record` names as the output's owner no longer
/// exists: an unedited output it left behind belongs to no input, and the
/// input that now maps to the same output (a renamed or migrated source)
/// takes it over. A support module's output always has its owner.
fn orphaned(record: &serde_json::Value) -> bool {
    record["support"].is_null()
        && record["source"].as_str().is_some_and(|recorded| {
            let recorded = Path::new(recorded);
            !StdModule::ALL
                .iter()
                .any(|module| recorded.ends_with(support_identity(*module)))
                && fs::symlink_metadata(recorded)
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        })
}

pub(super) fn check_output_owner(output: &Path, owner: OutputOwner) -> Result<(), String> {
    if let OutputOwner::Source(source) = owner {
        recorded_source(source)?;
    }
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
    if owned_output(output)
        && record(output).is_some_and(|record| owns(&record, owner) || orphaned(&record))
    {
        return Ok(());
    }
    Err(format!(
        "ttc: {}: output is not owned by this input or has been edited; refusing to overwrite it — remove it, or write the outputs to another directory with -o <dir>",
        output.display()
    ))
}

pub(super) fn write_owned_output(
    output: &Path,
    owner: OutputOwner,
    code: &str,
) -> Result<(), String> {
    check_output_owner(output, owner)?;
    if let Some(parent) = output.parent() {
        super::output::create_dir_all(parent)
            .map_err(|e| format!("ttc: {}: {e}", output.display()))?;
    }
    let record = match owner {
        OutputOwner::Source(source) => {
            serde_json::json!({ "version": 1, "source": recorded_source(source)?, "content": code })
        }
        OutputOwner::Support(module) => {
            serde_json::json!({ "version": 1, "support": support_identity(module), "content": code })
        }
    };
    let replaced = owned_output(output)
        .then(|| fs::read_to_string(output).ok())
        .flatten()
        .filter(|replaced| replaced != code);
    let mut publishing = record.clone();
    if let Some(replaced) = &replaced {
        publishing["replaced"] = serde_json::Value::from(replaced.as_str());
    }
    write_output(&record_path(output), &publishing.to_string())?;
    write_output(output, code)?;
    if replaced.is_some() {
        write_output(&record_path(output), &record.to_string())?;
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStrExt;

    #[test]
    fn non_unicode_owner_is_rejected_without_a_file() {
        let dir = crate::test_workspace::Workspace::new("non-unicode-owner");
        let path = dir.join(std::ffi::OsStr::from_bytes(b"bad\xff.tt"));
        let error = recorded_source(&path).unwrap_err();
        assert!(error.contains("input path is not valid UTF-8"));
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    }
}
