//! Typed checking, watch reporting, and declaration emission.

use super::*;

/// `ttc --check-types` / `ttc --types` as an engine consumer: open the
/// project once, take a snapshot per pass, print what the check found. The
/// engine owns the state (documents, projections, the running compiler);
/// this driver owns the terminal — wording, order and exit codes are the
/// CLI's contract.
pub(super) fn typed_check_mode(inputs: &[String], options: &TypedCheckOptions<'_>) -> ExitCode {
    let engine = ttc::engine::Engine::new(options.node.map(Path::to_path_buf));
    let mut project = match engine.open_project(
        inputs,
        &ttc::engine::ProjectOptions {
            tsconfig: options.project.map(Path::to_path_buf),
            out_dir: options.out_dir.map(Path::to_path_buf),
        },
    ) {
        Ok(project) => project,
        Err(e) => {
            eprintln!("ttc: {e}");
            return ExitCode::FAILURE;
        }
    };
    for (path, text) in options.overlay {
        project.open_document(path.clone(), text.clone());
    }

    if options.watch {
        return typed_watch(&mut project, options);
    }

    let files = project.initial_files();
    match typed_pass(&mut project, &files, options) {
        // The exit code answers "did the check pass?", in every mode — a
        // `--types` run still *writes* its sidecars when the code has type
        // errors (a stale sidecar is worse than one built from erroring
        // code), but it says so.
        //
        // 2 is the third answer: the check could not run, so nothing was
        // written. A caller holding a previous result — an editor showing
        // the last good sidecar — keeps it on 2 and replaces it on 1.
        Ok(report) if report.blocked => ExitCode::from(2),
        Ok(report) if report.reported == 0 => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("ttc: {e}");
            ExitCode::FAILURE
        }
    }
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
}

/// What one pass printed.
pub(super) struct TypedReport {
    /// How many diagnostics were printed. Zero is the only passing result.
    reported: usize,
    /// Whether the pass could not run at all — see [`ttc::engine::Blocked`].
    blocked: bool,
}

/// One snapshot, one check, everything printed.
pub(super) fn typed_pass(
    project: &mut ttc::engine::Project,
    files: &[PathBuf],
    options: &TypedCheckOptions<'_>,
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
            return Ok(TypedReport {
                reported: 1,
                blocked: true,
            });
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
    if options.emit && checked.backend_error.is_none() {
        write_declarations(
            &checked.declarations,
            options.inputs,
            options.out_dir,
            project.root(),
        )
        .map_err(|e| e.to_string())?;
    }

    // The snapshot, not the file on disk: an `--overlay` was checked
    // against text that was never saved, and quoting the disk would draw a
    // caret under a line the compiler did not see.
    for diagnostic in &checked.diagnostics {
        eprintln!(
            "{}",
            ttc::render::engine_diagnostic(
                diagnostic,
                snapshot.source_of(&diagnostic.path),
                &shown(&diagnostic.path),
                styles(),
            )
        );
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
            reported: checked.diagnostics.len().max(1),
            blocked: true,
        });
    }

    Ok(TypedReport {
        reported: checked.diagnostics.len(),
        blocked: false,
    })
}

/// Re-checks on every change, against the compiler started for the first
/// pass. The project is opened once and updated after that, which is what
/// makes the wait a re-check rather than a cold start — and the engine's
/// projection cache means only the files that changed are re-lowered.
pub(super) fn typed_watch(
    project: &mut ttc::engine::Project,
    options: &TypedCheckOptions<'_>,
) -> ExitCode {
    let mut stamps: std::collections::HashMap<PathBuf, std::time::SystemTime> =
        std::collections::HashMap::new();
    let mut first = true;
    loop {
        let files = match project.scan() {
            Ok(files) => files,
            // A file can disappear mid-edit; keep watching rather than
            // tearing the session down.
            Err(_) => {
                thread::sleep(WATCH_INTERVAL);
                continue;
            }
        };
        let current: std::collections::HashMap<PathBuf, std::time::SystemTime> = files
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
            match typed_pass(project, &files, options) {
                Ok(report) => eprintln!(
                    "ttc: {} file(s), {} reported in {} ms — watching",
                    files.len(),
                    report.reported,
                    started.elapsed().as_millis()
                ),
                Err(e) => eprintln!("ttc: {e}"),
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
) -> std::io::Result<()> {
    // Standard-library declarations mirror the generated `tt/` package, so
    // plain tsc can map the root and wildcard `@tt/std` entries to them.
    if !declarations.std.is_empty() {
        let dir = out_dir.unwrap_or(root);
        let std_dir = dir.join("tt");
        fs::create_dir_all(&std_dir)?;
        for declaration in &declarations.std {
            fs::write(
                std_dir
                    .join(declaration.module.file_name())
                    .with_extension("d.ts"),
                &declaration.text,
            )?;
        }
    }
    for declaration in &declarations.modules {
        let file = &declaration.file;
        // The same placement `--types` and `--sidecar` use: beside the
        // source, or mirroring the input layout under `-o`.
        let name = format!(
            "{}.d.ts",
            file.source_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );
        let target = match out_dir {
            Some(dir) => dir
                .join(input_relative(&file.source_path, inputs))
                .with_file_name(name),
            None => file.source_path.with_file_name(name),
        };
        let dir = target.parent().unwrap_or(Path::new(".")).to_path_buf();
        fs::create_dir_all(&dir)?;

        let sidecar = ttc::build_sidecar(
            &file.source,
            &declaration.text,
            &relative_path(&dir, &file.source_path),
        );
        fs::write(&target, &sidecar.declarations)?;
        fs::write(target.with_extension("ts.map"), &sidecar.map)?;
    }
    Ok(())
}

/// A path as a diagnostic should name it: relative to the directory the
/// command was run in, when it is under it.
///
/// The compiler resolves modules by absolute path, so that is what comes
/// back — but `ttc: /tmp/build-42/src/a.tt:3:1: ...` is not what the other
/// modes print, and not what an editor's problem matcher expects.
pub(super) fn shown(path: &Path) -> String {
    let relative = std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf));
    relative
        .unwrap_or_else(|| path.to_path_buf())
        .display()
        .to_string()
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
