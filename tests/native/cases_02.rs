#[test]
fn typed_diagnostic_ranges_follow_source_ownership_not_mapping_accidents() {
    require_tsgo!();
    let source = "import type { TResult } from \"@tt/std\";\n\
        import * as Result from \"@tt/std/result\";\n\
        variant Input { Blank, Num(value: number) }\n\
        variant InputError { Empty }\n\
        variant RangeError { TooLarge(value: number) }\n\
        variant Conn { Up(value: number), Down }\n\
        export function toPort(input: Input): TResult<number, InputError> {\n\
        \x20 return match (input) {\n\
        \x20   Blank => Result.Err(InputError.Empty),\n\
        \x20   Num(value) => Result.Err(RangeError.TooLarge(value)),\n\
        \x20 };\n\
        }\n\
        const test = (): TResult<string, number> => Result.Err(10);\n\
        export function bind(): TResult<number, InputError> {\n\
        \x20 return result {\n\
        \x20   const n = try test();\n\
        \x20   return n;\n\
        \x20 };\n\
        }\n\
        export const mixed = (c: Conn): string =>\n\
        \x20 match (c) { Up(value) => \"up\", 404 => \"gone\", Down => \"down\" };\n";
    let dir = project(&[("src/ranges.tt", source)]);
    let answer = typed_server(&dir, "src/ranges.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();

    let match_mismatch = diagnostics
        .iter()
        .find(|d| {
            d["message"]
                .as_str()
                .is_some_and(|m| m.contains("found `RangeError`"))
        })
        .unwrap_or_else(|| panic!("missing match mismatch: {answer}"));
    // The generated slot now carries the authored return annotation, so
    // TypeScript can point at the exact arm value that violates it instead
    // of discovering the mismatch only when the completed match is returned.
    assert_eq!(
        source_slice(source, match_mismatch),
        "Result.Err(RangeError.TooLarge(value))"
    );

    let result_mismatch = diagnostics
        .iter()
        .find(|d| {
            d["message"]
                .as_str()
                .is_some_and(|m| m.contains("required type: `TResult<number, InputError>`"))
                && d["line"].as_u64().is_some_and(|line| line > 10)
        })
        .unwrap_or_else(|| panic!("missing result mismatch: {answer}"));
    assert_eq!(source_slice(source, result_mismatch), "try test()");

    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == "match-mixed-patterns"),
        "the direct tt cause remains: {answer}"
    );
    assert!(
        diagnostics.iter().all(|d| d["code"] != "ts2678"),
        "checker consequences owned by the invalid match are suppressed: {answer}"
    );
}

#[test]
fn nested_result_return_is_reported_only_for_a_checker_proven_shape() {
    require_tsgo!();
    let source = r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
declare function read(): Res<number, string>;

const definite = result { const value = try read(); return Res.Ok(value); };
const 값: Res<number, string> = Res.Ok(1);
const definiteUnicode = result { const value = try read(); return 값; };
const union = result { const value = try read(); const candidate: Res<number, string> | number = value; return candidate; };
const nonResult = result { const value = try read(); return String(value); };
const unknown = result { const value = try read(); const candidate: unknown = value; return candidate; };
function generic<T>(candidate: T) { return result { const value = try read(); return candidate; }; }
"#;
    let dir = project(&[("src/nested.tt", source)]);
    let answer = typed_server(&dir, "src/nested.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let nested: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == "result-return-nested")
        .collect();
    assert_eq!(nested.len(), 2, "{answer}");
    assert_eq!(source_slice(source, nested[0]), "Res.Ok(value)");
    assert_eq!(source_slice(source, nested[1]), "값");
    let edit = &nested[0]["suggestions"][0]["edit"];
    assert_eq!(edit["replacement"], "try ");
}

#[test]
fn a_pattern_typo_suppresses_typed_exhaustiveness_for_that_match() {
    require_tsgo!();
    let dir = project(&[(
        "src/typo.tt",
        "variant Shape { Circle(radius: number), Square(size: number) }\n\
         export function area(shape: Shape): number {\n\
         \x20 return match (shape) { Circel(radius) => radius, Square(size) => size * size };\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("has no case `Circel`"),
        "the cause is reported: {out}"
    );
    assert!(
        !out.contains("not exhaustive"),
        "the typo's typed cascade is suppressed: {out}"
    );
}

#[test]
fn a_reported_imported_field_error_owns_only_its_match_exhaustiveness() {
    require_tsgo!();
    let source = "import { Fulfillment, PaymentMethod } from \"./domain.tt\";\n\
        export function label(state: Fulfillment): string {\n\
        \x20 return match (state) {\n\
        \x20   Pending => \"Pending\",\n\
        \x20   Shipped(carrier, trackng) => `${carrier} ${trackng}`,\n\
        \x20 };\n\
        }\n\
        export function fee(method: PaymentMethod): number {\n\
        \x20 return match (method) { Card(brand) => brand.length };\n\
        }\n";
    let dir = project(&[
        (
            "src/domain.tt",
            "export variant Fulfillment {\n\
             \x20 Pending,\n\
             \x20 Shipped(carrier: string, tracking: string),\n\
             \x20 Delivered,\n\
             \x20 Cancelled,\n\
             }\n\
             export variant PaymentMethod { Card(brand: string), BankTransfer(iban: string) }\n",
        ),
        ("src/combo.tt", source),
    ]);

    let out = check(&dir);
    assert!(
        out.contains("case `Shipped` has no field `trackng`"),
        "the source cause is reported: {out}"
    );
    assert!(
        !out.contains("Delivered") && !out.contains("Cancelled"),
        "the reported cause owns its match's coverage consequence: {out}"
    );
    assert!(
        out.contains("missing \"BankTransfer\""),
        "an independent match keeps its coverage result: {out}"
    );

    let answer = typed_server(&dir, "src/combo.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "unknown-field"),
        "the server reports the same owner: {answer}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == "match-not-exhaustive"
                && diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("BankTransfer"))
        }),
        "the server preserves the independent coverage result: {answer}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic["message"].as_str().is_none_or(|message| {
                !message.contains("Delivered") && !message.contains("Cancelled")
            })
        }),
        "the owned coverage consequence stays suppressed: {answer}"
    );
}

#[test]
fn an_imported_case_without_declaration_ownership_uses_checker_evidence() {
    require_tsgo!();
    let source = "import { PaymentMethod } from \"./domain.tt\";\n\
        export function fee(method: PaymentMethod): number {\n\
        \x20 return match (method) { Crad(brand) => 1, _ => 0 };\n\
        }\n";
    let dir = project(&[
        (
            "src/domain.tt",
            "export variant PaymentMethod { Card(brand: string), BankTransfer(iban: string) }\n",
        ),
        ("src/payment.tt", source),
    ]);

    let out = check(&dir);
    assert!(
        out.contains("error[ts2678]")
            && out.contains("Type '\"Crad\"' is not comparable")
            && !out.contains("unknown-case"),
        "the typed CLI reports the checker-proven incompatibility: {out}"
    );

    let answer = typed_server(&dir, "src/payment.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let case = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "ts2678")
        .unwrap_or_else(|| panic!("missing imported case diagnostic: {answer}"));
    assert!(
        case["message"]
            .as_str()
            .is_some_and(|message| message.contains("Crad")),
        "the checker fact names the incompatible case: {answer}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["code"] != "unknown-case"),
        "no spelling-based owner is inferred: {answer}"
    );
}

#[test]
fn parser_errors_do_not_hide_an_independent_type_error_in_the_same_file() {
    require_tsgo!();
    let dir = project(&[(
        "src/recovery.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         function read(value: number): TResult<number, string> {\n\
         \x20 return Result.Ok(value);\n\
         }\n\
         export function nested(value: number): TResult<number, string> {\n\
         \x20 return result {\n\
         \x20   const first = try read(value);\n\
         \x20   if (first > 0) { const second = try read(first); }\n\
         \x20   return first;\n\
         \x20 };\n\
         }\n\
         const wrong = (): TResult<string, number> => Result.Err(10);\n\
         export function bindNonResult(): TResult<number, string> {\n\
         \x20 return result { const value = try wrong(); return value; };\n\
         }\n\
         export const malformed = match value { Missing => 0 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("tt `match` could not be parsed")
            && out.contains("a match scrutinee is parenthesized"),
        "the malformed construct remains visible, with its fix: {out}"
    );
    assert!(
        out.contains("type mismatch: expected `string`, found `number`")
            && out.contains("required type: `TResult<number, string>`"),
        "the independent bindNonResult type error survives recovery: {out}"
    );
}

#[test]
fn a_ts_file_and_an_tt_file_share_one_project_graph() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/user.ts",
            "export type State = \"idle\" | \"loading\" | \"done\";\n",
        ),
        (
            "src/state.tt",
            "import type { State } from \"./user\";\n\
             export function render(state: State): number {\n\
             \x20 return match (state) { \"idle\" => 0, \"loading\" => 1, \"done\" => 2 };\n\
             }\n",
        ),
    ]);
    // The type comes from the `.ts` file; the match is exhaustive over it.
    assert_eq!(check(&dir), "");
}

#[test]
fn literal_exhaustiveness_uses_the_narrowed_type_at_the_match() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/user.ts",
            "export type State = \"idle\" | \"loading\" | \"done\";\n",
        ),
        (
            "src/state.tt",
            "import type { State } from \"./user\";\n\
             export function render(state: State): number {\n\
             \x20 if (state !== \"idle\") {\n\
             \x20   return match (state) { \"loading\" => 1 };\n\
             \x20 }\n\
             \x20 return 0;\n\
             }\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        out.contains("missing \"done\""),
        "the narrowed type still allows \"done\": {out}"
    );
    assert!(
        !out.contains("idle"),
        "the guard removed \"idle\" before the match: {out}"
    );
}

#[test]
fn variant_exhaustiveness_uses_the_narrowed_type_at_the_match() {
    require_tsgo!();
    let dir = project(&[(
        "src/shape.tt",
        "export variant Shape { Circle(radius: number), Square(side: number), Point }\n\
         export function area(s: Shape): number {\n\
         \x20 if (s.kind !== \"Point\") {\n\
         \x20   return match (s) { Circle(radius) => radius };\n\
         \x20 }\n\
         \x20 return 0;\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("missing \"Square\""),
        "the narrowed type still allows Square: {out}"
    );
    assert!(
        !out.contains("Point"),
        "the guard removed Point before the match: {out}"
    );
}

#[test]
fn val_holds_on_a_parameter_and_across_a_function_boundary() {
    require_tsgo!();
    let dir = project(&[(
        "src/pass.tt",
        "interface User { name: string; tags: string[] }\n         function update(user: User) { user.name = \"Lee\"; }\n         export function process(val user: User) {\n         \x20 user.name = \"Lee\";\n         \x20 user.tags.push(\"x\");\n         \x20 update(user);\n         }\n",
    )]);
    // `val` has two syntactic homes and three rules; a mode that checks
    // only declarations, or only mutation paths, silently passes code the
    // tt-level check rejects.
    let out = check(&dir);
    assert!(
        out.contains("cannot mutate through val binding `user`"),
        "a val parameter is a val binding: {out}"
    );
    assert!(
        out.contains("mutating method `push` through val binding `user`"),
        "and its access paths are read-only too: {out}"
    );
    assert!(
        out.contains("cannot pass val binding `user` to mutable parameter `user` of `update`"),
        "and it cannot be handed to a parameter that is not `val`: {out}"
    );
}

#[test]
fn exhaustiveness_holds_when_the_scrutinee_is_not_a_name() {
    require_tsgo!();
    let dir = project(&[(
        "src/shape.tt",
        "export variant Shape { Circle(radius: number), Rect(w: number, h: number) }\n         declare function getShape(): Shape;\n         type State = \"idle\" | \"loading\" | \"done\";\n         declare function getState(): State;\n         export const area = match (getShape()) { Circle(radius) => radius };\n         export const label = match (getState()) { \"idle\" => 0, \"loading\" => 1 };\n",
    )]);
    // The question is asked about the temporary the match binds, not about
    // the scrutinee's text: at `getShape` the checker answers "a function",
    // which has no cases and no literals, and both questions came back
    // silent when that was where they were asked.
    let out = check(&dir);
    assert!(
        out.contains("missing \"Rect\""),
        "a call scrutinee still has an variant type: {out}"
    );
    assert!(
        out.contains("missing \"done\""),
        "a call scrutinee still has a literal union type: {out}"
    );
}

#[test]
fn a_variant_from_another_module_needs_no_declaration_collecting() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/token.tt",
            "export variant Token { Num(value: number), Eof }\n",
        ),
        (
            "src/parse.tt",
            "import { Token } from \"./token.tt\";\n\
             export function width(t: Token): number {\n\
             \x20 return match (t) { Num(value) => value };\n\
             }\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        out.contains("missing \"Eof\""),
        "the variant's cases come from the imported module's own type: {out}"
    );
}

#[test]
fn val_mutation_is_decided_by_the_method_the_call_resolves_to() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/store.ts",
            "export class Store {\n  set(key: string, value: string): void {}\n}\n",
        ),
        (
            "src/use.tt",
            "import { Store } from \"./store\";\n\
             export function go(): void {\n\
             \x20 val const map = new Map<string, number>();\n\
             \x20 map.set(\"a\", 1);\n\
             \x20 val const store = new Store();\n\
             \x20 store.set(\"a\", \"b\");\n\
             }\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        out.contains("mutating method `set` through val binding `map`"),
        "Map#set is declared in TypeScript's own lib: {out}"
    );
    assert!(
        !out.contains("val binding `store`"),
        "Store#set only shares the name: {out}"
    );
}

#[test]
fn a_shadowing_binding_is_a_different_binding() {
    require_tsgo!();
    let dir = project(&[(
        "src/shadow.tt",
        "export function go(): void {\n\
         \x20 val const items = new Map<string, number>();\n\
         \x20 {\n\
         \x20   const items = new Map<string, number>();\n\
         \x20   items.set(\"inner\", 1);\n\
         \x20 }\n\
         }\n",
    )]);
    assert_eq!(
        check(&dir),
        "",
        "the inner `items` is an ordinary binding that shares a name"
    );
}

#[test]
fn a_direct_mutation_through_a_val_binding_is_reported() {
    require_tsgo!();
    let dir = project(&[(
        "src/direct.tt",
        "export function go(): void {\n\
         \x20 val const user = { name: \"a\", count: 0 };\n\
         \x20 user.name = \"b\";\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("cannot mutate through val binding `user`"),
        "an assignment mutates on syntax alone: {out}"
    );
}

#[test]
fn a_mutation_through_an_unmarked_binding_is_left_alone() {
    require_tsgo!();
    let dir = project(&[(
        "src/plain.tt",
        "export function go(): void {\n\
         \x20 const items: number[] = [];\n\
         \x20 items.push(1);\n\
         \x20 const user = { name: \"a\" };\n\
         \x20 user.name = \"b\";\n\
         }\n",
    )]);
    assert_eq!(check(&dir), "");
}

#[test]
fn an_any_receiver_is_never_called_a_mutation() {
    require_tsgo!();
    let dir = project(&[(
        "src/any.tt",
        "export function go(x: any): void {\n\
         \x20 val const y = x;\n\
         \x20 y.set(\"a\", 1);\n\
         \x20 y.push(1);\n\
         }\n",
    )]);
    assert_eq!(check(&dir), "");
}

#[test]
fn a_call_is_checked_against_the_declaration_it_resolves_to() {
    // Two functions share a name; which one a call names is the callee
    // symbol's answer, not the name's. The outer call reaches the
    // top-level declaration (mutable parameter — an error); the inner
    // call reaches the block's val-parameter arrow (fine). The
    // name-keyed model had to skip both as ambiguous.
    require_tsgo!();
    let dir = project(&[(
        "src/who.tt",
        "type U = { name: string };\n\
         export function go(): void {\n\
         \x20 val const user: U = { name: \"a\" };\n\
         \x20 handle(user);\n\
         \x20 {\n\
         \x20   const handle = (val u: U): void => {};\n\
         \x20   handle(user);\n\
         \x20 }\n\
         }\n\
         function handle(u: U): void { u.name = \"b\"; }\n",
    )]);
    let out = check(&dir);
    assert_eq!(
        out.lines()
            .filter(|l| l.contains("cannot pass val binding `user`"))
            .count(),
        1,
        "only the call that names the mutable-parameter declaration: {out}"
    );
    assert!(
        out.contains("src/who.tt:4:10") && out.contains("mutable parameter `u` of `handle`"),
        "reported at the outer call's argument: {out}"
    );
}

#[test]
fn an_answer_past_the_pipe_buffer_still_arrives() {
    // A few hundred diagnostics make the host's one-line answer larger
    // than a pipe buffer (64 KB on Linux). The host must flush the whole
    // line synchronously before it turns around to wait for the next
    // request — an async write that queued the tail past the buffer
    // deadlocked the session: the host blocked reading, the compiler
    // blocked waiting for the rest of the answer.
    require_tsgo!();
    let mut source = String::new();
    for i in 0..400 {
        source.push_str(&format!("export const a{i}: number = \"x{i}\";\n"));
    }
    let dir = project(&[("src/big.tt", source.as_str())]);
    let out = check(&dir);
    assert_eq!(
        out.lines()
            .filter(|l| l.contains("type mismatch: expected `number`"))
            .count(),
        400,
        "every diagnostic of a >64 KB answer arrives: {out}"
    );
}

#[test]
fn a_non_mutating_builtin_method_is_not_a_mutation() {
    // Collection asks about every method call through a `val` path; the
    // verdict is two halves — the checker's (a built-in's method) and tt's
    // policy (one of the mutating ones). A built-in read fails the second,
    // so widening collection must never widen what is reported.
    require_tsgo!();
    let dir = project(&[(
        "src/read.tt",
        "export function go(): void {\n\
         \x20 val const m = new Map<string, number>();\n\
         \x20 m.get(\"a\");\n\
         \x20 m.has(\"a\");\n\
         \x20 val const items: number[] = [];\n\
         \x20 items.at(0);\n\
         \x20 items.includes(1);\n\
         }\n",
    )]);
    assert_eq!(
        check(&dir),
        "",
        "a built-in method outside tt's mutator policy reads, it does not mutate"
    );
}

#[test]
fn batched_answers_land_on_their_own_questions() {
    // One ask carries every module's questions; the host groups them by
    // module for the checker's batch endpoints and scatters the answers
    // back by index. Each diagnostic must land on its own file and line,
    // whichever module its group ran under.
    require_tsgo!();
    let dir = project(&[
        (
            "src/a.tt",
            "declare const x: \"a\" | \"b\";\n\
             export const va = match (x) { \"a\" => 1 };\n\
             export function fa(): void {\n\
             \x20 val const ua = { n: 0 };\n\
             \x20 ua.n = 1;\n\
             }\n",
        ),
        (
            "src/b.tt",
            "declare const y: \"c\" | \"d\";\n\
             export const vb = match (y) { \"c\" => 1 };\n\
             export function fb(): void {\n\
             \x20 val const ub = { m: 0 };\n\
             \x20 ub.m = 1;\n\
             }\n",
        ),
    ]);
    let out = check(&dir);
    for (at, said) in [
        ("--> src/a.tt:2:", "missing \"b\""),
        ("--> src/b.tt:2:", "missing \"d\""),
        ("--> src/a.tt:5:3", "cannot mutate through val binding `ua`"),
        ("--> src/b.tt:5:3", "cannot mutate through val binding `ub`"),
    ] {
        // The message and the position have to be one diagnostic, not two
        // that happen to both be present.
        assert!(
            block(&out, said).contains(at),
            "expected {said} at {at}: {out}"
        );
    }
}

#[test]
fn a_type_error_is_reported_at_its_position_in_the_tt_source() {
    require_tsgo!();
    let dir = project(&[(
        "src/bad.tt",
        // A multi-byte prefix: TypeScript counts UTF-16 code units and the
        // `.tt` position is a byte offset, so the two have to be converted.
        "export function go(): void {\n  const 한글: string = 1;\n}\n",
    )]);
    let out = check(&dir);
    let reported = block(&out, "type mismatch:");
    assert!(
        reported.contains("--> src/bad.tt:2:22"),
        "the diagnostic belongs at the incompatible expression in the .tt file: {out}"
    );
}

#[test]
fn typed_exhaustiveness_sees_a_hole_inside_a_payload() {
    require_tsgo!();
    // The checker names the scrutinee's constituents; tt runs its own
    // exhaustiveness algorithm over that alphabet, so a nested pattern's
    // hole is seen on this path too (TASK-108). Before, the typed path
    // asked only "which top-level tags are missing?" and answered nothing
    // here, while `--check` reported the hole.
    let dir = project(&[(
        "src/nest.tt",
        "variant Inner { Yes(n: number), No }\n\
         variant Outer { Wrap(inner: Inner), Bare }\n\
         declare const o: Outer;\n\
         export const a = match (o) { Wrap(inner: Yes(n)) => n, Bare => -1 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("match is not exhaustive: missing \"Wrap(inner: No())\""),
        "the typed path sees the payload hole: {out}"
    );
}
