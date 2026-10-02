//! Programs that are tt must compile, and what comes out must be
//! TypeScript. The body is [`ttc_fuzz::generated_tt_compiles`], which
//! `tests/fuzz_regressions.rs` also replays over `fuzz/regressions/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ttc_fuzz::generated_tt_compiles(data));
