//! Command-line argument parsing and dispatch.

use super::*;

pub(super) fn entry() -> ExitCode {
    // Every panic from here on is reported as what it is — a bug in this
    // compiler — rather than as a Rust backtrace the reader has to
    // interpret (TASK-214).
    ttc::ice::install_reporter();
    match ttc::ice::catching(run) {
        Ok(code) => code,
        // The report is already on stderr, printed where the panic
        // happened. All that is left is to fail deliberately: 101 is the
        // code a Rust panic exits with, kept so a caller that already
        // knows it keeps working.
        //
        // Unwind safety: nothing outlives this call. The process is ending,
        // and every file this run wrote was written whole before it.
        Err(_) => ExitCode::from(101),
    }
}

/// Everything `main` does, so that a panic anywhere in it is caught rather
/// than aborting the process with a backtrace.
pub(super) fn run() -> ExitCode {
    ttc::ice::panic_for_test("cli");
    let argv: Vec<String> = std::env::args().skip(1).collect();

    // `ttc help [topic]` — only as the first argument, so a file that
    // happens to be named "help" can still be passed as `./help`.
    if argv.first().is_some_and(|a| a == "help") {
        return run_help(&argv[1..]);
    }

    // `ttc explain [code]` — the long form of a diagnostic's rule, the way
    // `error[match-not-exhaustive]` names it.
    if argv.first().is_some_and(|a| a == "explain") {
        return run_explain(&argv[1..]);
    }

    let mut inputs: Vec<String> = Vec::new();
    let mut out_dir: Option<PathBuf> = None;
    let mut emit_std: Option<StdModule> = None;
    let mut print = false;
    let mut watch = false;
    let mut check = false;
    let mut check_types = false;
    let mut types = false;
    let mut overlay_path: Option<PathBuf> = None;
    let mut tt_only = false;
    let mut project: Option<PathBuf> = None;
    let mut banner = true;
    let mut verify = true;
    let mut symbols = false;
    let mut emit_map = false;
    let mut sidecar_dir: Option<PathBuf> = None;
    let mut server = false;
    let mut content_mapper = false;
    let mut node: Option<PathBuf> = None;
    let mut rewrite_imports = ImportRewrite::default();
    let mut source_map = SourceMapMode::default();
    let mut jobs_limit: Option<usize> = None;

    let mut it = argv.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                usage();
                return ExitCode::SUCCESS;
            }
            "-v" | "--version" => {
                println!("{VERSION}");
                return ExitCode::SUCCESS;
            }
            "-p" | "--print" => print = true,
            "-w" | "--watch" => watch = true,
            "--check" => check = true,
            "--check-types" => check_types = true,
            "--types" => {
                check_types = true;
                types = true;
            }
            "--tt-only" => tt_only = true,
            "--server" => server = true,
            "--content-mapper" => content_mapper = true,
            "--overlay" => match it.next() {
                Some(path) => overlay_path = Some(PathBuf::from(path)),
                None => {
                    eprintln!("ttc: --overlay requires the path the buffer belongs to");
                    return ExitCode::FAILURE;
                }
            },
            "--project" => match it.next() {
                Some(path) => project = Some(PathBuf::from(path)),
                None => {
                    eprintln!("ttc: --project requires a path to a tsconfig.json");
                    return ExitCode::FAILURE;
                }
            },
            "--symbols" => symbols = true,
            "--emit-map" => emit_map = true,
            "--no-banner" => banner = false,
            "--no-verify" => verify = false,
            "--emit-std" => match it.next().map(String::as_str) {
                Some("types") => emit_std = Some(StdModule::Types),
                Some("option") => emit_std = Some(StdModule::Option),
                Some("result") => emit_std = Some(StdModule::Result),
                Some("runtime") => emit_std = Some(StdModule::Runtime),
                Some(other) => {
                    eprintln!(
                        "ttc: --emit-std expects types, option, result, or runtime (got {other})"
                    );
                    return ExitCode::FAILURE;
                }
                None => {
                    eprintln!(
                        "ttc: --emit-std requires a module (types, option, result, or runtime)"
                    );
                    return ExitCode::FAILURE;
                }
            },
            "--sidecar" => match it.next() {
                Some(dir) => sidecar_dir = Some(PathBuf::from(dir)),
                None => {
                    eprintln!("ttc: --sidecar requires a directory of tsc-emitted .d.ts files");
                    return ExitCode::FAILURE;
                }
            },
            "-j" | "--jobs" => match it.next().map(|n| n.parse::<usize>()) {
                Some(Ok(n)) if n >= 1 => jobs_limit = Some(n),
                Some(_) => {
                    eprintln!("ttc: --jobs expects a positive number of parallel compiles");
                    return ExitCode::FAILURE;
                }
                None => {
                    eprintln!("ttc: --jobs requires a value");
                    return ExitCode::FAILURE;
                }
            },
            "-o" | "--out-dir" => match it.next() {
                Some(dir) => out_dir = Some(PathBuf::from(dir)),
                None => {
                    eprintln!("ttc: --out-dir requires a value");
                    return ExitCode::FAILURE;
                }
            },
            "--node" => match it.next() {
                Some(path) => node = Some(PathBuf::from(path)),
                None => {
                    eprintln!("ttc: --node requires a path to the node binary");
                    return ExitCode::FAILURE;
                }
            },
            "--source-map" => match it.next().map(String::as_str) {
                Some("off") => source_map = SourceMapMode::Off,
                Some("file") => source_map = SourceMapMode::File,
                Some("inline") => source_map = SourceMapMode::Inline,
                Some(other) => {
                    eprintln!("ttc: --source-map expects off, file, or inline (got {other})");
                    return ExitCode::FAILURE;
                }
                None => {
                    eprintln!("ttc: --source-map requires a value (off, file, or inline)");
                    return ExitCode::FAILURE;
                }
            },
            "--rewrite-imports" => match it.next().map(String::as_str) {
                Some("js") => rewrite_imports = ImportRewrite::Js,
                Some("ts") => rewrite_imports = ImportRewrite::Ts,
                Some("off") => rewrite_imports = ImportRewrite::Off,
                Some(other) => {
                    eprintln!("ttc: --rewrite-imports expects js, ts, or off (got {other})");
                    return ExitCode::FAILURE;
                }
                None => {
                    eprintln!("ttc: --rewrite-imports requires a value (js, ts, or off)");
                    return ExitCode::FAILURE;
                }
            },
            other if other.starts_with('-') => {
                eprintln!("ttc: unknown option {other}");
                return ExitCode::FAILURE;
            }
            other => inputs.push(other.to_string()),
        }
    }

    // A TypeScript content mapper process — spawned by TypeScript itself,
    // never combined with anything: stdin and stdout are the protocol.
    if content_mapper {
        if server
            || !inputs.is_empty()
            || emit_std.is_some()
            || print
            || watch
            || check
            || check_types
            || symbols
            || emit_map
            || sidecar_dir.is_some()
            || overlay_path.is_some()
        {
            eprintln!("ttc: --content-mapper takes no inputs and combines with no other mode");
            return ExitCode::FAILURE;
        }
        return content_mapper::run();
    }

    // The engine behind a pipe — a session for tools that ask often. It
    // reads requests from stdin, so it combines with nothing else.
    if server {
        if !inputs.is_empty()
            || emit_std.is_some()
            || print
            || watch
            || check
            || check_types
            || symbols
            || emit_map
            || sidecar_dir.is_some()
            || overlay_path.is_some()
        {
            eprintln!("ttc: --server takes no inputs and combines with no other mode");
            return ExitCode::FAILURE;
        }
        return server::run(node);
    }

    // The standard library on stdout — how a bundler plugin serves the
    // module from memory. Since the build materializes it on its own
    // (`@tt/std` auto-emission), this combines with nothing else.
    if let Some(module) = emit_std {
        if !inputs.is_empty() {
            eprintln!("ttc: --emit-std takes no inputs (the build materializes support modules)");
            return ExitCode::FAILURE;
        }
        let mut code = module.source().to_string();
        if banner {
            code = format!("// @generated by ttc --emit-std — do not edit directly.\n{code}");
        }
        print!("{code}");
        return ExitCode::SUCCESS;
    }

    if inputs.is_empty() {
        usage();
        return ExitCode::FAILURE;
    }

    if !check_types && (overlay_path.is_some() || tt_only) {
        eprintln!("ttc: --overlay and --tt-only require --check-types");
        return ExitCode::FAILURE;
    }

    // A watch re-reads the files it is watching; text pinned on stdin would
    // stay the same forever, so the pair has no coherent meaning.
    if overlay_path.is_some() && watch {
        eprintln!("ttc: --overlay does not combine with --watch");
        return ExitCode::FAILURE;
    }

    if check_types && (print || check || symbols || emit_map || sidecar_dir.is_some()) {
        eprintln!(
            "ttc: --types/--check-types does not combine with -p, --check, --symbols, \
             --emit-map, or --sidecar"
        );
        return ExitCode::FAILURE;
    }

    // Tooling modes stay .tt-only; the compile modes carry hand-written
    // TypeScript along so the output tree is complete.
    let include_ts = !symbols && !emit_map && sidecar_dir.is_none();

    if check_types {
        // Both only make sense for a caller that is showing diagnostics
        // rather than producing files: unsaved text must not reach a written
        // sidecar, and a mode that writes is not one that hides half of what
        // it found.
        if types && (overlay_path.is_some() || tt_only) {
            eprintln!("ttc: --overlay and --tt-only work with --check-types, not --types");
            return ExitCode::FAILURE;
        }
        let mut overlay = std::collections::HashMap::new();
        if let Some(path) = &overlay_path {
            // The buffer's text arrives on stdin, keyed by the path the file
            // occupies in the project — canonical, because that is the form
            // the project's own file list is in.
            let text = match std::io::read_to_string(std::io::stdin()) {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("ttc: cannot read the overlay from stdin: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match path.canonicalize() {
                Ok(path) => {
                    overlay.insert(path, text);
                }
                Err(e) => {
                    eprintln!("ttc: --overlay {}: {e}", path.display());
                    return ExitCode::FAILURE;
                }
            }
        }
        // Sidecars keep the directory `--types` has always written them to,
        // so a project's tsconfig `paths` and `.gitignore` keep pointing at
        // the same place. A check that writes nothing needs no directory.
        let sidecar_out = types.then(|| out_dir.unwrap_or_else(|| PathBuf::from(TYPES_DIR)));
        return typed_check_mode(
            &inputs,
            &TypedCheckOptions {
                project: project.as_deref(),
                node: node.as_deref(),
                out_dir: sidecar_out.as_deref(),
                emit: types,
                watch,
                overlay: &overlay,
                tt_only,
                inputs: &inputs,
            },
        );
    }

    let jobs = match build_jobs(&inputs, out_dir.as_deref(), include_ts) {
        Ok(jobs) => jobs,
        Err(code) => return code,
    };

    if jobs.is_empty() {
        eprintln!("ttc: no sources found");
        return ExitCode::FAILURE;
    }

    if symbols {
        return symbols_mode(&jobs);
    }

    if emit_map {
        return emit_map_mode(&jobs);
    }

    if let Some(dir) = &sidecar_dir {
        return sidecar_mode(&jobs, dir);
    }

    let build = BuildOptions {
        banner,
        print,
        check,
        verify,
        rewrite_imports,
        source_map,
        out_dir: out_dir.clone(),
        jobs: jobs_limit,
    };

    if watch {
        return watch_mode(&inputs, out_dir.as_deref(), &build);
    }

    if compile_jobs(&jobs, &build) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
