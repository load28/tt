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

#[path = "main/build.rs"]
mod build;
#[path = "main/command.rs"]
mod command;
mod content_mapper;
#[path = "main/loading.rs"]
mod loading;
#[path = "main/modes.rs"]
mod modes;
#[path = "main/out.rs"]
mod out;
#[path = "main/output.rs"]
mod output;
mod server;
#[path = "main/typed.rs"]
mod typed;

#[cfg(test)]
#[path = "main/tests.rs"]
mod help_tests;

use ttc::engine::collect_sources;
use ttc::source_map::SourceMapRequest;
use ttc::{
    ExternVariant, ImportRewrite, ModuleScan, Options, StdImports, StdModule, TtImport,
    TtImportNames, VariantSymbol, compile_report,
};

use build::*;
use loading::*;
use modes::*;
use output::*;
use typed::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn usage() {
    out::line(&format!(
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
  -p, --print           print exactly one compiled source to stdout instead
                        of writing (--source-map supports off or inline)
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
  --symbols             print tt variant declarations (with positions) and the
                        direct .tt imports of each input as JSON; compiles
                        nothing (for language tooling)
  --emit-map            print each input's emitted TypeScript plus source<->
                        output byte mappings as JSON; parse + emit only (no
                        tt-level checks, .tt specifiers untouched) — the
                        editor's virtual-document feed (for language tooling)
  --server              keep the engine alive and answer check/emitMap/
                        typedCheck requests as JSON lines on stdin/stdout,
                        reusing one project session per project — the same
                        answers as the one-shot modes, without the startup
  --content-mapper      serve .tt/.ttx to TypeScript 7.1+ as a content
                        mapper process (JSON-RPC on stdin/stdout) — the
                        mode `contentMappers` entries in tsconfig.json and
                        the editor integration spawn; not for direct use
  --overlay <path>      check the buffer on stdin as if it were <path>, so an
                        editor can ask about text it has not saved; needs
                        --check-types or --types
  --tt-only             report the tt layer of --check-types/--types and
                        leave the type layer to TypeScript"
    ));
}

/// The language & workflow guide (docs/ai/tt.md), embedded so `ttc help`
/// serves documentation offline. The file is the source of truth; `##`
/// headings are the topic boundaries.
const GUIDE: &str = include_str!("../docs/ai/tt.md");

/// Help topics: (name, aliases, `##` heading prefix in GUIDE). An empty
/// prefix selects the preamble (everything before the first `## `).
const HELP_TOPICS: &[(&str, &[&str], &str)] = &[
    ("overview", &["contracts", "intro"], ""),
    ("variant", &["variants"], "## variant"),
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
            out::line(&format!(
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
            ));
            return ExitCode::SUCCESS;
        }
        [topic] => topic.to_lowercase(),
        _ => {
            eprintln!("ttc: help takes at most one topic (run `ttc help` for the list)");
            return ExitCode::FAILURE;
        }
    };
    if topic == "all" || topic == "guide" {
        out::text(GUIDE);
        return ExitCode::SUCCESS;
    }
    let found = HELP_TOPICS
        .iter()
        .find(|(name, aliases, _)| *name == topic || aliases.contains(&topic.as_str()));
    match found {
        Some((_, _, heading)) => {
            out::text(guide_section(heading));
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
            out::line("ttc explain <code> — what a diagnostic's rule is and why\n");
            out::line("Codes:");
            for code in ttc::DiagnosticCode::ALL {
                out::line(&format!("  {}", code.as_str()));
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
            out::line(&format!("error[{}]\n", code.as_str()));
            out::line(code.explanation());
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

fn main() -> ExitCode {
    command::entry()
}
