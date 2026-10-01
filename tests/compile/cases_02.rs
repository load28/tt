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

/* ------------------------------------------------------------------ */
/* guards                                                              */
/* ------------------------------------------------------------------ */

#[test]
fn call_arguments_wider_than_the_match_keep_their_authored_frame() {
    // A cast, an operator — anything that binds to the value itself — keeps
    // the authored call and its frame, with the match joined by its slot:
    // re-emitting `as number` around an arm's value would rebind it to that
    // value instead (TASK-332).
    let out = ok("consume(match (x) { A(v) => v, _ => 0 } as number);");
    assert!(out.contains("$tt_v1($tt_v0 as number);"), "{out}");
    let sum = ok("consume(1 + match (x) { A(v) => v, _ => 0 });");
    assert!(sum.contains("$tt_v2(1 + $tt_v0);"), "{sum}");
    let cast_in_literal = ok("consume({item: match (x) { A(v) => v, _ => 0 } as number});");
    assert!(
        cast_in_literal.contains("$tt_v1({item: $tt_v0 as number});"),
        "{cast_in_literal}"
    );
    // An earlier position that is not inert evaluates before the scrutinee.
    // Moving the literal into the arms would run it after, so the literal
    // stays where it was written.
    let effectful = ok("consume({a: effect(), item: match (x) { A(v) => v, _ => 0 }});");
    assert!(
        effectful.contains("$tt_v2({a: $tt_v1, item: $tt_v0});"),
        "{effectful}"
    );
    // A spread copies its operand where the literal is built, running that
    // operand's getters. The positions record only the operand, so the
    // literal is kept where it was written rather than moved past the
    // scrutinee.
    let spread = ok("consume({...rest, item: match (x) { A(v) => v, _ => 0 }});");
    assert!(
        spread.contains("$tt_v2({...$tt_v1, item: $tt_v0});"),
        "{spread}"
    );
    let array_spread = ok("consume([...items, match (x) { A(v) => v, _ => 0 }]);");
    assert!(
        array_spread.contains("$tt_v2([...$tt_v1, $tt_v0]);"),
        "{array_spread}"
    );
    // A block arm rewrites its exits through the string-built prefix, which
    // carries no source mapping — so a literal with authored bytes to place
    // stays outside the arms.
    let block = ok("consume({item: match (x) { A(v) => { return v; }, _ => 0 }});");
    assert!(block.contains("$tt_v1({item: $tt_v0});"), "{block}");
}
