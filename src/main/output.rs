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
    line_ending: &str,
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
            source_kind: ttc::SourceKind::from_path(&job.file).unwrap_or_default(),
        },
    );
    let (url, document) = match mode {
        SourceMapMode::Inline | SourceMapMode::Off => (map.to_data_url(), None),
        SourceMapMode::File => (
            ttc::source_map::url_path([
                format!("{}.map", out_name.as_deref().unwrap_or("output")).as_str()
            ]),
            Some(map.to_json()),
        ),
    };
    let comment = ttc::source_map::SourceMap::url_comment(&url);
    RenderedSourceMap {
        comment: format!("{}{line_ending}", comment.trim_end_matches('\n')),
        document,
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
    let segments = std::iter::repeat_n("..", base.len() - shared)
        .chain(path[shared..].iter().map(String::as_str));
    let out = ttc::source_map::url_path(segments);
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
    write_output_bytes(out_path, code.as_bytes())
}

pub(super) fn write_output_bytes(out_path: &Path, code: &[u8]) -> Result<(), String> {
    if let Some(parent) = out_path.parent()
        && let Err(e) = create_dir_all(parent)
    {
        return Err(format!("ttc: {}: {e}", out_path.display()));
    }
    replace_file(out_path, code).map_err(|e| format!("ttc: {}: {e}", out_path.display()))
}

pub(super) fn create_dir_all(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir).map_err(|error| {
        if !matches!(
            error.kind(),
            std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotADirectory
        ) {
            return error;
        }
        match dir
            .ancestors()
            .find(|ancestor| fs::metadata(ancestor).is_ok_and(|meta| !meta.is_dir()))
        {
            Some(file) => std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                format!("{}: not a directory", file.display()),
            ),
            None => error,
        }
    })
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
/// Every round places compiler support modules by the [`support_root`] of
/// the whole input set, as a one-shot build of it would, however few files
/// the round recompiles. When that root moves — an input added or removed
/// outside it — every output refers to the support modules anew, so the
/// round recompiles every job.
///
/// Runs until interrupted; the exit code is only reached on a fatal input
/// error.
pub(super) fn watch_mode(
    inputs: &[String],
    out_dir: Option<&Path>,
    project: Option<&Path>,
    opts: &BuildOptions,
) -> ExitCode {
    let mut stamps: HashMap<PathBuf, SystemTime> = HashMap::new();
    let mut reads: HashMap<PathBuf, (SystemTime, Vec<PathBuf>)> = HashMap::new();
    let mut placed: Option<PathBuf> = None;
    let mut configured: Option<JsxPreserve> = None;
    let mut first = true;
    let mut input_error = None;

    loop {
        let round = build_jobs(inputs, out_dir, true).and_then(|jobs| {
            if opts.print && jobs.len() != 1 {
                return Err("ttc: --print requires exactly one source file".to_string());
            }
            let files: Vec<PathBuf> = jobs.iter().map(|job| job.file.clone()).collect();
            let jsx_preserve = project_jsx_preserve(opts.rewrite_imports, &files, project)
                .map_err(|error| format!("ttc: {error}"))?;
            Ok((jobs, jsx_preserve))
        });
        let (jobs, jsx_preserve) = match round {
            Ok(round) => {
                if input_error.take().is_some() {
                    configured = None;
                }
                round
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
        let round_opts = BuildOptions {
            jsx_preserve: jsx_preserve.clone(),
            ..opts.clone()
        };
        let opts = &round_opts;

        let stamp = |file: &Path| {
            fs::metadata(file)
                .and_then(|meta| meta.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        };
        let mut current: HashMap<PathBuf, SystemTime> = HashMap::new();
        for job in &jobs {
            let job_stamp = stamp(&job.file);
            let job_reads = match reads.get(&job.file) {
                Some((read_at, files)) if *read_at == job_stamp => files.clone(),
                _ => compile_reads(&job.file),
            };
            for file in &job_reads {
                current.insert(file.clone(), stamp(file));
            }
            reads.insert(job.file.clone(), (job_stamp, job_reads));
        }
        reads.retain(|file, _| current.contains_key(file));

        let root = support_root(&jobs, out_dir);
        let moved = root != placed;
        let reconfigured = configured.as_ref().is_none_or(|configured| {
            jsx_preserve
                .iter()
                .any(|(file, value)| configured.get(file).is_some_and(|before| before != value))
        });
        let changed: Vec<PathBuf> = if first || moved || reconfigured {
            jobs.iter().map(|job| job.file.clone()).collect()
        } else {
            current
                .iter()
                .filter(|(file, stamp)| stamps.get(*file) != Some(stamp))
                .map(|(file, _)| file.clone())
                .chain(
                    stamps
                        .keys()
                        .filter(|file| !current.contains_key(*file))
                        .cloned(),
                )
                .collect()
        };

        let selected: Vec<Job> = if changed.is_empty() {
            Vec::new()
        } else {
            let targets = with_dependents(&jobs, &changed);
            jobs.iter()
                .filter(|job| targets.contains(&job.file))
                .cloned()
                .collect()
        };
        if !selected.is_empty() {
            let failed = compile_jobs(&selected, root.as_deref(), opts);
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
        placed = root;
        configured = Some(jsx_preserve);
        thread::sleep(WATCH_INTERVAL);
    }
}

/// Default `-o` of `--types` — where the sidecars land.
pub(super) const TYPES_DIR: &str = ".tt-types";

/// The file's path relative to whichever input directory contains it, so
/// the sidecar tree mirrors the source tree rather than the whole cwd.
pub(super) fn input_relative(file: &Path, inputs: &[String]) -> PathBuf {
    let file = normalized_absolute(file);
    if let Some(root) = input_root(inputs)
        && let Ok(relative) = file.strip_prefix(root)
    {
        return relative.to_path_buf();
    }
    PathBuf::from(file.file_name().unwrap_or_default())
}

/// The changed files plus every job that imports one of them.
pub(super) fn with_dependents(jobs: &[Job], changed: &[PathBuf]) -> HashSet<PathBuf> {
    let mut targets: HashSet<PathBuf> = changed.iter().cloned().collect();
    let identity = |path: &Path| {
        ttc::engine::normalize_document_path(path).unwrap_or_else(|_| normalized_absolute(path))
    };
    let changed_real: HashSet<PathBuf> = changed.iter().map(|file| identity(file)).collect();

    for job in jobs {
        if targets.contains(&job.file) {
            continue;
        }
        let Ok(source) = fs::read_to_string(&job.file) else {
            continue;
        };
        let dir = job.file.parent().unwrap_or(Path::new("."));
        let imports_changed = ttc::tt_imports(&source)
            .iter()
            .any(|import| changed_real.contains(&identity(&dir.join(import.path()))));
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
        let dir = crate::test_workspace::Workspace::new("output");
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
    }
}
