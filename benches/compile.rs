//! The ruler for compile time and editor latency.
//!
//! Three numbers, because tt's performance story has three parts:
//!
//! 1. **`single_file`** — how fast one file goes from tt text to
//!    TypeScript. What a build's throughput is made of.
//! 2. **`project_first_snapshot`** — opening a project and projecting
//!    every file once. What an editor pays when a workspace opens.
//! 3. **`project_recheck_one_file`** — one file edited, the project
//!    re-snapshotted. What an editor pays on **every keystroke**, and the
//!    number the `Engine → Project → Snapshot` design exists to keep small:
//!    an unchanged file is meant to cost a reference, not a re-projection.
//!    The bench asserts that reuse actually happens as well as timing it,
//!    because a fast wrong answer here would be a cache that never hit.
//!
//! No measurement framework: this reports its own noise, which is the
//! number a regression threshold has to be set from, and a compiler with
//! five dependencies should not grow thirty to own a stopwatch.
//!
//! ```sh
//! cargo bench                       # human-readable table
//! cargo bench -- --json             # one JSON object, for a comparison
//! TT_BENCH_ITERS=50 cargo bench     # more samples, less noise
//! ```
//!
//! Comparing two revisions is `scripts/bench-compare`, which builds both on
//! one machine — a baseline recorded on a different machine says nothing.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use ttc::Options;
use ttc::engine::{Engine, ProjectOptions};

/// How many timed iterations each case runs by default. Enough for a
/// median to be stable on a shared runner without making the suite a
/// coffee break.
const DEFAULT_ITERS: usize = 25;

/// Iterations run before timing starts, to pay for whatever the first pass
/// warms (allocator arenas, file cache, branch predictors).
const WARMUP: usize = 3;

/// How many files the project cases build. Big enough that per-file work
/// dominates the fixed cost of opening a project, small enough to run on a
/// laptop between edits.
const PROJECT_FILES: usize = 40;

fn main() {
    let json = std::env::args().any(|a| a == "--json");
    let iters = std::env::var("TT_BENCH_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_ITERS);

    let results = vec![
        single_file(iters),
        project_first_snapshot(iters),
        project_recheck_one_file(iters),
    ];

    if json {
        let body: Vec<String> = results.iter().map(Report::to_json).collect();
        println!("{{\"benchmarks\":[{}]}}", body.join(","));
        return;
    }
    println!(
        "{:<28} {:>10} {:>10} {:>10} {:>8}",
        "case", "median", "min", "p90", "noise"
    );
    for report in &results {
        println!(
            "{:<28} {:>10} {:>10} {:>10} {:>7.1}%",
            report.name,
            micros(report.median),
            micros(report.min),
            micros(report.p90),
            report.noise * 100.0,
        );
    }
    println!(
        "\n{} iterations each ({WARMUP} warmup). \"noise\" is the median \
         absolute deviation over the median — the spread a regression has \
         to beat to be one.",
        results.first().map_or(0, |r| r.samples),
    );
}

/// One case's timing summary.
struct Report {
    name: &'static str,
    samples: usize,
    median: Duration,
    min: Duration,
    p90: Duration,
    /// Median absolute deviation, relative to the median. The case's own
    /// noise floor on this machine.
    noise: f64,
}

impl Report {
    fn of(name: &'static str, mut samples: Vec<Duration>) -> Report {
        samples.sort_unstable();
        let median = percentile(&samples, 0.5);
        let mut deviations: Vec<Duration> = samples.iter().map(|d| d.abs_diff(median)).collect();
        deviations.sort_unstable();
        let mad = percentile(&deviations, 0.5);
        Report {
            name,
            samples: samples.len(),
            median,
            min: samples[0],
            p90: percentile(&samples, 0.9),
            noise: mad.as_secs_f64() / median.as_secs_f64().max(f64::MIN_POSITIVE),
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"name\":\"{}\",\"samples\":{},\"median_ns\":{},\"min_ns\":{},\
             \"p90_ns\":{},\"noise\":{:.6}}}",
            self.name,
            self.samples,
            self.median.as_nanos(),
            self.min.as_nanos(),
            self.p90.as_nanos(),
            self.noise,
        )
    }
}

fn percentile(sorted: &[Duration], at: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let index = ((sorted.len() - 1) as f64 * at).round() as usize;
    sorted[index]
}

fn micros(d: Duration) -> String {
    format!("{:.1}µs", d.as_secs_f64() * 1e6)
}

/// Times `run` `iters` times after `WARMUP` untimed passes.
fn measure(name: &'static str, iters: usize, mut run: impl FnMut()) -> Report {
    for _ in 0..WARMUP {
        run();
    }
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        let start = Instant::now();
        run();
        samples.push(start.elapsed());
    }
    Report::of(name, samples)
}

/// A file using every construct that costs something to lower, sized like
/// a real module rather than a snippet.
fn module(index: usize) -> String {
    let mut out = String::new();
    out.push_str("import { helper } from \"./helper.js\";\n\n");
    for n in 0..8 {
        out.push_str(&format!(
            "export enum Shape{index}_{n} {{\n\
             \x20 Circle(radius: number),\n\
             \x20 Rect(width: number, height: number),\n\
             \x20 Empty,\n\
             }}\n\n\
             export function area{index}_{n}(s: Shape{index}_{n}): number {{\n\
             \x20 return match (s) {{\n\
             \x20   Circle(radius) => Math.PI * radius ** 2,\n\
             \x20   Rect(width, height) => width * height,\n\
             \x20   Empty => 0,\n\
             \x20 }};\n\
             }}\n\n\
             export const label{index}_{n} = (s: Shape{index}_{n}): string => {{\n\
             \x20 if let Circle(radius) = s {{\n\
             \x20   return radius.toFixed(1);\n\
             \x20 }}\n\
             \x20 return helper(String(s.kind));\n\
             }};\n\n"
        ));
    }
    out
}

fn single_file(iters: usize) -> Report {
    let source = module(0);
    let options = Options::default();
    measure("single_file", iters, || {
        let out = ttc::compile(&source, &options).expect("the benchmark module compiles");
        std::hint::black_box(out);
    })
}

/// A project of [`PROJECT_FILES`] modules on disk, removed when dropped.
struct Workspace {
    dir: PathBuf,
    files: Vec<PathBuf>,
}

impl Workspace {
    fn new(tag: &str) -> Workspace {
        let dir = std::env::temp_dir().join(format!("tt-bench-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a writable temporary directory");
        std::fs::write(
            dir.join("helper.ts"),
            "export function helper(s: string): string { return s; }\n",
        )
        .expect("writable");
        let files = (0..PROJECT_FILES)
            .map(|i| {
                let path = dir.join(format!("module{i}.tt"));
                std::fs::write(&path, module(i)).expect("writable");
                path.canonicalize().expect("readable")
            })
            .collect();
        Workspace { dir, files }
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn open(workspace: &Workspace) -> ttc::engine::Project {
    Engine::new(None)
        .open_project(
            &[workspace.dir.to_string_lossy().to_string()],
            &ProjectOptions::default(),
        )
        .expect("the project opens without a toolchain")
}

fn project_first_snapshot(iters: usize) -> Report {
    let workspace = Workspace::new("first");
    measure("project_first_snapshot", iters, || {
        // A fresh project every time: this case is about the cost of the
        // *first* pass, so it must not measure a warm cache.
        let mut project = open(&workspace);
        let snapshot = project
            .update(&workspace.files)
            .expect("every module projects");
        std::hint::black_box(snapshot);
    })
}

fn project_recheck_one_file(iters: usize) -> Report {
    let workspace = Workspace::new("recheck");
    let mut project = open(&workspace);
    project
        .update(&workspace.files)
        .expect("every module projects");

    let edited = workspace.files[PROJECT_FILES / 2].clone();
    let mut revision = 0usize;
    let report = measure("project_recheck_one_file", iters, || {
        revision += 1;
        // An open document, the way an editor holds an unsaved buffer:
        // the disk is untouched and only this file's text differs.
        project.update_document(
            edited.clone(),
            format!("{}\nexport const revision = {revision};\n", module(0)),
        );
        let snapshot = project.update(&workspace.files).expect("the edit projects");
        std::hint::black_box(snapshot);
    });

    // The design claim, checked rather than assumed: one edited file means
    // one re-projection, and every other file crosses the snapshot
    // boundary as the same `Arc`. A fast number here would mean nothing if
    // the cache never hit.
    revision += 1;
    project.update_document(
        edited.clone(),
        format!("{}\nexport const revision = {revision};\n", module(0)),
    );
    let before = project.update(&workspace.files).expect("projects");
    revision += 1;
    project.update_document(
        edited.clone(),
        format!("{}\nexport const revision = {revision};\n", module(0)),
    );
    let after = project.update(&workspace.files).expect("projects");
    let reused = before
        .files()
        .iter()
        .zip(after.files())
        .filter(|(a, b)| std::sync::Arc::ptr_eq(a, b))
        .count();
    assert_eq!(
        reused,
        PROJECT_FILES - 1,
        "one file changed, so {} projections had to be reused",
        PROJECT_FILES - 1
    );
    report
}
