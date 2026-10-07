//! The line model: where a text's lines start, and what a byte offset is as a
//! line and column.
//!
//! Every surface that shows a position to a person or a program converts
//! here, so one text cannot have two sets of lines. Which code points end a
//! line is not one fact but two, and each consumer names the one it speaks:
//!
//! - [`LineBreaks::Ecma`] — ECMA-262's `LineTerminatorSequence` (§12.3):
//!   LF, CR, CR LF, U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR.
//!   These are the lines `tsc` reports a diagnostic on and the lines
//!   ECMA-426 source maps count, so the CLI, compile errors, the typed
//!   engine's diagnostics and source maps use them.
//! - [`LineBreaks::Lsp`] — the Language Server Protocol's end-of-line set
//!   (3.17, "Text Documents"): LF, CR LF and CR. An editor's buffer has no
//!   line break at U+2028 or U+2029, so every position that goes to an
//!   editor or to the TypeScript language server uses these.
//!
//! A CR LF pair is one line break under both. Positions are measured in the
//! decoded text: a leading byte-order mark is a signature, not a character,
//! so line 0 starts after it. Byte offsets in and out are offsets into the
//! text as given, signature included.
//!
//! The terminators are recognized byte by byte through the scanner's
//! ECMA-262 primitives; every other non-ASCII code point is opaque.

use crate::error::signature_len;
use crate::scanner;

/// Which code points end a line. See the [module documentation](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineBreaks {
    /// LF, CR, CR LF, U+2028 and U+2029 — ECMA-262's line terminators.
    Ecma,
    /// LF, CR LF and CR — the Language Server Protocol's.
    Lsp,
}

impl LineBreaks {
    /// The offset just past the line break starting at `at`, if one does.
    fn break_end(self, bytes: &[u8], at: usize) -> Option<usize> {
        match self {
            LineBreaks::Ecma => scanner::line_break_end(bytes, at, bytes.len()),
            LineBreaks::Lsp => match bytes.get(at)? {
                b'\r' if bytes.get(at + 1) == Some(&b'\n') => Some(at + 2),
                b'\r' | b'\n' => Some(at + 1),
                _ => None,
            },
        }
    }

    /// Whether a line break ends exactly at `at`: the byte before it closes
    /// one, and `at` is not the LF of a CR LF pair.
    fn break_ends_at(self, bytes: &[u8], at: usize) -> bool {
        if at == 0 || bytes.get(at) == Some(&b'\n') && bytes[at - 1] == b'\r' {
            return false;
        }
        match bytes[at - 1] {
            b'\n' | b'\r' => true,
            0xA8 | 0xA9 => {
                self == LineBreaks::Ecma
                    && at >= 3
                    && scanner::line_terminator_len(bytes, at - 3, at) == Some(3)
            }
            _ => false,
        }
    }
}

/// The lines of one text, measured once.
#[derive(Debug, Clone)]
pub struct LineMap<'a> {
    text: &'a str,
    signature: usize,
    /// Byte offset of each line's first byte.
    starts: std::borrow::Cow<'a, [usize]>,
    /// Byte offset of each line's break, or of the text's end on the last.
    ends: std::borrow::Cow<'a, [usize]>,
}

/// A [`LineMap`]'s measurements, kept apart from the text they measure.
#[derive(Debug, Clone, Default)]
pub(crate) struct LineIndex {
    signature: usize,
    starts: Vec<usize>,
    ends: Vec<usize>,
}

impl<'a> LineMap<'a> {
    /// Measures `text` under `breaks`.
    #[must_use]
    pub fn new(text: &'a str, breaks: LineBreaks) -> Self {
        crate::work::tick("line measurements");
        let bytes = text.as_bytes();
        let signature = signature_len(text);
        let mut starts = vec![signature];
        let mut ends = Vec::new();
        let mut at = signature;
        while at < bytes.len() {
            match breaks.break_end(bytes, at) {
                Some(next) => {
                    ends.push(at);
                    starts.push(next);
                    at = next;
                }
                None => at += 1,
            }
        }
        ends.push(bytes.len());
        Self {
            text,
            signature,
            starts: starts.into(),
            ends: ends.into(),
        }
    }

    pub(crate) fn index(&self) -> LineIndex {
        LineIndex {
            signature: self.signature,
            starts: self.starts.to_vec(),
            ends: self.ends.to_vec(),
        }
    }

    pub(crate) fn indexed(text: &'a str, index: &'a LineIndex) -> Self {
        Self {
            text,
            signature: index.signature,
            starts: std::borrow::Cow::Borrowed(&index.starts),
            ends: std::borrow::Cow::Borrowed(&index.ends),
        }
    }

    /// [`LineMap::new`] under [`LineBreaks::Ecma`].
    #[must_use]
    pub fn ecma(text: &'a str) -> Self {
        Self::new(text, LineBreaks::Ecma)
    }

    /// [`LineMap::new`] under [`LineBreaks::Lsp`].
    #[must_use]
    pub fn lsp(text: &'a str) -> Self {
        Self::new(text, LineBreaks::Lsp)
    }

    /// How many lines the text has; an empty text has one.
    #[must_use]
    pub fn len(&self) -> usize {
        self.starts.len()
    }

    /// Never true: every text has at least one line.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The zero-based line holding `byte`. An offset inside a code point
    /// counts as that code point's start, and one past the end as the end.
    #[must_use]
    pub fn line_of(&self, byte: usize) -> usize {
        let byte = self.clamp(byte);
        self.starts.partition_point(|start| *start <= byte) - 1
    }

    /// The byte offset line `line` starts at.
    #[must_use]
    pub fn line_start(&self, line: usize) -> Option<usize> {
        self.starts.get(line).copied()
    }

    /// The byte offset just past line `line`'s break — the next line's
    /// start, or the text's end on the last line.
    #[must_use]
    pub fn line_end(&self, line: usize) -> Option<usize> {
        self.ends.get(line)?;
        Some(
            self.starts
                .get(line + 1)
                .copied()
                .unwrap_or(self.text.len()),
        )
    }

    /// Line `line`'s text, without its line break.
    #[must_use]
    pub fn line_text(&self, line: usize) -> Option<&'a str> {
        Some(&self.text[*self.starts.get(line)?..self.ends[line]])
    }

    /// The line break that ends line `line` — empty on the last line.
    #[must_use]
    pub fn line_break(&self, line: usize) -> Option<&'a str> {
        Some(&self.text[*self.ends.get(line)?..self.line_end(line)?])
    }

    /// The zero-based line and zero-based column of `byte`, the column
    /// counted in UTF-16 code units — the unit editors, the TypeScript
    /// language server and source maps count in.
    #[must_use]
    pub fn utf16_position(&self, byte: usize) -> (usize, usize) {
        let (line, prefix) = self.prefix(byte);
        (line, prefix.encode_utf16().count())
    }

    /// The zero-based line and zero-based column of `byte`, the column
    /// counted in code points — what a rendered caret lines up with.
    #[must_use]
    pub fn char_position(&self, byte: usize) -> (usize, usize) {
        let (line, prefix) = self.prefix(byte);
        (line, prefix.chars().count())
    }

    /// The byte offset a zero-based line and UTF-16 column name, under the
    /// Language Server Protocol's rules: a column past the line's end means
    /// the line's end, and a line past the text's end means the text's end.
    /// A column inside a surrogate pair names the start of its character.
    #[must_use]
    pub fn utf16_offset(&self, line: usize, column: usize) -> usize {
        let Some(text) = self.line_text(line) else {
            return self.text.len();
        };
        let mut units = 0;
        for (byte, ch) in text.char_indices() {
            units += ch.len_utf16();
            if units > column {
                return self.starts[line] + byte;
            }
        }
        self.ends[line]
    }

    /// The byte offset a zero-based line and code-point column name, or
    /// `None` when the line has no such column. The line's end — one past
    /// its last character — is a position; past it is not.
    #[must_use]
    pub fn char_offset(&self, line: usize, column: usize) -> Option<usize> {
        let text = self.line_text(line)?;
        let start = self.starts[line];
        match text.char_indices().nth(column) {
            Some((byte, _)) => Some(start + byte),
            None => (column == text.chars().count()).then_some(self.ends[line]),
        }
    }

    fn clamp(&self, byte: usize) -> usize {
        let mut byte = byte.clamp(self.signature, self.text.len());
        while !self.text.is_char_boundary(byte) {
            byte -= 1;
        }
        byte
    }

    fn prefix(&self, byte: usize) -> (usize, &'a str) {
        let byte = self.clamp(byte);
        let line = self.starts.partition_point(|start| *start <= byte) - 1;
        (line, &self.text[self.starts[line]..byte])
    }
}

/// A file's text as TypeScript reads it, measured in the same UTF-16 units:
/// a UTF-16 file (one that starts with its byte order mark) is decoded
/// without the mark, and each byte that is not part of valid UTF-8 reads as
/// one U+FFFD, the one unit tsgo's `ast.ComputePositionMap` counts for it.
pub fn typescript_text(bytes: Vec<u8>) -> String {
    let utf16 = |rest: &[u8], unit: fn([u8; 2]) -> u16| {
        let units: Vec<u16> = rest
            .chunks(2)
            .map(|pair| unit([pair[0], pair.get(1).copied().unwrap_or(0)]))
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes.as_slice() {
        [0xFF, 0xFE, rest @ ..] => return utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => return utf16(rest, u16::from_be_bytes),
        _ => {}
    }
    let bytes = match String::from_utf8(bytes) {
        Ok(text) => return text,
        Err(error) => error.into_bytes(),
    };
    let mut text = String::with_capacity(bytes.len());
    let mut rest = &bytes[..];
    while let Some(chunk) = rest.utf8_chunks().next() {
        text.push_str(chunk.valid());
        rest = &rest[chunk.valid().len()..];
        if rest.is_empty() {
            break;
        }
        let width = if matches!(rest, [0xED, 0xA0..=0xBF, 0x80..=0xBF, ..]) {
            3
        } else {
            1
        };
        text.push('\u{FFFD}');
        rest = &rest[width..];
    }
    text
}

/// Byte offsets and UTF-16 code-unit offsets of one text, measured once.
///
/// TypeScript addresses text in UTF-16 code units; the compiler addresses
/// it in bytes. The two agree up to the first non-ASCII character, and each
/// such character shifts them by a known amount, so recording those
/// characters alone answers every conversion by binary search — the shape
/// of tsgo's `ast.PositionMap`. A leading byte-order mark is a signature:
/// UTF-16 offsets count the decoded text, byte offsets include it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Utf16Map {
    signature: usize,
    len: usize,
    units: usize,
    wide: Vec<WideChar>,
}

#[derive(Debug, Clone, Copy)]
struct WideChar {
    byte: usize,
    unit: usize,
    bytes: usize,
    units: usize,
}

impl WideChar {
    fn shift(self) -> usize {
        self.byte + self.bytes - (self.unit + self.units)
    }
}

impl Utf16Map {
    pub(crate) fn new(text: &str) -> Self {
        crate::work::tick("utf-16 measurements");
        let signature = signature_len(text);
        let decoded = crate::error::decoded(text);
        let mut shift = 0;
        let mut wide = Vec::new();
        for (byte, ch) in decoded.char_indices().filter(|(_, ch)| !ch.is_ascii()) {
            let entry = WideChar {
                byte,
                unit: byte - shift,
                bytes: ch.len_utf8(),
                units: ch.len_utf16(),
            };
            shift = entry.shift();
            wide.push(entry);
        }
        Self {
            signature,
            len: decoded.len(),
            units: decoded.len() - shift,
            wide,
        }
    }

    /// The UTF-16 offset of `byte`. An offset past the end, or one inside a
    /// character, counts the whole text.
    pub(crate) fn to_utf16(&self, byte: usize) -> usize {
        let byte = byte.saturating_sub(self.signature);
        if byte > self.len {
            return self.units;
        }
        let before = self.wide.partition_point(|wide| wide.byte < byte);
        match before.checked_sub(1).map(|at| self.wide[at]) {
            Some(wide) if byte < wide.byte + wide.bytes => self.units,
            Some(wide) => byte - wide.shift(),
            None => byte,
        }
    }

    /// The byte offset of the first character boundary at or after the
    /// UTF-16 offset `utf16`; the text's end past it.
    pub(crate) fn to_byte(&self, utf16: usize) -> usize {
        let before = self.wide.partition_point(|wide| wide.unit < utf16);
        let byte = match before.checked_sub(1).map(|at| self.wide[at]) {
            Some(wide) if utf16 < wide.unit + wide.units => wide.byte + wide.bytes,
            Some(wide) => utf16 + wide.shift(),
            None => utf16,
        };
        self.signature + byte.min(self.len)
    }
}

/// Positions in the editor protocol's coordinates, from the compiler's.
///
/// The compiler reports a 1-based line under [`LineBreaks::Ecma`] and a
/// 1-based code-point column; an editor protocol addresses the same place
/// as a 1-based line under [`LineBreaks::Lsp`] and a 1-based UTF-16
/// column. The two differ on every line after a U+2028 or U+2029
/// and by one column for every astral character earlier on the line, so a
/// surface that speaks to an editor converts here, through the byte both
/// name.
#[derive(Debug, Clone)]
pub struct ProtocolPositions<'a> {
    compiler: LineMap<'a>,
    protocol: LineMap<'a>,
}

impl<'a> ProtocolPositions<'a> {
    /// Measures `text` under both line models.
    #[must_use]
    pub fn new(text: &'a str) -> Self {
        Self {
            compiler: LineMap::ecma(text),
            protocol: LineMap::lsp(text),
        }
    }

    /// The protocol's 1-based line and UTF-16 column of `byte`.
    #[must_use]
    pub fn of_byte(&self, byte: usize) -> (usize, usize) {
        let (line, column) = self.protocol.utf16_position(byte);
        (line + 1, column + 1)
    }

    /// The protocol's position for a compiler position — a 1-based line
    /// and code-point column such as [`crate::line_col`] answers. `(0, 0)`,
    /// the compiler's "no position", stays itself, and so does a position
    /// the text does not have: an unmeasurable position is better left as
    /// it arrived than silently moved.
    #[must_use]
    pub fn of_position(&self, (line, column): (usize, usize)) -> (usize, usize) {
        line.checked_sub(1)
            .zip(column.checked_sub(1))
            .and_then(|(line, column)| self.compiler.char_offset(line, column))
            .map_or((line, column), |byte| self.of_byte(byte))
    }
}

/// The byte offset of the start of the ECMA-262 line holding `at`, found by
/// walking back from it rather than measuring the whole text — for asking
/// once where a token's line begins. It agrees with [`LineMap::ecma`].
pub(crate) fn line_start_before(text: &str, at: usize) -> usize {
    let bytes = text.as_bytes();
    let signature = signature_len(text);
    let mut at = at.min(bytes.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    let mut start = at;
    while start > signature && !LineBreaks::Ecma.break_ends_at(bytes, start) {
        start -= 1;
    }
    start.max(signature)
}

/// Whether `text` ends with an ECMA-262 line break, so what follows it
/// starts a line.
pub(crate) fn ends_with_line_break(text: &str) -> bool {
    LineBreaks::Ecma.break_ends_at(text.as_bytes(), text.len())
}

/// A byte offset as the 1-based line and 1-based code-point column the
/// compiler reports positions in — ECMA-262's lines, as `tsc` counts them.
pub(crate) fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let (line, column) = LineMap::ecma(text).char_position(offset);
    (line + 1, column + 1)
}

#[cfg(test)]
mod tests;
