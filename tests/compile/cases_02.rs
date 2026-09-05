#[test]
fn match_arms_reject_only_control_transfers_that_cross_the_arm() {
    let crossing = ttc::analyze(
        "outer: for (;;) { const value = match (x) { is Error => { continue outer; }, _ => 0 }; }\n",
        &Options::default(),
    );
    assert_eq!(crossing.len(), 1, "{crossing:#?}");
    assert_eq!(crossing[0].code, ttc::DiagnosticCode::MatchControlCrossing);

    let internal = ok(
        "const value = match (x) { is Error => { while (ready()) { if (stop()) break; continue; } return 1; }, _ => 0 };\n",
    );
    assert!(internal.contains("while (ready())"), "{internal}");

    let tuple = ttc::analyze(
        "outer: for (;;) { const value = match (a, b) { (A, B) => { continue outer; }, _ => 0 }; }\n",
        &Options::default(),
    );
    assert_eq!(tuple.len(), 1, "{tuple:#?}");
    assert_eq!(tuple[0].code, ttc::DiagnosticCode::MatchControlCrossing);

    let yielded = ttc::analyze(
        "function* values() { return match (x) { is Error => { yield x; return 1; }, _ => 0 }; }\n",
        &Options::default(),
    );
    assert_eq!(yielded.len(), 1, "{yielded:#?}");
    assert_eq!(yielded[0].code, ttc::DiagnosticCode::MatchControlCrossing);
}

#[test]
fn rejected_match_boundaries_do_not_emit_an_expression_helper() {
    let source = "variant E { A, B }\nfunction f(value = match (E.A) { A => 1, B => 0 }) { return value; }\n";
    let report = ttc::compile_report(source, &Options::default());
    assert!(report.emit.is_none());
    assert_eq!(
        report.diagnostics[0].code,
        ttc::DiagnosticCode::MatchPlacement
    );
}

#[test]
fn one_owner_schedules_multiple_tt_values_without_expression_boundaries() {
    let out = ok(
        "variant E { A(value: number), B }\nconst value = new (match (ctor) { A(value) => value, B => fallback })(match (arg) { A(value) => value, B => 0 });\n",
    );
    assert!(!out.contains("$tt_expr(() =>"), "{out}");
    assert_eq!(out.matches("switch (").count(), 2, "{out}");
    assert!(out.contains("const value = new ($tt_v0)($tt_v1);"), "{out}");
}

#[test]
fn reference_protocol_preserves_optional_calls_and_structures_tagged_templates() {
    // TASK-160 결정 17: a member optional call is one whole operation — the
    // receiver and callee evaluate once, the argument only past the nullish
    // check, and the call goes through the receiver.
    let optional = ok("variant E { A(value: number), B }\n\
         const value = receiver.method?.(match (input) { A(value) => value, B => 0 });\n");
    assert!(!optional.contains("$tt_expr"), "{optional}");
    assert!(optional.contains("!= null) {"), "{optional}");
    assert!(optional.contains(".call("), "{optional}");
    let check = optional.find("!= null").expect("nullish check");
    let lowering = optional.find("switch (").expect("value region");
    assert!(check < lowering, "{optional}");

    let tagged = ok("variant E { A(value: number), B }\n\
         const value = receiver.tag`value:${match (input) { A(value) => value, B => 0 }}`;\n");
    assert!(!tagged.contains("$tt_expr(() =>"), "{tagged}");
    assert!(tagged.contains(".bind("), "{tagged}");
    assert_eq!(tagged.matches("switch (").count(), 1, "{tagged}");
}

#[test]
fn match_wildcard_must_be_last_with_position() {
    let e = err("const r = match (x) { _ => 0, A => 1 };");
    assert!(e.message.contains("must be the last arm"), "{}", e.message);
    assert_eq!((e.line, e.col), (1, 23));
}

#[test]
fn match_duplicate_arm_is_error() {
    let e = err("const r = match (x) { A => 1, A => 2 };");
    assert!(e.message.contains("duplicate arm \"A\""), "{}", e.message);
    assert_eq!((e.line, e.col), (1, 31));
}

#[test]
fn direct_return_match_keeps_await_in_the_host_function() {
    let out = ok(
        "async function f(x: T) { return match (x) { A(url) => await fetch(url), _ => null }; }",
    );
    assert!(!out.contains("async () =>"), "{out}");
    assert!(out.contains("$tt_v0 = await fetch(url);"), "{out}");
    assert!(out.contains("return $tt_v0;"), "{out}");
}

#[test]
fn direct_return_match_does_not_require_a_semicolon() {
    let out = ok("function f(x: T) {\n  return match (x) { A(value) => value, _ => 0 }\n}\n");
    assert!(!out.contains("(() =>"), "{out}");
    assert!(out.contains("switch ($tt_m.kind)"), "{out}");
}

#[test]
fn match_nested_compiles_recursively() {
    let out = ok(r#"
const r = match (a) {
  X(inner) => match (inner) { Y => 1, _ => 2 },
  _ => 0,
};
"#);
    assert_eq!(out.matches("switch ($tt_m.kind)").count(), 2);
}

#[test]
fn match_inside_template_interpolation() {
    let out = ok("const s = `v=${match (x) { A => 1, _ => 0 }}`;");
    assert!(out.contains("switch ($tt_m.kind)"));
}

#[test]
fn match_binding_alias_and_block_body() {
    let out = ok(r#"
const r = match (m) {
  Move(x: px, y: py) => {
    const sum = px + py;
    return sum;
  },
  _ => 0,
};
"#);
    assert!(out.contains("const { x: px, y: py } = $tt_m;"));
    assert!(compact(&out).contains("break; }"));
}

#[test]
fn error_position_reported_inside_template_interpolation() {
    let e = err("const s = `${match (x) { A => 1, A => 2 }}`;\n");
    assert!(e.message.contains("duplicate arm"), "{}", e.message);
    assert_eq!((e.line, e.col), (1, 34));
}

/* ------------------------------------------------------------------ */
/* or-patterns                                                         */
/* ------------------------------------------------------------------ */

#[test]
fn or_pattern_emits_fallthrough_cases() {
    let out = ok(r#"
variant Key { Enter(), Escape, Tab, Char(ch: string) }
const action = match (key) {
  Enter => "submit",
  Escape | Tab => "cancel",
  Char(ch) => "type:" + ch,
};
"#);
    assert!(
        compact(&out).contains("case \"Escape\": case \"Tab\": { $tt_v0 = \"cancel\"; break; }"),
        "{out}"
    );
}

#[test]
fn or_pattern_with_identical_bindings_shares_destructuring() {
    let out = ok("const r = match (x) { A(v) | B(v) => v, _ => 0 };");
    assert!(
        compact(&out)
            .contains("case \"A\": case \"B\": { const { v } = $tt_m; $tt_v0 = v; break; }"),
        "{out}"
    );
}

#[test]
fn or_pattern_binding_order_is_insensitive() {
    let out = ok("const r = match (p) { A(x, y) | B(y, x) => x + y, _ => 0 };");
    assert!(
        compact(&out).contains("case \"A\": case \"B\": { const { x, y } = $tt_m;"),
        "{out}"
    );
}

#[test]
fn or_pattern_counts_for_exhaustiveness() {
    ok(r#"
variant Dir { North(), South, East, West }
const f = (d: Dir) => match (d) {
  North | South => 1,
  East | West => 2,
};
"#);
    let e = err(r#"
variant Dir { North(), South, East, West }
const f = (d: Dir) => match (d) {
  North | South => 1,
  East => 2,
};
"#);
    assert!(e.message.contains("missing \"West\""), "{}", e.message);
}

#[test]
fn or_pattern_duplicate_tag_is_error() {
    // duplicate inside one arm
    let e = err("const r = match (x) { A | A => 1, _ => 0 };");
    assert!(e.message.contains("duplicate arm \"A\""), "{}", e.message);
    // duplicate across arms
    let e = err("const r = match (x) { A | B => 1, B => 2, _ => 0 };");
    assert!(e.message.contains("duplicate arm \"B\""), "{}", e.message);
}

#[test]
fn or_pattern_binding_mismatch_is_error() {
    let e = err("const r = match (x) { A(v) | B(w) => v, _ => 0 };");
    assert!(
        e.message
            .contains("or-pattern alternatives must bind the same names"),
        "{}",
        e.message
    );
    // ... and the message names the binding that differs.
    assert!(
        e.message
            .contains("`v` is bound in `A(...)` but not in `B(...)`"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (1, 30)); // points at the offending alternative

    // an alias changes the bound name, so it must match too
    let e = err("const r = match (x) { A(v) | B(v: w) => w, _ => 0 };");
    assert!(
        e.message
            .contains("`v` is bound in `A(...)` but not in `B(...)`"),
        "{}",
        e.message
    );

    // a binding-free alternative cannot pair with a binding one
    let e = err("const r = match (x) { A | B(v) => 1, _ => 0 };");
    assert!(
        e.message
            .contains("`v` is bound in `B(...)` but not in `A(...)`"),
        "{}",
        e.message
    );

    // an arity mismatch names the extra binding
    let e = err("const r = match (x) { A(v) | B(v, w) => v, _ => 0 };");
    assert!(
        e.message
            .contains("`w` is bound in `B(...)` but not in `A(...)`"),
        "{}",
        e.message
    );

    // a wildcard-looking `_` is a binding like any other
    let e = err("const r = match (x) { A(v) | B(_) => v, _ => 0 };");
    assert!(
        e.message
            .contains("`v` is bound in `A(...)` but not in `B(...)`"),
        "{}",
        e.message
    );

    // same names, different fields: the pairing is named
    let e = err("const r = match (x) { A(v) | B(w: v) => v, _ => 0 };");
    assert!(
        e.message
            .contains("`v` is bound from field `v` in `A(...)` but from field `w` in `B(...)`"),
        "{}",
        e.message
    );
}

#[test]
fn or_pattern_double_pipe_is_not_tt_syntax() {
    // `A || B` is not an or-pattern; the surrounding arrow arm commits the
    // construct to tt, so the parser reports it directly.
    let e = err("const r = match (x) { A || B => 1 };");
    // Reported where the text that failed is — the `match` that stayed
    // verbatim — not at a position in the generated module.
    assert!(
        e.message.contains("tt `match` could not be parsed"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (1, 11));
    assert_eq!((e.end_line, e.end_col), (1, 16));
}

/* ------------------------------------------------------------------ */
/* guards                                                              */
/* ------------------------------------------------------------------ */

#[test]
fn guarded_match_compiles_to_if_chain() {
    let out = ok(r#"
variant Score { Graded(points: number), Pending }
const grade = match (s) {
  Graded(points) if points >= 90 => "A",
  Graded(points) => "F",
  Pending => "-",
};
"#);
    assert!(!out.contains("switch ("), "{out}");
    assert!(
        compact(&out).contains(
            "if ($tt_m.kind === \"Graded\") { const { points } = $tt_m; if (points >= 90) { $tt_v0 = \"A\"; break; } }"
        ),
        "{out}"
    );
    assert!(
        compact(&out).contains(
            "if ($tt_m.kind === \"Graded\") { const { points } = $tt_m; $tt_v0 = \"F\"; break; }"
        ),
        "{out}"
    );
    // the same fail-fast runtime guard as the switch emission
    assert!(
        out.contains("throw new Error(\"tt match: unexpected case \" + JSON.stringify($tt_m));"),
        "{out}"
    );
}

#[test]
fn guard_free_match_still_emits_switch() {
    let out = ok("const r = match (x) { A => 1, _ => 0 };");
    assert!(out.contains("switch ($tt_m.kind)"), "{out}");
    assert!(!out.contains("$tt_b"), "{out}");
}

#[test]
fn repeated_guarded_tags_are_allowed() {
    let out =
        ok("const r = match (x) { A(v) if v > 9 => 2, A(v) if v > 0 => 1, A => 0, _ => -1 };");
    assert_eq!(out.matches("$tt_m.kind === \"A\"").count(), 3, "{out}");
}

#[test]
fn guard_after_unguarded_same_tag_is_duplicate() {
    // the unguarded A already covers the tag, so the guarded arm is unreachable
    let e = err("const r = match (x) { A => 1, A if c => 2, _ => 0 };");
    assert!(e.message.contains("duplicate arm \"A\""), "{}", e.message);
}

#[test]
fn guarded_arms_do_not_satisfy_exhaustiveness() {
    let e = err(
        "const f = (o: Option<number>) => match (o) { Some(value) if value > 0 => value, None => 0 };",
    );
    assert!(
        e.message
            .contains("match on built-in variant Option is not exhaustive: missing \"Some\""),
        "{}",
        e.message
    );
}

#[test]
fn fully_guarded_match_is_not_exhaustive() {
    // guarded tags still identify the variant — they just cover nothing
    let e =
        err("const f = (o: Option<number>) => match (o) { Some(value) if value > 0 => value };");
    assert!(e.message.contains("\"None\""), "{}", e.message);
    assert!(e.message.contains("\"Some\""), "{}", e.message);
}

#[test]
fn guard_with_or_pattern_emits_combined_condition() {
    let out = ok("const r = match (x) { A(v) | B(v) if v > 0 => v, _ => 0 };");
    assert!(
        compact(&out).contains(
            "if ($tt_m.kind === \"A\" || $tt_m.kind === \"B\") { const { v } = $tt_m; if (v > 0) { $tt_v0 = v; break; } }"
        ),
        "{out}"
    );
}

#[test]
fn guarded_block_body_uses_labeled_break() {
    let out = ok("const r = match (x) { A(v) if v > 0 => { log(v); }, _ => 0 };");
    assert!(out.contains("$tt_b: {"), "{out}");
    assert!(out.contains("break $tt_b;"), "{out}");
}

#[test]
fn await_in_guard_makes_match_async() {
    let out = ok(
        "async function f(x: T) { return match (x) { A(u) if await allowed(u) => 1, _ => 0 }; }",
    );
    assert!(!out.contains("async () =>"), "{out}");
    assert!(
        compact(&out).contains("if (await allowed(u)) { $tt_v0 = 1; break; }"),
        "{out}"
    );
    assert!(out.contains("return $tt_v0;"), "{out}");
}

#[test]
fn nested_await_match_keeps_its_expression_boundary() {
    let out = ok(
        "async function f(x: T) { return consume(match (x) { A(url) => await fetch(url), _ => null }); }",
    );
    assert!(!out.contains("async () =>"), "{out}");
    // The consumed call completes inside each arm, so the awaited value
    // keeps the consumer's contextual position (TASK-327).
    assert!(out.contains("$tt_v0 = $tt_v1(await fetch(url));"), "{out}");
    assert!(out.contains("return $tt_v0;"), "{out}");
}

#[test]
fn call_arguments_wider_than_the_match_keep_their_authored_frame() {
    // The completion proof requires the argument to be exactly the tt
    // value. A cast (or any containing expression) around the value keeps
    // the authored call and its frame, with the match joined by its slot.
    let out = ok("consume(match (x) { A(v) => v, _ => 0 } as number);");
    assert!(out.contains("$tt_v1($tt_v0 as number);"), "{out}");
    let object = ok("consume({item: match (x) { A(v) => v, _ => 0 }});");
    assert!(object.contains("$tt_v1({item: $tt_v0});"), "{object}");
}

#[test]
fn control_flow_arm_completions_use_a_labeled_region() {
    // A completed call inside a `break`-capturing arm statement leaves the
    // region through a generated label seeded by the callee slot, and every
    // authored return carries the call (TASK-328).
    let out =
        ok("consume(match (x) { A(v) => { for (const s of [1]) { if (s === v) return s; } return 0; }, _ => 0 });");
    assert!(out.contains("$tt_y_v1: {"), "{out}");
    assert!(out.contains("$tt_v1(s); break $tt_y_v1;"), "{out}");
    assert!(out.contains("$tt_v1(0); break $tt_y_v1;"), "{out}");
    // A cleanup-bearing arm keeps the consumer outside the arm entirely.
    let cleanup =
        ok("consume(match (x) { A(v) => { try { return v; } finally { effect(); } }, _ => 0 });");
    assert!(cleanup.contains("$tt_v1($tt_v0);"), "{cleanup}");
}

#[test]
fn final_argument_completions_call_through_captured_earlier_arguments() {
    // Only the final argument's match performs the call; earlier arguments
    // keep their authored evaluation order in capture slots the arm reads
    // (TASK-329).
    let out = ok("pair(first(), match (x) { A(v) => v, _ => 0 });");
    assert!(out.contains("const $tt_v2 = (first());"), "{out}");
    assert!(out.contains("$tt_v1($tt_v2, v);"), "{out}");
    assert!(out.contains("$tt_v1($tt_v2, 0);"), "{out}");
    // A match that is not the final argument keeps its join slot: moving the
    // call into it would run the later argument's subject too early.
    let leading = ok("pair(match (x) { A(v) => v, _ => 0 }, last());");
    assert!(leading.contains("$tt_v0 = v;"), "{leading}");
    assert!(leading.contains("$tt_v1($tt_v0, last());"), "{leading}");
}

#[test]
fn wildcard_with_guard_is_not_tt_syntax() {
    // `_ if ...` does not parse as a tt match; the arrow arm has already
    // committed the construct to tt.
    let e = err("const r = match (x) { A => 1, _ if c => 0 };");
    assert!(
        e.message.contains("tt `match` could not be parsed"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (1, 11));
}

#[test]
fn try_inside_a_function_inside_a_guard_is_allowed() {
    // Rust's `?` inside a closure: the emitted `return` exits the arrow
    // the user wrote, not the match's IIFE — placement is a flow fact.
    let out = ok(
        "const r = match (x) {\n  A(v) if run(() => { try g(); return true; }) => v,\n  _ => 0,\n};\n",
    );
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{out}"
    );
}

/* ------------------------------------------------------------------ */
/* exhaustiveness — a ttc error, not a tsc error                      */
/* ------------------------------------------------------------------ */

#[test]
fn non_exhaustive_match_is_an_ttc_error_with_position() {
    let e = err(
        r#"variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  Point => 0,
};
"#,
    );
    assert!(
        e.message
            .contains("match on variant Shape is not exhaustive: missing \"Rect\""),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (2, 25)); // points at the `match` keyword
}

#[test]
fn exhaustive_match_compiles() {
    let out = ok(r#"
variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  Rect(w, h) => w * h,
  Point => 0,
};
"#);
    assert!(out.contains("case \"Rect\""));
}

#[test]
fn wildcard_satisfies_exhaustiveness() {
    ok(r#"
variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  _ => 0,
};
"#);
}

#[test]
fn exhaustiveness_is_declaration_order_independent() {
    // match appears before the variant declaration — still checked.
    let e = err(r#"const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
};
variant Shape { Circle(radius: number), Point }
"#);
    assert!(e.message.contains("missing \"Point\""), "{}", e.message);
}

#[test]
fn match_on_unknown_tags_is_not_checked() {
    // Hand-written unions / imported variants: ttc has no type info, so no
    // exhaustiveness check — the runtime guard still protects.
    let out = ok(r#"
type AppEvent = { kind: "click"; x: number } | { kind: "key"; code: string };
const f = (e: AppEvent) => match (e) {
  click(x) => x,
};
"#);
    assert!(out.contains("case \"click\""));
}

#[test]
fn match_on_builtin_option_is_exhaustiveness_checked() {
    // Option/Result are built-in variants: checked without a local declaration.
    let e = err("const f = (o: Option<number>) => match (o) { Some(value) => value };\n");
    assert!(
        e.message
            .contains("match on built-in variant Option is not exhaustive: missing \"None\""),
        "{}",
        e.message
    );
}

#[test]
fn match_on_builtin_result_is_exhaustiveness_checked() {
    let e = err("const f = (r: Result<number, string>) => match (r) { Err(error) => error };\n");
    assert!(
        e.message
            .contains("match on built-in variant Result is not exhaustive: missing \"Ok\""),
        "{}",
        e.message
    );
}

#[test]
fn full_match_on_builtin_variants_compiles() {
    let out = ok(r#"
const f = (o: Option<number>) => match (o) { Some(value) => value, None => 0 };
const g = (r: Result<number, string>) => match (r) { Ok(value) => value, Err(error) => error.length };
"#);
    assert!(out.contains("case \"Some\""));
    assert!(out.contains("case \"Err\""));
}

#[test]
fn wildcard_exempts_builtin_exhaustiveness() {
    ok("const f = (o: Option<number>) => match (o) { Some(value) => value, _ => 0 };\n");
}

#[test]
fn local_variant_shadows_builtin() {
    // A file-local tt variant named Option replaces the built-in for this file.
    let e = err(
        "variant Option { Some(), Stale }\nconst f = (o: Option) => match (o) { Some => 1 };\n",
    );
    assert!(
        e.message
            .contains("match on variant Option is not exhaustive: missing \"Stale\""),
        "{}",
        e.message
    );
    assert!(!e.message.contains("built-in"), "{}", e.message);
}

#[test]
fn a_candidate_the_arms_satisfy_makes_the_match_exhaustive() {
    // Two variants contain every arm tag. The arms cover `Small` completely,
    // so nothing is missing — the check names an variant only when *no*
    // candidate is satisfied, and then the one left fewest cases.
    ok(
        "variant Big { A(s: string), B, C }\nvariant Small { A(s: string), B }\nconst f = (v: Small) => match (v) { A(s) => s, B => \"b\" };\n",
    );

    let e = err(
        "variant Big { A(s: string), B, C, D }\nvariant Small { A(s: string), B, C }\nconst f = (v: Small) => match (v) { A(s) => s, B => \"b\" };\n",
    );
    assert!(
        e.message
            .contains("match on variant Small is not exhaustive: missing \"C\""),
        "{}",
        e.message
    );
}

#[test]
fn missing_cases_are_all_listed() {
    let e = err(r#"variant Dir { North, South, East, West(deg: number) }
const f = (d: Dir) => match (d) { North => 1 };
"#);
    assert!(e.message.contains("\"East\""), "{}", e.message);
    assert!(e.message.contains("\"South\""), "{}", e.message);
    assert!(e.message.contains("\"West\""), "{}", e.message);
}

/* ------------------------------------------------------------------ */
/* try — Rust-style error propagation                                  */
/* ------------------------------------------------------------------ */

#[test]
fn try_decl_emits_early_return_and_bind() {
    let out = ok("function f(): X {\n  const n = try g();\n  return h(n);\n}\n");
    assert!(
        compact(&out).contains(
            "const $tt_t0 = g(); if (!(\"value\" in $tt_t0)) { return $tt_t0; } const n = $tt_t0.value;"
        ),
        "{out}"
    );
}

#[test]
fn try_bare_statement_emits_early_return_only() {
    let out = ok("function f(): X {\n  try g();\n  return h();\n}\n");
    assert!(
        compact(&out)
            .contains("const $tt_t0 = g(); if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{out}"
    );
    assert!(!out.contains("$tt_t0.value"), "{out}");
}

#[test]
fn try_temporaries_are_unique_and_keep_declaration_keyword() {
    let out = ok(
        "function f(): X {\n  let a: number = try g();\n  var b = try h(a);\n  return k(b);\n}\n",
    );
    assert!(out.contains("let a: number = $tt_t0.value;"), "{out}");
    assert!(out.contains("var b = $tt_t1.value;"), "{out}");
}

#[test]
fn try_destructuring_binding_is_kept_verbatim() {
    let out = ok("function f(): X {\n  const { a, b } = try g();\n  return a + b;\n}\n");
    assert!(out.contains("const { a, b } = $tt_t0.value;"), "{out}");
}

#[test]
fn try_expression_may_contain_a_match() {
    let out = ok(
        "function f(): X {\n  const x = try match (m) { Ok(value) => wrap(value), Err(error) => rewrap(error) };\n  return x;\n}\n",
    );
    assert!(out.contains("const $tt_t0 = $tt_v0;"), "{out}");
    assert!(out.contains("switch ($tt_m.kind)"), "{out}");

    let nested = ok(
        "function f(): X {\n  const x = try wrap(match (m) { Ok(value) => value, Err(_) => 0 });\n  return x;\n}\n",
    );
    assert!(nested.contains("const $tt_v1 = (wrap);"), "{nested}");
    // Each arm performs the consuming call itself (TASK-327); the try
    // propagation then reads the completed result.
    assert!(nested.contains("$tt_v0 = $tt_v1(value);"), "{nested}");
    assert!(nested.contains("const $tt_t0 = $tt_v0;"), "{nested}");
    assert!(!nested.contains("$tt_expr"), "{nested}");

    let discarded = ok(
        "function f(): X {\n  try (match (m) { Ok(value) => wrap(value), Err(error) => rewrap(error) });\n}\n",
    );
    assert!(discarded.contains("switch ($tt_m.kind)"), "{discarded}");
}
