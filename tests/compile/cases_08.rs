#[test]
fn val_binding_replacement_is_not_a_val_error() {
    // Whether the binding itself can be replaced is `const`/`let`'s
    // question, not `val`'s — tsc reports the `const` case.
    let src = "val let a = 1;\na = 2;\nval const b = 1;\nb = 2;\n";
    assert_eq!(ok(src), src.replace("val ", ""));
}

#[test]
fn val_parameter_is_read_only_inside_the_function() {
    let e = err("function read(val user: User) {\n  user.name = \"Lee\";\n}\n");
    assert_eq!((e.line, e.col), (2, 3));
    let e = err("const read = (val user: User) => user.name = \"Lee\";\n");
    assert!(e.message.contains("val binding `user`"));
    // a parameter without the modifier keeps TypeScript's semantics
    let src = "function update(user: User) {\n  user.name = \"Lee\";\n}\n";
    assert_eq!(ok(src), src);
}

#[test]
fn val_parameter_positions_beyond_plain_identifiers() {
    let e = err("function foo(val { user }: Ctx) {\n  user.name = \"x\";\n}\n");
    assert!(e.message.contains("val binding `user`"));
    let e = err("for (val const item of items) {\n  item.a = 1;\n}\n");
    assert!(e.message.contains("val binding `item`"));
    let e = err("try {\n  f();\n} catch (val error: any) {\n  error.code = 1;\n}\n");
    assert!(e.message.contains("val binding `error`"));
    let e = err("class B {\n  constructor(private val inner: I) {\n    inner.a = 1;\n  }\n}\n");
    assert!(e.message.contains("val binding `inner`"));
}

#[test]
fn val_argument_may_only_reach_a_val_parameter() {
    let src = "\
function read(val user: User) { log(user.name); }
function update(user: User) { user.name = \"Lee\"; }
function process(val user: User) {
  read(user);
}
";
    assert_eq!(ok(src), src.replace("val ", ""));

    let e = err("\
function read(val user: User) { log(user.name); }
function update(user: User) { user.name = \"Lee\"; }
function process(val user: User) {
  update(user);
}
");
    assert_eq!((e.line, e.col), (4, 10));
    assert!(
        e.message
            .contains("cannot pass val binding `user` to mutable parameter `user` of `update`"),
        "{}",
        e.message,
    );
}

#[test]
fn a_mutable_argument_may_reach_any_parameter() {
    let src = "\
function read(val user: User) { log(user.name); }
function update(user: User) { user.name = \"Lee\"; }
function process(user: User) {
  read(user);
  update(user);
}
";
    assert_eq!(ok(src), src.replace("val ", ""));
}

#[test]
fn val_capability_flows_through_arrow_declarations() {
    let e = err("\
const update = (user: User) => { user.name = \"x\"; };
function process(val user: User) {
  update(user);
}
");
    assert!(e.message.contains("mutable parameter `user` of `update`"));
}

#[test]
fn val_is_an_access_path_restriction_not_object_immutability() {
    // An alias keeps its own capability: the `val` binding cannot mutate,
    // the original binding still can.
    let src = "let original = { count: 0 };\nval const view = original;\noriginal.count++;\n";
    assert_eq!(ok(src), src.replacen("val ", "", 1));
    let e = err("let original = { count: 0 };\nval const view = original;\nview.count++;\n");
    assert_eq!((e.line, e.col), (3, 1));
    assert!(e.message.contains("val binding `view`"));
}

#[test]
fn an_inner_declaration_shadows_an_outer_val() {
    let src = "val const x = { a: 1 };\n{\n  const x = { a: 2 };\n  x.a = 3;\n}\n";
    assert_eq!(ok(src), src.replacen("val ", "", 1));
    let src = "val const cfg = { a: 1 };\nfunction f(cfg: C) {\n  cfg.a = 2;\n}\n";
    assert_eq!(ok(src), src.replacen("val ", "", 1));
    // ... and the outer binding is still `val` after the inner scope ends
    let e = err("val const x = { a: 1 };\n{\n  const x = { a: 2 };\n  x.a = 3;\n}\nx.a = 4;\n");
    assert_eq!((e.line, e.col), (6, 1));
}

#[test]
fn val_never_calls_a_method_a_mutation_from_its_name() {
    // Whether `q.set(k)` mutates depends on what `q` is, and `compile` has
    // no types: a user-defined `set`/`add`/`push` must not be rejected on
    // its name alone (TASK-071). The typed path decides — see
    // `val_probes_collect_every_method_call_for_the_verdict` below and the
    // `--types` tests in tests/cli.rs.
    for src in [
        "class Query {\n  set(key: string): Query {\n    return new Query();\n  }\n}\nval const query = new Query();\nquery.set(\"name\");\n",
        "class Collection {\n  add(v: number): Collection {\n    return new Collection();\n  }\n}\nval const collection = new Collection();\ncollection.add(1);\n",
        // ... and neither are the built-in shapes, without types to prove it
        "val const items: number[] = [];\nitems.push(1);\n",
        "val const m = new Map<string, number>();\nm.set(\"a\", 1);\n",
        "val const s = { u: { p: { tags: [] as string[] } } };\ns.u.p.tags.push(\"tt\");\n",
        // reading methods were never in question
        "val const items: number[] = [];\nconst n = items.map((v) => v).filter(Boolean).length;\n",
    ] {
        assert_eq!(ok(src), src.replacen("val ", "", 1), "{src}");
    }
}

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

#[test]
fn a_type_argument_list_does_not_declare_a_val_binding() {
    // `<...>` is not a bracket the scanner matches, so the comma in
    // `Map<string, number>` used to look like a declarator separator and
    // made `number` a val binding — after which the `number[]` of a later
    // annotation read as a mutation (TASK-071).
    let src = "val const m = new Map<string, number>();\nval const items: number[] = [];\n";
    assert_eq!(ok(src), src.replace("val ", ""));
    // multi-declarator forms still bind every name
    let e = err("val let a, b, c;\nb.x = 1;\n");
    assert!(e.message.contains("val binding `b`"), "{}", e.message);
    let e = err("val const p = 1, q = { n: 0 };\nq.n = 2;\n");
    assert!(e.message.contains("val binding `q`"), "{}", e.message);
}

#[test]
fn val_is_checked_inside_nested_tt_constructs() {
    let e =
        err("val const cfg = { a: 1 };\nconst msg = `${(() => { cfg.a = 2; return 1; })()}`;\n");
    assert!(e.message.contains("val binding `cfg`"));
    let e = err("\
variant Shape { Circle(r: number), Point }
val const s = Shape.Circle(1);
const v = match (s) {
  Circle(r) => { s.kind = \"Point\"; return r; },
  Point => 0,
};
");
    assert!(e.message.contains("val binding `s`"));
}

#[test]
fn val_on_a_let_else_pattern_covers_the_names_it_binds() {
    let e = err("\
variant Opt { Some(value: Box), None }
function f(o: Opt) {
  val const Some(value) = o else { return; };
  value.n = 1;
}
");
    assert!(e.message.contains("val binding `value`"), "{}", e.message);
}

#[test]
fn val_covers_every_or_pattern_alternatives_bindings() {
    let e = err("\
variant E { A(x: Box), B(x: Box) }
function f(e: E) {
  val const A(x) | B(x) = e else { return; };
  x.n = 1;
}
");
    assert!(e.message.contains("val binding `x`"), "{}", e.message);
}

#[test]
fn val_capability_check_only_covers_resolvable_callees() {
    // An imported (or otherwise unknown) function has no signature ttc can
    // read, so passing a `val` binding to it is allowed — the documented
    // limit of the check (language.md §10.7).
    let src = "import { save } from \"./io.js\";\nfunction f(val user: User) {\n  save(user);\n  user.save();\n}\n";
    assert_eq!(ok(src), src.replacen("val ", "", 1));
    // A name declared twice with different signatures is ambiguous and
    // drops out of the table rather than guessing.
    let src = "\
function apply(user: User) {}
function apply(val user: User) {}
function f(val user: User) {
  apply(user);
}
";
    assert_eq!(ok(src), src.replace("val ", ""));
    // A computed argument is not an access path, so it is not checked.
    let src = "\
function update(user: User) { user.name = \"x\"; }
function f(val user: User) {
  update({ ...user });
}
";
    assert_eq!(ok(src), src.replacen("val ", "", 1));
}

#[test]
fn val_capability_check_reads_annotated_declarators() {
    let e = err("\
type Handler = (u: Box) => void;
const update: Handler = (u) => { u.n = 1; };
function f(val b: Box) {
  update(b);
}
");
    assert!(
        e.message.contains("mutable parameter `u` of `update`"),
        "{}",
        e.message
    );
}

/* ------------------------------------------------------------------ */
/* name resolution (TASK-102)                                          */
/* ------------------------------------------------------------------ */

#[test]
fn misspelled_case_in_a_match_arm_names_the_case_meant() {
    let e = err(r#"variant Shape { Circle(radius: number), Empty }
const a = match (s) {
  Circel(radius) => radius,
  Empty => 0,
};
"#);
    assert!(
        e.message.contains("variant Shape has no case `Circel`"),
        "{}",
        e.message
    );
    // reported at the tag, not at the match
    assert_eq!((e.line, e.col), (3, 3));
}

#[test]
fn a_misspelled_case_is_reported_instead_of_the_exhaustiveness_it_breaks() {
    // The typo removes Shape from the candidate table, which used to turn
    // the exhaustiveness check off *silently* — the bug this pass fixes.
    let e = err(r#"variant Shape { Circle(radius: number), Empty }
const a = match (s) { Circel(radius) => radius, Empty => 0 };
"#);
    assert!(e.message.contains("has no case `Circel`"), "{}", e.message);
    assert!(!e.message.contains("exhaustive"), "{}", e.message);
}

#[test]
fn single_pattern_spelling_without_a_subject_owner_waits_for_typescript() {
    let out = ok(r#"variant Shape { Circle(radius: number), Empty }
function f(): number {
  const Circel(radius) = s else { return 0; };
  return radius;
}
"#);
    assert!(out.contains("kind !== \"Circel\""), "{out}");

    let out = ok(r#"variant Shape { Circle(radius: number), Empty }
if let Circel(radius) = s { log(radius); }
"#);
    assert!(out.contains("kind === \"Circel\""), "{out}");
}

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
    assert_eq!(edit.replacement, "radius");
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
fn misspelled_field_names_the_field_meant() {
    let e = err(r#"variant Shape { Circle(radius: number), Empty }
const a = match (s) { Circle(radiuz) => radiuz, Empty => 0 };
"#);
    assert!(
        e.message
            .contains("variant Shape: case `Circle` has no field `radiuz`"),
        "{}",
        e.message
    );
}

#[test]
fn misspelled_field_is_reported_in_let_else_too() {
    let e = err(r#"variant Shape { Circle(radius: number), Empty }
function f(): number {
  const Circle(radiuz) = s else { return 0; };
  return radiuz;
}
"#);
    assert!(e.message.contains("has no field `radiuz`"), "{}", e.message);
}

#[test]
fn misspelled_case_of_a_nested_pattern_is_resolved_through_the_field_type() {
    let e = err(r#"variant Inner { Yes(n: number), No }
variant Outer { Wrap(inner: Inner), Bare }
const a = match (o) { Wrap(inner: Yess(n)) => n, Bare => 0 };
"#);
    assert!(
        e.message.contains("variant Inner has no case `Yess`"),
        "{}",
        e.message
    );
}

#[test]
fn a_misspelled_builtin_case_is_reported() {
    let e = err("const n = match (o) { Some(value) => value, Non => 0 };\n");
    assert!(
        e.message
            .contains("built-in variant Option has no case `Non`"),
        "{}",
        e.message
    );
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

#[test]
fn tags_of_a_hand_written_union_are_not_resolution_errors() {
    // A tag pattern matches any `kind`-tagged union (language.md §3.2), so
    // names no declaration table holds are not wrong — they are the point.
    let out = ok(
        r#"type Msg = { kind: "Ping" } | { kind: "Pong"; n: number };
const a = match (m) { Ping => 0, Pong(n) => n, _ => -1 };
"#,
    );
    assert!(out.contains("case \"Ping\""));
}

#[test]
fn a_shared_tag_name_does_not_drag_an_unrelated_union_into_a_variant() {
    // `Empty` is also a Shape case, so the analysis identifies Shape — but
    // `Full` is nobody's misspelling, so nothing is reported.
    let out = ok(r#"variant Shape { Circle(radius: number), Empty }
type Msg = { kind: "Empty" } | { kind: "Full"; n: number };
const a = match (m) { Empty => 0, Full(n) => n };
"#);
    assert!(out.contains("case \"Full\""));
}

#[test]
fn a_hand_written_payload_field_is_not_a_misspelling() {
    // The tags are exactly Option's, so the analysis reads Option's
    // declaration — but `v` is not `value` misspelled, so it stays quiet.
    let out = ok("const n = match (o) { Some(v) => v, None => 0 };\n");
    assert!(out.contains("const { v } = $tt_m"));
}

#[test]
fn a_two_edit_case_typo_needs_a_match_to_corroborate_the_variant() {
    // `Cyrcla` is two edits from `Circle`. In a match another arm names
    // the variant, so the typo is reported...
    let e = err(r#"variant Shape { Circle(radius: number), Empty }
const a = match (s) { Cyrcla(radius) => radius, Empty => 0 };
"#);
    assert!(e.message.contains("has no case `Cyrcla`"), "{}", e.message);

    // ...but a let-else has only its own tag, so two edits are not enough
    // evidence that this is Shape at all. One edit is (`Circel` above).
    let out = ok(r#"variant Shape { Circle(radius: number), Empty }
function f(): number {
  const Cyrcla(radius) = s else { return 0; };
  return radius;
}
"#);
    assert!(out.contains("\"Cyrcla\""));
}

#[test]
fn a_misspelled_case_in_a_tuple_match_position_is_reported() {
    // Payload cases make these tt variants rather than TypeScript enums.
    let e = err(r#"variant Dir { North(dx: number), South }
variant Speed { Fast(v: number), Slow }
const n = match (d, s) {
  (North(dx), Fast(v)) => dx + v,
  (Nrth(dx), Slow) => dx,
  (South, _) => 3,
};
"#);
    assert!(
        e.message.contains("variant Dir has no case `Nrth`"),
        "{}",
        e.message
    );
}

/* ------------------------------------------------------------------ */
/* exhaustiveness by usefulness (TASK-103)                             */
/* ------------------------------------------------------------------ */

#[test]
fn nested_patterns_that_cover_the_payload_are_exhaustive() {
    // The old rule counted tags, so an arm with a nested pattern covered
    // nothing and this exhaustive match was rejected.
    let out = ok(r#"variant Inner { Yes(n: number), No }
variant Outer { Wrap(inner: Inner), Bare }
const a = match (o) {
  Wrap(inner: Yes(n)) => n,
  Wrap(inner: No()) => 0,
  Bare => -1,
};
"#);
    assert!(out.contains("$tt_m.inner.kind === \"Yes\""), "{out}");
}

#[test]
fn a_generic_payload_is_typed_by_the_patterns_written_in_it() {
    // `Ok`'s payload is declared `T`, which names no variant — but `Some`
    // and `None` written there name Option, exactly as arm tags name a
    // match's subject.
    let out = ok(r#"const n = match (r) {
  Ok(value: Some(value: v)) => v,
  Ok(value: None()) => 0,
  Err(error) => -1,
};
"#);
    assert!(out.contains("$tt_m.value.kind === \"Some\""), "{out}");
}
