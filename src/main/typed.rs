//! Typed checking, watch reporting, and declaration emission.

use super::*;

/// `ttc --check-types` / `ttc --types` as an engine consumer: open the
/// project once, take a snapshot per pass, print what the check found. The
/// engine owns the state (documents, projections, the running compiler);
/// this driver owns the terminal — wording, order and exit codes are the
/// CLI's contract.
pub(super) fn typed_check_mode(inputs: &[String], options: &TypedCheckOptions<'_>) -> ExitCode {
    let engine = ttc::engine::Engine::new(options.node.map(Path::to_path_buf));
    let project_options = ttc::engine::ProjectOptions {
        tsconfig: options.project.map(Path::to_path_buf),
        out_dir: options.out_dir.map(Path::to_path_buf),
    };
    let groups = project_groups(inputs, &project_options, options);
    if groups.len() > 1 {
        if options.watch {
            eprintln!(
                "ttc: the inputs belong to {} TypeScript projects; watch one project at a time, \
                 or name one with --project",
                groups.len()
            );
            return ExitCode::FAILURE;
        }
        let mut report = TypedReport::checked();
        let mut printed = HashSet::new();
        for group in &groups {
            report.absorb(typed_check_group(
                &engine,
                group,
                &project_options,
                options,
                &mut printed,
            ));
        }
        if options.json_report {
            crate::out::line(&report.to_json());
        }
        return report.exit_code();
    }
    if options.watch {
        let mut reported = None;
        loop {
            match open_typed_project(&engine, inputs, &project_options, options) {
                Ok(project) => {
                    return typed_watch(&engine, project, inputs, &project_options, options);
                }
                Err(e) => {
                    if reported.as_ref() != Some(&e) {
                        eprintln!("ttc: {e}");
                        reported = Some(e);
                    }
                    thread::sleep(WATCH_INTERVAL);
                }
            }
        }
    }
    let report = match open_typed_project(&engine, inputs, &project_options, options) {
        Ok(mut project) => {
            let mut files = project.initial_files();
            files.extend(
                options
                    .overlay
                    .keys()
                    .filter(|path| ttc::SourceKind::from_tt_path(path).is_some())
                    .cloned(),
            );
            files.sort();
            files.dedup();
            typed_pass(&mut project, &files, options, &mut HashSet::new()).unwrap_or_else(|e| {
                eprintln!("ttc: {e}");
                TypedReport::unchecked(0)
            })
        }
        Err(e) => {
            eprintln!("ttc: {e}");
            TypedReport::unchecked(0)
        }
    };
    if options.json_report {
        crate::out::line(&report.to_json());
    }
    report.exit_code()
}

fn project_groups(
    inputs: &[String],
    project_options: &ttc::engine::ProjectOptions,
    options: &TypedCheckOptions<'_>,
) -> Vec<Vec<String>> {
    if project_options.tsconfig.is_some() || !options.overlay.is_empty() {
        return vec![inputs.to_vec()];
    }
    let mut groups: Vec<(Option<PathBuf>, Vec<String>)> = Vec::new();
    for input in inputs {
        let path = Path::new(input);
        let probe = if path.is_dir() {
            path.join("tsconfig.json")
        } else {
            path.to_path_buf()
        };
        let tsconfig = ttc::engine::Engine::document_project_identity(&probe, project_options)
            .ok()
            .and_then(|(tsconfig, _)| tsconfig);
        match groups.iter_mut().find(|(key, _)| *key == tsconfig) {
            Some((_, group)) => group.push(input.clone()),
            None => groups.push((tsconfig, vec![input.clone()])),
        }
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

fn typed_check_group(
    engine: &ttc::engine::Engine,
    inputs: &[String],
    project_options: &ttc::engine::ProjectOptions,
    options: &TypedCheckOptions<'_>,
    printed: &mut HashSet<String>,
) -> TypedReport {
    match open_typed_project(engine, inputs, project_options, options) {
        Ok(mut project) => {
            let mut files = project.initial_files();
            files.sort();
            files.dedup();
            typed_pass(&mut project, &files, options, printed).unwrap_or_else(|e| {
                eprintln!("ttc: {e}");
                TypedReport::unchecked(0)
            })
        }
        Err(e) => {
            eprintln!("ttc: {e}");
            TypedReport::unchecked(0)
        }
    }
}

/// Opens the project `inputs` belong to, with the overlays standing in for
/// their files.
fn open_typed_project(
    engine: &ttc::engine::Engine,
    inputs: &[String],
    project_options: &ttc::engine::ProjectOptions,
    options: &TypedCheckOptions<'_>,
) -> Result<ttc::engine::Project, String> {
    let unsaved = |input: &String| {
        let path = Path::new(input);
        (!path.exists())
            .then(|| ttc::engine::normalize_document_path(path).ok())
            .flatten()
            .filter(|document| options.overlay.contains_key(document))
    };
    let on_disk: Vec<String> = inputs
        .iter()
        .filter(|input| unsaved(input).is_none())
        .cloned()
        .collect();
    let mut project = match inputs.iter().find_map(unsaved) {
        Some(document) if on_disk.is_empty() => {
            engine.open_document_project(&document, project_options)?
        }
        _ => engine.open_project(&on_disk, project_options)?,
    };
    for (path, text) in options.overlay {
        project.open_document(path.clone(), text.clone());
    }
    Ok(project)
}

/// What the typed modes were asked for, beside their inputs.
pub(super) struct TypedCheckOptions<'a> {
    pub(super) project: Option<&'a Path>,
    pub(super) node: Option<&'a Path>,
    /// Where the sidecars go, in the mode that writes them.
    pub(super) out_dir: Option<&'a Path>,
    /// `--types`: emit declarations and write them. `--check-types` does not.
    pub(super) emit: bool,
    pub(super) watch: bool,
    /// `--overlay`: unsaved text standing in for a file on disk, keyed by
    /// canonical path.
    pub(super) overlay: &'a std::collections::HashMap<PathBuf, String>,
    /// `--tt-only`: report only the tt layer.
    pub(super) tt_only: bool,
    /// The raw inputs, for mirroring their layout under `-o`.
    pub(super) inputs: &'a [String],
    pub(super) json_report: bool,
}

/// What one pass printed.
pub(super) struct TypedReport {
    /// How many diagnostics were printed. Zero is the only passing result.
    reported: usize,
    /// Whether the pass could not run at all — see [`ttc::engine::Blocked`].
    blocked: bool,
    writes: WriteOutcome,
}

impl TypedReport {
    fn unchecked(reported: usize) -> Self {
        Self {
            reported,
            blocked: true,
            writes: WriteOutcome::default(),
        }
    }

    fn checked() -> Self {
        Self {
            reported: 0,
            blocked: false,
            writes: WriteOutcome::default(),
        }
    }

    fn absorb(&mut self, other: TypedReport) {
        self.reported += other.reported;
        self.blocked |= other.blocked;
        self.writes.written.extend(other.writes.written);
        self.writes.failed.extend(other.writes.failed);
    }

    fn exit_code(&self) -> ExitCode {
        if self.blocked {
            ExitCode::from(2)
        } else if !self.writes.failed.is_empty() {
            ExitCode::from(3)
        } else if self.reported == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }

    fn to_json(&self) -> String {
        serde_json::json!({
            "checked": !self.blocked,
            "diagnostics": self.reported,
            "written": self.writes.written.iter().map(|path| path.to_string_lossy()).collect::<Vec<_>>(),
            "failed": self.writes.failed.iter().map(|(path, error)| serde_json::json!({
                "path": path.to_string_lossy(),
                "error": error,
            })).collect::<Vec<_>>(),
        })
        .to_string()
    }
}

#[derive(Default)]
pub(super) struct WriteOutcome {
    pub(super) written: Vec<PathBuf>,
    pub(super) failed: Vec<(PathBuf, String)>,
}

impl WriteOutcome {
    fn record(&mut self, path: &Path, result: std::io::Result<()>) -> bool {
        match result {
            Ok(()) => {
                self.written.push(normalized_absolute(path));
                true
            }
            Err(error) => {
                self.fail(path, error.to_string());
                false
            }
        }
    }

    fn fail(&mut self, path: &Path, error: String) {
        self.failed.push((normalized_absolute(path), error));
    }
}

/// One snapshot, one check, everything printed.
pub(super) fn typed_pass(
    project: &mut ttc::engine::Project,
    files: &[PathBuf],
    options: &TypedCheckOptions<'_>,
    printed: &mut HashSet<String>,
) -> Result<TypedReport, String> {
    let snapshot = match project.update(files) {
        Ok(snapshot) => snapshot,
        Err(blocked) => {
            // No snapshot exists yet, so there is no text to quote: the
            // header and the location are the whole report.
            eprintln!(
                "{}",
                ttc::render::compile_error(&blocked.error, None, &shown(&blocked.path), styles())
            );
            return Ok(TypedReport::unchecked(1));
        }
    };
    let checked = project.check(
        &snapshot,
        &ttc::engine::CheckRequest {
            emit_declarations: options.emit,
            tt_only: options.tt_only,
        },
    )?;

    // The declarations the compiler emitted for the lowered modules, laid
    // out under `-o` the way the sources are laid out under the project.
    let writes = if options.emit && checked.backend_error.is_none() {
        write_declarations(
            &checked.declarations,
            options.inputs,
            options.out_dir,
            project.root(),
        )
    } else {
        WriteOutcome::default()
    };
    for (path, error) in &writes.failed {
        eprintln!("ttc: cannot write {}: {error}", shown(path));
    }

    // The snapshot, not the file on disk: an `--overlay` was checked
    // against text that was never saved, and quoting the disk would draw a
    // caret under a line the compiler did not see.
    let mut disk: HashMap<&Path, Option<String>> = HashMap::new();
    for diagnostic in &checked.diagnostics {
        if snapshot.source_of(&diagnostic.path).is_none() {
            disk.entry(&diagnostic.path).or_insert_with(|| {
                fs::read(&diagnostic.path)
                    .ok()
                    .map(ttc::lines::typescript_text)
            });
        }
    }
    let mut measured: HashMap<&Path, Option<ttc::lines::LineMap<'_>>> = HashMap::new();
    let mut reported = 0;
    for diagnostic in &checked.diagnostics {
        let lines = measured.entry(&diagnostic.path).or_insert_with(|| {
            snapshot
                .source_of(&diagnostic.path)
                .or_else(|| disk.get(diagnostic.path.as_path())?.as_deref())
                .map(ttc::lines::LineMap::ecma)
        });
        let rendered = ttc::render::engine_diagnostic_measured(
            diagnostic,
            lines.as_ref(),
            &shown(&diagnostic.path),
            styles(),
        );
        if printed.insert(rendered.clone()) {
            eprintln!("{rendered}");
            reported += 1;
        }
    }

    // A backend that could not run is the pass failing to *run*, not the
    // code failing the check — the tt diagnostics above are complete, the
    // typed layer is missing, and the exit code says "could not check".
    if let Some(error) = &checked.backend_error {
        if error.kind == ttc::engine::BackendErrorKind::Internal {
            panic!("{}", error.message);
        }
        eprintln!("ttc: {error}");
        eprintln!("ttc: the TypeScript layer did not run — only tt-level diagnostics are shown");
        return Ok(TypedReport {
            reported,
            blocked: true,
            writes,
        });
    }

    Ok(TypedReport {
        reported,
        blocked: false,
        writes,
    })
}

/// Re-checks on every change, against the compiler started for the first
/// pass. The project is opened once and updated after that, which is what
/// makes the wait a re-check rather than a cold start — and the engine's
/// projection cache means only the files that changed are re-lowered.
pub(super) fn typed_watch(
    engine: &ttc::engine::Engine,
    mut project: ttc::engine::Project,
    inputs: &[String],
    project_options: &ttc::engine::ProjectOptions,
    options: &TypedCheckOptions<'_>,
) -> ExitCode {
    let mut stamps: std::collections::HashMap<PathBuf, std::time::SystemTime> =
        std::collections::HashMap::new();
    let mut first = true;
    let mut reopen_error = None;
    loop {
        // A discovered configuration created or deleted since the last pass
        // puts the inputs in another project, as a fresh run would find them.
        let identity = ttc::engine::Engine::project_identity(inputs, project_options);
        if project_options.tsconfig.is_none()
            && let Ok((tsconfig, root)) = &identity
            && (tsconfig.as_deref(), root.as_path()) != project.identity()
        {
            match open_typed_project(engine, inputs, project_options, options) {
                Ok(reopened) => {
                    project = reopened;
                    reopen_error = None;
                }
                Err(e) => {
                    if reopen_error.as_ref() != Some(&e) {
                        eprintln!("ttc: {e}");
                        reopen_error = Some(e);
                    }
                    thread::sleep(WATCH_INTERVAL);
                    continue;
                }
            }
        }
        let project = &mut project;
        let files = match project.scan() {
            Ok(files) => files,
            // A file can disappear mid-edit; keep watching rather than
            // tearing the session down.
            Err(_) => {
                thread::sleep(WATCH_INTERVAL);
                continue;
            }
        };
        let watch_paths = project.watch_paths().unwrap_or_else(|_| files.clone());
        let mut current: std::collections::HashMap<PathBuf, std::time::SystemTime> = watch_paths
            .iter()
            .map(|file| {
                let stamp = fs::metadata(file)
                    .and_then(|meta| meta.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                (file.clone(), stamp)
            })
            .collect();

        if first || current != stamps {
            let started = std::time::Instant::now();
            match typed_pass(project, &files, options, &mut HashSet::new()) {
                Ok(report) if report.writes.failed.is_empty() => eprintln!(
                    "ttc: {} file(s), {} reported in {} ms — watching",
                    files.len(),
                    report.reported,
                    started.elapsed().as_millis()
                ),
                Ok(report) => eprintln!(
                    "ttc: {} file(s), {} reported, {} not written in {} ms — watching",
                    files.len(),
                    report.reported,
                    report.writes.failed.len(),
                    started.elapsed().as_millis()
                ),
                Err(e) => eprintln!("ttc: {e}"),
            }
        }
        // Establish baselines for dependencies discovered by this check.
        // Retain pre-check stamps for existing inputs so edits during checking
        // still trigger the next pass.
        if let Ok(paths) = project.watch_paths() {
            for file in paths {
                current.entry(file.clone()).or_insert_with(|| {
                    fs::metadata(file)
                        .and_then(|meta| meta.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                });
            }
        }
        if first {
            eprintln!("ttc: watching {} file(s) — Ctrl-C to stop", files.len());
            first = false;
        }
        stamps = current;
        thread::sleep(WATCH_INTERVAL);
    }
}

/// Writes the emitted declarations under `out_dir`, mirroring their layout
/// under the project root — never beside the sources.
///
/// A declaration emitted for a lowered module becomes an **editor sidecar**:
/// `src/token.tt.d.ts` plus a `.d.ts.map` whose `sources` is the `.tt` file,
/// so "go to definition" lands in what the user wrote rather than in a
/// declaration. The body is the compiler's; only the map is ttc's, and it is
/// built by the same [`ttc::build_sidecar`] the `--sidecar` mode uses.
pub(super) fn write_declarations(
    declarations: &ttc::engine::Declarations,
    inputs: &[String],
    out_dir: Option<&Path>,
    root: &Path,
) -> WriteOutcome {
    let mut outcome = WriteOutcome::default();
    let std_dir = out_dir.unwrap_or(root).join("tt");
    let std_files: Vec<_> = declarations
        .std
        .iter()
        .map(|declaration| {
            (
                std_dir
                    .join(declaration.module.file_name())
                    .with_extension("d.ts"),
                declaration.module,
                declaration.text.as_str(),
            )
        })
        .collect();
    let targets: Vec<_> = declarations
        .modules
        .iter()
        .map(|declaration| {
            let source = &declaration.file.source_path;
            let name = format!(
                "{}.d.ts",
                source.file_name().unwrap_or_default().to_string_lossy()
            );
            match out_dir {
                Some(dir) => dir
                    .join(input_relative(source, inputs))
                    .with_file_name(name),
                None => source.with_file_name(name),
            }
        })
        .collect();
    let mut claims = HashMap::new();
    let collision = declarations
        .modules
        .iter()
        .zip(&targets)
        .find_map(|(declaration, target)| {
            claims
                .insert(normalized_absolute(target), &declaration.file.source_path)
                .map(|previous| {
                    format!(
                        "multiple declaration inputs claim this output: {} and {}",
                        previous.display(),
                        declaration.file.source_path.display()
                    )
                })
        });
    if let Some(error) = collision {
        // Every planned file fails once. The plan is of files, and colliding
        // declarations are two claims on one of them.
        let mut planned = HashSet::new();
        let files = std_files.iter().map(|(path, ..)| path.clone()).chain(
            targets
                .iter()
                .flat_map(|target| [target.clone(), target.with_extension("ts.map")]),
        );
        for path in files {
            if planned.insert(normalized_absolute(&path)) {
                outcome.fail(&path, error.clone());
            }
        }
        return outcome;
    }
    // Standard-library declarations mirror the generated `tt/` package, so
    // plain tsc can map the root and wildcard `@tt/std` entries to them.
    for (path, module, text) in &std_files {
        outcome.record(
            path,
            super::output::create_dir_all(&std_dir).and_then(|()| {
                super::ownership::write_owned_output(
                    path,
                    super::ownership::OutputOwner::SupportDeclaration(*module),
                    text,
                )
                .map_err(std::io::Error::other)
            }),
        );
    }
    for (declaration, target) in declarations.modules.iter().zip(targets) {
        let file = &declaration.file;
        let dir = target.parent().unwrap_or(Path::new(".")).to_path_buf();
        let map = target.with_extension("ts.map");
        let created = super::output::create_dir_all(&dir);
        let sidecar = ttc::build_sidecar(
            &file.source,
            &declaration.text,
            &relative_path(&dir, &file.source_path),
        );
        let owned = |path: &Path, code: &str| {
            super::ownership::write_owned_output(
                path,
                super::ownership::OutputOwner::Source(&file.source_path),
                code,
            )
            .map_err(std::io::Error::other)
        };
        let declared = outcome.record(
            &target,
            created.and_then(|()| owned(&target, &sidecar.declarations)),
        );
        if declared {
            outcome.record(&map, owned(&map, &sidecar.map));
        } else {
            outcome.fail(
                &map,
                format!("not written because {} was not written", target.display()),
            );
        }
    }
    outcome
}

/// A path as a diagnostic should name it: relative to the directory the
/// command was run in, when it is under it.
///
/// The compiler resolves modules by absolute path, so that is what comes
/// back — but `ttc: /tmp/build-42/src/a.tt:3:1: ...` is not what the other
/// modes print, and not what an editor's problem matcher expects.
pub(super) fn shown(path: &Path) -> String {
    let Ok(cwd) = std::env::current_dir() else {
        return path.display().to_string();
    };
    if let Ok(relative) = path.strip_prefix(&cwd) {
        return relative.display().to_string();
    }
    let same_root = cwd.components().next() == path.components().next();
    if path.is_absolute() && same_root {
        return relative_path(&cwd, path);
    }
    path.display().to_string()
}

/// What diagnostics are painted with, decided once for the process.
///
/// Every diagnostic this binary prints goes to stderr, so one question
/// settles it: is stderr a terminal, and does the reader want colour
/// there ([`ttc::render::Styles::for_stderr`]). Deciding once also means a
/// parallel job's report cannot be painted differently from the one before
/// it.
pub(super) fn styles() -> ttc::render::Styles {
    static STYLES: std::sync::OnceLock<ttc::render::Styles> = std::sync::OnceLock::new();
    *STYLES.get_or_init(ttc::render::Styles::for_stderr)
}
