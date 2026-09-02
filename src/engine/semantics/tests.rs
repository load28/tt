use super::*;

/// The declaration table of a source, as a translation sees it.
fn table(source: &str) -> Vec<DeclaredVariant> {
    crate::pattern_analyses(source, &[]).declarations
}

/// The motivating example of TASK-118: the propagated `Err` and the
/// return type it does not fit, both said as the user declared them.
#[test]
fn a_case_is_named_and_a_full_union_is_its_variant() {
    let declarations = table(
        "variant Test { OutOfRange(value: number), Empty }\n\
             variant ParseError { NotANumber(text: string) }\n",
    );
    let said = translate(
        AnchorKind::Try,
        2322,
        "Type 'Err<{ kind: \"OutOfRange\"; value: number; }>' is not assignable to type \
             'Result<string, { kind: \"NotANumber\"; text: string; }>'.",
        &declarations,
    )
    .expect("translated");
    assert!(
        said.contains(
            "(in tt's names: Type 'Err<Test.OutOfRange>' is not assignable to type \
                 'Result<string, ParseError>'.)"
        ),
        "{said}"
    );
    // The original rides along, unchanged — the names are checkable.
    assert!(
        said.contains("(ts2322: Type 'Err<{ kind: \"OutOfRange\""),
        "{said}"
    );
}

#[test]
fn a_partial_union_stays_a_union_of_named_cases() {
    let declarations = table("variant E { A(x: number), B, C }\n");
    let named = name_types(
        "Type '{ kind: \"A\"; x: number; } | { kind: \"B\"; }' is not assignable to type 'E'.",
        &declarations,
    )
    .expect("named");
    assert_eq!(
        named,
        "Type 'E.A | E.B' is not assignable to type 'E'.".to_string()
    );
}

#[test]
fn a_tag_two_declarations_answer_to_is_not_named() {
    let declarations = table("variant A { Empty }\nvariant B { Empty }\n");
    assert_eq!(
        name_types("Type '{ kind: \"Empty\"; }'.", &declarations),
        None
    );
}

#[test]
fn a_type_that_only_shares_a_tag_is_not_named() {
    // Same tag, different payload — some other type, not this case.
    let declarations = table("variant E { A(x: number) }\n");
    assert_eq!(
        name_types("Type '{ kind: \"A\"; y: string; }'.", &declarations),
        None
    );
}

#[test]
fn a_message_with_no_structural_case_is_left_alone() {
    let declarations = table("variant E { A(x: number), B }\n");
    assert_eq!(
        name_types(
            "Property 'kind' does not exist on type 'number'.",
            &declarations
        ),
        None,
    );
    // ... and a translation of it carries no naming clause.
    let said = translate(
        AnchorKind::Try,
        2339,
        "Property 'kind' does not exist.",
        &declarations,
    )
    .expect("translated");
    assert!(!said.contains("in tt's names"), "{said}");
}

#[test]
fn a_case_nested_in_another_type_is_named_where_it_stands() {
    let declarations = table("variant E { A(x: number), B }\n");
    let named = name_types(
        "Type 'Array<{ kind: \"A\"; x: number; }>' is not assignable.",
        &declarations,
    )
    .expect("named");
    assert_eq!(named, "Type 'Array<E.A>' is not assignable.".to_string());
}

#[test]
fn a_tag_union_names_nothing() {
    // `{ kind: "A" | "B" }` is not one case, so it is not one name.
    let declarations = table("variant E { A(x: number), B }\n");
    assert_eq!(
        name_types("Type '{ kind: \"A\" | \"B\"; }'.", &declarations),
        None
    );
}

#[test]
fn translation_classes_group_incidental_ts_codes_by_tt_meaning() {
    assert_eq!(translation_class(AnchorKind::Try, 2339), Some("not-result"));
    assert_eq!(translation_class(AnchorKind::Try, 2571), Some("not-result"));
    assert_eq!(
        translation_class(AnchorKind::Try, 2322),
        Some("try-error-type")
    );
    assert_eq!(translation_class(AnchorKind::Pipe, 2339), None);
    assert_eq!(
        translation_class(AnchorKind::Pipe, 2345),
        Some("pipe-step-input")
    );
}

#[test]
fn a_pipe_mismatch_translates_to_the_steps_expectation() {
    let said = translate(
        AnchorKind::Pipe,
        2345,
        "Argument of type 'number' is not assignable to parameter of type 'string'.",
        &[],
    )
    .expect("translated");
    assert!(
        said.starts_with("this pipeline step expects `string`, but receives `number`"),
        "{said}"
    );
    assert!(said.contains("ts2345:"), "the original rides along: {said}");
}

#[test]
fn a_pipe_translation_uses_the_deepest_elaborated_pair() {
    // A flow boundary mismatches as two function types; the elaboration
    // descends to the value types, and the deepest pair is the boundary.
    let said = translate(
        AnchorKind::Pipe,
        2345,
        "Argument of type '(n: number) => number' is not assignable to parameter of type \
             '(n: number) => string'.\n  Type 'number' is not assignable to type 'string'.",
        &[],
    )
    .expect("translated");
    assert!(
        said.starts_with("this pipeline step expects `string`, but receives `number`"),
        "{said}"
    );
}

#[test]
fn an_unparseable_pipe_mismatch_still_says_what_the_step_meant() {
    let said = translate(AnchorKind::Pipe, 2345, "something unusual.", &[]).expect("translated");
    assert!(
        said.starts_with("this pipeline step cannot accept the value flowing into it"),
        "{said}"
    );
}

#[test]
fn assignability_pairs_parse_both_sentence_forms() {
    assert_eq!(
        assignability_pair("Argument of type 'A' is not assignable to parameter of type 'B'."),
        Some(("A".to_string(), "B".to_string()))
    );
    assert_eq!(
        assignability_pair("Type 'A' is not assignable to type 'B'."),
        Some(("A".to_string(), "B".to_string()))
    );
    assert_eq!(assignability_pair("This expression is not callable."), None);
}

#[test]
fn an_ordinary_ts_message_also_uses_tt_names() {
    let declarations = table("variant E { A(x: number), B }\n");
    let said = ts_message(
        "Type '{ kind: \"A\"; x: number; }' is not assignable.",
        &declarations,
    );
    assert_eq!(
        said,
        "Type '{ kind: \"A\"; x: number; }' is not assignable. \
             (in tt's names: Type 'E.A' is not assignable.)"
    );
}

#[test]
fn diagnostics_are_source_sorted_and_display_duplicates_are_merged() {
    let at = |line, code: &str| Diagnostic {
        path: PathBuf::from("/p/a.tt"),
        position: Some((line, 1)),
        end: Some((line, 2)),
        message: "same".to_string(),
        code: Some(code.to_string()),
        suggestions: Vec::new(),
        labels: Vec::new(),
    };
    let finished = finish_diagnostics(vec![at(2, "ts9999"), at(1, "ts1000"), at(2, "ts1001")]);
    assert_eq!(finished.len(), 2);
    assert_eq!(finished[0].position, Some((1, 1)));
    assert_eq!(finished[1].position, Some((2, 1)));
}
