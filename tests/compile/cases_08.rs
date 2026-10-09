#[test]
fn val_probes_collect_every_method_call_for_the_verdict() {
    // The delegated form collects method calls whatever they are called:
    // the mutator-name policy is applied at the verdict, beside the
    // checker's built-in answer, so a name outside the policy can never
    // hide a question — and never make a report on its own.
    const SRC: &str = "\
val const d = mk();
d.setHours(1);
d.at(0);
d.count = 2;
";
    let probes = ttc::val_probes(SRC);
    let seen: Vec<(&str, Option<&str>)> = probes
        .mutations
        .iter()
        .map(|m| (m.name.as_str(), m.method.as_ref().map(|(n, _)| n.as_str())))
        .collect();
    assert_eq!(
        seen,
        [("d", Some("setHours")), ("d", Some("at")), ("d", None),]
    );
    // The policy half of the verdict, stated as the library's own answer.
    assert!(ttc::is_builtin_mutator_name("push"));
    assert!(!ttc::is_builtin_mutator_name("at"));
    assert!(!ttc::is_builtin_mutator_name("get"));
}

#[test]
fn val_probes_carry_the_callee_and_the_declarations_it_may_name() {
    // The call-capability check's pairing is delegated: probes hand over
    // every declaration (as a node) and every call's callee (as a node),
    // and which call names which declaration is symbol identity — so
    // nothing is matched by name here, and an "ambiguous" name is not a
    // concept collection needs.
    const SRC: &str = "\
val const user = { name: \"a\" };
function handle(u: { name: string }): void {}
handle(user);
handle(user.name, user);
";
    let probes = ttc::val_probes(SRC);
    assert_eq!(probes.functions.len(), 1);
    let function = &probes.functions[0];
    assert_eq!(function.name, "handle");
    assert_eq!(&SRC[function.ident..function.ident + 6], "handle");
    assert_eq!(
        function.params,
        vec![ttc::ValParam {
            name: Some("u".into()),
            is_val: false,
        }]
    );
    let seen: Vec<(&str, &str, usize)> = probes
        .passes
        .iter()
        .map(|p| (p.name.as_str(), p.callee.as_str(), p.arg_index))
        .collect();
    // Every plain-path argument is collected with its position — including
    // `user.name` at index 0 and `user` at index 1 of the second call.
    assert_eq!(
        seen,
        [
            ("user", "handle", 0),
            ("user", "handle", 0),
            ("user", "handle", 1),
        ]
    );
    for pass in &probes.passes {
        assert_eq!(&SRC[pass.callee_at..pass.callee_at + 6], "handle");
    }
}

/* ------------------------------------------------------------------ */
/* name resolution (TASK-102)                                          */
/* ------------------------------------------------------------------ */

/// Applies one of a diagnostic's suggestions to `source` — what an
/// editor's quick fix does when the reader picks that action.
///
/// One suggestion, not all of them: the suggestions on a diagnostic are
/// *alternative* ways to resolve it (`Suggestion`'s own contract), and
/// closing a match's holes by writing the arms and by writing `_` are two
/// of them.
fn with_suggestion_applied(source: &str, diagnostic: &ttc::Diagnostic, which: usize) -> String {
    let edit = diagnostic.suggestions[which]
        .edit
        .as_ref()
        .expect("an applicable edit");
    let mut out = source.to_string();
    out.replace_range(edit.start..edit.end, &edit.replacement);
    out
}

#[test]
fn a_misspelled_case_carries_its_replacement_as_an_edit() {
    let src = "variant Shape { Circle(radius: number), Empty }\nconst a = match (s) { Circel(radius) => radius, Empty => 0 };\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let d = &diagnostics[0];
    assert_eq!(d.code, ttc::DiagnosticCode::UnknownCase);
    // The fix is data, not a sentence: the message must not spell it.
    assert!(!d.message.contains("Circle`?"), "{}", d.message);
    let edit = d.suggestions[0]
        .edit
        .as_ref()
        .expect("a named replacement is an applicable edit");
    assert_eq!(&src[edit.start..edit.end], "Circel");
    assert_eq!(edit.replacement, "Circle");
}

#[test]
fn a_misspelled_field_carries_its_replacement_as_an_edit() {
    let src = "variant Shape { Circle(radius: number), Empty }\nconst a = match (s) { Circle(radiuz) => radiuz, Empty => 0 };\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let d = &diagnostics[0];
    assert_eq!(d.code, ttc::DiagnosticCode::UnknownField);
    let edit = d.suggestions[0].edit.as_ref().expect("an applicable edit");
    assert_eq!(&src[edit.start..edit.end], "radiuz");
    assert_eq!(edit.replacement, "radius: radiuz");
}

#[test]
fn applying_a_suggested_edit_resolves_the_diagnostic_it_came_from() {
    // The contract that makes a suggestion worth carrying: what it says to
    // write is what makes the error go away.
    let src = "variant Shape { Circle(radius: number), Empty }\nconst a = match (s) { Circel(radius) => radius, Empty => 0 };\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let fixed = with_suggestion_applied(src, &diagnostics[0], 0);
    assert!(
        ttc::analyze(&fixed, &Options::default()).is_empty(),
        "{fixed}\n{:#?}",
        ttc::analyze(&fixed, &Options::default()),
    );
}

/// The `match-not-exhaustive` diagnostic of `src`, or a panic.
fn hole(src: &str) -> ttc::Diagnostic {
    ttc::analyze(src, &Options::default())
        .into_iter()
        .find(|d| d.code == ttc::DiagnosticCode::MatchNotExhaustive)
        .expect("the hole is reported")
}

#[test]
fn a_match_with_holes_carries_the_arms_that_close_them() {
    // The compiler writes the arms: it is the only party that knows what
    // is missing, what each case's payload is called, and where the body's
    // braces are (TASK-216).
    let src = "variant Shape { Circle(r: number), Empty }\nconst a = match (s) {\n  Circle(r) => r,\n};\n";
    let d = hole(src);
    assert!(!d.message.contains("add the missing arms"), "{}", d.message);
    assert_eq!(d.suggestions.len(), 2);
    assert_eq!(d.suggestions[0].message, "add the missing arms");
    assert_eq!(d.suggestions[1].message, "or add a final `_` arm");
    let edit = d.suggestions[0].edit.as_ref().expect("an applicable edit");
    assert_eq!(edit.replacement, "  Empty => undefined,\n");
    // Inserted above the closing brace, so the arms land inside the body.
    assert_eq!(&src[edit.start..edit.start + 2], "};");
}

#[test]
fn an_authored_arm_binds_the_payload_the_body_will_need() {
    // The message names the value (`Circle`); the arm has to bind what the
    // body will use, and the field name comes from the declaration the
    // analysis already read.
    let src =
        "variant Shape { Circle(r: number), Empty }\nconst a = match (s) {\n  Empty => 0,\n};\n";
    let d = hole(src);
    assert!(d.message.contains("missing \"Circle\""), "{}", d.message);
    let edit = d.suggestions[0].edit.as_ref().expect("an applicable edit");
    assert_eq!(edit.replacement, "  Circle(r) => undefined,\n");
}

#[test]
fn applying_the_authored_arms_makes_the_match_exhaustive() {
    // The contract that makes the edit worth carrying, for a rule whose
    // fix is an insertion rather than a replacement.
    for src in [
        "variant Shape { Circle(r: number), Square(s: number), Empty }\nconst a = match (v) {\n  Empty => 0,\n};\n",
        "variant Shape { Circle(r: number), Empty }\nconst a = match (v) { Empty => 0 };\n",
        // A tuple match: the fix is a combination per position.
        "variant Dir { North(), South }\nvariant Speed { Fast(), Slow }\nconst step = match (d, s) {\n  (North, Fast) => 2,\n  (North, Slow) => 1,\n  (South, Fast) => -1,\n};\n",
        // A payload hole: the witness constrains one field and binds the rest.
        "variant Inner { Yes, No }\nvariant Outer { Wrap(inner: Inner, tag: number), Empty }\nconst a = match (v) {\n  Wrap(inner: Yes()) => 1,\n  Empty => 0,\n};\n",
    ] {
        let d = hole(src);
        let fixed = with_suggestion_applied(src, &d, 0);
        let left = ttc::analyze(&fixed, &Options::default());
        assert!(
            left.iter()
                .all(|d| d.code != ttc::DiagnosticCode::MatchNotExhaustive),
            "{fixed}\n{left:#?}"
        );
    }
}

#[test]
fn the_wildcard_arm_closes_the_hole_too() {
    let src =
        "variant Shape { Circle(r: number), Empty }\nconst a = match (v) {\n  Empty => 0,\n};\n";
    let d = hole(src);
    let fixed = with_suggestion_applied(src, &d, 1);
    assert!(fixed.contains("  _ => undefined,"), "{fixed}");
    assert!(
        ttc::analyze(&fixed, &Options::default())
            .iter()
            .all(|d| d.code != ttc::DiagnosticCode::MatchNotExhaustive),
        "{fixed}"
    );
}

#[test]
fn an_authored_arm_keeps_a_one_line_match_on_one_line() {
    let src = "variant Shape { Circle(r: number), Empty }\nconst a = match (v) { Empty => 0 };\n";
    let d = hole(src);
    let edit = d.suggestions[0].edit.as_ref().expect("an applicable edit");
    assert_eq!(edit.replacement, ", Circle(r) => undefined, ");
    assert_eq!(
        with_suggestion_applied(src, &d, 0),
        "variant Shape { Circle(r: number), Empty }\nconst a = match (v) { Empty => 0, Circle(r) => undefined, };\n"
    );
}

#[test]
fn every_authored_arm_edit_compiles_after_the_last_written_arm() {
    let prelude = "variant Shape { Circle(radius: number), Rect(width: number, height: number), Point }\ndeclare const s: Shape;\n";
    for body in [
        "const a = match (s) {\n  Circle(radius) => radius\n};\n",
        "const a = match (s) {\n  Circle(r) => { return radius; }\n};\n",
        "const a = match (s) {\n  Circle(radius) => radius // trailing\n};\n",
        "const a = match (s) {\n  Circle(radius) => radius\n  // trailing\n};\n",
        "const a = match (s) {\n  Circle(radius) => radius /* note, here */\n};\n",
        "const a = match (s) {\n  Circle(radius) => radius, // after\n};\n",
        "const a = match (s) {\n  Circle(radius) => radius /* , */\n};\n",
        "const a = match (s) { Circle(r) => { return radius; } };\n",
        "const a = match (s) { Circle(radius) => radius /* note */ };\n",
        "const a = match (s) { Circle(radius) => radius, /* note */ };\n",
        "const a = match (s) { Circle(radius) => radius,};\n",
        "const a = match (s) { Circle(radius) => radius\n};\n",
        "const a = match (s, s) {\n  (Circle(radius), _) => radius\n};\n",
    ] {
        let src = format!("{prelude}{body}");
        let d = hole(&src);
        for which in 0..d.suggestions.len() {
            let fixed = with_suggestion_applied(&src, &d, which);
            let left = ttc::analyze(&fixed, &Options::default());
            assert!(left.is_empty(), "{fixed}\n{left:#?}");
        }
    }
}

#[test]
fn authored_arms_take_the_indentation_of_the_written_arms() {
    let src = "variant Shape { Circle(r: number), Empty }\nfunction f(s: Shape) {\n\tconst a = match (s) {\n\t\tCircle(r) => r\n\t};\n}\n";
    let d = hole(src);
    assert_eq!(
        with_suggestion_applied(src, &d, 0),
        "variant Shape { Circle(r: number), Empty }\nfunction f(s: Shape) {\n\tconst a = match (s) {\n\t\tCircle(r) => r,\n\t\tEmpty => undefined,\n\t};\n}\n"
    );
    assert_eq!(
        with_suggestion_applied(src, &d, 1),
        "variant Shape { Circle(r: number), Empty }\nfunction f(s: Shape) {\n\tconst a = match (s) {\n\t\tCircle(r) => r,\n\t\t_ => undefined,\n\t};\n}\n"
    );
    let src = "variant Shape { Circle(r: number), Empty }\nconst a = match (s) {\n    Circle(r) => r, // kept\n};\n";
    let d = hole(src);
    assert_eq!(
        with_suggestion_applied(src, &d, 0),
        "variant Shape { Circle(r: number), Empty }\nconst a = match (s) {\n    Circle(r) => r, // kept\n    Empty => undefined,\n};\n"
    );
}

#[test]
fn authored_arm_lines_end_with_the_line_ending_of_a_crlf_file() {
    for (src, arms) in [
        (
            "variant Shape { Circle(r: number), Point }\r\ndeclare const s: Shape;\r\nconst a = match (s) {\r\n    Circle(r) => r,\r\n};\r\n",
            "    Point => undefined,\r\n",
        ),
        (
            "variant Dir { North, South }\r\ndeclare const d: Dir;\r\nconst b = match (d, d) {\r\n  (North, _) => 1,\r\n  (South, North) => 2,\r\n};\r\n",
            "  (South, South) => undefined,\r\n",
        ),
        (
            "variant Shape { Circle(r: number), Point }\r\ndeclare const s: Shape;\r\nconst c = match (s) {\r\n    Circle(r) => r\r\n};\r\n",
            "    Point => undefined,\r\n",
        ),
    ] {
        let d = hole(src);
        let edit = d.suggestions[0].edit.as_ref().expect("an applicable edit");
        assert!(edit.replacement.ends_with(arms), "{:?}", edit.replacement);
        for which in 0..d.suggestions.len() {
            let fixed = with_suggestion_applied(src, &d, which);
            assert_eq!(
                fixed.matches('\n').count(),
                fixed.matches("\r\n").count(),
                "{fixed:?}"
            );
            assert!(
                ttc::analyze(&fixed, &Options::default()).is_empty(),
                "{fixed:?}"
            );
        }
    }
}

/// A CR-only file's arm lines are found by the same line model the
/// positions use, and the authored lines end with CR (TASK-498).
#[test]
fn authored_arm_lines_end_with_the_line_ending_of_a_cr_file() {
    let src = "variant Shape { Circle(r: number), Point }\rdeclare const s: Shape;\rfunction f() {\r\tconst a = match (s) {\r\t\tCircle(r) => r,\r\t};\r}\r";
    let d = hole(src);
    let edit = d.suggestions[0].edit.as_ref().expect("an applicable edit");
    assert_eq!(edit.replacement, "\t\tPoint => undefined,\r");
    assert_eq!(
        with_suggestion_applied(src, &d, 0),
        "variant Shape { Circle(r: number), Point }\rdeclare const s: Shape;\rfunction f() {\r\tconst a = match (s) {\r\t\tCircle(r) => r,\r\t\tPoint => undefined,\r\t};\r}\r"
    );
    for which in 0..d.suggestions.len() {
        let fixed = with_suggestion_applied(src, &d, which);
        assert!(!fixed.contains('\n'), "{fixed:?}");
        assert!(
            ttc::analyze(&fixed, &Options::default()).is_empty(),
            "{fixed:?}"
        );
    }
}

#[test]
fn a_misspelled_case_of_an_imported_variant_names_its_origin() {
    let externs = [token_extern()];
    let opts = Options {
        extern_variants: &externs,
        ..Options::default()
    };
    let e = compile(
        "const s = match (t) { Num(value) => value, Idnet(name) => 0, Eof => -1 };\n",
        &opts,
    )
    .expect_err("expected a resolution error");
    assert!(
        e.message
            .contains("variant Token (imported from \"./token.tt\") has no case `Idnet`"),
        "{}",
        e.message
    );
}
