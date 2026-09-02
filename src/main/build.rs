//! Build planning, standard-library placement, and parallel compilation.

use super::*;

/// Everything the compile step needs beyond the file list.
/// `--source-map`: whether the build writes a Source Map v3 for each
/// compiled file, and where it goes.
///
/// The default is [`SourceMapMode::Off`]. Emitting a map appends a
/// `//# sourceMappingURL=` line to the output, and a hand-written `.ts`
/// passes through byte for byte by contract — so a map is something a
/// build asks for, never something it gets by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum SourceMapMode {
    #[default]
    Off,
    /// A `<output>.map` beside the output, named by a relative URL.
    File,
    /// A `data:` URL carrying the map, so the output is self-contained —
    /// what a bundler plugin reading `ttc -p` needs.
    Inline,
}

pub(super) struct BuildOptions {
    pub(super) banner: bool,
    pub(super) print: bool,
    pub(super) check: bool,
    pub(super) verify: bool,
    pub(super) rewrite_imports: ImportRewrite,
    pub(super) source_map: SourceMapMode,
    /// Output root, when `-o` was given — also where the standard library
    /// module is written if an input imports it.
    pub(super) out_dir: Option<PathBuf>,
    /// Worker threads for the parallel phases (`--jobs`); `None` means one
    /// per available core.
    pub(super) jobs: Option<usize>,
}

/// Where the generated `tt/` standard-library package goes.
pub(super) fn std_placement(jobs: &[Job], needed: bool, out_dir: Option<&Path>) -> Option<PathBuf> {
    if !needed {
        return None;
    }
    let dir = match out_dir {
        Some(dir) => dir.to_path_buf(),
        None => common_ancestor(jobs)?,
    };
    Some(dir.join("tt"))
}

/// The deepest directory every output shares.
pub(super) fn common_ancestor(jobs: &[Job]) -> Option<PathBuf> {
    let mut dirs = jobs.iter().map(|job| {
        job.out_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf()
    });
    let first = dirs.next()?;
    Some(dirs.fold(first, |acc, dir| {
        let shared: PathBuf = acc
            .components()
            .zip(dir.components())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a.as_os_str())
            .collect();
        if shared.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            shared
        }
    }))
}

/// How one output refers to one generated standard-library module.
pub(super) fn std_specifier(
    job: &Job,
    std_dir: &Path,
    rewrite: ImportRewrite,
    module: StdModule,
) -> Option<String> {
    let extension = match rewrite {
        ImportRewrite::Js => "js",
        ImportRewrite::Ts => "ts",
        ImportRewrite::Off => return None,
    };
    let job_dir = job.out_path.parent().unwrap_or(Path::new("."));
    let rel = relative_path(job_dir, std_dir);
    let stem = Path::new(module.file_name())
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let name = format!("{stem}.{extension}");
    Some(if rel == "." {
        format!("./{name}")
    } else if rel.starts_with('.') {
        format!("{rel}/{name}")
    } else {
        format!("./{rel}/{name}")
    })
}

/// Expands the command line's inputs into one job per source file. `.tt` and
/// `.ttx` files compile to `.ts` and `.tsx`; hand-written TypeScript/TSX
/// (collected when `include_ts` is set) keeps its file name and passes
/// through with its `.tt` import specifiers rewritten.
pub(super) fn build_jobs(
    inputs: &[String],
    out_dir: Option<&Path>,
    include_ts: bool,
) -> Result<Vec<Job>, ExitCode> {
    let mut jobs: Vec<Job> = Vec::new();
    for input in inputs {
        let input_path = Path::new(input);
        if !input_path.exists() {
            eprintln!("ttc: no such file or directory: {input}");
            return Err(ExitCode::FAILURE);
        }
        let is_dir = input_path.is_dir();
        let mut files = Vec::new();
        if let Err(e) = collect_sources(input_path, include_ts, &mut files) {
            eprintln!("ttc: {input}: {e}");
            return Err(ExitCode::FAILURE);
        }
        for file in files {
            let out_name = if let Some(kind) = ttc::SourceKind::from_tt_path(&file) {
                file.with_extension(kind.output_extension())
            } else {
                file.clone()
            };
            let out_path = match out_dir {
                Some(dir) => {
                    let rel = if is_dir {
                        out_name
                            .strip_prefix(input_path)
                            .unwrap_or(&out_name)
                            .to_path_buf()
                    } else {
                        // A named input is a file, so it has a file name.
                        // A path is still user input and this is the CLI,
                        // so an odd shape gets the whole path rather than a
                        // crash (TASK-221).
                        out_name
                            .file_name()
                            .map_or_else(|| out_name.clone(), PathBuf::from)
                    };
                    dir.join(rel)
                }
                None => out_name,
            };
            jobs.push(Job { file, out_path });
        }
    }
    Ok(jobs)
}

/// Whether two paths name the same file. The output side may not exist
/// yet, so the parents are compared canonically and the file names
/// literally.
pub(super) fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    if a.file_name() != b.file_name() {
        return false;
    }
    let canon = |p: &Path| p.parent().unwrap_or(Path::new(".")).canonicalize();
    matches!((canon(a), canon(b)), (Ok(x), Ok(y)) if x == y)
}

/// What compiling one job produced: the diagnostics it wants printed (in
/// job order), whether it failed, and any text the parent still has to hand
/// out — stdout under `-p`, or an output file whose path more than one job
/// claims, which only the parent can serialize deterministically.
#[derive(Default)]
pub(super) struct Outcome {
    messages: Vec<String>,
    failed: bool,
    pending: Option<String>,
}

/// Compiles every job. Returns true if any of them failed.
///
/// The run is staged so each input is touched once: read and scanned in
/// parallel, then compiled in parallel against a shared table of imported
/// declarations. Diagnostics are collected per job and printed in job
/// order, so the output of a parallel run is identical to a sequential one.
pub(super) fn compile_jobs(jobs: &[Job], opts: &BuildOptions) -> bool {
    let mut failed = false;
    let loaded = load_jobs(jobs, opts.jobs);

    // Compiler-owned support modules are written once for the project, not
    // once per source file. Standard-library imports materialize its three
    // public modules; a pipeline materializes only the private runtime.
    let needs_std = loaded
        .iter()
        .any(|loaded| loaded.as_ref().is_ok_and(|loaded| loaded.scan.imports_std));
    let needs_runtime = loaded.iter().any(|loaded| {
        loaded
            .as_ref()
            .is_ok_and(|loaded| loaded.scan.uses_pipeline)
    });
    let modules: Vec<_> = StdModule::ALL
        .into_iter()
        .filter(|module| match module {
            StdModule::Runtime => needs_runtime,
            _ => needs_std,
        })
        .collect();
    let std_dir = std_placement(jobs, !modules.is_empty(), opts.out_dir.as_deref());
    if let Some(dir) = &std_dir
        && !opts.check
        && !opts.print
        && modules
            .iter()
            .copied()
            .map(|module| dir.join(module.file_name()))
            .any(|file| jobs.iter().any(|job| same_file(&job.file, &file)))
    {
        eprintln!(
            "ttc: {}: the standard library would overwrite an input — pass -o <dir>",
            dir.display()
        );
        failed = true;
    } else if let Some(dir) = &std_dir
        && !opts.check
        && !opts.print
    {
        let wrote = fs::create_dir_all(dir).and_then(|()| {
            for module in &modules {
                let mut code = module.source().to_string();
                if opts.banner {
                    code = format!("// @generated by ttc — do not edit directly.\n{code}");
                }
                fs::write(dir.join(module.file_name()), code)?;
            }
            Ok(())
        });
        match wrote {
            Ok(()) => eprintln!("ttc: std → {}", dir.display()),
            Err(e) => {
                eprintln!("ttc: {}: {e}", dir.display());
                failed = true;
            }
        }
    }

    // Two inputs can claim one output path (`x.tt` and a hand-written
    // `x.ts` both emit `x.ts`), and the later job wins. Those writes go
    // back to the parent so the winner stays the same as in a sequential
    // run; every other job writes itself, straight from its worker.
    let mut claims: HashMap<&Path, usize> = HashMap::with_capacity(jobs.len());
    for job in jobs {
        *claims.entry(job.out_path.as_path()).or_default() += 1;
    }
    let contested = |job: &Job| claims[job.out_path.as_path()] > 1;

    let cache = ExternCache::new(
        jobs.iter()
            .zip(&loaded)
            .filter_map(|(job, l)| Some((job.file.as_path(), l.as_ref().ok()?.source.as_str())))
            .collect(),
    );

    let outcomes = par_map(
        &jobs.iter().zip(&loaded).collect::<Vec<_>>(),
        opts.jobs,
        |(job, loaded)| {
            ttc::ice::working_on(&job.file, || {
                ttc::ice::panic_for_test("compile");
                let mut out = Outcome::default();
                let filename = job.file.display().to_string();
                // A pass-through `.ts` compiled in place would land on top of
                // its own source (with specifiers rewritten) — refuse rather
                // than destroy hand-written code.
                if !opts.print && !opts.check && same_file(&job.file, &job.out_path) {
                    out.messages.push(format!(
                        "ttc: {filename}: output would overwrite the input — pass -o <dir>"
                    ));
                    out.failed = true;
                    return out;
                }
                let loaded = match loaded {
                    Ok(loaded) => loaded,
                    Err(e) => {
                        out.messages.push(format!("ttc: {e}"));
                        out.failed = true;
                        return out;
                    }
                };
                let extern_variants =
                    collect_extern_variants(&job.file, &loaded.scan.imports, &cache);
                let std_imports_owned = std_dir.as_ref().map(|dir| {
                    StdModule::ALL
                        .map(|module| std_specifier(job, dir, opts.rewrite_imports, module))
                });
                let std_imports = match &std_imports_owned {
                    Some([types, option, result, runtime]) => StdImports {
                        types: types.as_deref(),
                        option: option.as_deref(),
                        result: result.as_deref(),
                        runtime: runtime.as_deref(),
                    },
                    None => StdImports::default(),
                };
                let options = Options {
                    filename: Some(&filename),
                    source_kind: ttc::SourceKind::from_path(&job.file).unwrap_or_default(),
                    verify: opts.verify,
                    rewrite_imports: opts.rewrite_imports,
                    extern_variants: &extern_variants,
                    defer_to_checker: false,
                    std_imports,
                };
                // Every tt-level diagnostic of the file, not the first one —
                // the reader fixes a file in one pass (TASK-120). Output is
                // only produced (and only written) when the file is clean.
                let report = compile_report(&loaded.source, &options);
                let errors: Vec<_> = report
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity == ttc::Severity::Error)
                    .collect();
                if !errors.is_empty() {
                    for diagnostic in errors {
                        // Trailing newline: `eprintln!` then separates the
                        // blocks with a blank line, so two diagnostics do not
                        // read as one.
                        out.messages.push(format!(
                            "{}\n",
                            ttc::render::diagnostic(
                                diagnostic,
                                &loaded.source,
                                &filename,
                                styles(),
                            )
                        ));
                    }
                    out.failed = true;
                    return out;
                }
                let Some(emit) = report.emit else {
                    // Unreachable in practice: emission is only withheld for
                    // an error-severity diagnostic. Stay total.
                    out.failed = true;
                    return out;
                };
                let mut code = emit.code.clone();
                if opts.banner {
                    let base = job
                        .file
                        .file_name()
                        .unwrap_or(job.file.as_os_str())
                        .to_string_lossy();
                    code =
                        format!("// @generated from {base} by ttc — do not edit directly.\n{code}");
                }
                // A map describes a translation. A hand-written `.ts` is not
                // translated — it passes through byte for byte by contract — so
                // there is nothing for a map to say about it, and appending a
                // `sourceMappingURL` line would be the one thing that contract
                // forbids. Only the surfaces ttc compiles get one.
                //
                // The map is built against the emission's own offsets, so the
                // banner is declared as the lines it prepends rather than
                // measured back out of the text.
                let map = match opts.source_map {
                    SourceMapMode::Off => None,
                    _ if ttc::SourceKind::from_tt_path(&job.file).is_none() => None,
                    mode => Some(source_map_for(job, &emit, &loaded.source, opts, mode)),
                };
                if let Some(rendered) = &map {
                    if !code.ends_with('\n') {
                        code.push('\n');
                    }
                    code.push_str(&rendered.comment);
                }
                if opts.print || (!opts.check && contested(job)) {
                    out.pending = Some(code);
                    return out;
                }
                if !opts.check {
                    if let Err(e) = write_output(&job.out_path, &code) {
                        out.messages.push(e);
                        out.failed = true;
                        return out;
                    }
                    if let Some(rendered) = &map
                        && let Some(document) = &rendered.document
                        && let Err(e) = write_output(&map_path(&job.out_path), document)
                    {
                        out.messages.push(e);
                        out.failed = true;
                        return out;
                    }
                    out.messages.push(format!(
                        "ttc: {} → {}",
                        job.file.display(),
                        job.out_path.display()
                    ));
                }
                out
            })
        },
    );

    for (job, outcome) in jobs.iter().zip(outcomes) {
        for message in &outcome.messages {
            eprintln!("{message}");
        }
        failed |= outcome.failed;
        let Some(code) = outcome.pending else {
            continue;
        };
        if opts.print {
            print!("{code}");
            continue;
        }
        match write_output(&job.out_path, &code) {
            Ok(()) => eprintln!("ttc: {} → {}", job.file.display(), job.out_path.display()),
            Err(e) => {
                eprintln!("{e}");
                failed = true;
            }
        }
    }
    failed
}
