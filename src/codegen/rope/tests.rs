use super::*;

#[test]
fn target_assigns_exact_construct_and_file_origins() {
    let mut rope = Rope::new();
    rope.push_src("const ", 0);
    let mut generated = Rope::new();
    generated.push_lit("value");
    rope.anchored(AnchorKind::Match, 6, 11, 11, generated);
    rope.push_lit(";\n");

    let target = TargetFile::from_rope(rope, 11);
    assert_eq!(target.validate(), Ok(()));
    assert!(matches!(
        target.pieces[0],
        TargetPiece::Source {
            origin: ExactOrigin { start: 0, end: 6 },
            ..
        }
    ));
    assert!(matches!(
        target.pieces[2],
        TargetPiece::Generated {
            origin: SourceOrigin::Construct {
                kind: AnchorKind::Match,
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        target.pieces[4],
        TargetPiece::Generated {
            origin: SourceOrigin::Synthetic {
                reason: SyntheticReason::UnanchoredGenerated,
                ..
            },
            ..
        }
    ));
    let flat = target.print();
    assert_eq!(flat.code, "const value;\n");
    assert_eq!(
        flat.mappings,
        [EmitMapping {
            src: 0,
            out: 0,
            len: 6
        }]
    );
}

#[test]
fn target_rejects_a_source_piece_outside_the_input() {
    let mut rope = Rope::new();
    rope.push_src("x", 2);
    let target = TargetFile::from_rope(rope, 2);
    assert_eq!(
        target.validate(),
        Err(TargetError::SourceOutOfBounds { start: 2, end: 3 })
    );
}

#[test]
fn target_rejects_a_break_with_no_layout_scope() {
    // A break's indentation is meaningless without a scope to measure
    // it from, so the target refuses one — an emitter that writes block
    // structure has to say where that structure starts.
    let mut loose = Rope::new();
    loose.push_lit("x");
    loose.push_break(1);
    let target = TargetFile::from_rope(loose, 0);
    assert_eq!(target.validate(), Err(TargetError::BreakOutsideScope));

    let mut scoped = Rope::new();
    scoped.push_lit("x");
    scoped.push_break(1);
    let target = TargetFile::from_rope(Rope::scoped(scoped), 0);
    assert_eq!(target.validate(), Ok(()));
}

#[test]
fn trailing_line_comment_detection_uses_lexical_context() {
    let rope = |text| {
        let mut rope = Rope::new();
        rope.push_src(text, 0);
        rope
    };
    assert!(rope("value // note").last_line_has_line_comment(SourceKind::TypeScript));
    assert!(!rope("value /").last_line_has_line_comment(SourceKind::TypeScript));
    assert!(!rope("\"//\"").last_line_has_line_comment(SourceKind::TypeScript));
    assert!(!rope("`//`").last_line_has_line_comment(SourceKind::TypeScript));
    assert!(!rope(r"/\/\//").last_line_has_line_comment(SourceKind::TypeScript));
    assert!(!rope("<div>//</div>").last_line_has_line_comment(SourceKind::Tsx));

    let mut split = Rope::new();
    split.push_src("value /", 0);
    split.push_lit("/ note");
    assert!(split.last_line_has_line_comment(SourceKind::TypeScript));
}

#[test]
fn source_in_a_top_level_construct_is_an_insertion_boundary() {
    let source = "`value`";
    let mut inner = Rope::new();
    inner.push_src(source, 0);
    let mut rope = Rope::new();
    rope.anchored(AnchorKind::Pipe, 0, source.len(), source.len(), inner);

    rope.insert_lit_at_source(0, "import { helper } from \"runtime\";\n");

    assert_eq!(
        rope.resolved_text().as_deref(),
        Some("import { helper } from \"runtime\";\n`value`")
    );
}

#[test]
fn target_rejects_unbalanced_anchor_structure() {
    let rope = Rope {
        pieces: vec![Piece::Close],
        len: 0,
    };
    let target = TargetFile::from_rope(rope, 0);
    assert_eq!(target.validate(), Err(TargetError::CloseWithoutOpen));
}

fn preserved<'a>(source: &'a str, rope: Rope<'a>) -> TargetFile<'a> {
    let mut target = TargetFile::from_rope(rope, source.len());
    target.source = Some(source);
    target
}

fn owned_whole(source: &str) -> SourcePreservation {
    SourcePreservation {
        owned: vec![SourceSpan {
            start: 0,
            end: source.len(),
        }],
        relocated: Vec::new(),
        rewritten: Vec::new(),
    }
}

#[test]
fn preservation_accepts_a_faithful_pass_through() {
    let source = "const a = 1;";
    let mut rope = Rope::new();
    rope.push_src(&source[..6], 0);
    rope.push_src(&source[6..], 6);
    let target = preserved(source, rope);
    assert_eq!(
        target.validate_source_preservation(&owned_whole(source)),
        Ok(())
    );
}

#[test]
fn preservation_rejects_a_pass_through_byte_printed_twice() {
    // The defect shape: a relocated range whose bytes also stay in
    // place — the relocation excuses the order, never the count.
    let source = "const a = 1;";
    let mut rope = Rope::new();
    rope.push_src(source, 0);
    rope.push_src(&source[6..7], 6);
    let target = preserved(source, rope);
    let preservation = SourcePreservation {
        owned: vec![SourceSpan {
            start: 0,
            end: source.len(),
        }],
        relocated: vec![SourceSpan { start: 6, end: 7 }],
        rewritten: Vec::new(),
    };
    let error = target
        .validate_source_preservation(&preservation)
        .expect_err("duplicate must be rejected");
    assert_eq!(error.invariant, Invariant::SourceEmittedTwice);
    assert_eq!(error.stage, LoweringStage::TargetSourcePreservation);
}

#[test]
fn preservation_rejects_a_dropped_pass_through_byte() {
    let source = "const a = 1;";
    let mut rope = Rope::new();
    rope.push_src(&source[..6], 0);
    // bytes 6.. never emitted
    let target = preserved(source, rope);
    let error = target
        .validate_source_preservation(&owned_whole(source))
        .expect_err("drop must be rejected");
    assert_eq!(error.invariant, Invariant::SourceOmitted);
    assert_eq!(error.span, Some(SourceSpan { start: 6, end: 7 }));
}

#[test]
fn preservation_allows_dropped_unicode_whitespace_only() {
    let source = "a\u{b}\u{2003}b";
    let mut rope = Rope::new();
    rope.push_src(&source[..1], 0);
    rope.push_src(&source[5..], 5);
    let target = preserved(source, rope);
    assert_eq!(
        target.validate_source_preservation(&owned_whole(source)),
        Ok(())
    );
}

#[test]
fn preservation_rejects_an_unregistered_reorder() {
    let source = "ab";
    let mut rope = Rope::new();
    rope.push_src(&source[1..], 1);
    rope.push_src(&source[..1], 0);
    let target = preserved(source, rope);
    let error = target
        .validate_source_preservation(&owned_whole(source))
        .expect_err("reorder must be rejected");
    assert_eq!(error.invariant, Invariant::SourceReordered);
}

#[test]
fn preservation_accepts_a_registered_relocation() {
    let source = "ab";
    let mut rope = Rope::new();
    rope.push_src(&source[1..], 1);
    rope.push_src(&source[..1], 0);
    let target = preserved(source, rope);
    let preservation = SourcePreservation {
        owned: vec![SourceSpan { start: 0, end: 2 }],
        relocated: vec![SourceSpan { start: 1, end: 2 }],
        rewritten: Vec::new(),
    };
    assert_eq!(target.validate_source_preservation(&preservation), Ok(()));
}

#[test]
fn preservation_exempts_a_registered_rewrite_from_coverage() {
    let source = "return x;";
    let mut rope = Rope::new();
    // The exit rewrite prints only the argument; the frame is claimed.
    rope.push_src(&source[7..8], 7);
    let target = preserved(source, rope);
    let preservation = SourcePreservation {
        owned: vec![SourceSpan { start: 0, end: 9 }],
        relocated: Vec::new(),
        rewritten: vec![
            SourceSpan { start: 0, end: 7 },
            SourceSpan { start: 8, end: 9 },
        ],
    };
    assert_eq!(target.validate_source_preservation(&preservation), Ok(()));
}
