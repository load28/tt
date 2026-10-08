//! The three coordinate spaces, and the conversions between them.
//!
//! A position travels: `.tt` source byte → emitted TypeScript byte
//! ([`crate::EmitMapping`]) → UTF-16 code unit, which is what TypeScript
//! itself counts in. Questions travel that way; diagnostics travel back.
//!
//! Only bytes copied **verbatim** from the source have an exact source
//! position. Compiler-written glue — a `switch` region or destructuring —
//! belongs to no `.tt` byte, so diagnostics crossing it use the syntax
//! anchor that owns the lowering instead of inventing a partial mapping.

use crate::{EmitAnchor, EmitMapping};

/// Where a checker diagnostic span belongs in the original source.
///
/// A span is exact only when one verbatim mapping covers it completely, or
/// when it lies in glue that starts where copied text ends and opens no
/// construct: TypeScript reports there what it expected after that text,
/// and the span is that position. Crossing even one byte of other
/// compiler-written glue transfers ownership to the innermost lowering
/// anchor. `Nearest` is the last-resort position for
/// output that has neither a complete mapping nor a recorded origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticOrigin {
    Exact { start: usize, end: usize },
    Anchor(EmitAnchor),
    Nearest { start: usize },
}

pub(crate) fn shared_binding_origin(
    shared: &[crate::SharedBinding],
    start: usize,
    end: usize,
) -> Option<DiagnosticOrigin> {
    shared
        .iter()
        .find(|binding| binding.out <= start && end.max(start) <= binding.out_end)
        .and_then(|binding| {
            binding
                .occurrences
                .iter()
                .find(|occurrence| !occurrence.declared)
                .or_else(|| binding.occurrences.first())
        })
        .map(|occurrence| DiagnosticOrigin::Exact {
            start: occurrence.src,
            end: occurrence.src_end,
        })
}

/// A span made of copied text and of the reads of operands moved out of
/// their place, back to back in the output and in source order, is the
/// source text they stand for. `None` when the span holds no such read, or
/// any glue, or pieces out of source order.
pub(crate) fn relocated_origin(
    mappings: &[EmitMapping],
    relocated: &[crate::RelocatedOperand],
    start: usize,
    end: usize,
) -> Option<DiagnosticOrigin> {
    if relocated.is_empty() || end <= start {
        return None;
    }
    let mut at = start;
    let mut source: Option<(usize, usize)> = None;
    let mut read = false;
    while at < end {
        let (from, to, next) = if let Some(operand) = relocated
            .iter()
            .find(|operand| operand.out == at && operand.out_end <= end)
        {
            read = true;
            (operand.src, operand.src_end, operand.out_end)
        } else {
            let mapping = chunk_holding(mappings, at)?;
            let stop = end.min(mapping.out + mapping.len);
            (
                mapping.src + (at - mapping.out),
                mapping.src + (stop - mapping.out),
                stop,
            )
        };
        source = match source {
            None => Some((from, to)),
            Some((first, last)) if last == from => Some((first, to)),
            Some(_) => return None,
        };
        at = next;
    }
    let (start, end) = source?;
    read.then_some(DiagnosticOrigin::Exact { start, end })
}

/// Projects one emitted diagnostic span without inventing a partially
/// mapped source range.
pub(crate) fn diagnostic_origin(
    mappings: &[EmitMapping],
    anchors: &[EmitAnchor],
    start: usize,
    end: usize,
    code: &str,
    source: &str,
) -> Option<DiagnosticOrigin> {
    let end = end.max(start);
    // Mappings are in output order and do not overlap
    // ([`in_output_order`]): the ones ending at or before `start` come
    // first, and the next one is the only one that can cover the span.
    let after = mappings.partition_point(|mapping| mapping.out + mapping.len <= start);
    let covering =
        |mapping: &&EmitMapping| mapping.out <= start && end <= mapping.out + mapping.len;
    if let Some(mapping) = mappings
        .get(after)
        .filter(covering)
        .or_else(|| mappings[..after].last().filter(covering))
    {
        return Some(DiagnosticOrigin::Exact {
            start: mapping.src + (start - mapping.out),
            end: mapping.src + (end - mapping.out),
        });
    }

    // TypeScript reports a missing token at the token it found in its
    // place: `Identifier expected.` on the `;` a match arm's glue writes
    // after `radius.`. A range that starts where copied text ends, copies
    // nothing, and opens no construct's glue is that position after the
    // user's text, as the cursor before the glue is (`Affinity::Preceding`).
    // The chunk that ends last before `start` is the only candidate: an
    // anchor or a non-blank byte between it and `start` lies between every
    // earlier chunk and `start` too.
    let occupied_end = end.max(start.saturating_add(1));
    crate::work::tick_by("diagnostic origin entries", anchors.len());
    let last_anchor = anchors
        .iter()
        .map(|anchor| anchor.out)
        .filter(|&out| out <= start)
        .max();
    if !mappings.get(after).is_some_and(|m| m.out < occupied_end)
        && last_anchor != Some(start)
        && let Some(chunk) = mappings[..after]
            .iter()
            .rev()
            .find(|m| m.len > 0)
            .filter(|m| last_anchor.is_none_or(|anchor| anchor < m.out + m.len))
            .filter(|m| {
                m.out + m.len == start
                    || code
                        .get(m.out + m.len..start)
                        .is_some_and(|gap| gap.bytes().all(|b| b.is_ascii_whitespace()))
            })
    {
        let at = chunk.src + chunk.len;
        // The delimiter belongs to generated glue, but the missing operand
        // belongs at the next source token. Preserve a real zero-width EOF;
        // never split a UTF-8 code point when displaying a source boundary.
        let width = source
            .get(at..)
            .and_then(|tail| tail.chars().next())
            .map_or(0, char::len_utf8);
        return Some(DiagnosticOrigin::Exact {
            start: at,
            end: at + width,
        });
    }
    // A lowering owns a diagnostic whose whole span lies in its output. An
    // anchor that holds only where the span starts (a pipeline step's input
    // at the head of `head.m()`) is part of a larger construct the checker
    // is speaking about.
    if let Some(anchor) = anchors
        .iter()
        .find(|anchor| anchor.out <= start && end <= anchor.end)
        .or_else(|| {
            anchors
                .iter()
                .find(|anchor| anchor.out <= start && start < anchor.end)
        })
        .or_else(|| {
            anchors
                .iter()
                .find(|anchor| anchor.out < occupied_end && start < anchor.end)
        })
    {
        return Some(DiagnosticOrigin::Anchor(*anchor));
    }

    to_source_or_nearest(mappings, start).map(|(start, _)| DiagnosticOrigin::Nearest { start })
}

/// Offset of `byte` in `text`, counted in UTF-16 code units — TypeScript's
/// own coordinate space. An offset past the end clamps to the end.
pub(crate) fn to_utf16(text: &str, byte: usize) -> usize {
    crate::work::tick("utf-16 scans");
    let signature = crate::error::signature_len(text);
    let text = crate::error::decoded(text);
    let byte = byte.saturating_sub(signature);
    match text.get(..byte) {
        Some(prefix) => prefix.encode_utf16().count(),
        None => text.encode_utf16().count(),
    }
}

/// The inverse of [`to_utf16`]: the byte offset a UTF-16 offset names. An
/// offset past the end clamps to the length; one landing inside a surrogate
/// pair clamps to the start of that character.
pub(crate) fn from_utf16(text: &str, utf16: usize) -> usize {
    crate::work::tick("utf-16 scans");
    let signature = crate::error::signature_len(text);
    let text = crate::error::decoded(text);
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if units >= utf16 {
            return signature + byte;
        }
        units += ch.len_utf16();
    }
    signature + text.len()
}

/// Where a source byte landed in the emitted output, or `None` when it was
/// not copied verbatim.
pub(crate) fn to_output(mappings: &[EmitMapping], src: usize) -> Option<usize> {
    mappings
        .iter()
        .find(|m| src >= m.src && src < m.src + m.len)
        .map(|m| m.out + (src - m.src))
}

/// Where an emitted byte came from in the source, or `None` when it is
/// compiler-written glue.
pub(crate) fn to_source(mappings: &[EmitMapping], out: usize) -> Option<usize> {
    chunk_holding(mappings, out).map(|m| m.src + (out - m.out))
}

/// Whether `mappings` are in output order and do not overlap in the output
/// — what [`crate::MappedEmit::mappings`] promises, and what lets an output
/// offset be found by binary search.
pub(crate) fn in_output_order(mappings: &[EmitMapping]) -> bool {
    mappings
        .windows(2)
        .all(|pair| pair[0].out + pair[0].len <= pair[1].out)
}

/// The chunk whose output holds byte `out`. Chunks are in output order and
/// do not overlap ([`in_output_order`]), so their ends do not decrease and
/// the first chunk ending past `out` is the only one that can hold it.
fn chunk_holding(mappings: &[EmitMapping], out: usize) -> Option<&EmitMapping> {
    let index = mappings.partition_point(|m| m.out + m.len <= out);
    mappings.get(index).filter(|m| m.out <= out)
}

/// [`to_output`], but a chunk's **end** offset belongs to it too — and when
/// two chunks touch, the later one wins.
///
/// The language-service positions travel through this variant: a cursor sits
/// *between* bytes, and completion or hover at the end of what was just
/// typed names the boundary offset — exclusive lookup would call it glue and
/// lose the answer. (Diagnostic spans keep the exclusive [`to_output`]:
/// a byte either was copied or was not.)
pub(crate) fn to_output_inclusive(mappings: &[EmitMapping], src: usize) -> Option<usize> {
    mappings
        .iter()
        .filter(|m| m.src <= src)
        .max_by_key(|m| m.src)
        .filter(|m| src <= m.src + m.len)
        .map(|m| m.out + (src - m.src))
}

/// Which text a cursor between two source bytes belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Affinity {
    /// The text before it: what is being typed there.
    Preceding,
    /// The text after it.
    Following,
}

/// Where a cursor at source byte `src` lands in the output. The chunk
/// holding the byte on the cursor's `affinity` side wins, then the one on
/// the other side. Two chunks that touch in the source need not touch in
/// the output — lowering hoists an operand out of the text around it — so
/// the side decides which neighbour the output position keeps.
pub(crate) fn cursor_to_output(
    mappings: &[EmitMapping],
    src: usize,
    affinity: Affinity,
) -> Option<usize> {
    let ending = || {
        mappings
            .iter()
            .find(|m| m.len > 0 && m.src < src && src <= m.src + m.len)
            .map(|m| m.out + (src - m.src))
    };
    let starting = || {
        mappings
            .iter()
            .find(|m| m.src <= src && src < m.src + m.len)
            .map(|m| m.out + (src - m.src))
    };
    match affinity {
        Affinity::Preceding => ending().or_else(starting),
        Affinity::Following => starting().or_else(ending),
    }
    .or_else(|| to_output_inclusive(mappings, src))
}

/// Where a cursor at source byte `src` lands for a question about what is
/// being typed before it (completion, signature help): [`cursor_to_output`]
/// with [`Affinity::Preceding`], except at the end of a construct whose
/// trailing blanks the output does not copy (an operand whose argument list
/// is still open, with the rest of its line). The cursor is in that list,
/// and the text after the construct is not where it is typed: it has no
/// place in the output, and the caller asks a probe.
pub(crate) fn typed_cursor_to_output(
    mappings: &[EmitMapping],
    anchors: &[EmitAnchor],
    source: &str,
    src: usize,
) -> Option<usize> {
    let copied_before = mappings
        .iter()
        .any(|m| m.len > 0 && m.src < src && src <= m.src + m.len);
    let blanks_before = source.as_bytes()[..src.min(source.len())]
        .iter()
        .rev()
        .take_while(|&&byte| matches!(byte, b' ' | b'\t'))
        .count();
    if !copied_before
        && blanks_before > 0
        && anchors
            .iter()
            .any(|anchor| anchor.src_end == src && anchor.src + blanks_before < src)
    {
        return None;
    }
    cursor_to_output(mappings, src, Affinity::Preceding)
}

/// The inverse of [`to_output_inclusive`], for answers coming back.
pub(crate) fn to_source_inclusive(mappings: &[EmitMapping], out: usize) -> Option<usize> {
    let starting = mappings.partition_point(|m| m.out <= out);
    starting
        .checked_sub(1)
        .map(|index| &mappings[index])
        .filter(|m| out <= m.out + m.len)
        .map(|m| m.src + (out - m.out))
}

pub(crate) fn to_source_span(
    mappings: &[EmitMapping],
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    if end <= start {
        let at = to_source_inclusive(mappings, start)?;
        return (end == start).then_some((at, at));
    }
    let first = chunk_holding(mappings, start)?;
    let mut last = first;
    while end > last.out + last.len {
        let (out, src) = (last.out + last.len, last.src + last.len);
        let from = mappings.partition_point(|m| m.out < out);
        last = mappings[from..]
            .iter()
            .take_while(|m| m.out == out)
            .find(|m| m.len > 0 && m.src == src)?;
    }
    Some((first.src + (start - first.out), last.src + (end - last.out)))
}

/// Where an emitted byte came from, or — when it is compiler-written glue —
/// where the nearest preceding verbatim byte came from.
///
/// A diagnostic on glue still belongs somewhere the user can look: the
/// construct it was generated for starts at the last source byte copied
/// before it. The caller says which of the two happened, so the message can
/// too.
pub(crate) fn to_source_or_nearest(mappings: &[EmitMapping], out: usize) -> Option<(usize, bool)> {
    if let Some(exact) = to_source(mappings, out) {
        return Some((exact, true));
    }
    mappings[..mappings.partition_point(|m| m.out < out)]
        .last()
        .map(|m| (m.src + m.len.saturating_sub(1), false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_origins_cost_one_pass_over_the_anchors_each() {
        let entries = |n: usize| {
            let mappings: Vec<EmitMapping> = (0..n)
                .map(|i| EmitMapping {
                    src: i,
                    out: 3 * i,
                    len: 1,
                })
                .collect();
            let anchors: Vec<EmitAnchor> = (0..n)
                .map(|i| EmitAnchor {
                    out: 3 * i + 2,
                    end: 3 * i + 3,
                    src: i,
                    src_end: i + 1,
                    owner_end: i + 1,
                    context: None,
                    kind: AnchorKind::Try,
                })
                .collect();
            let code = "a  ".repeat(n);
            let source = "a".repeat(n);
            crate::work::measure(|| {
                for i in 0..n {
                    diagnostic_origin(&mappings, &anchors, 3 * i + 1, 3 * i + 2, &code, &source);
                }
            })["diagnostic origin entries"]
        };
        let (small, large) = (entries(200), entries(400));
        assert!(large <= 4 * small + 4 * 400, "{small} -> {large}");
    }
    use crate::AnchorKind;

    #[test]
    fn utf16_offsets_are_measured_in_the_decoded_text() {
        let text = "\u{feff}const 한 = 1;\n";
        let name = text.find('한').unwrap();
        assert_eq!(to_utf16(text, name), 6);
        assert_eq!(from_utf16(text, 6), name);
        assert_eq!(to_utf16(text, 0), 0);
        assert_eq!(to_utf16(text, 3), 0);
        assert_eq!(from_utf16(text, 0), 3);
        assert_eq!(to_utf16(text, text.len()), text.len() - 3 - 2);
    }

    #[test]
    fn a_partially_mapped_diagnostic_belongs_to_its_lowering_anchor() {
        let mappings = [EmitMapping {
            src: 20,
            out: 100,
            len: 5,
        }];
        let anchor = EmitAnchor {
            out: 90,
            end: 140,
            src: 12,
            src_end: 25,
            owner_end: 40,
            context: None,
            kind: AnchorKind::Match,
        };
        assert_eq!(
            diagnostic_origin(&mappings, &[anchor], 100, 130, "", ""),
            Some(DiagnosticOrigin::Anchor(anchor))
        );
        assert_eq!(
            diagnostic_origin(&mappings, &[anchor], 101, 104, "", ""),
            Some(DiagnosticOrigin::Exact { start: 21, end: 24 })
        );
    }

    #[test]
    fn a_diagnostic_belongs_to_the_anchor_that_holds_its_whole_span() {
        // `"a".trim()`: the step anchor holds only the piped value at the
        // start; the whole pipeline's anchor holds the whole call.
        let step = EmitAnchor {
            out: 100,
            end: 103,
            src: 30,
            src_end: 37,
            owner_end: 37,
            context: Some((20, 23)),
            kind: AnchorKind::Pipe,
        };
        let pipeline = EmitAnchor {
            out: 100,
            end: 110,
            src: 20,
            src_end: 37,
            owner_end: 37,
            context: None,
            kind: AnchorKind::Pipe,
        };
        assert_eq!(
            diagnostic_origin(&[], &[step, pipeline], 100, 110, "", ""),
            Some(DiagnosticOrigin::Anchor(pipeline))
        );
        assert_eq!(
            diagnostic_origin(&[], &[step, pipeline], 100, 103, "", ""),
            Some(DiagnosticOrigin::Anchor(step))
        );
    }

    #[test]
    fn a_missing_operand_crosses_trivia_but_never_a_new_owner() {
        let source = "😀 +, next";
        let code = "😀 +  : next";
        let mappings = [EmitMapping {
            src: 0,
            out: 0,
            len: 6,
        }];
        assert_eq!(
            diagnostic_origin(&mappings, &[], 8, 9, code, source),
            Some(DiagnosticOrigin::Exact { start: 6, end: 7 })
        );
        let opened = EmitAnchor {
            out: 7,
            end: 12,
            src: 8,
            src_end: 12,
            owner_end: 12,
            context: None,
            kind: AnchorKind::Match,
        };
        assert_eq!(
            diagnostic_origin(&mappings, &[opened], 8, 9, code, source),
            Some(DiagnosticOrigin::Anchor(opened))
        );
        let unicode_boundary = "😀 +다음";
        assert_eq!(
            diagnostic_origin(&mappings, &[], 8, 9, code, unicode_boundary),
            Some(DiagnosticOrigin::Exact { start: 6, end: 9 })
        );
    }

    #[test]
    fn a_token_right_after_copied_text_is_the_position_after_it() {
        let mappings = [EmitMapping {
            src: 20,
            out: 100,
            len: 5,
        }];
        let anchor = EmitAnchor {
            out: 90,
            end: 140,
            src: 12,
            src_end: 25,
            owner_end: 40,
            context: None,
            kind: AnchorKind::Match,
        };
        // The glue `;` after the copied `radius.`.
        assert_eq!(
            diagnostic_origin(&mappings, &[anchor], 105, 106, "", ""),
            Some(DiagnosticOrigin::Exact { start: 25, end: 25 })
        );
        // Glue a construct opens there is that construct's.
        let opened = EmitAnchor { out: 105, ..anchor };
        assert_eq!(
            diagnostic_origin(&mappings, &[opened, anchor], 105, 106, "", ""),
            Some(DiagnosticOrigin::Anchor(opened))
        );
        // A range reaching back into the copied text keeps its anchor.
        assert_eq!(
            diagnostic_origin(&mappings, &[anchor], 104, 106, "", ""),
            Some(DiagnosticOrigin::Anchor(anchor))
        );
    }

    #[test]
    fn utf16_round_trips_through_multibyte_text() {
        let text = "const 한글 = \"안녕😀\";\nconst bad = 1;";
        let byte = text.find("bad").unwrap();
        let units = to_utf16(text, byte);
        assert_ne!(units, byte, "the prefix has multi-byte characters");
        assert_eq!(from_utf16(text, units), byte);
    }

    #[test]
    fn utf16_offsets_count_a_surrogate_pair_as_two() {
        let text = "😀x";
        assert_eq!(to_utf16(text, text.find('x').unwrap()), 2);
        assert_eq!(from_utf16(text, 2), "😀".len());
    }

    #[test]
    fn offsets_past_the_end_clamp() {
        let text = "abc";
        assert_eq!(to_utf16(text, 99), 3);
        assert_eq!(from_utf16(text, 99), 3);
    }

    #[test]
    fn glue_falls_back_to_the_nearest_preceding_source_byte() {
        let mappings = [
            EmitMapping {
                src: 0,
                out: 0,
                len: 4,
            },
            EmitMapping {
                src: 10,
                out: 20,
                len: 6,
            },
        ];
        assert_eq!(to_source_or_nearest(&mappings, 22), Some((12, true)));
        // Between the chunks: the last byte of the chunk before it.
        assert_eq!(to_source_or_nearest(&mappings, 10), Some((3, false)));
        // Before any chunk: nothing to point at.
        assert_eq!(to_source_or_nearest(&mappings, 0), Some((0, true)));
    }

    #[test]
    fn mappings_round_trip_and_reject_glue() {
        let mappings = [
            EmitMapping {
                src: 0,
                out: 0,
                len: 4,
            },
            EmitMapping {
                src: 10,
                out: 20,
                len: 6,
            },
        ];
        assert_eq!(to_output(&mappings, 2), Some(2));
        assert_eq!(to_output(&mappings, 12), Some(22));
        assert_eq!(to_source(&mappings, 22), Some(12));
        // Between the chunks is compiler-written glue.
        assert_eq!(to_output(&mappings, 8), None);
        assert_eq!(to_source(&mappings, 10), None);
    }

    #[test]
    fn a_span_maps_only_over_copied_source_bytes() {
        let mappings = [
            EmitMapping {
                src: 0,
                out: 0,
                len: 10,
            },
            EmitMapping {
                src: 10,
                out: 10,
                len: 4,
            },
            EmitMapping {
                src: 50,
                out: 20,
                len: 5,
            },
        ];
        assert_eq!(to_source_span(&mappings, 2, 6), Some((2, 6)));
        assert_eq!(to_source_span(&mappings, 8, 12), Some((8, 12)));
        assert_eq!(to_source_span(&mappings, 21, 25), Some((51, 55)));
        assert_eq!(to_source_span(&mappings, 14, 20), None);
        assert_eq!(to_source_span(&mappings, 12, 22), None);
        assert_eq!(to_source_span(&mappings, 14, 14), Some((14, 14)));
    }
}
