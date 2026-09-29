//! Editor sidecars: `x.tt.d.ts` + `x.tt.d.ts.map`.
//!
//! A `.ts` file that imports `"./x.tt"` gets `TS2307` from tsserver, which
//! does not know the extension. TypeScript's own escape hatch is in that
//! error text — "or its corresponding type declarations" — so placing a
//! declaration file next to the module resolves it.
//!
//! The declaration *body* needs type inference, which is tsc's job (run it
//! with `--emitDeclarationOnly` over ttc's output). What tsc cannot know is
//! where each declaration lives in the original `.tt`; this module supplies
//! that as a declaration map whose `sources` points at the `.tt` file, which
//! is what sends "go to definition" to the original instead of the `.d.ts`.

use crate::lines::LineMap;
use crate::variant_symbols;

/// The two files that make up a module's editor sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecar {
    /// Contents of `<name>.d.ts` — the declarations plus a
    /// `sourceMappingURL` comment.
    pub declarations: String,
    /// Contents of `<name>.d.ts.map` — a source map v3 document whose
    /// `sources` is the original `.tt` file.
    pub map: String,
}

/// Builds the sidecar for one module.
///
/// `source` is the original `.tt` text, `declarations` is what tsc emitted
/// for ttc's output of that module, and `tt_path` is the path to the `.tt`
/// file **relative to where the sidecar will be written** — `"notice.tt"`
/// when the two sit together, `"../src/notice.tt"` when declarations live
/// in their own tree (which TypeScript merges back with `rootDirs`). Written
/// as a relative URL, it becomes the map's `sources`, and its file name
/// becomes the stem of the written files (`notice.tt.d.ts`) and, as a URL,
/// of the `sourceMappingURL` comment.
///
/// Every exported declaration that can be located in the source gets two
/// mapping segments: one at column 0 and one at the column where its name
/// starts. The second is the one that matters — "go to definition" asks
/// about the name's position, and without a segment there the editor stops
/// at the `.d.ts`.
pub fn build_sidecar(source: &str, declarations: &str, tt_path: &str) -> Sidecar {
    let tt_file_name = tt_path
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(tt_path);
    let source_lines = LineMap::ecma(source);
    let variants = variant_symbols(source);

    // The declarations are rewritten line by line, and every line the map
    // counts is one ECMA-262 line of what is written: a line ending becomes
    // LF, and a U+2028 or U+2029 — a line terminator that is also text, as
    // inside a string literal — stays itself.
    let declaration_lines = LineMap::ecma(declarations);
    let mut body = String::new();
    let mut hits: Vec<Option<Hit>> = Vec::new();
    for index in 0..declaration_lines.len() {
        let line = declaration_lines.line_text(index).unwrap_or_default();
        if line.trim_start().starts_with("//# sourceMappingURL=") {
            continue;
        }
        if !hits.is_empty() {
            body.push_str(match declaration_lines.line_break(index - 1) {
                Some(separator @ ("\u{2028}" | "\u{2029}")) => separator,
                _ => "\n",
            });
        }
        body.push_str(line);
        hits.push(declared_name(line).and_then(|name| {
            let (line_number, column) = locate(&source_lines, &variants, name)?;
            Some(Hit {
                generated_column: line[..line.find(name)?].encode_utf16().count(),
                line: line_number,
                column,
            })
        }));
    }

    let map_name = format!("{tt_file_name}.d.ts.map");
    // The banner costs one generated line, so the mappings are shifted by
    // one (the leading `;` below).
    let declarations = format!(
        "// @generated from {tt_file_name} by ttc --sidecar — do not edit.\n{}\n//# sourceMappingURL={}\n",
        body.trim_end(),
        crate::source_map::url_path([map_name.as_str()])
    );
    // A leading `;` skips the banner line the declarations open with.
    let mappings = format!(";{}", encode_mappings(&hits));
    let map = format!(
        "{{\"version\":3,\"file\":{},\"sourceRoot\":\"\",\"sources\":[{}],\"names\":[],\"mappings\":\"{}\"}}\n",
        json_string(&format!("{tt_file_name}.d.ts")),
        json_string(&crate::source_map::url_path(tt_path.split('/'))),
        mappings,
    );

    Sidecar { declarations, map }
}

struct Hit {
    generated_column: usize,
    line: usize,
    column: usize,
}

/// The name declared by a `.d.ts` line, if it declares one.
fn declared_name(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("export ")?.trim_start();
    let rest = rest.strip_prefix("declare ").unwrap_or(rest).trim_start();
    let rest = rest.strip_prefix("abstract ").unwrap_or(rest).trim_start();
    for keyword in [
        "function ",
        "const ",
        "let ",
        "var ",
        "class ",
        "interface ",
        "type ",
        "enum ",
    ] {
        if let Some(after) = rest.strip_prefix(keyword) {
            let name = identifier_prefix(after.trim_start());
            return (!name.is_empty()).then_some(name);
        }
    }
    None
}

/// The leading ASCII identifier of `text`.
fn identifier_prefix(text: &str) -> &str {
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(text.len());
    &text[..end]
}

/// Where `name` is declared in the source, as a zero-based line and UTF-16
/// column — ECMA-262's lines, which are the ones the map counts.
///
/// tt variants come from the parsed declarations, so their positions are exact.
/// Everything else lives in a passthrough region and is found by scanning
/// for its declaration keyword; the first match wins.
fn locate(
    source_lines: &LineMap<'_>,
    variants: &[crate::VariantSymbol],
    name: &str,
) -> Option<(usize, usize)> {
    if let Some(symbol) = variants.iter().find(|e| e.name == name) {
        // `offset` points at the declaration keyword; move to the name.
        let line = source_lines.line_of(symbol.offset);
        let at = source_lines
            .line_text(line)
            .and_then(|text| text.find(name))
            .and_then(|byte| Some(source_lines.line_start(line)? + byte))
            .unwrap_or(symbol.offset);
        return Some(source_lines.utf16_position(at));
    }

    (0..source_lines.len()).find_map(|index| {
        let text = source_lines.line_text(index)?;
        if !declares(text, name) {
            return None;
        }
        let byte = source_lines.line_start(index)? + text.find(name)?;
        Some(source_lines.utf16_position(byte))
    })
}

/// Whether a source line declares `name` at the top of a statement.
fn declares(line: &str, name: &str) -> bool {
    let rest = line.trim_start();
    let rest = rest.strip_prefix("export ").unwrap_or(rest).trim_start();
    let rest = rest.strip_prefix("declare ").unwrap_or(rest).trim_start();
    let rest = rest.strip_prefix("abstract ").unwrap_or(rest).trim_start();
    for keyword in [
        "function ",
        "const ",
        "let ",
        "var ",
        "class ",
        "interface ",
        "type ",
        "enum ",
    ] {
        if let Some(after) = rest.strip_prefix(keyword) {
            return identifier_prefix(after.trim_start()) == name;
        }
    }
    false
}

/// Encodes one segment per located declaration into a source map v3
/// `mappings` string.
fn encode_mappings(hits: &[Option<Hit>]) -> String {
    let mut previous_line: i64 = 0;
    let mut previous_column: i64 = 0;
    let mut lines: Vec<String> = Vec::with_capacity(hits.len());

    for hit in hits {
        let Some(hit) = hit else {
            lines.push(String::new());
            continue;
        };
        let mut generated: Vec<usize> = vec![0];
        if hit.generated_column > 0 {
            generated.push(hit.generated_column);
        }

        let mut previous_generated: i64 = 0;
        let mut segments: Vec<String> = Vec::with_capacity(generated.len());
        for column in generated {
            let mut segment = String::new();
            vlq(column as i64 - previous_generated, &mut segment);
            vlq(0, &mut segment);
            vlq(hit.line as i64 - previous_line, &mut segment);
            vlq(hit.column as i64 - previous_column, &mut segment);
            previous_generated = column as i64;
            previous_line = hit.line as i64;
            previous_column = hit.column as i64;
            segments.push(segment);
        }
        lines.push(segments.join(","));
    }

    lines.join(";")
}

/// Base64 VLQ, the source map encoding for a signed delta.
fn vlq(value: i64, out: &mut String) {
    const DIGITS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits = if value < 0 {
        ((-value) << 1) | 1
    } else {
        value << 1
    };
    loop {
        let mut digit = (bits & 31) as usize;
        bits >>= 5;
        if bits > 0 {
            digit |= 32;
        }
        out.push(DIGITS[digit] as char);
        if bits == 0 {
            break;
        }
    }
}

/// Minimal JSON string literal (the map only ever carries file names).
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
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
