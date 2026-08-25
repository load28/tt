//! ttc — compile .tt/.ttx sources into a complete TypeScript tree.
//!
//!   ttc -o build src/              builds src/ under build/: .tt/.ttx files
//!                                  are compiled, hand-written .ts/.tsx files
//!                                  pass through (with .tt/.ttx specifiers
//!                                  rewritten), and the standard library is
//!                                  materialized when something imports it
//!   ttc --check-types src/         type-checks the tree with the real
//!                                  TypeScript compiler, reporting at
//!                                  positions in the .tt sources
//!   ttc --types src/               the same check, and writes the editor/
//!                                  typecheck sidecars it emits
//!                                  (.tt-types/<name>.tt.d.ts + .map)
//!   ttc --check src/               compiles without writing anything
//!   ttc -p file.tt                 prints one compiled module to stdout

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

mod server;

use ttc::engine::collect_sources;
use ttc::source_map::SourceMapRequest;
use ttc::{
    EnumSymbol, ExternEnum, ImportRewrite, ModuleScan, Options, StdImports, StdModule, TtImport,
    TtImportNames, compile_report,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn usage() {
    println!(
        "ttc v{VERSION} — tt to TypeScript compiler

Usage: ttc [options] <file | dir> ...
       ttc help [topic]      language & workflow reference (topics: ttc help)
       ttc explain [code]    what a diagnostic's rule is (codes: ttc explain)

Builds a complete TypeScript tree: .tt/.ttx files are compiled, hand-written
.ts/.tsx files pass through byte-for-byte (with relative .tt/.ttx specifiers
rewritten), and the standard library is materialized when an input
imports \"@tt/std\". Types come from the same sources via --types.

Options:
  -o, --out-dir <dir>   write outputs under <dir> (mirrors input paths)
  -j, --jobs <n>        compile n files at a time (default: one per core;
                        1 compiles sequentially). Output is identical
                        either way.
  -w, --watch           keep running; recompile inputs (and their importers)
                        as they change
  --check               compile only; write nothing (tt-level checks; needs
                        no TypeScript)
  --check-types         also type-check: the tree is lowered into the real
                        TypeScript project and the compiler answers, with
                        every diagnostic at a position in the .tt source
  --types               --check-types, and write the editor/typecheck
                        sidecars the compiler emits: <name>.tt.d.ts + .map
                        under -o (default .tt-types)
  --project <path>      tsconfig.json the two modes above check against
                        (default: the nearest one at or above the inputs)
  --node <path>         node binary the TypeScript compiler's client runs
                        with (default: node on PATH)
  -h, --help            show this help
  -v, --version         show version

Tooling options (bundler plugins, editors):
  -p, --print           print compiled output to stdout instead of writing
  --emit-std <module>   print one support module: types, option, result, runtime
  --no-banner           omit the \"generated\" banner comment
  --no-verify           skip swc validation of types and generated output
  --source-map <off|file|inline>
                        emit a source map for each compiled file so a stack
                        trace and a debugger point at the .tt source:
                        file = <output>.map beside it (default: off),
                        inline = a data: URL in the output itself
  --rewrite-imports <js|ts|off>
                        how relative .tt/.ttx specifiers are emitted:
                        js = ./x.js/.jsx (default), ts = ./x.ts/.tsx,
                        off = untouched
  --sidecar <dir>       write <name>.tt.d.ts and .map next to each input from
                        <dir>/<name>.d.ts (tsc --emitDeclarationOnly output);
                        compiles nothing (--types runs this step for you)
  --symbols             print tt enum declarations (with positions) and the
                        direct .tt imports of each input as JSON; compiles
                        nothing (for language tooling)
  --emit-map            print each input's emitted TypeScript plus source<->
                        output byte mappings as JSON; parse + emit only (no
                        tt-level checks, .tt specifiers untouched) — the
                        editor's virtual-document feed (for language tooling)
  --server              keep the engine alive and answer check/emitMap/
                        typedCheck requests as JSON lines on stdin/stdout,
                        reusing one project session per project — the same
                        answers as the one-shot modes, without the startup"
    );
}

/// The language & workflow guide (docs/ai/tt.md), embedded so `ttc help`
/// serves documentation offline. The file is the source of truth; `##`
/// headings are the topic boundaries.
const GUIDE: &str = include_str!("../docs/ai/tt.md");

/// Help topics: (name, aliases, `##` heading prefix in GUIDE). An empty
/// prefix selects the preamble (everything before the first `## `).
const HELP_TOPICS: &[(&str, &[&str], &str)] = &[
    ("overview", &["contracts", "intro"], ""),
    ("enum", &["enums"], "## enum"),
    ("match", &["tuple", "patterns"], "## match"),
    ("try", &[], "## try"),
    ("let-else", &["letelse"], "## let-else"),
    ("if-let", &["iflet"], "## if let"),
    ("pipe", &["pipeline", "|>", "flow"], "## |>"),
    ("result", &["do", "result-block"], "## result block"),
    ("val", &["mutation", "readonly"], "## val"),
    ("std", &["option"], "## @tt/std"),
    ("modules", &["imports"], "## Modules"),
    ("install", &["update"], "## Install"),
    ("setup", &["init"], "## Setup"),
    ("workflow", &["dev", "build"], "## Workflow"),
    ("errors", &[], "## Errors"),
    ("checklist", &[], "## Checklist"),
];

/// The slice of GUIDE for one topic: from its heading line up to the next
/// `## ` heading. An empty `heading` returns the preamble.
fn guide_section(heading: &str) -> &'static str {
    let start = if heading.is_empty() {
        0
    } else {
        match GUIDE
            .lines()
            .scan(0usize, |off, line| {
                let at = *off;
                *off += line.len() + 1;
                Some((at, line))
            })
            .find(|(_, line)| line.starts_with(heading))
        {
            Some((at, _)) => at,
            None => return "",
        }
    };
    let body = &GUIDE[start..];
    // A section's own heading sits at offset 0 (no leading newline), so
    // searching for "\n## " only ever finds the NEXT heading.
    match body.find("\n## ") {
        Some(end) => &body[..end + 1],
        None => body,
    }
}

/// `ttc help [topic]` — print the embedded guide (whole, one section, or
/// the topic list).
fn run_help(args: &[String]) -> ExitCode {
    let topic = match args {
        [] => {
            println!(
                "ttc help <topic> — tt language & workflow reference\n\n\
                 Topics:\n  {}\n\n\
                 `ttc help all` prints the whole guide; `ttc -h` shows CLI options.",
                HELP_TOPICS
                    .iter()
                    .map(|(name, aliases, _)| if aliases.is_empty() {
                        (*name).to_string()
                    } else {
                        format!("{name} ({})", aliases.join(", "))
                    })
                    .collect::<Vec<_>>()
                    .join("\n  ")
            );
            return ExitCode::SUCCESS;
        }
        [topic] => topic.to_lowercase(),
        _ => {
            eprintln!("ttc: help takes at most one topic (run `ttc help` for the list)");
            return ExitCode::FAILURE;
        }
    };
    if topic == "all" || topic == "guide" {
        print!("{GUIDE}");
        return ExitCode::SUCCESS;
    }
    let found = HELP_TOPICS
        .iter()
        .find(|(name, aliases, _)| *name == topic || aliases.contains(&topic.as_str()));
    match found {
        Some((_, _, heading)) => {
            print!("{}", guide_section(heading));
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("ttc: unknown help topic \"{topic}\" (run `ttc help` for the list)");
            ExitCode::FAILURE
        }
    }
}

/// `ttc explain [code]` — one rule at length, or the list of rules.
fn run_explain(args: &[String]) -> ExitCode {
    let code = match args {
        [] => {
            println!("ttc explain <code> — what a diagnostic's rule is and why\n");
            println!("Codes:");
            for code in ttc::DiagnosticCode::ALL {
                println!("  {}", code.as_str());
            }
            return ExitCode::SUCCESS;
        }
        [code] => code.as_str(),
        _ => {
            eprintln!("ttc: explain takes one code (run `ttc explain` for the list)");
            return ExitCode::FAILURE;
        }
    };
    // A reader who copied `error[match-not-exhaustive]` out of a build log
    // has the brackets too; they are not part of the code, so drop them
    // rather than reject the paste.
    let code = code
        .trim()
        .trim_start_matches("error[")
        .trim_start_matches("warning[")
        .trim_end_matches(']');
    match ttc::DiagnosticCode::parse(code) {
        Some(code) => {
            println!("error[{}]\n", code.as_str());
            println!("{}", code.explanation());
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("ttc: unknown diagnostic code \"{code}\" (run `ttc explain` for the list)");
            ExitCode::FAILURE
        }
    }
}

#[derive(Clone)]
struct Job {
    file: PathBuf,
    out_path: PathBuf,
}

/// `--symbols`: prints, as a JSON array on stdout, each input file's tt
/// enum declarations (positions included) and its direct relative `.tt`
/// imports with the referenced files' exported declarations — the symbol
/// interface language tooling consumes (module graph phase 3). Compiles
/// nothing; unreadable *imported* files yield `"resolved": null` while
/// unreadable *input* files fail the run.
fn symbols_mode(jobs: &[Job]) -> ExitCode {
    let mut entries: Vec<String> = Vec::new();
    let mut failed = false;
    for job in jobs {
        let filename = job.file.display().to_string();
        let source = match fs::read_to_string(&job.file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ttc: {filename}: {e}");
                failed = true;
                continue;
            }
        };
        let mut entry = format!("{{\"file\":{}", json_str(&filename));
        entry.push_str(",\"enums\":");
        entry.push_str(&enums_json(&source, &ttc::enum_symbols(&source)));
        entry.push_str(",\"imports\":[");
        let dir = job.file.parent().unwrap_or(Path::new("."));
        let imports = ttc::tt_imports(&source)
            .iter()
            .map(|import| {
                let mut o = format!("{{\"specifier\":{}", json_str(&import.specifier));
                o.push_str(",\"names\":");
                o.push_str(&names_json(&import.names));
                let target = dir.join(&import.specifier);
                match fs::read_to_string(&target) {
                    Ok(imported_src) => {
                        o.push_str(&format!(
                            ",\"resolved\":{}",
                            json_str(&target.display().to_string())
                        ));
                        let exported: Vec<EnumSymbol> = ttc::enum_symbols(&imported_src)
                            .into_iter()
                            .filter(|e| e.exported)
                            .collect();
                        o.push_str(",\"enums\":");
                        o.push_str(&enums_json(&imported_src, &exported));
                    }
                    Err(_) => o.push_str(",\"resolved\":null,\"enums\":[]"),
                }
                o.push('}');
                o
            })
            .collect::<Vec<_>>();
        entry.push_str(&imports.join(","));
        entry.push_str("]}");
        entries.push(entry);
    }
    println!("[{}]", entries.join(","));
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `--emit-map`: prints, as a JSON array on stdout, each input file's
/// emitted TypeScript and the source<->output byte mappings of every chunk
/// copied verbatim from the source (`ttc::emit_mapped`). Parse + emit only —
/// no tt-level checks, no verification, `.tt`/`@tt/std` specifiers left
/// untouched — so a buffer mid-edit still emits. This is the feed for the
/// language server's virtual TypeScript documents (TASK-050).
fn emit_map_mode(jobs: &[Job]) -> ExitCode {
    let mut entries: Vec<String> = Vec::new();
    let mut failed = false;
    for job in jobs {
        let filename = job.file.display().to_string();
        let source = match fs::read_to_string(&job.file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ttc: {filename}: {e}");
                failed = true;
                continue;
            }
        };
        let mapped = ttc::emit_mapped(&source);
        let mappings = mapped
            .mappings
            .iter()
            .map(|m| format!("{{\"src\":{},\"out\":{},\"len\":{}}}", m.src, m.out, m.len))
            .collect::<Vec<_>>()
            .join(",");
        entries.push(format!(
            "{{\"file\":{},\"code\":{},\"mappings\":[{}]}}",
            json_str(&filename),
            json_str(&mapped.code),
            mappings
        ));
    }
    println!("[{}]", entries.join(","));
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `--sidecar <dir>`: writes `<name>.tt.d.ts` and `<name>.tt.d.ts.map` next
/// to each input `.tt`, from the declarations tsc emitted for that module
/// (`<dir>/<name>.d.ts`, produced with `--emitDeclarationOnly` over ttc's
/// output). The map's `sources` is the `.tt` file, so an editor's "go to
/// definition" from a `.ts` importer lands in the original — not in the
/// generated declarations. Compiles nothing.
fn sidecar_mode(jobs: &[Job], decl_dir: &Path) -> ExitCode {
    let mut failed = false;
    for job in jobs {
        let Some(stem) = job
            .file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
        else {
            continue;
        };
        let Some(file_name) = job
            .file
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
        else {
            continue;
        };

        let source = match fs::read_to_string(&job.file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ttc: {}: {e}", job.file.display());
                failed = true;
                continue;
            }
        };
        let decl_path = decl_dir.join(format!("{stem}.d.ts"));
        let declarations = match fs::read_to_string(&decl_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ttc: {}: {e}", decl_path.display());
                failed = true;
                continue;
            }
        };

        // `-o` puts the declarations in their own tree (mirroring the input
        // layout); without it they sit next to the source.
        let dts_path = job.out_path.with_file_name(format!("{file_name}.d.ts"));
        let map_path = job.out_path.with_file_name(format!("{file_name}.d.ts.map"));
        let dir = dts_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("ttc: {}: {e}", dir.display());
            failed = true;
            continue;
        }

        // The map's `sources` is read relative to the map itself, so it has
        // to point back across whatever distance `-o` introduced.
        let sidecar = ttc::build_sidecar(&source, &declarations, &relative_path(&dir, &job.file));
        if let Err(e) = fs::write(&dts_path, &sidecar.declarations) {
            eprintln!("ttc: {}: {e}", dts_path.display());
            failed = true;
            continue;
        }
        if let Err(e) = fs::write(&map_path, &sidecar.map) {
            eprintln!("ttc: {}: {e}", map_path.display());
            failed = true;
            continue;
        }
        eprintln!("ttc: {} → {}", job.file.display(), dts_path.display());
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Path from `from_dir` to `to_file`, `/`-separated — the form a source map
/// needs for its `sources`.
fn relative_path(from_dir: &Path, to_file: &Path) -> String {
    // Canonicalize both or neither: an output directory may not exist yet,
    // and mixing an absolute path with a relative one yields nonsense.
    let (from, to) = match (from_dir.canonicalize(), to_file.canonicalize()) {
        (Ok(from), Ok(to)) => (from, to),
        _ => (from_dir.to_path_buf(), to_file.to_path_buf()),
    };

    let from_parts: Vec<_> = from.components().collect();
    let to_parts: Vec<_> = to.components().collect();
    let shared = from_parts
        .iter()
        .zip(&to_parts)
        .take_while(|(a, b)| a == b)
        .count();

    let mut parts: Vec<String> = vec!["..".to_string(); from_parts.len() - shared];
    parts.extend(
        to_parts[shared..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().to_string()),
    );
    if parts.is_empty() {
        return ".".to_string();
    }
    parts.join("/")
}

fn enums_json(source: &str, symbols: &[EnumSymbol]) -> String {
    let objects = symbols
        .iter()
        .map(|e| {
            let (line, col) = ttc::line_col(source, e.offset);
            let cases = e
                .cases
                .iter()
                .map(|c| {
                    let (line, col) = ttc::line_col(source, c.offset);
                    let fields = match &c.fields {
                        None => "null".to_string(),
                        Some(fields) => format!(
                            "[{}]",
                            fields
                                .iter()
                                .map(|f| format!(
                                    "{{\"name\":{},\"optional\":{},\"type\":{}}}",
                                    json_str(&f.name),
                                    f.optional,
                                    json_str(&f.ty)
                                ))
                                .collect::<Vec<_>>()
                                .join(",")
                        ),
                    };
                    format!(
                        "{{\"tag\":{},\"line\":{line},\"col\":{col},\"fields\":{fields}}}",
                        json_str(&c.tag)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"name\":{},\"exported\":{},\"generics\":{},\"line\":{line},\"col\":{col},\"cases\":[{cases}]}}",
                json_str(&e.name),
                e.exported,
                json_str(&e.generics)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", objects.join(","))
}

fn names_json(names: &TtImportNames) -> String {
    match names {
        TtImportNames::Namespace(ns) => {
            format!("{{\"kind\":\"namespace\",\"name\":{}}}", json_str(ns))
        }
        TtImportNames::Named(entries) => format!(
            "{{\"kind\":\"named\",\"entries\":[{}]}}",
            entries
                .iter()
                .map(|(name, alias)| format!(
                    "{{\"name\":{},\"alias\":{}}}",
                    json_str(name),
                    alias.as_deref().map_or("null".to_string(), json_str)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        TtImportNames::None => "{\"kind\":\"none\"}".to_string(),
    }
}

/// Minimal JSON string encoding (quotes, backslashes, control characters).
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Declaration tables of the `.tt` modules a run imports, shared by every
/// job.
///
/// The same module is typically imported by many files, and each import
/// used to mean another disk read and another full parse of that module.
/// Here every module is read and parsed at most once per run, and modules
/// that are themselves inputs are served from the sources the run already
/// holds — no second read at all.
struct ExternCache<'a> {
    /// The run's own input sources, keyed by path.
    inputs: HashMap<&'a Path, &'a str>,
    /// Exported declarations per imported path, filled on first use. An
    /// unreadable module caches as an empty table: module resolution is
    /// tsc's domain (`TS2307`), so its enums simply stay unknown.
    decls: Mutex<HashMap<PathBuf, Arc<Vec<ExternEnum>>>>,
}

impl<'a> ExternCache<'a> {
    fn new(inputs: HashMap<&'a Path, &'a str>) -> Self {
        ExternCache {
            inputs,
            decls: Mutex::new(HashMap::new()),
        }
    }

    fn exported_enums(&self, path: &Path) -> Arc<Vec<ExternEnum>> {
        if let Some(hit) = self.decls.lock().expect("extern cache").get(path) {
            return Arc::clone(hit);
        }
        // Parsed outside the lock: a slow miss must not stall other jobs.
        // Two jobs racing on the same module both parse it once; the first
        // insertion wins and both see the same table.
        let source_kind = ttc::SourceKind::from_path(path).unwrap_or_default();
        let decls = Arc::new(match self.inputs.get(path) {
            Some(source) => ttc::exported_enums_with_kind(source, source_kind),
            None => match fs::read_to_string(path) {
                Ok(source) => ttc::exported_enums_with_kind(&source, source_kind),
                Err(_) => Vec::new(),
            },
        });
        Arc::clone(
            self.decls
                .lock()
                .expect("extern cache")
                .entry(path.to_path_buf())
                .or_insert(decls),
        )
    }
}

/// Collects enum declarations from the file's direct relative `.tt`
/// imports, so matches over imported enums get exhaustiveness-checked
/// (module graph phase 2). One hop, import declarations only — re-exports
/// bring nothing into scope. A specifier that cannot be read is skipped
/// silently: module resolution is tsc's domain (`TS2307`), and an unknown
/// enum simply stays unchecked, exactly as before.
fn collect_extern_enums(file: &Path, imports: &[TtImport], cache: &ExternCache) -> Vec<ExternEnum> {
    let dir = file.parent().unwrap_or(Path::new("."));
    let mut externs: Vec<ExternEnum> = Vec::new();
    for import in imports {
        if matches!(import.names, TtImportNames::None) {
            continue;
        }
        let decls = cache.exported_enums(&dir.join(&import.specifier));
        let from = Some(import.specifier.clone());
        match &import.names {
            TtImportNames::Namespace(ns) => {
                externs.extend(decls.iter().map(|d| ExternEnum {
                    name: format!("{ns}.{}", d.name),
                    tags: d.tags.clone(),
                    from: from.clone(),
                }));
            }
            TtImportNames::Named(entries) => {
                for (name, alias) in entries {
                    if let Some(d) = decls.iter().find(|d| &d.name == name) {
                        externs.push(ExternEnum {
                            name: alias.clone().unwrap_or_else(|| name.clone()),
                            tags: d.tags.clone(),
                            from: from.clone(),
                        });
                    }
                }
            }
            TtImportNames::None => unreachable!(),
        }
    }
    externs
}

/// One input, read and scanned once for the whole run — or the diagnostic
/// its read failed with.
struct Loaded {
    source: String,
    scan: ModuleScan,
}

/// Reads and scans every job's source, in parallel.
fn load_jobs(jobs: &[Job], jobs_limit: Option<usize>) -> Vec<Result<Loaded, String>> {
    par_map(jobs, jobs_limit, |job| {
        let source =
            fs::read_to_string(&job.file).map_err(|e| format!("{}: {e}", job.file.display()))?;
        let scan = ttc::scan_module_with_kind(
            &source,
            ttc::SourceKind::from_path(&job.file).unwrap_or_default(),
        );
        Ok(Loaded { source, scan })
    })
}

/// How many worker threads a parallel phase should use: the `--jobs` value
/// when given, otherwise one per available core. Never more than there is
/// work for.
fn worker_count(items: usize, jobs_limit: Option<usize>) -> usize {
    let want = jobs_limit.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });
    want.clamp(1, items.max(1))
}

/// Maps `f` over `items` across worker threads, returning the results in
/// input order — so diagnostics and outputs stay byte-identical to a
/// sequential run whatever order the work actually finished in.
///
/// Compilation is per-file and shares nothing mutable, which is what makes
/// this the compiler's main lever on large trees; the ordered result is
/// what keeps the CLI deterministic.
fn par_map<T, R>(items: &[T], jobs_limit: Option<usize>, f: impl Fn(&T) -> R + Sync) -> Vec<R>
where
    T: Sync,
    R: Send,
{
    let workers = worker_count(items.len(), jobs_limit);
    if workers <= 1 || items.len() <= 1 {
        return items.iter().map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let f = &f;
    let batches: Vec<Vec<(usize, R)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let next = &next;
                scope.spawn(move || {
                    let mut done: Vec<(usize, R)> = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        match items.get(i) {
                            Some(item) => done.push((i, f(item))),
                            None => return done,
                        }
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    });
    let mut slots: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
    for (i, r) in batches.into_iter().flatten() {
        slots[i] = Some(r);
    }
    slots
        .into_iter()
        .map(|r| r.expect("every index is produced exactly once"))
        .collect()
}

/// `ttc --check-types` / `ttc --types` as an engine consumer: open the
/// project once, take a snapshot per pass, print what the check found. The
/// engine owns the state (documents, projections, the running compiler);
/// this driver owns the terminal — wording, order and exit codes are the
/// CLI's contract.
fn typed_check_mode(inputs: &[String], options: &TypedCheckOptions<'_>) -> ExitCode {
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
struct TypedCheckOptions<'a> {
    project: Option<&'a Path>,
    node: Option<&'a Path>,
    /// Where the sidecars go, in the mode that writes them.
    out_dir: Option<&'a Path>,
    /// `--types`: emit declarations and write them. `--check-types` does not.
    emit: bool,
    watch: bool,
    /// `--overlay`: unsaved text standing in for a file on disk, keyed by
    /// canonical path.
    overlay: &'a std::collections::HashMap<PathBuf, String>,
    /// `--tt-only`: report only the tt layer.
    tt_only: bool,
    /// The raw inputs, for mirroring their layout under `-o`.
    inputs: &'a [String],
}

/// What one pass printed.
struct TypedReport {
    /// How many diagnostics were printed. Zero is the only passing result.
    reported: usize,
    /// Whether the pass could not run at all — see [`ttc::engine::Blocked`].
    blocked: bool,
}

/// One snapshot, one check, everything printed.
fn typed_pass(
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
                ttc::render::compile_error(&blocked.error, None, &shown(&blocked.path))
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
            )
        );
    }

    // A backend that could not run is the pass failing to *run*, not the
    // code failing the check — the tt diagnostics above are complete, the
    // typed layer is missing, and the exit code says "could not check".
    if let Some(error) = &checked.backend_error {
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
fn typed_watch(project: &mut ttc::engine::Project, options: &TypedCheckOptions<'_>) -> ExitCode {
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
fn write_declarations(
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
fn shown(path: &Path) -> String {
    let relative = std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf));
    relative
        .unwrap_or_else(|| path.to_path_buf())
        .display()
        .to_string()
}

fn main() -> ExitCode {
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
fn run() -> ExitCode {
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

/// Everything the compile step needs beyond the file list.
/// `--source-map`: whether the build writes a Source Map v3 for each
/// compiled file, and where it goes.
///
/// The default is [`SourceMapMode::Off`]. Emitting a map appends a
/// `//# sourceMappingURL=` line to the output, and a hand-written `.ts`
/// passes through byte for byte by contract — so a map is something a
/// build asks for, never something it gets by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SourceMapMode {
    #[default]
    Off,
    /// A `<output>.map` beside the output, named by a relative URL.
    File,
    /// A `data:` URL carrying the map, so the output is self-contained —
    /// what a bundler plugin reading `ttc -p` needs.
    Inline,
}

struct BuildOptions {
    banner: bool,
    print: bool,
    check: bool,
    verify: bool,
    rewrite_imports: ImportRewrite,
    source_map: SourceMapMode,
    /// Output root, when `-o` was given — also where the standard library
    /// module is written if an input imports it.
    out_dir: Option<PathBuf>,
    /// Worker threads for the parallel phases (`--jobs`); `None` means one
    /// per available core.
    jobs: Option<usize>,
}

/// Where the generated `tt/` standard-library package goes.
fn std_placement(jobs: &[Job], needed: bool, out_dir: Option<&Path>) -> Option<PathBuf> {
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
fn common_ancestor(jobs: &[Job]) -> Option<PathBuf> {
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
fn std_specifier(
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
fn build_jobs(
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
                        PathBuf::from(out_name.file_name().unwrap())
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
fn same_file(a: &Path, b: &Path) -> bool {
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
struct Outcome {
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
fn compile_jobs(jobs: &[Job], opts: &BuildOptions) -> bool {
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
                let extern_enums = collect_extern_enums(&job.file, &loaded.scan.imports, &cache);
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
                    extern_enums: &extern_enums,
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
                            ttc::render::diagnostic(diagnostic, &loaded.source, &filename)
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
                    let base = job.file.file_name().unwrap().to_string_lossy();
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

/// Writes one compiled output, creating its directory. The error is already
/// formatted as a diagnostic line.
/// A built map, ready to attach: the `//# sourceMappingURL=` line the
/// output ends with, and the document to write beside it (`None` when the
/// comment carries the map itself).
struct RenderedSourceMap {
    comment: String,
    document: Option<String>,
}

/// The map file's path: the output's, with `.map` appended — which is what
/// the relative URL in the comment names.
fn map_path(out_path: &Path) -> PathBuf {
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
fn source_map_for(
    job: &Job,
    emit: &ttc::MappedEmit,
    source: &str,
    opts: &BuildOptions,
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
            generated_line_offset: usize::from(opts.banner),
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
fn relative_to(path: &Path, base: &Path) -> String {
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
fn lexical_absolute(path: &Path) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let rooted = path.is_absolute();
    let prefix = if rooted {
        Vec::new()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect()
    };
    for component in prefix.into_iter().chain(
        path.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    ) {
        match component.as_str() {
            "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(component),
        }
    }
    parts
}

fn write_output(out_path: &Path, code: &str) -> Result<(), String> {
    if let Some(parent) = out_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return Err(format!("ttc: {}: {e}", parent.display()));
    }
    fs::write(out_path, code).map_err(|e| format!("ttc: {}: {e}", out_path.display()))
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
fn watch_mode(inputs: &[String], out_dir: Option<&Path>, opts: &BuildOptions) -> ExitCode {
    let mut stamps: HashMap<PathBuf, SystemTime> = HashMap::new();
    let mut first = true;

    loop {
        let jobs = match build_jobs(inputs, out_dir, true) {
            Ok(jobs) => jobs,
            // An input can disappear mid-edit; keep watching rather than
            // tearing the session down.
            Err(_) => {
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
            eprintln!(
                "ttc: {} file(s) {} — watching",
                selected.len(),
                if failed { "failed" } else { "ok" }
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
const TYPES_DIR: &str = ".tt-types";

/// The file's path relative to whichever input directory contains it, so
/// the sidecar tree mirrors the source tree rather than the whole cwd.
fn input_relative(file: &Path, inputs: &[String]) -> PathBuf {
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
fn with_dependents(jobs: &[Job], changed: &[PathBuf]) -> HashSet<PathBuf> {
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
mod help_tests {
    use super::*;

    #[test]
    fn every_topic_resolves_to_a_nonempty_section() {
        for (name, _, heading) in HELP_TOPICS {
            let section = guide_section(heading);
            assert!(
                !section.trim().is_empty(),
                "topic {name}: heading {heading:?} not found in docs/ai/tt.md"
            );
            if !heading.is_empty() {
                assert!(section.starts_with(heading), "topic {name}: wrong slice");
            }
        }
    }

    #[test]
    fn sections_stop_at_the_next_heading() {
        let section = guide_section("## match");
        assert!(section.contains("or-pattern"));
        assert!(!section.contains("\n## try"), "section leaked past its end");
        let preamble = guide_section("");
        assert!(preamble.contains("CONTRACTS"));
        assert!(!preamble.contains("\n## "));
    }

    #[test]
    fn topic_names_and_aliases_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for (name, aliases, _) in HELP_TOPICS {
            assert!(seen.insert(*name), "duplicate topic {name}");
            for alias in *aliases {
                assert!(seen.insert(*alias), "duplicate alias {alias}");
            }
        }
        assert!(!seen.contains("all") && !seen.contains("guide"));
    }
}
