//! Arbitrary text through the compiler: **it may reject, it may not
//! crash.**
//!
//! TASK-214 made a panic report itself as a compiler bug and let the entry
//! points survive it. That is the safety net, not the absence of the
//! accident — the accident is still a `ttc` bug, and this target is what
//! finds one without waiting for a user to type it.
//!
//! Every `unwrap`, index and slice in the lexer, parser, HIR, resolver,
//! sema and codegen is reachable from here: the input is whatever bytes
//! libFuzzer is holding, and the only claim is that the compiler answers
//! rather than aborts. `ttc::compile` does not catch unwinds — only the CLI
//! and the server do — so a panic reaches libFuzzer as a crash.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ttc::{Options, SourceKind};

fuzz_target!(|data: &[u8]| {
    // Not UTF-8 is not a compiler question: the CLI reads files as text and
    // a non-text file never reaches this layer.
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    // Both source kinds: `.ttx` takes a different path through the scanner
    // (JSX is claimed by text, not by tokens), so half the interesting
    // states are only reachable with it.
    for kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let options = Options {
            source_kind: kind,
            ..Options::default()
        };
        // `analyze` and `compile` walk different amounts of the pipeline —
        // `analyze` stops after the semantic passes, `compile` goes on to
        // emission and the output self-check.
        let _ = std::hint::black_box(ttc::analyze(source, &options));
        let _ = std::hint::black_box(ttc::compile(source, &options));
    }
});
