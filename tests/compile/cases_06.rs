#[test]
fn flow_emits_nested_composition_helper_calls() {
    let out = ok("const f = flow |> parse |> double |> label;\n");
    assert!(
        out.contains("const f = $tt_fl($tt_fl(parse, double), label);"),
        "{out}"
    );
    assert!(out.contains("import { $tt_fl } from \"@tt/runtime\";"));
    // a composition is not a value pipeline — no apply helper
    assert!(!out.contains("$tt_ap"), "{out}");
}

#[test]
fn flow_method_step_becomes_a_contextually_typed_arrow() {
    let out = ok("const f = flow |> parse |> .toFixed(1);\n");
    assert!(
        out.contains("const f = $tt_fl(parse, (($tt_v) => ($tt_v).toFixed(1)));"),
        "{out}"
    );
}

#[test]
fn flow_optional_postfix_step_becomes_a_contextually_typed_arrow() {
    let out = ok("const f = flow |> parse |> ?.value?.toFixed(1);\n");
    assert!(
        out.contains("const f = $tt_fl(parse, (($tt_v) => ($tt_v)?.value?.toFixed(1)));"),
        "{out}"
    );
}

#[test]
fn flow_with_a_single_step_is_that_step_and_needs_no_helper() {
    let out = ok("const f = flow |> parse;\n");
    assert!(out.contains("const f = parse;"), "{out}");
    assert!(!out.contains("$tt_fl"), "{out}");
}

#[test]
fn flow_runtime_is_imported_once_per_file() {
    let out = ok("const a = flow |> f |> g;\nconst b = flow |> h |> i;\n");
    assert_eq!(out.matches("$tt_fl(").count(), 2, "{out}");
    assert_eq!(out.matches("from \"@tt/runtime\"").count(), 1, "{out}");
}

#[test]
fn file_without_flow_gets_no_composition_helper() {
    let out = ok("const a = x |> f;\n");
    assert!(!out.contains("$tt_fl"), "{out}");
}

#[test]
fn flow_is_a_contextual_keyword_only_at_a_pipeline_head() {
    // a `flow` variable still pipes when parenthesized, and a dotted or
    // called head is an ordinary value head
    let out = ok("const a = (flow) |> f;\nconst b = o.flow |> f;\nconst c = flow() |> f;\n");
    assert!(out.contains("const a = $tt_ap((flow), f);"), "{out}");
    assert!(out.contains("const b = $tt_ap(o.flow, f);"), "{out}");
    assert!(out.contains("const c = $tt_ap(flow(), f);"), "{out}");
    assert!(!out.contains("$tt_fl"), "{out}");
}

#[test]
fn flow_composes_inside_expressions() {
    let out = ok("const a = xs.map(flow |> parse |> double);\nconst b = `${flow |> f |> g}`;\n");
    assert!(
        out.contains("const a = xs.map($tt_fl(parse, double));"),
        "{out}"
    );
    assert!(out.contains("const b = `${$tt_fl(f, g)}`;"), "{out}");
}

#[test]
fn flow_step_can_be_a_parenthesized_arrow() {
    let out = ok("const f = flow |> parse |> (n => n + 1);\n");
    assert!(
        out.contains("const f = $tt_fl(parse, (n => n + 1));"),
        "{out}"
    );
}

#[test]
fn flow_first_step_cannot_be_a_method_step() {
    let e = err("const f = flow |> .trim() |> lower;\n");
    assert!(
        e.message.contains("the first step cannot be a method step"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (1, 19));
}

#[test]
fn flow_without_a_step_is_an_error() {
    let e = err("const f = flow |>;\n");
    assert!(e.message.contains("could not be parsed"), "{}", e.message);
}

/* ------------------------------------------------------------------ */
/* tuple match                                                         */
/* ------------------------------------------------------------------ */

#[test]
fn tuple_match_emits_joint_if_chain() {
    let out = ok(r#"
variant Dir { North(), South }
variant Speed { Fast(), Slow }
const step = match (dir, speed) {
  (North, Fast) => 2,
  (North, Slow) => 1,
  (South, _) => -1,
};
"#);
    assert!(out.contains("const $tt_m0 = dir;"), "{out}");
    assert!(out.contains("const $tt_m1 = speed;"), "{out}");
    assert!(
        compact(&out).contains(
            "if ($tt_m0.kind === \"North\" && $tt_m1.kind === \"Fast\") { $tt_v0 = 2; break; }"
        ),
        "{out}"
    );
    assert!(
        compact(&out).contains("if ($tt_m0.kind === \"South\") { $tt_v0 = -1; break; }"),
        "{out}"
    );
    assert!(out.contains("JSON.stringify([$tt_m0, $tt_m1])"), "{out}");
}

#[test]
fn tuple_match_binds_fields_from_each_position() {
    let out = ok(r#"
const r = match (a, b) {
  (Some(value: x), Some(value: y)) => x + y,
  _ => 0,
};
"#);
    assert!(
        compact(&out).contains(
            "{ const { value: x } = $tt_m0; const { value: y } = $tt_m1; $tt_v0 = x + y; break; }"
        ),
        "{out}"
    );
}

#[test]
fn comma_expression_scrutinee_is_still_a_single_match() {
    // No tuple pattern in the arms → the comma is a comma expression,
    // exactly as before tuple matches existed.
    let out = ok(
        "const r = match ((a, b)) { A => 1, _ => 0 };\nconst s = match (a, b) { A => 1, _ => 0 };\n",
    );
    // The scrutinee is a comma expression, so the parentheses codegen
    // writes around it are the ones that keep it one value.
    assert_eq!(out.matches("const $tt_m = (a, b);").count(), 2, "{out}");
    assert!(!out.contains("$tt_m0"), "{out}");
}

#[test]
fn tuple_match_product_exhaustiveness_reports_missing_combination() {
    let e = err(r#"
variant Dir { North(), South }
variant Speed { Fast(), Slow }
const step = match (d, s) {
  (North, Fast) => 2,
  (North, Slow) => 1,
  (South, Fast) => -1,
};
"#);
    assert!(
        e.message
            .contains("match on (Dir, Speed) is not exhaustive: missing (South, Slow)"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (4, 14));
}

#[test]
fn tuple_match_wildcard_element_and_or_pattern_cover_the_product() {
    let out = ok(r#"
variant Dir { North(), South, East, West }
variant Speed { Fast(), Slow }
const step = match (d, s) {
  (North | South, _) => 1,
  (East, Fast | Slow) => 2,
  (West, _) => 3,
};
"#);
    assert!(out.contains("$tt_m0"), "{out}");
    assert!(
        compact(&out).contains(
            "if (($tt_m0.kind === \"North\" || $tt_m0.kind === \"South\")) { $tt_v0 = 1; break; }"
        ),
        "{out}"
    );
}

#[test]
fn tuple_match_guarded_arm_covers_nothing() {
    let e = err(r#"
variant Coin { Heads(), Tails }
const r = match (a, b) {
  (Heads, Heads) if lucky() => 1,
  (Heads, Heads) => 2,
  (Heads, Tails) => 3,
  (Tails, Heads) => 4,
};
"#);
    assert!(
        e.message.contains("missing (Tails, Tails)"),
        "{}",
        e.message
    );
}

#[test]
fn tuple_match_bare_wildcard_arm_skips_the_check_and_must_be_last() {
    let out = ok(r#"
variant Coin { Heads(), Tails }
const r = match (a, b) {
  (Heads, Heads) => 1,
  _ => 0,
};
"#);
    assert!(compact(&out).contains("$tt_v0 = 0; break;"), "{out}");

    let e = err("const r = match (a, b) {\n  _ => 0,\n  (A, B) => 1,\n};\n");
    assert!(e.message.contains("must be the last arm"), "{}", e.message);
}

#[test]
fn tuple_match_arity_mismatch_is_an_error() {
    let src = "const r = match (a, b) {\n  (A, B, C) => 1,\n  _ => 0,\n};\n";
    let e = err(src);
    assert!(
        e.message
            .contains("tuple pattern has 3 elements but the match has 2 scrutinees"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (2, 3));
    let diagnostic = ttc::analyze(src, &Options::default())
        .into_iter()
        .find(|d| d.code == ttc::DiagnosticCode::MatchTupleArity)
        .unwrap();
    assert_eq!(
        &src[diagnostic.start.unwrap()..diagnostic.end.unwrap()],
        "(A, B, C)"
    );
}

#[test]
fn one_element_tuple_pattern_reports_the_exact_arity() {
    let e = err("const r = match (a, b) {\n  (A) => 1,\n  _ => 0,\n};\n");
    assert!(
        e.message
            .contains("tuple pattern has 1 element but the match has 2 scrutinees"),
        "{e}"
    );
    assert_eq!((e.line, e.col), (2, 3));

    let e = err("const r = match (a) {\n  (A, B) => 1,\n  _ => 0,\n};\n");
    assert!(
        e.message
            .contains("tuple pattern has 2 elements but the match has 1 scrutinee"),
        "{e}"
    );
}

#[test]
fn match_without_scrutinee_parentheses_is_a_malformed_tt_match() {
    let src = "const r = match value { A => 1, _ => 0 };\n";
    let e = err(src);
    assert!(e.message.contains("could not be parsed"), "{e}");
    assert_eq!((e.line, e.col), (1, 11));
    // The one malformed-match shape whose fix the parser can write: the
    // scrutinee text is there, it only lacks its parentheses.
    let d = &ttc::analyze(src, &Options::default())[0];
    assert_eq!(d.code, ttc::DiagnosticCode::MalformedMatch);
    let edit = d.suggestions[0].edit.as_ref().expect("an applicable edit");
    assert_eq!(&src[edit.start..edit.end], "value");
    assert_eq!(edit.replacement, "(value)");
    assert_eq!(
        with_suggestion_applied(src, d, 0),
        "const r = match (value) { A => 1, _ => 0 };\n"
    );
}

#[test]
fn tuple_match_duplicate_binding_across_elements_is_an_error() {
    let e =
        err("const r = match (a, b) {\n  (Some(value), Some(value)) => value,\n  _ => 0,\n};\n");
    assert!(
        e.message.contains("binding `value` is used more than once"),
        "{}",
        e.message
    );
}

#[test]
fn tuple_match_or_alternatives_must_bind_the_same_fields_per_element() {
    let e = err("const r = match (a, b) {\n  (Some(value) | None, _) => 1,\n  _ => 0,\n};\n");
    assert!(
        e.message
            .contains("or-pattern alternatives must bind the same names"),
        "{}",
        e.message
    );
    assert!(
        e.message
            .contains("`value` is bound in `Some(...)` but not in `None(...)`"),
        "{}",
        e.message
    );
}

#[test]
fn tuple_match_over_builtin_variants() {
    let e = err(
        "const r = match (o, r2) {\n  (Some(value), Ok(value: v)) => value + v,\n  (None, _) => 0,\n};\n",
    );
    assert!(
        e.message
            .contains("match on (Option, Result) is not exhaustive: missing (Some, Err)"),
        "{}",
        e.message
    );
}

#[test]
fn tuple_match_block_bodies_and_guards_leave_through_the_region() {
    let out = ok(r#"
variant Coin { Heads(), Tails }
const r = match (a, b) {
  (Heads, Tails) if go() => { log(); return 1; },
  _ => 0,
};
"#);
    assert!(out.contains("if (go()) {"), "{out}");
    // The arm's body always leaves, through the region's own
    // `do { … } while (false)` — so neither the chain's fall-through label
    // nor a second exit label around the region is written.
    assert!(out.contains("$tt_v0 = 1; break;"), "{out}");
    assert!(!out.contains("$tt_b"), "{out}");
    assert!(!out.contains("$tt_y_"), "{out}");
}

#[test]
fn tuple_match_await_in_scrutinee_makes_it_async() {
    let out = ok(
        "async function f() {\n  return match (await a, b) {\n    (X, Y) => 1,\n    _ => 0,\n  };\n}\n",
    );
    assert!(!out.contains("async () =>"), "{out}");
    assert!(out.contains("const $tt_m0 = await a;"), "{out}");
}

#[test]
fn tuple_match_three_positions() {
    let e = err(r#"
variant B { T(), F }
const r = match (x, y, z) {
  (T, _, _) => 1,
  (F, T, _) => 2,
  (F, F, T) => 3,
};
"#);
    assert!(e.message.contains("missing (F, F, F)"), "{}", e.message);
}

/* ------------------------------------------------------------------ */
/* nested patterns                                                     */
/* ------------------------------------------------------------------ */

#[test]
fn nested_pattern_emits_path_conditions_and_binds() {
    let out = ok(r#"
const n = match (r) {
  Ok(value: Some(value: v)) => v,
  Ok(value: None()) => 0,
  _ => -1,
};
"#);
    assert!(
        compact(&out).contains("if ($tt_m.kind === \"Ok\" && $tt_m.value.kind === \"Some\") { const { value: v } = $tt_m.value; $tt_v0 = v; break; }"),
        "{out}"
    );
    assert!(
        compact(&out).contains(
            "if ($tt_m.kind === \"Ok\" && $tt_m.value.kind === \"None\") { $tt_v0 = 0; break; }"
        ),
        "{out}"
    );
    // nested patterns force the if-chain form
    assert!(!out.contains("switch ($tt_m.kind)"), "{out}");
}

#[test]
fn nested_pattern_two_levels_deep() {
    let out = ok(r#"
const n = match (r) {
  Ok(value: Some(value: Pair(a, b))) => a + b,
  _ => 0,
};
"#);
    assert!(
        out.contains("$tt_m.kind === \"Ok\" && $tt_m.value.kind === \"Some\" && $tt_m.value.value.kind === \"Pair\""),
        "{out}"
    );
    assert!(out.contains("const { a, b } = $tt_m.value.value;"), "{out}");
}

#[test]
fn nested_pattern_mixes_plain_bindings_at_each_level() {
    let out = ok(r#"
const n = match (r) {
  Both(left, right: Some(value)) => left + value,
  _ => 0,
};
"#);
    assert!(
        compact(&out).contains(
            "{ const { left } = $tt_m; const { value } = $tt_m.right; $tt_v0 = left + value; break; }"
        ),
        "{out}"
    );
}

#[test]
fn plain_alias_is_still_an_alias_not_a_nested_pattern() {
    // `value: v` (no parens) binds; only `value: Tag(...)` nests. A match
    // without nested patterns keeps the switch form.
    let out = ok("const n = match (o) { Some(value: None) => None, _ => 0 };");
    assert!(out.contains("const { value: None } = $tt_m;"), "{out}");
    assert!(out.contains("switch ($tt_m.kind)"), "{out}");
}

#[test]
fn a_nested_pattern_covers_exactly_what_it_matches() {
    // The exhaustiveness recursion descends into the payload, so the
    // witness names the *value* that is missing, not just its outer case.
    let e = err(r#"
const n = match (r) {
  Ok(value: Some(value: v)) => v,
  Err(error) => 0,
};
"#);
    assert!(
        e.message.contains(
            "match on built-in variant Result is not exhaustive: missing \"Ok(value: None())\""
        ),
        "{}",
        e.message
    );
}

#[test]
fn nested_pattern_arm_may_repeat_a_tag() {
    // Two Ok arms with different inner patterns are not duplicates —
    // exactly like two guarded arms of one tag.
    let out = ok(r#"
const n = match (r) {
  Ok(value: Some(value: v)) => v,
  Ok(value) => 0,
  Err(error) => -1,
};
"#);
    assert!(out.contains("$tt_m.value.kind === \"Some\""), "{out}");
}

#[test]
fn plain_arm_before_nested_arm_is_a_duplicate() {
    let e = err("const n = match (r) { Ok(value) => 1, Ok(value: Some(value: v)) => v, _ => 0 };");
    assert!(e.message.contains("duplicate arm \"Ok\""), "{}", e.message);
}

#[test]
fn nested_pattern_inside_or_pattern_is_an_error() {
    let e = err("const n = match (r) { Ok(value: Some(v)) | Err(error) => 1, _ => 0 };");
    assert!(
        e.message
            .contains("nested patterns cannot be combined with or-patterns"),
        "{}",
        e.message
    );
}

#[test]
fn duplicate_binding_within_one_pattern_is_an_error() {
    let e = err("const n = match (r) { Ok(value: Some(value), error: value) => value, _ => 0 };");
    assert!(
        e.message.contains("binding `value` is used more than once"),
        "{}",
        e.message
    );
}

#[test]
fn nested_pattern_in_tuple_match_elements() {
    let out = ok(r#"
const n = match (a, b) {
  (Ok(value: Some(value: x)), Ok(value: Some(value: y))) => x + y,
  _ => 0,
};
"#);
    assert!(
        out.contains("$tt_m0.kind === \"Ok\" && $tt_m0.value.kind === \"Some\" && $tt_m1.kind === \"Ok\" && $tt_m1.value.kind === \"Some\""),
        "{out}"
    );
    assert!(
        compact(&out)
            .contains("const { value: x } = $tt_m0.value; const { value: y } = $tt_m1.value;"),
        "{out}"
    );
}

#[test]
fn nested_pattern_with_guard() {
    let out = ok(r#"
const n = match (r) {
  Ok(value: Some(value: v)) if v > 0 => v,
  _ => 0,
};
"#);
    assert!(
        compact(&out).contains("if (v > 0) { $tt_v0 = v; break; }"),
        "{out}"
    );
}

#[test]
fn let_else_does_not_take_nested_patterns() {
    // `const Some(value: Ok(v)) = ...` is not tt let-else syntax; the
    // candidate passes through and (being invalid TS) fails the output
    // self-check — same as any malformed candidate.
    let e = err("function f() {\n  const Some(value: Ok(v)) = g() else { return; };\n}\n");
    assert!(
        e.message.contains("generated TypeScript failed to parse"),
        "{}",
        e.message
    );
}

/* ------------------------------------------------------------------ */
/* if let                                                              */
/* ------------------------------------------------------------------ */

#[test]
fn if_let_emits_a_self_contained_block() {
    let out =
        ok("function f() {\n  if let Some(value: user) = find() {\n    greet(user);\n  }\n}\n");
    assert!(
        compact(&out).contains("{ const $tt_t0 = find(); if ($tt_t0.kind === \"Some\") { const { value: user } = $tt_t0; greet(user); } }"),
        "{out}"
    );
}

#[test]
fn if_let_else_block_and_chaining() {
    let out = ok(r#"
function f() {
  if let Some(value) = a() {
    use1(value);
  } else if let Ok(value: v) = b() {
    use2(v);
  } else {
    fallback();
  }
}
"#);
    assert!(
        compact(&out).contains("} else { const $tt_t1 = b(); if ($tt_t1.kind === \"Ok\""),
        "{out}"
    );
    assert!(compact(&out).contains("else { fallback(); } } }"), "{out}");
}

#[test]
fn if_let_takes_nested_patterns() {
    let out = ok("function f(r: Res) {\n  if let Ok(value: Some(value: v)) = r { use(v); }\n}\n");
    assert!(
        compact(&out).contains("if ($tt_t0.kind === \"Ok\" && $tt_t0.value.kind === \"Some\") { const { value: v } = $tt_t0.value; use(v); }"),
        "{out}"
    );
}

#[test]
fn if_let_shares_the_temp_counter_with_try() {
    let out = ok(
        "function f(): Result<number, string> {\n  const a = try g();\n  if let Some(value) = h(a) { use(value); }\n  return Result.Ok(a);\n}\n",
    );
    assert!(out.contains("$tt_t0"), "{out}");
    assert!(out.contains("const $tt_t1 = h(a);"), "{out}");
}

#[test]
fn if_let_allowed_in_statement_contexts() {
    // A match arm's block body and a let-else else block are statement
    // positions — if let works there.
    let out = ok(r#"
function f(x: X, o: O) {
  const r = match (x) {
    A => {
      if let Some(value) = o { return value; }
      return 0;
    },
    _ => 1,
  };
  return r;
}
"#);
    assert!(out.contains("if ($tt_t0.kind === \"Some\")"), "{out}");
}

#[test]
fn if_let_in_expression_position_is_an_error() {
    let e = err("const s = `${if let Some(value) = o { 1 }}`;\n");
    assert!(
        e.message
            .contains("`if let` cannot be used in expression position"),
        "{}",
        e.message
    );
}

#[test]
fn if_let_inside_a_function_inside_an_expression_region_is_allowed() {
    // The same flow fact that places `try` (TASK-131), from the other
    // side: an arrow written in a scrutinee or interpolation provides the
    // statement position the emitted block needs.
    let out = ok(
        "const v = match (run(() => { if let A(x) = e { return x; } return 0; })) {\n  Ok(value) => value,\n  _ => 0,\n};\n",
    );
    assert!(out.contains("if ($tt_t0.kind === \"A\")"), "{out}");

    let out = ok("const s = `${run(() => { if let A(x) = e { log(x); } return 1; })}`;\n");
    assert!(out.contains("if ($tt_t0.kind === \"A\")"), "{out}");
}

#[test]
fn malformed_if_let_is_an_error_with_position() {
    // `if let` cannot be passed through (never valid TS), so a candidate
    // that fails to parse is reported instead of failing the self-check.
    let e = err("function f() {\n  if let Some = o { g(); }\n}\n");
    assert!(
        e.message.contains("`if let` could not be parsed here"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (2, 3));

    let e = err("function f() {\n  if let Some(v) = o { g(); } else if (x) { h(); }\n}\n");
    assert!(
        e.message.contains("`if let` could not be parsed here"),
        "{}",
        e.message
    );
}
