//! Editor sidecar tests: `.tt.d.ts` + `.tt.d.ts.map`.
//!
//! The map is what makes "go to definition" from a `.ts` importer land in
//! the original `.tt`, so these tests decode it and check the positions
//! rather than comparing opaque VLQ strings.

use ttc::build_sidecar;

const SOURCE: &str = r#"import { pad } from "./format.ts";

/** 알림 한 건. */
export variant Notice {
  Info(text: string),
  Warn(text: string),
}

export function render(notice: Notice): string {
  return match (notice) {
    Info(text) => pad(text, 4),
    Warn(text) => pad(text, 4),
  };
}
"#;

const DECLARATIONS: &str = r#"/** 알림 한 건. */
export type Notice = {
    kind: "Info";
    text: string;
} | {
    kind: "Warn";
    text: string;
};
export declare const Notice: {
    Info: (text: string) => Notice;
    Warn: (text: string) => Notice;
};
export declare function render(notice: Notice): string;
"#;

/// One decoded mapping segment: where a generated position points to.
#[derive(Debug, PartialEq, Eq)]
struct Segment {
    generated_line: usize,
    generated_column: usize,
    source_line: usize,
    source_column: usize,
}

fn decode(mappings: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    let (mut source_line, mut source_column) = (0i64, 0i64);
    for (generated_line, line) in mappings.split(';').enumerate() {
        let mut generated_column = 0i64;
        if line.is_empty() {
            continue;
        }
        for field in line.split(',') {
            let values = decode_vlq(field);
            assert_eq!(values.len(), 4, "segment should carry four fields");
            generated_column += values[0];
            source_line += values[2];
            source_column += values[3];
            out.push(Segment {
                generated_line,
                generated_column: generated_column as usize,
                source_line: source_line as usize,
                source_column: source_column as usize,
            });
        }
    }
    out
}

fn decode_vlq(field: &str) -> Vec<i64> {
    const DIGITS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut value, mut shift) = (0i64, 0);
    for byte in field.bytes() {
        let digit = DIGITS
            .iter()
            .position(|d| *d == byte)
            .expect("base64 digit") as i64;
        value += (digit & 31) << shift;
        if digit & 32 != 0 {
            shift += 5;
            continue;
        }
        let magnitude = value >> 1;
        out.push(if value & 1 == 1 {
            -magnitude
        } else {
            magnitude
        });
        value = 0;
        shift = 0;
    }
    out
}

fn field(map: &str, key: &str) -> String {
    let start = map.find(key).expect("key present") + key.len();
    let rest = &map[start..];
    let end = rest.find(['"']).unwrap_or(rest.len());
    rest[..end].to_string()
}

#[test]
fn declarations_open_with_a_generated_banner() {
    // The file sits next to the source (TypeScript resolves `./x.tt` to a
    // sibling `x.tt.d.ts` and nowhere else), so it says what it is.
    let sidecar = build_sidecar(SOURCE, DECLARATIONS, "notice.tt");
    assert!(
        sidecar
            .declarations
            .starts_with("// @generated from notice.tt by ttc --sidecar"),
        "{}",
        sidecar.declarations
    );
}

#[test]
fn declarations_carry_a_source_mapping_url() {
    let sidecar = build_sidecar(SOURCE, DECLARATIONS, "notice.tt");
    assert!(
        sidecar
            .declarations
            .trim_end()
            .ends_with("//# sourceMappingURL=notice.tt.d.ts.map"),
        "{}",
        sidecar.declarations
    );
}

#[test]
fn an_existing_source_mapping_url_is_replaced_not_duplicated() {
    let input = format!("{DECLARATIONS}//# sourceMappingURL=notice.d.ts.map\n");
    let sidecar = build_sidecar(SOURCE, &input, "notice.tt");
    assert_eq!(sidecar.declarations.matches("sourceMappingURL").count(), 1);
    assert!(!sidecar.declarations.contains("notice.d.ts.map\n//#"));
}

#[test]
fn map_names_the_tt_file_as_its_source() {
    let sidecar = build_sidecar(SOURCE, DECLARATIONS, "notice.tt");
    assert!(
        sidecar.map.contains("\"sources\":[\"notice.tt\"]"),
        "{}",
        sidecar.map
    );
    assert!(
        sidecar.map.contains("\"file\":\"notice.tt.d.ts\""),
        "{}",
        sidecar.map
    );
}

#[test]
fn a_relative_source_path_records_the_distance_but_not_in_the_file_names() {
    // With `-o` the declarations live in their own tree, which TypeScript
    // merges back via `rootDirs`. Only `sources` carries the distance —
    // the map and the banner still name the file itself.
    let sidecar = build_sidecar(SOURCE, DECLARATIONS, "../src/notice.tt");
    assert!(
        sidecar.map.contains("\"sources\":[\"../src/notice.tt\"]"),
        "{}",
        sidecar.map
    );
    assert!(
        sidecar.map.contains("\"file\":\"notice.tt.d.ts\""),
        "{}",
        sidecar.map
    );
    assert!(
        sidecar
            .declarations
            .starts_with("// @generated from notice.tt by ttc --sidecar"),
        "{}",
        sidecar.declarations
    );
    assert!(
        sidecar
            .declarations
            .contains("//# sourceMappingURL=notice.tt.d.ts.map"),
        "{}",
        sidecar.declarations
    );
}

#[test]
fn every_declaration_maps_to_its_position_in_the_tt_source() {
    let sidecar = build_sidecar(SOURCE, DECLARATIONS, "notice.tt");
    let segments = decode(&field(&sidecar.map, "\"mappings\":\""));

    // `export variant Notice` is on source line 4 (zero-based 3), name at
    // column 15. Both the type alias and the constructor const map there.
    let notice: Vec<&Segment> = segments.iter().filter(|s| s.source_line == 3).collect();
    assert!(!notice.is_empty(), "{segments:?}");
    assert!(notice.iter().all(|s| s.source_column == 15), "{notice:?}");
    // Generated lines are shifted by one: line 0 is the @generated banner.
    assert_eq!(
        notice.iter().map(|s| s.generated_line).collect::<Vec<_>>(),
        vec![2, 2, 9, 9],
        "type alias and constructor const both point at the variant"
    );

    // `export function render` is on source line 9 (zero-based 8), name at
    // column 16.
    let render: Vec<&Segment> = segments.iter().filter(|s| s.source_line == 8).collect();
    assert!(!render.is_empty(), "{segments:?}");
    assert!(render.iter().all(|s| s.source_column == 16), "{render:?}");
}

#[test]
fn each_declaration_gets_a_segment_at_its_name_column() {
    // Go-to-definition asks about the column where the name starts; a
    // segment only at column 0 leaves the editor stranded in the .d.ts.
    let sidecar = build_sidecar(SOURCE, DECLARATIONS, "notice.tt");
    let segments = decode(&field(&sidecar.map, "\"mappings\":\""));

    // "render" starts at column 24 of `export declare function render(...)`.
    assert!(
        segments
            .iter()
            .any(|s| s.source_line == 8 && s.generated_column == 24),
        "{segments:?}"
    );
    assert!(segments.iter().any(|s| s.generated_column == 0));
}

#[test]
fn every_line_terminator_counts_on_both_sides_of_the_map() {
    // A source map's lines are ECMA-262's (ECMA-426 §11.1.2.1): CR,
    // U+2028 and U+2029 end a line just as LF does, and a byte-order mark
    // is not a column (TASK-498).
    let expected = build_sidecar(SOURCE, DECLARATIONS, "notice.tt");
    let expected_segments = decode(&field(&expected.map, "\"mappings\":\""));
    for source_break in ["\r\n", "\r", "\u{2028}", "\u{2029}"] {
        for declaration_break in ["\n", "\r\n", "\r"] {
            let source = format!("\u{feff}{}", SOURCE.replace('\n', source_break));
            let declarations = DECLARATIONS.replace('\n', declaration_break);
            let sidecar = build_sidecar(&source, &declarations, "notice.tt");
            assert_eq!(
                decode(&field(&sidecar.map, "\"mappings\":\"")),
                expected_segments,
                "{source_break:?} {declaration_break:?}"
            );
            assert_eq!(
                sidecar.declarations, expected.declarations,
                "{source_break:?} {declaration_break:?}"
            );
        }
    }
}

#[test]
fn declarations_with_no_source_match_are_skipped_without_panicking() {
    let sidecar = build_sidecar(SOURCE, "export declare const ghost: number;\n", "notice.tt");
    let segments = decode(&field(&sidecar.map, "\"mappings\":\""));
    assert!(segments.is_empty(), "{segments:?}");
}

/// The name segments of a map: every segment but a line's column-0 one.
fn name_segments(map: &str) -> Vec<(usize, usize, usize, usize)> {
    decode(&field(map, "\"mappings\":\""))
        .into_iter()
        .filter(|segment| segment.generated_column > 0)
        .map(|segment| {
            (
                segment.generated_line,
                segment.generated_column,
                segment.source_line,
                segment.source_column,
            )
        })
        .collect()
}

#[test]
fn a_name_maps_to_the_module_level_declaration_that_exports_it() {
    // An inner local of the same name comes first in the text, and each name
    // also occurs inside an earlier word on its own line: `port` and `ex` in
    // `export`, `a` in `declare`.
    let source = "function sum() {\n  const total = 1;\n  return total;\n}\nexport const total = 2;\nexport function port(): number { return sum(); }\nexport const ex = 3;\nexport let a = 1;\n";
    let declarations = "export declare const total = 2;\nexport declare function port(): number;\nexport declare const ex = 3;\nexport declare let a: number;\n";
    let sidecar = build_sidecar(source, declarations, "loc.tt");
    assert_eq!(
        name_segments(&sidecar.map),
        [
            (1, 21, 4, 13),
            (2, 24, 5, 16),
            (3, 21, 6, 13),
            (4, 19, 7, 11)
        ],
        "{}",
        sidecar.map
    );
}

#[test]
fn every_name_a_declaration_line_declares_gets_its_own_segment() {
    let source = "const pair = { left: 1, right: 2 };\nexport const b = 1, c = \"c\";\nexport const { left, right } = pair;\n";
    let declarations = "export declare const b = 1, c = \"c\";\nexport declare const left: number, right: number;\n";
    let sidecar = build_sidecar(source, declarations, "pair.tt");
    assert_eq!(
        name_segments(&sidecar.map),
        [
            (1, 21, 1, 13),
            (1, 28, 1, 20),
            (2, 21, 2, 15),
            (2, 35, 2, 21)
        ],
        "{}",
        sidecar.map
    );
}
