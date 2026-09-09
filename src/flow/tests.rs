use super::*;

#[test]
fn concise_arrow_is_the_innermost_function_target() {
    let source = "function* outer() { const step = (_: unknown) => (try load()); }";
    let tokens = crate::lexer::lex(source, 0, source.len());
    let at = tokens
        .iter()
        .position(|token| {
            matches!(token.kind, TokenKind::Ident)
                && &source[token.span.start..token.span.end] == "try"
        })
        .expect("try token");
    assert_eq!(
        function_target_at(source, &tokens, at),
        Some(FunctionTarget::Ordinary)
    );
}

#[test]
fn semicolon_free_concise_arrow_does_not_own_the_next_try_statement() {
    let source = "function* outer() {\n  const step = () => flag ? 1 : 2\n  try load();\n}";
    let tokens = crate::lexer::lex(source, 0, source.len());
    let at = tokens
        .iter()
        .position(|token| {
            matches!(token.kind, TokenKind::Ident)
                && &source[token.span.start..token.span.end] == "try"
        })
        .expect("try token");
    assert_eq!(
        function_target_at(source, &tokens, at),
        Some(FunctionTarget::Generator)
    );
}

/// Answers the divergence question the way the compiler asks it: the
/// region is parsed first, so tt's own constructs reach the graph.
fn check(body: &str) -> bool {
    let tokens = crate::lexer::lex(body, 0, body.len());
    program_diverges(body, &tokens, &crate::parser::parse(body))
}

#[test]
fn a_flow_body_query_reuses_the_same_structural_body() {
    let source = "if (ready) return 1; else throw error;";
    let tokens = crate::lexer::lex(source, 0, source.len());
    let program = crate::parser::parse(source);
    let queries = FlowBodyQueries::default();
    let span = crate::ast::Span {
        start: 0,
        end: source.len(),
    };
    assert!(queries.diverges(source, span, &tokens, &program));
    assert!(queries.diverges(source, span, &tokens, &program));
    assert_eq!(queries.hits(), 1);
}

#[test]
fn the_four_keywords_diverge_as_before() {
    assert!(check("return 0;"));
    assert!(check("throw new Error(\"x\");"));
    assert!(check("break;"));
    assert!(check("continue;"));
    assert!(check("log(\"x\"); return 0;"));
    assert!(check("return { k: 1 };"));
}

#[test]
fn plain_statements_do_not() {
    assert!(!check("log(\"x\");"));
    assert!(!check(""));
    assert!(!check("const o = { n: 1 };"));
}

#[test]
fn an_if_needs_both_branches_to_diverge() {
    assert!(!check("if (c) { return 1; }"));
    assert!(check("if (c) { return 1; } else { return 2; }"));
    assert!(check("if (c) return 1; else return 2;"));
    assert!(!check("if (c) { return 1; } else { log(\"x\"); }"));
}

#[test]
fn else_if_chains_are_walked() {
    assert!(check(
        "if (a) { return 1; } else if (b) { throw e; } else { return 2; }"
    ));
    assert!(!check("if (a) { return 1; } else if (b) { throw e; }"));
}

#[test]
fn a_bare_block_diverges_when_its_body_does() {
    assert!(check("{ return 1; }"));
    assert!(!check("{ log(\"x\"); }"));
}

#[test]
fn code_after_a_diverging_statement_is_unreachable_not_a_hole() {
    assert!(check("return 0; log(\"never\");"));
}

#[test]
fn a_functions_return_does_not_leave_the_enclosing_block() {
    assert!(!check("function g() { return 1; }"));
    assert!(!check("const g = () => { return 1; };"));
    // …but a diverging statement after one still counts.
    assert!(check("function g() { return 1; } return g();"));
}

#[test]
fn a_loop_diverges_only_when_it_has_no_normal_exit() {
    // An omitted or literal-`true` test is the whole rule: such a loop
    // is left only by `break`, `return`, or `throw`.
    assert!(check("for (;;) { log(\"x\"); }"));
    assert!(check("while (true) { log(\"x\"); }"));
    assert!(check("while ((true)) { log(\"x\"); }"));
    assert!(check("do { log(\"x\"); } while (true);"));
    assert!(check("for (let i = 0;; i += 1) { log(\"x\"); }"));
    // A test that can fail is a normal exit, however the body ends.
    assert!(!check("while (c) { return 1; }"));
    assert!(!check("for (let i = 0; i < n; i += 1) { return 1; }"));
    // An iteration may end, or never begin.
    assert!(!check("for (const x of xs) { return 1; }"));
    assert!(!check("for (const k in o) { return 1; }"));
    assert!(!check("for await (const x of xs) { return 1; }"));
    // `do … while` runs its body before the test, so a body that
    // diverges takes the statement with it.
    assert!(check("do { return 1; } while (c);"));
    assert!(check("do { throw e; } while (c)"));
}

#[test]
fn a_break_leaves_the_loop_it_names_not_the_block() {
    assert!(!check("while (true) { break; }"));
    assert!(!check("for (;;) { if (c) { break; } }"));
    assert!(!check("outer: while (true) { break outer; }"));
    // …but a `break` whose target is outside the analyzed body leaves
    // the body, which is what the four-keyword rule always meant.
    assert!(check("break;"));
    assert!(check("continue;"));
    assert!(check("break outer;"));
    // A `continue` cannot leave the loop it names.
    assert!(check("while (true) { continue; }"));
    assert!(check(
        "outer: while (true) { while (c) { continue outer; } }"
    ));
    // A `break` naming an outer loop leaves the inner one only.
    assert!(!check(
        "outer: while (true) { while (true) { break outer; } }"
    ));
}

#[test]
fn a_labeled_block_is_breakable_but_not_continuable() {
    assert!(!check("outer: { break outer; }"));
    assert!(check("outer: { break outer; } return 0;"));
    assert!(check("outer: { return 1; }"));
    // An unlabeled `break` is not captured by a labeled block, so it
    // still leaves the analyzed body.
    assert!(check("outer: { break; }"));
}

#[test]
fn a_switch_diverges_when_it_has_a_default_and_no_clause_falls_out() {
    assert!(check(
        "switch (k) { case \"a\": return 1; default: throw e; }"
    ));
    // No `default` — an unmatched discriminant walks past the whole
    // statement.
    assert!(!check("switch (k) { case \"a\": return 1; }"));
    // A `break` targets the switch, so the clause reaches the
    // statement's successor.
    assert!(!check(
        "switch (k) { case \"a\": break; default: return 1; }"
    ));
    assert!(!check(
        "switch (k) { case \"a\": return 1; default: break; }"
    ));
    // Clauses fall through to the next one.
    assert!(check(
        "switch (k) { case \"a\": case \"b\": return 1; default: return 2; }"
    ));
    // The last clause falls out of the statement when it completes.
    assert!(!check("switch (k) { default: log(\"x\"); }"));
    // A `continue` is not the switch's to capture — it belongs to the
    // enclosing loop, which is outside the analyzed body here.
    assert!(check("switch (k) { default: continue; }"));
    // A conditional in a `case` label does not close it early.
    assert!(check(
        "switch (k) { case c ? 1 : 2: return 1; default: return 2; }"
    ));
    // A nested `switch`'s clauses belong to it, not to the outer one.
    assert!(check(
        "switch (k) { default: switch (j) { case 1: return 1; default: return 2; } }"
    ));
}

#[test]
fn a_try_diverges_when_every_half_that_can_complete_does_not() {
    assert!(check("try { return 1; } catch (e) { throw e; }"));
    assert!(check("try { return 1; } catch { return 2; }"));
    // The handler can run in place of the guarded block, so one half
    // completing normally is enough to reach the successor.
    assert!(!check("try { return 1; } catch (e) { log(e); }"));
    assert!(!check("try { log(\"x\"); } catch (e) { return 1; }"));
    // Without a handler an exception leaves the function; normal
    // completion is the only edge out.
    assert!(check("try { return 1; } finally { log(\"x\"); }"));
    assert!(!check("try { log(\"x\"); } finally { log(\"x\"); }"));
    // Everything leaving normally runs the `finally` first.
    assert!(check("try { log(\"x\"); } finally { return 1; }"));
    assert!(check(
        "try { return 1; } catch (e) { log(e); } finally { throw e; }"
    ));
    // tt's `try` statement has neither tail, so it stays opaque.
    assert!(!check("try load();"));
}

#[test]
fn a_brace_on_its_own_line_does_not_start_a_statement() {
    // Allman braces are why the automatic-semicolon rule splits only
    // before the statement *keywords* and never before `{`: after
    // `function g()` or `= function ()` a newline and a brace still
    // open that function's body, and splitting there would let its
    // `return` escape into the analyzed block.
    assert!(!check("function g()\n{\n  return 1;\n}"));
    assert!(!check("const g = function ()\n{\n  return 1;\n};"));
    assert!(!check("const g = () =>\n{\n  return 1;\n};"));
    assert!(!check("const o =\n{\n  n: 1\n};"));
    // Control-flow bodies are found structurally, so Allman braces
    // read the same there.
    assert!(check("if (c)\n{\n  return 1;\n}\nelse\n{\n  return 2;\n}"));
    assert!(check("while (true)\n{\n  log(\"x\");\n}"));
    assert!(check(
        "switch (k)\n{\n  case \"a\": return 1;\n  default: throw e;\n}"
    ));
}

#[test]
fn a_labeled_statement_carries_its_bodys_flow() {
    assert!(check("label: return 1;"));
    assert!(check("label: { return 1; }"));
    assert!(check("variant: { return 1; }"));
    assert!(!check("label: { break label; }"));
    assert!(!check("variant: { break variant; }"));
    assert!(!check("label: log(\"x\");"));
}

#[test]
fn typescript_enum_and_tt_variant_are_distinct_statement_heads() {
    assert!(check("enum E { A } throw e;"));
    assert!(check("const enum E { A } throw e;"));
    assert!(check("declare enum E { A } throw e;"));
    assert!(check("export enum E { A } throw e;"));
    assert!(check("export const enum E { A } throw e;"));
    assert!(check("export declare enum E { A } throw e;"));
    assert!(check("export default enum E { A } throw e;"));
    assert!(check("variant V { A } throw e;"));
    assert!(check("variant V<T> { A(value: T) } throw e;"));
    assert!(check("export variant V { A } throw e;"));
}

#[test]
fn an_if_let_diverges_when_both_of_its_inline_halves_do() {
    // An `if let` body and its `else` are inline — an exit written in
    // either leaves the enclosing function — so the statement carries
    // divergence exactly as an `if` does.
    assert!(check(
        "if let Ok(value) = r { return value; } else { return 1; }"
    ));
    assert!(check(
        "if let Ok(value) = r { return value; } else { throw e; }"
    ));
    // A chained `else if let` is walked like an `else if`.
    assert!(check(
        "if let Ok(value) = r { return value; } else if let Err(error) = r { throw error; } else { return 0; }"
    ));
    // Nested in either half.
    assert!(check(
        "if let Ok(value) = r { if let Ok(inner) = r { return inner; } else { return value; } } else { return 1; }"
    ));
    // Without an `else` the unmatched pattern walks past the statement.
    assert!(!check("if let Ok(value) = r { return value; }"));
    assert!(!check(
        "if let Ok(value) = r { log(value); } else { return 1; }"
    ));
    assert!(!check(
        "if let Ok(value) = r { return value; } else { log(\"x\"); }"
    ));
    assert!(!check(
        "if let Ok(value) = r { return value; } else if let Err(error) = r { throw error; }"
    ));
}

#[test]
fn an_isolated_value_region_cannot_carry_the_blocks_divergence() {
    // A match arm, a `result` block and a `try` statement are not
    // approximations left as fall-through: an exit written in an
    // isolated value region belongs to the construct's value and can
    // never leave the block, and a `try` statement's early return is
    // conditional. "Does not diverge" is the exact answer.
    assert!(!check(
        "const x = match (o) { Some(n) => n, None => 0 }; log(x);"
    ));
    assert!(!check(
        "const y = result { const a = try load(); return a; }; log(y);"
    ));
    assert!(!check("try load();"));
    assert!(!check(
        "const Ok(value) = r else { return 1; }; log(value);"
    ));
    // …and a let-else whose own block diverges still falls through to
    // the statement after it.
    assert!(check(
        "const Ok(value) = r else { return 1; }; return value;"
    ));
}

#[test]
fn statements_read_the_same_without_semicolons() {
    assert!(check("log(\"x\")\nreturn 0"));
    assert!(check(
        "const o = { a: 1 }\nif (o.a) { return 1 } else { return 2 }"
    ));
    assert!(check("const n = 1\nwhile (true) { log(n) }"));
    // A nested function body is still opaque across a line break.
    assert!(!check("const g = () => { return 1 }\nlog(g())"));
    // A line terminator after `break` ends the statement, so the next
    // line is not its label.
    assert!(check("break\nouter;"));
}

/// Whether the byte at `needle`'s position sits inside a function body.
fn inside(src: &str, needle: &str) -> bool {
    let offset = src.find(needle).expect("needle");
    let tokens = crate::lexer::lex(src, 0, src.len());
    let at = tokens
        .iter()
        .position(|t| t.span.start >= offset)
        .unwrap_or(tokens.len());
    in_function_body(src, &tokens, at)
}

#[test]
fn function_method_and_arrow_bodies_are_returnable() {
    assert!(inside("function f() { HERE; }", "HERE"));
    assert!(inside("const f = function () { HERE; };", "HERE"));
    assert!(inside("const f = function* () { HERE; };", "HERE"));
    assert!(inside("const f = () => { HERE; };", "HERE"));
    assert!(inside("const o = { m() { HERE; } };", "HERE"));
    assert!(inside("class A { m() { HERE; } }", "HERE"));
    assert!(inside("class A { constructor() { HERE; } }", "HERE"));
    assert!(inside("class A { get x() { HERE; } }", "HERE"));
    assert!(inside("function f<T>(x: T) { HERE; }", "HERE"));
    // Return-type annotations sit between the parameter list and the
    // body; the walk crosses them.
    assert!(inside("function f(): void { HERE; }", "HERE"));
    assert!(inside("function f(): Promise<void> { HERE; }", "HERE"));
    assert!(inside("function f(): { a: number } { HERE; }", "HERE"));
    assert!(inside("function f(): X[] | null { HERE; }", "HERE"));
    assert!(inside("function f(): variant { HERE; }", "HERE"));
    assert!(inside("const o = { m(): number { HERE; } };", "HERE"));
    // Nested in control flow inside the function — the `return` still
    // exits the function.
    assert!(inside(
        "function f() { if (c) { for (;;) { HERE; } } }",
        "HERE"
    ));
}

#[test]
fn module_and_non_function_braces_are_not() {
    assert!(!inside("HERE;", "HERE"));
    assert!(!inside("{ HERE; }", "HERE"));
    assert!(!inside("if (c) { HERE; }", "HERE"));
    assert!(!inside("for (;;) { HERE; }", "HERE"));
    assert!(!inside("for await (const x of y) { HERE; }", "HERE"));
    assert!(!inside("while (c) { HERE; }", "HERE"));
    assert!(!inside("switch (x) { default: HERE; }", "HERE"));
    assert!(!inside("try { HERE; } catch (e) {}", "HERE"));
    assert!(!inside("do { HERE; } while (c);", "HERE"));
    assert!(!inside("namespace N { HERE; }", "HERE"));
    assert!(!inside("class A extends mixin(B) { HERE; }", "HERE"));
    assert!(!inside("class A { static { HERE; } }", "HERE"));
    assert!(!inside("class A<T> { x = HERE; }", "HERE"));
    // A function body *closed before* the position provides nothing.
    assert!(!inside("function f() {} HERE;", "HERE"));
}
