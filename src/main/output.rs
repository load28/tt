//! Source-map rendering, output writing, and file-watch dependency expansion.

use super::*;

/// A built map, ready to attach: the `//# sourceMappingURL=` line the
/// output ends with, and the document to write beside it (`None` when the
/// comment carries the map itself).
pub(super) struct RenderedSourceMap {
    pub(super) comment: String,
    pub(super) document: Option<String>,
}

/// The map file's path: the output's, with `.map` appended — which is what
/// the relative URL in the comment names.
pub(super) fn map_path(out_path: &Path) -> PathBuf {
    let mut name = out_path.as_os_str().to_os_string();
    name.push(".map");
    PathBuf::from(name)
}

/// Builds one file's source map.
///
/// The map names the `.tt` source relative to the map file, so a debugger
/// resolves it the way it resolves any map beside its output; the source
/// text is embedded as well, so a consumer that cannot reach the path (a
/// bundle, a `data:` URL) still shows the original.
pub(super) fn source_map_for(
    job: &Job,
    emit: &ttc::MappedEmit,
    source: &str,
    banner: BannerPlacement,
    mode: SourceMapMode,
) -> RenderedSourceMap {
    let out_name = job
        .out_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let map_file = map_path(&job.out_path);
    let source_name = relative_to(&job.file, map_file.parent().unwrap_or(Path::new(".")));
    let map = emit.source_map(
        source,
        &SourceMapRequest {
            file: out_name.as_deref(),
            source: &source_name,
            embed_source: true,
            generated_line_offset: banner.lines,
            generated_line_offset_at: banner.at_line,
        },
    );
    match mode {
        SourceMapMode::Inline | SourceMapMode::Off => RenderedSourceMap {
            comment: ttc::source_map::SourceMap::url_comment(&map.to_data_url()),
            document: None,
        },
        SourceMapMode::File => RenderedSourceMap {
            comment: ttc::source_map::SourceMap::url_comment(&format!(
                "{}.map",
                out_name.as_deref().unwrap_or("output")
            )),
            document: Some(map.to_json()),
        },
    }
}

/// `path` as seen from the directory `base`, as a `/`-separated URL — how
/// a map beside its output names its source.
///
/// The answer is computed from the paths themselves, never from the
/// filesystem: the output directory usually does not exist yet when the map
/// is built, and a map whose `sources` depended on that would name the file
/// correctly or incorrectly depending on whether this is a first build.
pub(super) fn relative_to(path: &Path, base: &Path) -> String {
    let path = lexical_absolute(path);
    let base = lexical_absolute(base);
    let shared = path
        .iter()
        .zip(base.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut out = String::new();
    for _ in 0..base.len() - shared {
        out.push_str("../");
    }
    out.push_str(&path[shared..].join("/"));
    if out.is_empty() { path.join("/") } else { out }
}

/// A path's components against the current directory, with `.` and `..`
/// folded away. Purely lexical — it neither reads the filesystem nor
/// resolves symlinks, which is the model a source map's `sources` uses.
pub(super) fn lexical_absolute(path: &Path) -> Vec<String> {
    normalized_absolute(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect()
}

pub(super) fn normalized_absolute(path: &Path) -> PathBuf {
    use std::path::Component;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .expect("current directory is available")
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

/// Writes one output file, whole or not at all.
///
/// A build that fails partway must not leave a half-written file where a
/// good one was — that is what `main`'s panic path already promises about
/// every file a run wrote. Writing in place cannot keep that promise: the
/// open truncates, so a failure between the truncation and the last byte
/// (a full disk, a signal) publishes the truncated file. The bytes go to a
/// sibling temporary first and the rename replaces the target in one step,
/// so a reader sees the previous file or the new one, never a prefix.
pub(super) fn write_output(out_path: &Path, code: &str) -> Result<(), String> {
    if let Some(parent) = out_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return Err(format!("ttc: {}: {e}", parent.display()));
    }
    replace_file(out_path, code.as_bytes()).map_err(|e| format!("ttc: {}: {e}", out_path.display()))
}

/// Publishes bytes through an exclusively owned sibling staging file.
/// Exclusive creation handles concurrent writers and stale files from old runs.
pub(super) fn replace_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_STAGING: AtomicU64 = AtomicU64::new(0);
    let (staging, mut file) = loop {
        let sequence = NEXT_STAGING.fetch_add(1, Ordering::Relaxed);
        let mut name = path.as_os_str().to_os_string();
        name.push(format!(".{}.{sequence}.tmp", std::process::id()));
        let staging = PathBuf::from(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
        {
            Ok(file) => break (staging, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = file.write_all(bytes);
    drop(file);
    let result = result.and_then(|()| fs::rename(&staging, path));
    if result.is_err() {
        let _ = fs::remove_file(&staging);
    }
    result
}

/// How often `--watch` re-reads the inputs' timestamps.
pub(crate) const WATCH_INTERVAL: Duration = Duration::from_millis(300);

/// `--watch`: compile once, then keep compiling what changes.
///
/// Inputs are re-expanded every round, so files added to a watched directory
/// are picked up. A changed file drags its **dependents** along: a `.tt` that
/// imports it is checked against the new declarations, which is what makes
/// project-wide exhaustiveness errors appear on the importing side.
///
/// Runs until interrupted; the exit code is only reached on a fatal input
/// error.
pub(super) fn watch_mode(
    inputs: &[String],
    out_dir: Option<&Path>,
    opts: &BuildOptions,
) -> ExitCode {
    let mut stamps: HashMap<PathBuf, SystemTime> = HashMap::new();
    let mut first = true;
    let mut input_error = None;

    loop {
        let jobs = match build_jobs(inputs, out_dir, true) {
            Ok(jobs) => {
                input_error = None;
                jobs
            }
            // An input can disappear mid-edit; keep watching rather than
            // tearing the session down.
            Err(error) => {
                if input_error.as_ref() != Some(&error) {
                    eprintln!("{error}");
                    input_error = Some(error);
                }
                thread::sleep(WATCH_INTERVAL);
                continue;
            }
        };

        let current: HashMap<PathBuf, SystemTime> = jobs
            .iter()
            .map(|job| {
                let stamp = fs::metadata(&job.file)
                    .and_then(|meta| meta.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                (job.file.clone(), stamp)
            })
            .collect();

        let changed: Vec<PathBuf> = if first {
            jobs.iter().map(|job| job.file.clone()).collect()
        } else {
            current
                .iter()
                .filter(|(file, stamp)| stamps.get(*file) != Some(stamp))
                .map(|(file, _)| file.clone())
                .collect()
        };

        if !changed.is_empty() {
            let targets = with_dependents(&jobs, &changed);
            let selected: Vec<Job> = jobs
                .iter()
                .filter(|job| targets.contains(&job.file))
                .cloned()
                .collect();
            let failed = compile_jobs(&selected, opts);
            // The count is what was rebuilt; only the word after it says
            // how the round went, so "failed" must not borrow it.
            eprintln!(
                "ttc: {} file(s) {} — watching",
                selected.len(),
                if failed { "rebuilt, with errors" } else { "ok" }
            );
        }

        if first {
            eprintln!("ttc: watching {} file(s) — Ctrl-C to stop", jobs.len());
            first = false;
        }
        stamps = current;
        thread::sleep(WATCH_INTERVAL);
    }
}

/// Default `-o` of `--types` — where the sidecars land.
pub(super) const TYPES_DIR: &str = ".tt-types";

/// The file's path relative to whichever input directory contains it, so
/// the sidecar tree mirrors the source tree rather than the whole cwd.
pub(super) fn input_relative(file: &Path, inputs: &[String]) -> PathBuf {
    for input in inputs {
        let root = Path::new(input);
        if root.is_dir()
            && let Ok(relative) = file.strip_prefix(root)
        {
            return relative.to_path_buf();
        }
    }
    PathBuf::from(file.file_name().unwrap_or_default())
}

/// The changed files plus every job that imports one of them.
pub(super) fn with_dependents(jobs: &[Job], changed: &[PathBuf]) -> HashSet<PathBuf> {
    let mut targets: HashSet<PathBuf> = changed.iter().cloned().collect();
    let changed_real: HashSet<PathBuf> = changed
        .iter()
        .filter_map(|file| file.canonicalize().ok())
        .collect();

    for job in jobs {
        if targets.contains(&job.file) {
            continue;
        }
        let Ok(source) = fs::read_to_string(&job.file) else {
            continue;
        };
        let dir = job.file.parent().unwrap_or(Path::new("."));
        let imports_changed = ttc::tt_imports(&source).iter().any(|import| {
            dir.join(&import.specifier)
                .canonicalize()
                .is_ok_and(|target| changed_real.contains(&target))
        });
        if imports_changed {
            targets.insert(job.file.clone());
        }
    }
    targets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_replacements_publish_complete_files_and_remove_staging() {
        let dir = std::env::temp_dir().join(format!("tt-output-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.ts");
        let barrier = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            for byte in b'a'..=b'h' {
                let path = &path;
                let barrier = &barrier;
                scope.spawn(move || {
                    let bytes = vec![byte; 65536];
                    barrier.wait();
                    replace_file(path, &bytes).unwrap();
                });
            }
        });
        let output = fs::read(&path).unwrap();
        assert_eq!(output.len(), 65536);
        assert!(output.iter().all(|byte| *byte == output[0]));
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
