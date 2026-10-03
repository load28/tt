//! Exact-revision exchange between one language-service session and its mapper.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const ENVIRONMENT: &str = "TTC_SERVICE_PROJECTIONS";
pub const PROTOCOL: u32 = 1;
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct ProjectionRecord {
    pub protocol: u32,
    pub revision: u64,
    pub path: PathBuf,
    pub source: String,
    pub response: serde_json::Value,
}

#[derive(Debug)]
pub struct ProjectionExchange {
    directory: PathBuf,
    records: BTreeMap<String, serde_json::Value>,
}

fn identity(path: &Path) -> Result<String, String> {
    crate::engine::normalize_document_path(path)?
        .into_os_string()
        .into_string()
        .map_err(|_| "projection path is not valid UTF-8".to_string())
}

fn atomic_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    use std::io::Write;
    let temporary = path.with_extension(format!(
        "{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(value.to_string().as_bytes())?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("cannot publish projection exchange: {error}"))
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("cannot read projection exchange: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid projection exchange: {error}"))
}

fn check_protocol(value: &serde_json::Value) -> Result<(), String> {
    if value["protocol"].as_u64() != Some(u64::from(PROTOCOL)) {
        return Err("incompatible content mapper projection protocol (expected version 1)".into());
    }
    Ok(())
}

impl ProjectionExchange {
    pub fn new() -> Result<Self, String> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "tt-service-projections-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&directory)
            .map_err(|error| format!("cannot create projection exchange: {error}"))?;
        let exchange = Self {
            directory,
            records: BTreeMap::new(),
        };
        exchange.flush()?;
        Ok(exchange)
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    fn flush(&self) -> Result<(), String> {
        atomic_json(
            &self.directory.join("projections.json"),
            &serde_json::json!({ "protocol": PROTOCOL, "records": self.records }),
        )
    }

    pub fn publish(&mut self, record: &ProjectionRecord) -> Result<(), String> {
        if record.protocol != PROTOCOL {
            return Err("incompatible projection record".into());
        }
        let key = identity(&record.path)?;
        let previous = self.records.insert(
            key.clone(),
            serde_json::json!({
                "revision": record.revision, "source": record.source, "response": record.response
            }),
        );
        if let Err(error) = self.flush() {
            match previous {
                Some(value) => {
                    self.records.insert(key, value);
                }
                None => {
                    self.records.remove(&key);
                }
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn replace(&mut self, records: &[ProjectionRecord]) -> Result<(), String> {
        let mut next = BTreeMap::new();
        for record in records {
            if record.protocol != PROTOCOL {
                return Err("incompatible projection record".into());
            }
            next.insert(identity(&record.path)?, serde_json::json!({
                "revision": record.revision, "source": record.source, "response": record.response
            }));
        }
        let previous = std::mem::replace(&mut self.records, next);
        if let Err(error) = self.flush() {
            self.records = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn remove(&mut self, path: &Path) -> Result<(), String> {
        let key = identity(path)?;
        let previous = self.records.remove(&key);
        if let Err(error) = self.flush() {
            if let Some(value) = previous {
                self.records.insert(key, value);
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn verify_acknowledged(&self) -> Result<(), String> {
        read_json(&self.directory.join("acknowledged.json"))
            .and_then(|value| check_protocol(&value))
            .map_err(|error| {
                format!(
                    "content mapper did not acknowledge this service's projection protocol: {error}"
                )
            })
    }
}

impl Drop for ProjectionExchange {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

pub fn read_projection(
    directory: &Path,
    path: &Path,
    source: &str,
) -> Result<Option<ProjectionRecord>, String> {
    let manifest = read_json(&directory.join("projections.json"))?;
    check_protocol(&manifest)?;
    let records = manifest["records"]
        .as_object()
        .ok_or("projection exchange records must be an object")?;
    let Some(record) = records.get(&identity(path)?) else {
        return Ok(None);
    };
    let original = record["source"]
        .as_str()
        .ok_or("projection record has no source")?;
    if original != source {
        return Ok(None);
    }
    let revision = record["revision"]
        .as_u64()
        .ok_or("projection record has no revision")?;
    let response = &record["response"];
    if !response["text"].is_string()
        || !response["mappings"].is_array()
        || !matches!(response["extension"].as_str(), Some(".ts" | ".tsx"))
        || !response["diagnostics"].is_array()
    {
        return Err("projection record has an invalid mapper response".into());
    }
    Ok(Some(ProjectionRecord {
        protocol: PROTOCOL,
        revision,
        path: path.to_path_buf(),
        source: original.into(),
        response: response.clone(),
    }))
}

pub struct MapperExchange {
    directory: PathBuf,
}

impl MapperExchange {
    pub fn open(directory: PathBuf) -> Result<Self, String> {
        check_protocol(&read_json(&directory.join("projections.json"))?)?;
        atomic_json(
            &directory.join("acknowledged.json"),
            &serde_json::json!({"protocol": PROTOCOL}),
        )?;
        Ok(Self { directory })
    }

    pub fn from_environment() -> Result<Option<Self>, String> {
        std::env::var_os(ENVIRONMENT)
            .map(|path| Self::open(PathBuf::from(path)))
            .transpose()
    }

    pub fn lookup(&self, path: &Path, source: &str) -> Result<Option<ProjectionRecord>, String> {
        read_projection(&self.directory, path, source)
    }
}

use crate::{Diagnostic, EmitAnchor, EmitMapping};

/// `SpanMapKind.Verbatim` — same length and content in both texts; the
/// only kind edits may be written back through.
pub const SPAN_VERBATIM: u64 = 0;
/// `SpanMapKind.Atom` — a correspondence with different text, used here
/// for compiler-written glue.
pub const SPAN_ATOM: u64 = 1;
/// `SpanMapFeature.None` — the span maps diagnostics (which are not
/// feature-gated) and nothing else.
pub const FEATURES_NONE: u64 = 0;

/// Suggestions ride along in the text: the wire has one string, and "what
/// is wrong" without "what to do about it" would strand the half of the
/// diagnostic ttc keeps separate for editors.
pub fn mapper_diagnostic(diagnostic: &Diagnostic) -> serde_json::Value {
    let start = diagnostic.start.unwrap_or(0);
    let length = diagnostic.end.unwrap_or(start).saturating_sub(start);
    let mut message = diagnostic.message.clone();
    for suggestion in &diagnostic.suggestions {
        message.push_str("\nhelp: ");
        message.push_str(&suggestion.message);
    }
    serde_json::json!({
        "messageText": message,
        "start": start,
        "length": length,
        "code": diagnostic.code.number(),
    })
}

/// The span map of one emission: verbatim chunks as `Verbatim`, glue as
/// `Atom` spans owned by the construct that wrote it.
///
/// A projection that recovered from malformed syntax compiled a copy of the
/// source whose `recovered` byte ranges hold placeholders, so a chunk copied
/// from inside one of them is not the original text there. TypeScript
/// rejects a `Verbatim` span whose two sides differ (TS100029), so those
/// stretches are `Atom` spans owned by the whole recovered range, with no
/// features, like glue.
///
/// Virtual spans must not overlap, and anchors both nest and contain the
/// verbatim chunks of their construct's copied text (a match's arm
/// bodies), so each anchor contributes only the stretches nothing else
/// claimed — innermost first, the same priority [`crate::MappedEmit::anchor_at`]
/// gives a consumer. `Atom` glue spans carry `SpanMapFeature.None`:
/// diagnostics are not feature-gated and land on the construct's own
/// source range, while navigation and rename — which must never resolve
/// into glue — stay off.
pub fn span_mappings(
    mappings: &[EmitMapping],
    anchors: &[EmitAnchor],
    recovered: &[(usize, usize)],
) -> Vec<serde_json::Value> {
    // Occupied intervals of the virtual text, kept sorted by start.
    let mut occupied: Vec<(usize, usize)> =
        mappings.iter().map(|m| (m.out, m.out + m.len)).collect();
    occupied.sort_unstable();

    let mut recovered = recovered.to_vec();
    recovered.sort_unstable();
    let mut spans: Vec<(usize, serde_json::Value)> = Vec::new();
    for mapping in mappings {
        let src_end = mapping.src + mapping.len;
        let mut cursor = mapping.src;
        for &(recovery_start, recovery_end) in &recovered {
            if recovery_end <= cursor || recovery_start >= src_end {
                continue;
            }
            let overlap_start = recovery_start.max(cursor);
            let overlap_end = recovery_end.min(src_end);
            if overlap_start > cursor {
                spans.push(verbatim_span(mapping, cursor, overlap_start));
            }
            let out = mapping.out + (overlap_start - mapping.src);
            spans.push((
                out,
                serde_json::json!([
                    out,
                    overlap_end - overlap_start,
                    recovery_start,
                    recovery_end - recovery_start,
                    SPAN_ATOM,
                    FEATURES_NONE,
                ]),
            ));
            cursor = overlap_end;
        }
        if cursor < src_end || mapping.len == 0 {
            spans.push(verbatim_span(mapping, cursor, src_end));
        }
    }

    for anchor in anchors {
        let original_start = anchor.src;
        let original_length = anchor.src_end.saturating_sub(anchor.src);
        for (start, end) in free_intervals(anchor.out, anchor.end, &occupied) {
            spans.push((
                start,
                serde_json::json!([
                    start,
                    end - start,
                    original_start,
                    original_length,
                    SPAN_ATOM,
                    FEATURES_NONE,
                ]),
            ));
            let position = occupied.partition_point(|&(s, _)| s < start);
            occupied.insert(position, (start, end));
        }
    }

    spans.sort_by_key(|(start, _)| *start);
    spans.into_iter().map(|(_, span)| span).collect()
}

fn verbatim_span(mapping: &EmitMapping, start: usize, end: usize) -> (usize, serde_json::Value) {
    let out = mapping.out + (start - mapping.src);
    let len = end - start;
    (
        out,
        serde_json::json!([out, len, start, len, SPAN_VERBATIM]),
    )
}

/// The stretches of `[start, end)` not covered by any `occupied` interval.
pub fn free_intervals(
    start: usize,
    end: usize,
    occupied: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    let mut free = Vec::new();
    let mut cursor = start;
    for &(taken_start, taken_end) in occupied {
        if taken_end <= cursor {
            continue;
        }
        if taken_start >= end {
            break;
        }
        if taken_start > cursor {
            free.push((cursor, taken_start.min(end)));
        }
        cursor = cursor.max(taken_end);
        if cursor >= end {
            return free;
        }
    }
    if cursor < end {
        free.push((cursor, end));
    }
    free
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(source: &str, revision: u64) -> ProjectionRecord {
        ProjectionRecord {
            protocol: 1,
            revision,
            path: std::env::current_dir().unwrap().join("unsaved.tt"),
            source: source.into(),
            response: serde_json::json!({"text": format!("// revision {revision}"), "extension": ".ts", "mappings": [], "diagnostics": []}),
        }
    }

    #[test]
    fn exchange_requires_exact_source_revision() {
        let mut exchange = ProjectionExchange::new().unwrap();
        let first = record("export const n = 1;", 1);
        exchange.publish(&first).unwrap();
        assert_eq!(
            read_projection(exchange.directory(), &first.path, &first.source)
                .unwrap()
                .unwrap()
                .response,
            first.response
        );
        assert!(
            read_projection(exchange.directory(), &first.path, "export const n = 2;")
                .unwrap()
                .is_none()
        );
        let next = record(&first.source, 2);
        exchange.publish(&next).unwrap();
        assert_eq!(
            read_projection(exchange.directory(), &first.path, &first.source)
                .unwrap()
                .unwrap()
                .revision,
            2
        );
        exchange.remove(&first.path).unwrap();
        assert!(
            read_projection(exchange.directory(), &first.path, &first.source)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn exchanges_are_isolated_and_cleanup_is_owned() {
        let mut a = ProjectionExchange::new().unwrap();
        let mut b = ProjectionExchange::new().unwrap();
        let first = record("same source", 1);
        let second = record("same source", 2);
        a.publish(&first).unwrap();
        b.publish(&second).unwrap();
        let a_directory = a.directory().to_path_buf();
        assert_ne!(a.directory(), b.directory());
        drop(a);
        assert!(!a_directory.exists());
        assert_eq!(
            read_projection(b.directory(), &second.path, &second.source)
                .unwrap()
                .unwrap()
                .revision,
            2
        );
    }

    #[test]
    fn incompatible_and_malformed_exchanges_are_errors() {
        let exchange = ProjectionExchange::new().unwrap();
        let entry = record("source", 1);
        std::fs::write(
            exchange.directory().join("projections.json"),
            r#"{"protocol":2,"records":{}}"#,
        )
        .unwrap();
        assert!(read_projection(exchange.directory(), &entry.path, &entry.source).is_err());
        std::fs::write(exchange.directory().join("projections.json"), "{").unwrap();
        assert!(read_projection(exchange.directory(), &entry.path, &entry.source).is_err());
    }
}
