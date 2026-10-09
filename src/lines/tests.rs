use super::*;

const LS: &str = "\u{2028}";
const PS: &str = "\u{2029}";

fn starts(text: &str, breaks: LineBreaks) -> Vec<usize> {
    let map = LineMap::new(text, breaks);
    (0..map.len())
        .map(|line| map.line_start(line).unwrap())
        .collect()
}

#[test]
fn every_line_break_style_starts_a_line_under_ecma() {
    for (text, expected) in [
        ("a\nb\nc", vec![0, 2, 4]),
        ("a\r\nb\r\nc", vec![0, 3, 6]),
        ("a\rb\rc", vec![0, 2, 4]),
        ("a\u{2028}b\u{2028}c", vec![0, 4, 8]),
        ("a\u{2029}b\u{2029}c", vec![0, 4, 8]),
        ("a\r\nb\rc\nd\u{2028}e\u{2029}f", vec![0, 3, 5, 7, 11, 15]),
        ("\n\r\n\r", vec![0, 1, 3, 4]),
        ("", vec![0]),
    ] {
        assert_eq!(starts(text, LineBreaks::Ecma), expected, "{text:?}");
    }
}

#[test]
fn the_protocol_breaks_lines_at_lf_crlf_and_cr_only() {
    let text = format!("a\r\nb\rc\nd{LS}e{PS}f");
    assert_eq!(starts(&text, LineBreaks::Lsp), vec![0, 3, 5, 7]);
    let map = LineMap::lsp(&text);
    assert_eq!(map.line_text(3), Some(format!("d{LS}e{PS}f").as_str()));
}

#[test]
fn line_text_leaves_out_the_break() {
    let text = "one\r\ntwo\rthree\u{2028}four";
    let map = LineMap::ecma(text);
    let lines: Vec<_> = (0..map.len())
        .map(|line| map.line_text(line).unwrap())
        .collect();
    assert_eq!(lines, ["one", "two", "three", "four"]);
    assert_eq!(map.line_end(0), Some(5));
    assert_eq!(map.line_end(3), Some(text.len()));
    assert_eq!(map.line_text(4), None);
    assert_eq!(map.line_end(4), None);
}

#[test]
fn positions_follow_the_policy_the_consumer_names() {
    let cr = "export {};\rval const b = { c: 1 };\rb.c = 2;\r";
    let at = cr.find("b.c").unwrap();
    assert_eq!(LineMap::ecma(cr).char_position(at), (2, 0));
    assert_eq!(LineMap::lsp(cr).utf16_position(at), (2, 0));
    assert_eq!(line_col(cr, at), (3, 1));

    let separated = format!("export {{}};{LS}val const b = {{ c: 1 }};{PS}b.c = 2;\n");
    let at = separated.find("b.c").unwrap();
    assert_eq!(LineMap::ecma(&separated).char_position(at), (2, 0));
    assert_eq!(
        LineMap::lsp(&separated).utf16_position(at),
        (0, separated[..at].encode_utf16().count())
    );
}

#[test]
fn a_crlf_pair_is_one_break_and_its_lf_belongs_to_the_line_it_ends() {
    let text = "ab\r\ncd";
    let map = LineMap::ecma(text);
    assert_eq!(map.char_position(2), (0, 2));
    assert_eq!(map.char_position(3), (0, 3));
    assert_eq!(map.char_position(4), (1, 0));
    assert_eq!(map.line_of(3), 0);
}

#[test]
fn columns_count_utf16_units_or_code_points() {
    let text = "x\n\u{1F389}é = 1;";
    let at = text.find('=').unwrap();
    let map = LineMap::ecma(text);
    assert_eq!(map.utf16_position(at), (1, 4));
    assert_eq!(map.char_position(at), (1, 3));
    assert_eq!(map.utf16_offset(1, 4), at);
    assert_eq!(map.char_offset(1, 3), Some(at));
    assert_eq!(map.utf16_offset(1, 1), text.find('\u{1F389}').unwrap());
}

#[test]
fn a_byte_order_mark_is_not_a_column() {
    let text = "\u{feff}ab\rcd";
    let map = LineMap::ecma(text);
    assert_eq!(map.line_start(0), Some(3));
    assert_eq!(map.line_text(0), Some("ab"));
    assert_eq!(map.char_position(0), (0, 0));
    assert_eq!(map.char_position(4), (0, 1));
    assert_eq!(map.utf16_position(text.find('d').unwrap()), (1, 1));
    assert_eq!(map.utf16_offset(0, 0), 3);
    assert_eq!(line_col(text, 0), (1, 1));
}

#[test]
fn offsets_inside_a_code_point_or_past_the_end_clamp() {
    let text = "a\u{2028}b";
    let map = LineMap::ecma(text);
    assert_eq!(map.char_position(2), (0, 1));
    assert_eq!(map.char_position(99), (1, 1));
    let astral = "\u{1F389}x";
    assert_eq!(LineMap::lsp(astral).utf16_position(2), (0, 0));
}

#[test]
fn protocol_positions_past_a_line_or_the_text_clamp_as_lsp_says() {
    let text = "ab\r\ncd\n";
    let map = LineMap::lsp(text);
    assert_eq!(map.utf16_offset(0, 99), 2);
    assert_eq!(map.utf16_offset(1, 1), 5);
    assert_eq!(map.utf16_offset(9, 0), text.len());
    assert_eq!(map.char_offset(0, 2), Some(2));
    assert_eq!(map.char_offset(0, 3), None);
    assert_eq!(map.char_offset(9, 0), None);
}

#[test]
fn compiler_positions_become_protocol_positions_through_their_byte() {
    let text = format!("\u{feff}a{LS}\u{1F389}b\rc");
    let positions = ProtocolPositions::new(&text);
    let b = text.find('b').unwrap();
    assert_eq!(line_col(&text, b), (2, 2));
    assert_eq!(positions.of_position((2, 2)), (1, 5));
    assert_eq!(positions.of_byte(b), (1, 5));
    assert_eq!(positions.of_position((3, 1)), (2, 1));
    assert_eq!(positions.of_position((0, 0)), (0, 0));
    assert_eq!(positions.of_position((2, 9)), (2, 9));
    assert_eq!(positions.of_position((9, 1)), (9, 1));
}

#[test]
fn a_text_ends_a_line_only_after_a_whole_break() {
    for (text, expected) in [
        ("a\n", true),
        ("a\r\n", true),
        ("a\r", true),
        ("a\u{2028}", true),
        ("a\u{2029}", true),
        ("a", false),
        ("", false),
        ("\u{feff}", false),
        ("\u{03A8}", false),
    ] {
        assert_eq!(ends_with_line_break(text), expected, "{text:?}");
    }
}

#[test]
fn walking_back_agrees_with_the_measured_lines() {
    let texts = [
        "a\nb\r\nc\rd".to_string(),
        format!("\u{feff}x{LS}\u{1F389}y{PS}\r\n\n\rz"),
        "\u{03A8}\u{00A9}\r\n".to_string(),
        String::new(),
    ];
    for text in &texts {
        let map = LineMap::ecma(text);
        for at in 0..=text.len() {
            let expected = map.line_start(map.line_of(at)).unwrap();
            assert_eq!(line_start_before(text, at), expected, "{text:?} at {at}");
        }
    }
}

#[test]
fn a_utf16_map_answers_what_scanning_the_text_answers() {
    for text in [
        "",
        "abc",
        "\u{feff}abc",
        "a\u{e9}b",
        "\u{1f389}ab",
        "\u{feff}x\u{1f389}\u{2028}\u{e9}y\u{10ffff}",
        "\u{e9}\u{e9}\u{1f389}\u{1f389}",
    ] {
        let map = Utf16Map::new(text);
        for byte in 0..=text.len() + 2 {
            assert_eq!(
                map.to_utf16(byte),
                crate::typescript::mapper::to_utf16(text, byte),
                "{text:?} byte {byte}"
            );
        }
        for units in 0..=text.encode_utf16().count() + 2 {
            assert_eq!(
                map.to_byte(units),
                crate::typescript::mapper::from_utf16(text, units),
                "{text:?} units {units}"
            );
        }
    }
}

#[test]
fn columns_on_one_long_ascii_line_take_constant_work_per_position() {
    let ask = |count: usize| {
        let text = "export const v = h(1) + \"ab\".length; ".repeat(count);
        crate::work::measure(|| {
            let map = LineMap::lsp(&text);
            for at in (0..text.len()).step_by(16) {
                let _ = map.utf16_position(at);
                let _ = map.char_position(at);
                let _ = map.utf16_offset(0, at);
                let _ = map.char_offset(0, at);
            }
        })
    };
    let small = ask(200).get("column scan bytes").copied().unwrap_or(0);
    let large = ask(400).get("column scan bytes").copied().unwrap_or(0);
    assert!(
        large <= 2 * small + 64,
        "column scan bytes: {small} for n statements on one line but {large} for 2n"
    );
}
