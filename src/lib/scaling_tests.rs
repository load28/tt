use std::collections::HashMap;
use std::path::Path;

use crate::work::measure;

fn statement_matches(count: usize) -> String {
    (0..count)
        .map(|i| format!("function f{i}(s: number) {{ match (s) {{ 1 => 1, _ => 2 }} }}\n"))
        .collect()
}

fn expression_matches(count: usize) -> String {
    (0..count)
        .map(|i| format!("const v{i} = (s: number) => match (s) {{ 1 => 1, _ => 2 }};\n"))
        .collect()
}

fn every_request(source: &str) {
    let path = Path::new("/scaling/main.tt");
    crate::compile(source, &crate::Options::default()).expect("the file compiles");
    crate::engine::semantic_tokens(source);
    crate::engine::tt_declarations(path, source);
    crate::engine::tt_hints(path, source);
}

fn assert_linear(small: &HashMap<&'static str, usize>, large: &HashMap<&'static str, usize>) {
    for (name, &work) in large {
        let before = small.get(name).copied().unwrap_or(0);
        assert!(
            work <= 2 * before + 64,
            "{name}: {before} units for n matches but {work} for 2n"
        );
    }
}

#[test]
fn every_request_does_linear_work_in_the_number_of_statement_matches() {
    let small = measure(|| every_request(&statement_matches(150)));
    let large = measure(|| every_request(&statement_matches(300)));
    assert_linear(&small, &large);
    assert!(large["generated name probes"] > 0);
    assert!(large["denied token visits"] > 0);
}

#[test]
fn every_request_does_linear_work_in_the_number_of_expression_matches() {
    let small = measure(|| every_request(&expression_matches(150)));
    let large = measure(|| every_request(&expression_matches(300)));
    assert_linear(&small, &large);
    assert!(large["concise arrow scans"] > 0);
}

#[test]
fn tt_matches_never_need_more_host_parses_as_they_multiply() {
    let small = measure(|| crate::parser::parse(&statement_matches(150)));
    let large = measure(|| crate::parser::parse(&statement_matches(300)));
    assert_eq!(small.get("host parses"), large.get("host parses"));
}

#[test]
fn many_utf16_offsets_answer_what_one_offset_answers() {
    for source in [
        "",
        "abc",
        "\u{feff}const 한 = \"🎉\";\nx",
        "🎉ab",
        "a\u{feff}b",
    ] {
        let offsets = crate::Utf16Offsets::new(source);
        for byte in 0..source.len() + 3 {
            assert_eq!(
                offsets.offset(byte),
                crate::utf16_offset(source, byte),
                "{source:?} at {byte}"
            );
        }
    }
}
