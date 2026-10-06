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

#[derive(Clone)]
pub(super) struct BuildOptions {
    pub(super) banner: bool,
    pub(super) print: bool,
    pub(super) check: bool,
    pub(super) verify: bool,
    pub(super) rewrite_imports: ImportRewrite,
    pub(super) jsx_preserve: JsxPreserve,
    pub(super) source_map: SourceMapMode,
    /// Output root, when `-o` was given — also where the standard library
    /// module is written if an input imports it.
    pub(super) out_dir: Option<PathBuf>,
    /// Worker threads for the parallel phases (`--jobs`); `None` means one
    /// per available core.
    pub(super) jobs: Option<usize>,
    pub(super) node: Option<PathBuf>,
}

pub(super) type JsxPreserve = std::collections::BTreeMap<PathBuf, bool>;

pub(super) fn project_jsx_preserve(
    rewrite: ImportRewrite,
    files: &[PathBuf],
    project: Option<&Path>,
) -> Result<JsxPreserve, String> {
    let mut preserve = JsxPreserve::new();
    if rewrite != ImportRewrite::Js {
        return Ok(preserve);
    }
    let names_ttx = |file: &PathBuf| {
        file.extension().is_some_and(|extension| extension == "ttx")
            || std::fs::read_to_string(file).is_ok_and(|source| {
                ttc::tt_imports(&source)
                    .iter()
                    .any(|import| import.specifier.ends_with(".ttx"))
            })
    };
    for file in files {
        match ttc::engine::jsx_preserve(std::slice::from_ref(file), project) {
            Ok(value) => {
                preserve.insert(file.clone(), value);
            }
            Err(_) if !names_ttx(file) => {}
            Err(error) => {
                return Err(format!(
                    "cannot read the project's `jsx` option, which names a .ttx import's output: \
                     {error} (name the configuration with --project, or choose --rewrite-imports \
                     ts or off)"
                ));
            }
        }
    }
    Ok(preserve)
}

/// The directory that holds the generated `tt/` package: the output root
/// when `-o` was given, otherwise the deepest directory every output of the
/// whole build shares. It belongs to the build's full input set, so a
/// compile of any subset of it places support modules where a build of the
/// whole set does.
pub(super) fn support_root(jobs: &[Job], out_dir: Option<&Path>) -> Option<PathBuf> {
    match out_dir {
        Some(dir) => Some(dir.to_path_buf()),
        None => common_ancestor(jobs),
    }
}

/// Where the generated `tt/` standard-library package goes. Outputs name
/// it before anyone knows whether they import it; a build writes the
/// modules its outputs turned out to import.
pub(super) fn std_placement(root: Option<&Path>) -> Option<PathBuf> {
    Some(root?.join("tt"))
}

fn std_imports_of(specifiers: &[Option<String>; 4]) -> StdImports<'_> {
    let [types, option, result, runtime] = specifiers;
    StdImports {
        types: types.as_deref(),
        option: option.as_deref(),
        result: result.as_deref(),
        runtime: runtime.as_deref(),
        commonjs: None,
    }
}

pub(super) fn support_commonjs_dir(std_dir: &Path) -> PathBuf {
    std_dir.join(ttc::STD_PACKAGE_COMMONJS_DIR)
}

/// The deepest directory every input is inside, a directory input counting
/// as itself and a file as its parent — what `-o` mirrors the inputs under.
pub(super) fn input_root(inputs: &[String]) -> Option<PathBuf> {
    deepest_shared_directory(inputs.iter().map(Path::new).filter_map(|path| {
        let absolute = normalized_absolute(path);
        if path.is_dir() {
            Some(absolute)
        } else {
            absolute.parent().map(Path::to_path_buf)
        }
    }))
}

/// The deepest directory every output shares.
pub(super) fn common_ancestor(jobs: &[Job]) -> Option<PathBuf> {
    let shared = deepest_shared_directory(
        jobs.iter()
            .map(|job| normalized_absolute(job.out_path.parent().unwrap_or(Path::new(".")))),
    )?;
    let cwd = normalized_absolute(Path::new("."));
    Some(match shared.strip_prefix(&cwd) {
        Ok(relative) if relative.as_os_str().is_empty() => PathBuf::from("."),
        Ok(relative) => relative.to_path_buf(),
        Err(_) => shared,
    })
}

/// The components every path in `dirs` begins with — the deepest directory
/// all of them are inside. Empty when they share nothing, and `None` when
/// there are no paths at all.
pub(super) fn deepest_shared_directory(dirs: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    let mut dirs = dirs.into_iter();
    let first = dirs.next()?;
    let mut shared: Vec<_> = first.components().collect();
    for dir in dirs {
        let common = dir
            .components()
            .zip(&shared)
            .take_while(|(component, kept)| component == *kept)
            .count();
        shared.truncate(common);
    }
    Some(shared.iter().collect())
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
) -> Result<Vec<Job>, String> {
    let input_root = input_root(inputs);
    let mut jobs: Vec<Job> = Vec::new();
    for input in inputs {
        let input_path = Path::new(input);
        if !input_path.exists() {
            return Err(format!("ttc: no such file or directory: {input}"));
        }
        let is_dir = input_path.is_dir();
        let mut files = Vec::new();
        collect_sources(input_path, include_ts, &mut files).map_err(|e| format!("ttc: {e}"))?;
        if is_dir
            && let Some(dir) = out_dir
            && output_tree_inside(input_path, dir)
        {
            files.retain(|file| !path_is_within(file, dir));
        }
        for file in files {
            if is_dir && ttc::ownership::owned_output(&file) {
                continue;
            }
            let out_name = if let Some(kind) = ttc::SourceKind::from_tt_path(&file) {
                file.with_extension(kind.output_extension())
            } else {
                file.clone()
            };
            let out_path = match out_dir {
                Some(dir) => {
                    let root = input_root
                        .clone()
                        .ok_or_else(|| format!("ttc: no output root for {}", file.display()))?;
                    let mirrored = normalized_absolute(&out_name);
                    let rel = mirrored.strip_prefix(&root).map_err(|_| {
                        format!(
                            "ttc: {} is outside output source root {}",
                            file.display(),
                            root.display()
                        )
                    })?;
                    dir.join(rel)
                }
                None => out_name,
            };
            // One source/output identity owns exactly one emission job.
            if !jobs.iter().any(|job| {
                same_file(&job.file, &file)
                    && normalized_absolute(&job.out_path) == normalized_absolute(&out_path)
            }) {
                jobs.push(Job { file, out_path });
            }
        }
    }
    let compiled_outputs: Vec<PathBuf> = jobs
        .iter()
        .filter(|job| !same_file(&job.file, &job.out_path))
        .map(|job| job.out_path.clone())
        .collect();
    jobs.retain(|job| {
        !(same_file(&job.file, &job.out_path)
            && compiled_outputs
                .iter()
                .any(|output| same_file(output, &job.file)))
    });
    Ok(jobs)
}

/// Whether `path` is inside `dir`, accepting the relative or absolute spellings
/// a caller may mix on the command line. Existing paths are canonicalized so a
/// symlink to an output tree cannot make a previous build become a new input.
fn path_is_within(path: &Path, dir: &Path) -> bool {
    path.starts_with(dir)
        || matches!(
            (path.canonicalize(), dir.canonicalize()),
            (Ok(path), Ok(dir)) if path.starts_with(&dir)
        )
}

/// Whether the output root is a subtree of a directory input, so the
/// input's walk would read back what a previous build wrote there. Only a
/// root strictly inside the input is: the input directory itself, or one
/// enclosing it, holds every source, and its earlier outputs are already
/// skipped by their ownership records.
fn output_tree_inside(input: &Path, out_dir: &Path) -> bool {
    let lexically_same = normalized_absolute(input) == normalized_absolute(out_dir);
    let canonically_same = matches!(
        (input.canonicalize(), out_dir.canonicalize()),
        (Ok(input), Ok(out_dir)) if input == out_dir
    );
    !lexically_same && !canonically_same && path_is_within(out_dir, input)
}

/// Whether two paths name the same file. The output side may not exist
/// yet, so the parents are compared canonically and the file names
/// literally.
pub(super) fn same_file(a: &Path, b: &Path) -> bool {
    if normalized_absolute(a) == normalized_absolute(b) {
        return true;
    }
    if let (Ok(x), Ok(y)) = (a.canonicalize(), b.canonicalize()) {
        return x == y;
    }
    if a.file_name() != b.file_name() {
        return false;
    }
    let canon = |p: &Path| p.parent().unwrap_or(Path::new(".")).canonicalize();
    matches!((canon(a), canon(b)), (Ok(x), Ok(y)) if x == y)
}

/// What compiling one job produced: the diagnostics it wants printed (in
/// job order), whether it failed, and the output it emitted, which the
/// parent writes or hands out on stdout under `-p`.
#[derive(Default)]
pub(super) struct Outcome {
    messages: Vec<String>,
    failed: bool,
    output: Option<Emitted>,
}

/// One job's emitted output, not yet written.
struct Emitted {
    code: String,
    /// The `--source-map file` document that goes beside the output.
    map: Option<String>,
    /// The compiler support modules `code` imports.
    support_imports: Vec<StdModule>,
    commonjs: bool,
    in_place: bool,
}

/// Compiles every job. Returns true if any of them failed.
///
/// `support_root` is the [`support_root`] of the build's full input set,
/// which `jobs` may be only part of.
///
/// The run is staged so each input is touched once: read and scanned in
/// parallel, then compiled in parallel against a shared table of imported
/// declarations, then — once the support modules the emitted outputs import
/// are in place — written in parallel. Diagnostics are collected per job and
/// printed in job order, so the output of a parallel run is identical to a
/// sequential one.
pub(super) fn compile_jobs(jobs: &[Job], support_root: Option<&Path>, opts: &BuildOptions) -> bool {
    if !opts.check && !opts.print {
        let mut claims: HashMap<&Path, &Path> = HashMap::with_capacity(jobs.len());
        let mut outputs: Vec<(&Path, &Path)> = Vec::with_capacity(jobs.len());
        let mut conflicted = false;
        for job in jobs {
            if let Some(first) = claims.get(job.out_path.as_path()) {
                if !same_file(first, &job.file) {
                    eprintln!(
                        "ttc: {}: multiple inputs claim this output: {} and {}",
                        job.out_path.display(),
                        first.display(),
                        job.file.display()
                    );
                    conflicted = true;
                }
            } else {
                claims.insert(&job.out_path, &job.file);
            }
            // The other half of the same contract: overlapping input roots
            // give one source two outputs, so the build would write it
            // twice, at two paths, and say nothing. Both sides are compared
            // by identity — the same source reached through two roots is
            // spelled differently on each.
            match outputs
                .iter()
                .find(|(source, _)| same_file(source, &job.file))
            {
                Some((_, first)) if !same_file(first, &job.out_path) => {
                    eprintln!(
                        "ttc: {}: one input claims two outputs: {} and {} (overlapping input roots)",
                        job.file.display(),
                        first.display(),
                        job.out_path.display()
                    );
                    conflicted = true;
                }
                Some(_) => {}
                None => outputs.push((job.file.as_path(), job.out_path.as_path())),
            }
        }
        for job in jobs
            .iter()
            .filter(|job| !same_file(&job.file, &job.out_path))
        {
            if let Err(error) = check_output_owner(&job.out_path, OutputOwner::Source(&job.file)) {
                eprintln!("{error}");
                conflicted = true;
            }
            if opts.source_map == SourceMapMode::File
                && let Err(error) =
                    check_output_owner(&map_path(&job.out_path), OutputOwner::Source(&job.file))
            {
                eprintln!("{error}");
                conflicted = true;
            }
        }
        if conflicted {
            return true;
        }
    }

    let outcomes = compile_outcomes(jobs, support_root, opts);

    if opts.print || opts.check {
        let mut failed = false;
        for outcome in outcomes {
            for message in &outcome.messages {
                eprintln!("{message}");
            }
            failed |= outcome.failed;
            if let Some(output) = outcome.output {
                crate::out::text(&output.code);
            }
        }
        return failed;
    }

    write_outcomes(jobs, &outcomes, support_root, opts)
}

/// What `ttc -p <input>` answers: the text it prints on stdout, present
/// exactly when it exits successfully, and each message it writes to
/// stderr.
pub(super) struct Printed {
    pub(super) code: Option<String>,
    pub(super) messages: Vec<String>,
}

/// `ttc -p <input>` answered instead of printed: the same job, support
/// root and compile the command line runs, so a long-lived caller gets the
/// bytes the one-shot prints.
pub(super) fn print_input(input: &str, opts: &BuildOptions) -> Printed {
    let failure = |message: String| Printed {
        code: None,
        messages: vec![message],
    };
    let jobs = match build_jobs(&[input.to_string()], None, true) {
        Ok(jobs) => jobs,
        Err(error) => return failure(error),
    };
    if jobs.is_empty() {
        return failure("ttc: no sources found".to_string());
    }
    if jobs.len() != 1 {
        return failure("ttc: --print requires exactly one source file".to_string());
    }
    let root = support_root(&jobs, None);
    let mut printed = Printed {
        code: None,
        messages: Vec::new(),
    };
    for outcome in compile_outcomes(&jobs, root.as_deref(), opts) {
        printed.messages.extend(outcome.messages);
        if !outcome.failed {
            printed.code = outcome.output.map(|output| output.code);
        }
    }
    printed
}

/// Compiles every job without writing or printing anything: each job's
/// diagnostics, whether it failed, and its output, in job order. The
/// command line prints or writes these; `ttc --server` answers with them.
fn compile_outcomes(
    jobs: &[Job],
    support_root: Option<&Path>,
    opts: &BuildOptions,
) -> Vec<Outcome> {
    let loaded = load_jobs(jobs, opts.jobs);

    let std_dir = std_placement(support_root);

    let cache = ExternCache::new(
        jobs.iter()
            .zip(&loaded)
            .filter_map(|(job, l)| Some((job.file.as_path(), l.as_ref().ok()?.source.as_str())))
            .collect(),
    );

    par_map(
        &jobs.iter().zip(&loaded).collect::<Vec<_>>(),
        opts.jobs,
        |(job, loaded)| {
            ttc::ice::working_on(&job.file, || {
                ttc::ice::panic_for_test("compile");
                let mut out = Outcome::default();
                let filename = job.file.display().to_string();
                let in_place = !opts.print && !opts.check && same_file(&job.file, &job.out_path);
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
                let specifiers = |dir: &Path| {
                    StdModule::ALL
                        .map(|module| std_specifier(job, dir, opts.rewrite_imports, module))
                };
                let std_imports_owned = std_dir
                    .as_ref()
                    .map(|dir| (specifiers(dir), specifiers(&support_commonjs_dir(dir))));
                let commonjs_imports = std_imports_owned
                    .as_ref()
                    .map(|(_, commonjs)| std_imports_of(commonjs));
                let std_imports = match &std_imports_owned {
                    Some((module, _)) => StdImports {
                        commonjs: commonjs_imports.as_ref(),
                        ..std_imports_of(module)
                    },
                    None => StdImports::default(),
                };
                let options = Options {
                    filename: Some(&filename),
                    source_kind: ttc::SourceKind::from_path(&job.file).unwrap_or_default(),
                    verify: opts.verify,
                    rewrite_imports: opts.rewrite_imports,
                    jsx_preserve: opts.jsx_preserve.get(&job.file).copied().unwrap_or(false),
                    extern_variants: &extern_variants,
                    defer_to_checker: false,
                    std_imports,
                    node: opts.node.as_deref(),
                };
                // Every tt-level diagnostic of the file, not the first one —
                // the reader fixes a file in one pass (TASK-120). Output is
                // only produced (and only written) when the file is clean.
                let report = if opts.check {
                    ttc::check_report(&loaded.source, &options)
                } else {
                    compile_report(&loaded.source, &options)
                };
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
                let mut banner = BannerPlacement::default();
                // Same contract the source map below answers to: a
                // hand-written `.ts`/`.tsx` passes through byte for byte,
                // save for its relative tt specifiers. A banner calling it
                // generated, and telling its author not to edit it, is both
                // untrue and a byte the contract does not allow. Only the
                // surfaces ttc compiles carry one.
                if opts.banner && ttc::SourceKind::from_tt_path(&job.file).is_some() {
                    let base = job
                        .file
                        .file_name()
                        .unwrap_or(job.file.as_os_str())
                        .to_string_lossy();
                    let banner_text = format!(
                        "// @generated from {base} by ttc — do not edit directly.{}",
                        ttc::line_ending(&code)
                    );
                    banner = write_banner(&mut code, &banner_text);
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
                    mode => Some(source_map_for(
                        job,
                        &emit,
                        &loaded.source,
                        banner,
                        mode,
                        ttc::line_ending(&code),
                    )),
                };
                if let Some(rendered) = &map {
                    if !code.ends_with('\n') {
                        code.push_str(ttc::line_ending(&code));
                    }
                    code.push_str(&rendered.comment);
                }
                if in_place && code != loaded.source {
                    out.messages.push(format!(
                        "ttc: {filename}: output would overwrite the input — pass -o <dir>"
                    ));
                    out.failed = true;
                    return out;
                }
                if !opts.check {
                    out.output = Some(Emitted {
                        code,
                        map: map.and_then(|rendered| rendered.document),
                        support_imports: emit.support_imports,
                        commonjs: emit.commonjs,
                        in_place,
                    });
                }
                out
            })
        },
    )
}

/// Writes the compiled outputs and the support modules they import.
/// Returns true if any job failed or any write did.
fn write_outcomes(
    jobs: &[Job],
    outcomes: &[Outcome],
    support_root: Option<&Path>,
    opts: &BuildOptions,
) -> bool {
    let mut failed = false;
    let std_dir = std_placement(support_root);
    // Compiler-owned support modules are written once for the project, not
    // once per source file, and only when an output being written imports
    // one. What an output imports is what codegen emitted, not what the
    // source looks like: a pipeline may lower to a direct call, and a script
    // inlines its helpers. Standard-library imports materialize its three
    // public modules; the pipeline runtime is written on its own.
    let forms: Vec<(PathBuf, bool, Vec<StdModule>)> = std_dir
        .iter()
        .flat_map(|dir| [(dir.clone(), false), (support_commonjs_dir(dir), true)])
        .map(|(dir, commonjs)| {
            let imports = |module: StdModule| {
                outcomes
                    .iter()
                    .filter_map(|outcome| outcome.output.as_ref())
                    .filter(|output| output.commonjs == commonjs)
                    .any(|output| output.support_imports.contains(&module))
            };
            let needs_std = StdModule::STANDARD.into_iter().any(imports);
            let needs_runtime = imports(StdModule::Runtime);
            let modules = StdModule::ALL
                .into_iter()
                .filter(|module| match module {
                    StdModule::Runtime => needs_runtime,
                    _ => needs_std,
                })
                .collect();
            (dir, commonjs, modules)
        })
        .filter(|(_, _, modules): &(PathBuf, bool, Vec<StdModule>)| !modules.is_empty())
        .collect();
    for (dir, _, modules) in &forms {
        for module in modules {
            let support = dir.join(module.file_name());
            if let Err(error) = check_output_owner(&support, OutputOwner::Support(*module)) {
                eprintln!("{error}");
                return true;
            }
            for job in jobs {
                if same_file(&job.file, &support) {
                    eprintln!(
                        "ttc: {}: the compiler support module would overwrite input {} — pass -o <dir>",
                        support.display(),
                        job.file.display()
                    );
                    return true;
                }
                if same_file(&job.out_path, &support) {
                    eprintln!(
                        "ttc: {}: compiler support module and input {} claim this output",
                        support.display(),
                        job.file.display()
                    );
                    return true;
                }
            }
        }
    }
    for (dir, commonjs, modules) in &forms {
        let wrote = create_dir_all(dir).and_then(|()| {
            for module in modules {
                let mut code = if *commonjs {
                    module.commonjs_source().into_owned()
                } else {
                    module.source().to_string()
                };
                if opts.banner {
                    code = format!("// @generated by ttc — do not edit directly.\n{code}");
                }
                write_owned_output(
                    &dir.join(module.file_name()),
                    OutputOwner::Support(*module),
                    &code,
                )
                .map_err(std::io::Error::other)?;
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

    let writes = par_map(
        &jobs.iter().zip(outcomes).collect::<Vec<_>>(),
        opts.jobs,
        |(job, outcome)| write_emitted(job, outcome.output.as_ref()),
    );
    for (outcome, (messages, write_failed)) in outcomes.iter().zip(writes) {
        for message in outcome.messages.iter().chain(&messages) {
            eprintln!("{message}");
        }
        failed |= outcome.failed || write_failed;
    }
    failed
}

/// Writes one job's emitted output and its source map, returning the
/// messages to print and whether the write failed.
fn write_emitted(job: &Job, output: Option<&Emitted>) -> (Vec<String>, bool) {
    let Some(output) = output.filter(|output| !output.in_place) else {
        return (Vec::new(), false);
    };
    if let Err(e) = write_owned_output(&job.out_path, OutputOwner::Source(&job.file), &output.code)
    {
        return (vec![e], true);
    }
    if let Some(document) = &output.map
        && let Err(e) = write_owned_output(
            &map_path(&job.out_path),
            OutputOwner::Source(&job.file),
            document,
        )
    {
        return (vec![e], true);
    }
    (
        vec![format!(
            "ttc: {} → {}",
            job.file.display(),
            job.out_path.display()
        )],
        false,
    )
}
