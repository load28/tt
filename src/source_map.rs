//! Source Map v3 for compiled output.
//!
//! The map is built from what emission already recorded — the
//! source↔output byte runs of every verbatim chunk ([`EmitMapping`]) and the
//! construct that wrote each stretch of generated glue ([`EmitAnchor`]) — so
//! a position in the output is answered by the same structures a diagnostic
//! is. Nothing here reads the emitted text for meaning: it is scanned only
//! to count lines and UTF-16 columns, which is what the format's coordinates
//! are.
//!
//! Granularity is one segment per *cut point*: the start of each verbatim
//! run, the byte after it, the start of every output line, and the start of
//! every source token a run copies. A consumer does not interpolate inside a
//! segment — it answers with the nearest mapping at or before the position
//! (ECMA-426) — so a run needs a mapping at each token for a position inside
//! it to name its own column. Inside a run the two sides advance byte for
//! byte, so each of those segments maps a token to itself: the line and the
//! column are exact at every token a frame can name. Tokens come from the
//! source's own lexer, so a run that begins or ends inside a string,
//! comment or template is still cut at the source's token boundaries, and a
//! byte outside ASCII never starts one. Glue maps to the construct that
//! wrote it, which is what makes a frame inside generated code point at the
//! `match` (or `try`, or pipeline) it came from.

use std::collections::BTreeSet;

use crate::lexer::{self, Token, TokenKind, TplPart};
use crate::{EmitAnchor, EmitMapping, SourceKind};

/// What the caller wants the map to say about the files it names.
#[derive(Debug, Clone)]
pub struct SourceMapRequest<'a> {
    /// The generated file's name (`file`), as the map's consumer will see
    /// it. `None` omits the field.
    pub file: Option<&'a str>,
    /// The original file's path, as the map should name it (`sources[0]`) —
    /// relative to the map itself for a written map, or whatever identity
    /// the host uses for an inline one.
    pub source: &'a str,
    /// Whether to embed the original text (`sourcesContent`). A map that
    /// carries its source works in a debugger that cannot resolve the path,
    /// which is every bundled and every `data:` case.
    pub embed_source: bool,
    /// Lines the caller inserts into the emitted code after this emission —
    /// a banner comment. Segments at or after
    /// [`SourceMapRequest::generated_line_offset_at`] shift down by exactly
    /// this many lines.
    pub generated_line_offset: usize,
    /// The generated line the insertion happens at. A banner goes to the top
    /// of an ordinary file, but a `#!` line has to stay first, so the banner
    /// follows it and that first line does not move.
    pub generated_line_offset_at: usize,
    /// The original file's surface kind, which decides how its tokens are
    /// read (a `.ttx` admits JSX).
    pub source_kind: SourceKind,
}

impl Default for SourceMapRequest<'_> {
    fn default() -> Self {
        Self {
            file: None,
            source: "<input>",
            embed_source: true,
            generated_line_offset: 0,
            generated_line_offset_at: 0,
            source_kind: SourceKind::TypeScript,
        }
    }
}

/// A Source Map v3 document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap {
    file: Option<String>,
    source: String,
    source_content: Option<String>,
    /// The encoded `mappings` field.
    mappings: String,
}

impl SourceMap {
    /// The map as its JSON document.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(self.mappings.len() + 128);
        out.push_str("{\"version\":3");
        if let Some(file) = &self.file {
            out.push_str(",\"file\":");
            push_json_string(&mut out, file);
        }
        out.push_str(",\"sources\":[");
        push_json_string(&mut out, &self.source);
        out.push(']');
        if let Some(content) = &self.source_content {
            out.push_str(",\"sourcesContent\":[");
            push_json_string(&mut out, content);
            out.push(']');
        }
        out.push_str(",\"names\":[],\"mappings\":");
        push_json_string(&mut out, &self.mappings);
        out.push('}');
        out
    }

    /// The `//# sourceMappingURL=` comment naming `url`, as a line to append
    /// to the generated file.
    #[must_use]
    pub fn url_comment(url: &str) -> String {
        format!("//# sourceMappingURL={url}\n")
    }

    /// The map as a `data:` URL, for a `//# sourceMappingURL=` comment that
    /// carries the map itself.
    #[must_use]
    pub fn to_data_url(&self) -> String {
        format!(
            "data:application/json;charset=utf-8;base64,{}",
            base64(self.to_json().as_bytes())
        )
    }

    /// The encoded `mappings` field.
    #[must_use]
    pub fn mappings(&self) -> &str {
        &self.mappings
    }
}

/// Builds the map for one emission.
pub(crate) fn build(
    source: &str,
    code: &str,
    mappings: &[EmitMapping],
    anchors: &[EmitAnchor],
    request: &SourceMapRequest<'_>,
) -> SourceMap {
    let source_lines = LineTable::new(source);
    let code_lines = LineTable::new(code);

    // A segment is needed wherever the answer changes: at each verbatim
    // run's edges, at the start of every output line (a run and a stretch
    // of glue both continue across line breaks), and at every token a run
    // copies, since the column a consumer reports is its segment's own.
    let token_starts = token_starts(source, request.source_kind);
    let mut cuts: BTreeSet<usize> = code_lines.starts.iter().copied().collect();
    for run in mappings {
        cuts.insert(run.out);
        cuts.insert(run.out.saturating_add(run.len));
        let first = token_starts.partition_point(|start| *start < run.src);
        let copied = token_starts[first..]
            .iter()
            .take_while(|start| **start < run.src.saturating_add(run.len));
        cuts.extend(copied.map(|start| run.out + (start - run.src)));
    }

    let mut encoded = String::new();
    let mut previous_source_line = 0i64;
    let mut previous_source_column = 0i64;
    let mut previous_generated_column = 0i64;
    let mut current_line = 0usize;
    let mut line_has_segment = false;

    for out in cuts {
        if out >= code.len() {
            continue;
        }
        let Some(src) = source_byte_at(out, mappings, anchors) else {
            continue;
        };
        let (generated_line, generated_column) = code_lines.position(code, out);
        let generated_line = generated_line
            + if generated_line >= request.generated_line_offset_at {
                request.generated_line_offset
            } else {
                0
            };
        let (source_line, source_column) = source_lines.position(source, src.min(source.len()));
        while current_line < generated_line {
            encoded.push(';');
            current_line += 1;
            previous_generated_column = 0;
            line_has_segment = false;
        }
        if line_has_segment {
            encoded.push(',');
        }
        line_has_segment = true;
        let generated_column = generated_column as i64;
        let source_line = source_line as i64;
        let source_column = source_column as i64;
        push_vlq(&mut encoded, generated_column - previous_generated_column);
        push_vlq(&mut encoded, 0); // one source
        push_vlq(&mut encoded, source_line - previous_source_line);
        push_vlq(&mut encoded, source_column - previous_source_column);
        previous_generated_column = generated_column;
        previous_source_line = source_line;
        previous_source_column = source_column;
    }

    SourceMap {
        file: request.file.map(str::to_owned),
        source: request.source.to_owned(),
        source_content: request.embed_source.then(|| source.to_owned()),
        mappings: encoded,
    }
}

/// The source byte an output byte came from: the verbatim run that covers
/// it, else the innermost construct whose glue it is, else nothing.
fn source_byte_at(out: usize, mappings: &[EmitMapping], anchors: &[EmitAnchor]) -> Option<usize> {
    let run = mappings.partition_point(|run| run.out + run.len <= out);
    if let Some(run) = mappings.get(run)
        && run.out <= out
        && out < run.out + run.len
    {
        return Some(run.src + (out - run.out));
    }
    anchors
        .iter()
        .find(|anchor| anchor.out <= out && out < anchor.end)
        .map(|anchor| anchor.src)
}

#[doc(hidden)]
pub fn url_path<'a>(segments: impl IntoIterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for (index, segment) in segments.into_iter().enumerate() {
        if index > 0 {
            out.push('/');
        }
        for byte in segment.bytes() {
            let encode = match byte {
                b'%' | b'\\' | b'?' | b'#' | b' ' | b'"' | b'<' | b'>' | b'^' | b'`' | b'{'
                | b'}' => true,
                b':' => index == 0,
                _ => !(0x21..0x7f).contains(&byte),
            };
            if encode {
                out.push_str(&format!("%{byte:02X}"));
            } else {
                out.push(byte as char);
            }
        }
    }
    out
}

/// The byte offset of every token in `source`, ascending — including the
/// tokens inside template interpolations and JSX expression containers.
/// A byte outside ASCII is opaque, so it never starts a token.
fn token_starts(source: &str, source_kind: SourceKind) -> Vec<usize> {
    fn collect(tokens: &[Token], starts: &mut Vec<usize>) {
        for token in tokens {
            match &token.kind {
                TokenKind::Punct(byte) if !byte.is_ascii() => {}
                TokenKind::Template(parts) => {
                    for part in parts.iter() {
                        match part {
                            TplPart::Raw(span) => starts.push(span.start),
                            TplPart::Interp { tokens, .. } => collect(tokens, starts),
                        }
                    }
                }
                _ => starts.push(token.span.start),
            }
        }
    }
    let mut starts = Vec::new();
    collect(
        &lexer::lex_with_kind(source, 0, source.len(), source_kind),
        &mut starts,
    );
    starts.sort_unstable();
    starts.dedup();
    starts
}

/// Byte offsets of every line start, for turning a byte into the format's
/// line and UTF-16 column.
struct LineTable {
    starts: Vec<usize>,
}

impl LineTable {
    fn new(text: &str) -> Self {
        let mut starts = vec![0usize];
        starts.extend(
            text.bytes()
                .enumerate()
                .filter(|(_, byte)| *byte == b'\n')
                .map(|(at, _)| at + 1),
        );
        Self { starts }
    }

    /// The zero-based line, and the column in UTF-16 code units — the unit
    /// the format counts in.
    fn position(&self, text: &str, byte: usize) -> (usize, usize) {
        let line = self.starts.partition_point(|start| *start <= byte) - 1;
        let start = self.starts[line];
        let column = text
            .get(start..byte)
            .map_or(0, |prefix| prefix.encode_utf16().count());
        (line, column)
    }
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Appends one Base64 VLQ number.
fn push_vlq(out: &mut String, value: i64) {
    let mut bits = if value < 0 {
        ((-value) as u64) << 1 | 1
    } else {
        (value as u64) << 1
    };
    loop {
        let mut digit = (bits & 0b1_1111) as usize;
        bits >>= 5;
        if bits > 0 {
            digit |= 0b10_0000;
        }
        out.push(BASE64[digit] as char);
        if bits == 0 {
            return;
        }
    }
}

fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64[(triple >> 18) as usize & 0x3f] as char);
        out.push(BASE64[(triple >> 12) as usize & 0x3f] as char);
        out.push(if chunk.len() > 1 {
            BASE64[(triple >> 6) as usize & 0x3f] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64[triple as usize & 0x3f] as char
        } else {
            '='
        });
    }
    out
}

fn push_json_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlq_encodes_the_formats_signed_numbers() {
        let cases = [(0, "A"), (1, "C"), (-1, "D"), (16, "gB"), (-16, "hB")];
        for (value, expected) in cases {
            let mut out = String::new();
            push_vlq(&mut out, value);
            assert_eq!(out, expected, "{value}");
        }
    }

    #[test]
    fn columns_are_counted_in_utf16_code_units() {
        // An astral character is two UTF-16 code units and four UTF-8 bytes;
        // the format counts the former.
        let text = "const a = \"🙂\";\n";
        let table = LineTable::new(text);
        let quote = text.rfind('"').unwrap();
        let (line, column) = table.position(text, quote);
        assert_eq!(line, 0);
        assert_eq!(column, "const a = \"🙂".encode_utf16().count());
    }

    #[test]
    fn a_position_inside_a_run_answers_from_that_run() {
        let mappings = [EmitMapping {
            src: 10,
            out: 4,
            len: 6,
        }];
        assert_eq!(source_byte_at(4, &mappings, &[]), Some(10));
        assert_eq!(source_byte_at(7, &mappings, &[]), Some(13));
        assert_eq!(source_byte_at(10, &mappings, &[]), None);
    }

    #[test]
    fn glue_answers_from_the_construct_that_wrote_it() {
        let anchors = [EmitAnchor {
            out: 0,
            end: 20,
            src: 42,
            src_end: 51,
            owner_end: 60,
            context: None,
            kind: crate::AnchorKind::Match,
        }];
        assert_eq!(source_byte_at(5, &[], &anchors), Some(42));
        assert_eq!(source_byte_at(25, &[], &anchors), None);
    }

    /// Per generated line, its segments as `(generated column, source line,
    /// source column)`.
    type Decoded = Vec<Vec<(i64, i64, i64)>>;

    /// Decodes `mappings` into [`Decoded`] segments.
    fn decode(mappings: &str) -> Decoded {
        let mut lines = Vec::new();
        let (mut source_line, mut source_column) = (0i64, 0i64);
        for line in mappings.split(';') {
            let mut segments = Vec::new();
            let mut generated_column = 0i64;
            for segment in line.split(',').filter(|segment| !segment.is_empty()) {
                let mut values = Vec::new();
                let (mut value, mut shift) = (0i64, 0);
                for byte in segment.bytes() {
                    let digit = BASE64.iter().position(|c| *c == byte).unwrap() as i64;
                    value |= (digit & 0b1_1111) << shift;
                    shift += 5;
                    if digit & 0b10_0000 == 0 {
                        values.push(if value & 1 == 1 {
                            -(value >> 1)
                        } else {
                            value >> 1
                        });
                        (value, shift) = (0, 0);
                    }
                }
                generated_column += values[0];
                source_line += values[2];
                source_column += values[3];
                segments.push((generated_column, source_line, source_column));
            }
            lines.push(segments);
        }
        lines
    }

    /// What a consumer answers for a generated position: the nearest
    /// segment at or before it on its line — no interpolation.
    fn original_at(map: &Decoded, (line, column): (usize, usize)) -> (usize, usize) {
        let (_, source_line, source_column) = map[line]
            .iter()
            .rev()
            .find(|segment| segment.0 <= column as i64)
            .copied()
            .expect("a segment at or before the position");
        (source_line as usize, source_column as usize)
    }

    /// Where `needle` first sits in `text` on the line that holds `line`,
    /// as the format's line and UTF-16 column.
    fn located(text: &str, line: &str, needle: &str) -> (usize, usize) {
        let from = text.find(line).unwrap();
        let at = from + text[from..].find(needle).unwrap();
        LineTable::new(text).position(text, at)
    }

    fn map_of(source: &str) -> (String, Decoded) {
        let emit = crate::emit_mapped(source);
        let map = build(
            source,
            &emit.code,
            &emit.mappings,
            &emit.anchors,
            &SourceMapRequest::default(),
        );
        (emit.code, decode(map.mappings()))
    }

    #[test]
    fn every_token_of_a_copied_line_maps_to_its_own_column() {
        let source = "variant E { A(v: number), B }\n\
                      export const n = match (E.B) { A(v) => v, B => 0 };\n\
                      const  x = 1;   function boom() { return [1].map(() => \
                      { throw new Error(`b${\"o\"}om`); }); }\n";
        let (code, map) = map_of(source);
        const LINE: &str = "const  x";
        for token in [
            "const  x", "x = 1", "= 1", "1;", "function", "boom", "return", "[1]", ".map", "map",
            "=>", "throw", "new", "Error", "`b", "\"o\"", "om`", "});",
        ] {
            assert_eq!(
                original_at(&map, located(&code, LINE, token)),
                located(source, LINE, token),
                "{token:?}"
            );
        }
    }

    #[test]
    fn a_byte_outside_ascii_never_starts_a_segment() {
        let source = "variant E { A, B }\nconst café = \"🙂\"; boom(café);\n";
        let (code, map) = map_of(source);
        const LINE: &str = "const café";
        let (line, accent) = located(&code, LINE, "é = ");
        assert!(
            map[line].iter().all(|segment| segment.0 != accent as i64),
            "{:?}",
            map[line]
        );
        for token in ["boom", "café)", "\"🙂\"", ");"] {
            assert_eq!(
                original_at(&map, located(&code, LINE, token)),
                located(source, LINE, token),
                "{token:?}"
            );
        }
    }

    #[test]
    fn a_banner_shifts_every_segment_down() {
        let source = "const a = 1;\n";
        let mappings = [EmitMapping {
            src: 0,
            out: 0,
            len: source.len(),
        }];
        let plain = build(
            source,
            source,
            &mappings,
            &[],
            &SourceMapRequest {
                source: "a.tt",
                ..SourceMapRequest::default()
            },
        );
        let bannered = build(
            source,
            source,
            &mappings,
            &[],
            &SourceMapRequest {
                source: "a.tt",
                generated_line_offset: 1,
                ..SourceMapRequest::default()
            },
        );
        assert_eq!(bannered.mappings(), format!(";{}", plain.mappings()));
    }

    #[test]
    fn the_document_names_its_source_and_carries_it() {
        let source = "const a = 1;\n";
        let map = build(
            source,
            source,
            &[EmitMapping {
                src: 0,
                out: 0,
                len: source.len(),
            }],
            &[],
            &SourceMapRequest {
                file: Some("a.tt.ts"),
                source: "a.tt",
                embed_source: true,
                generated_line_offset: 0,
                generated_line_offset_at: 0,
                source_kind: SourceKind::TypeScript,
            },
        );
        let json = map.to_json();
        assert!(json.starts_with("{\"version\":3"), "{json}");
        assert!(json.contains("\"file\":\"a.tt.ts\""), "{json}");
        assert!(json.contains("\"sources\":[\"a.tt\"]"), "{json}");
        assert!(
            json.contains("\"sourcesContent\":[\"const a = 1;\\n\"]"),
            "{json}"
        );
        assert!(
            map.to_data_url().starts_with("data:application/json"),
            "{map:?}"
        );
    }
}
