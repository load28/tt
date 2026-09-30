//! Arbitrary text through every untyped pipeline: it may reject, it may not
//! crash. The body is [`ttc_fuzz::compile_any_bytes`], which
//! `tests/fuzz_regressions.rs` also replays over `fuzz/regressions/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| ttc_fuzz::compile_any_bytes(data));
