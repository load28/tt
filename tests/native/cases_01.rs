#[test]
fn a_declaration_carries_a_map_back_to_the_tt_source() {
    require_emit!();
    let dir = project(&[(
        "src/token.tt",
        "export variant Token { Num(value: number), Eof }\n\
         export function width(t: Token): number {\n\
         \x20 return match (t) { Num(value) => value, Eof => 0 };\n\
         }\n",
    )]);
    let out_dir = dir.join("out");
    let out = run(&dir, &["--types", "src", "-o", out_dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The sidecar takes the name a `"./token.tt"` specifier resolves to
    // when no compiler is running — which is what makes it a sidecar.
    let declarations = fs::read_to_string(out_dir.join("token.tt.d.ts")).expect("the sidecar");
    assert!(
        declarations.contains("//# sourceMappingURL=token.tt.d.ts.map"),
        "and points at its map: {declarations}"
    );
    let map = fs::read_to_string(out_dir.join("token.tt.d.ts.map")).expect("the map");
    assert!(
        map.contains("token.tt\"") && map.contains("\"mappings\""),
        "whose sources is the .tt file, so go-to-definition lands there: {map}"
    );
}

#[test]
fn declarations_are_emitted_by_the_compiler_itself() {
    require_emit!();
    let dir = project(&[(
        "src/shape.tt",
        "export variant Shape { Circle(radius: number), Point }\n\
         export function area(s: Shape): number {\n\
         \x20 return match (s) { Circle(radius) => radius, Point => 0 };\n\
         }\n",
    )]);
    let out_dir = dir.join("out");
    let out = run(&dir, &["--types", "src", "-o", out_dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let declaration = fs::read_to_string(out_dir.join("shape.tt.d.ts")).expect("a .d.ts");
    // ttc writes no declaration syntax of its own: this is what the compiler
    // emits for the module ttc lowered, exactly as for a hand-written one.
    assert!(
        declaration.contains("kind: \"Circle\"") && declaration.contains("radius: number"),
        "the variant's union type: {declaration}"
    );
    assert!(
        declaration.contains("export declare function area(s: Shape): number;"),
        "the function's signature: {declaration}"
    );
}

#[test]
fn the_standard_library_enters_the_graph_as_a_module_of_the_project() {
    require_emit!();
    let dir = project(&[(
        "src/parse.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         export function parse(text: string): TResult<number, string> {\n\
         \x20 const n = Number(text);\n\
         \x20 return Number.isNaN(n) ? Result.Err(\"not a number\") : Result.Ok(n);\n\
         }\n",
    )]);
    let out_dir = dir.join("out");
    let out = run(&dir, &["--types", "src", "-o", out_dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "@tt/std has to resolve, and its types have to check: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    // The library is a module of the project, resolved by ordinary node
    // resolution — so the specifier stays bare, in the source and in the
    // declaration alike, and no `paths` entry is involved in this compile.
    let declaration = fs::read_to_string(out_dir.join("parse.tt.d.ts")).expect("a .d.ts");
    assert!(
        declaration.contains("from \"@tt/std\""),
        "the declaration keeps the specifier the user wrote: {declaration}"
    );
}

#[test]
fn the_pipeline_runtime_enters_the_typed_project_once() {
    require_emit!();
    let dir = project(&[
        (
            "src/a.tt",
            "declare const input: number;\ndeclare const step: (value: number) => string;\nexport const value = input |> step;\n",
        ),
        (
            "src/b.tt",
            "declare const input: number;\ndeclare const step: (value: number) => string;\nexport const value = input |> step;\n",
        ),
    ]);
    let out = run(&dir, &["--check-types", "src"]);
    assert!(
        out.status.success(),
        "@tt/runtime has to resolve once for every pipeline module: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn a_diagnostic_on_generated_code_is_restated_in_tts_words() {
    require_tsgo!();
    // A plain TypeScript enum is not a tt variant, so matching on one lowers
    // to a `.kind` switch over a value that has no `kind`. The error is
    // real and it is the user's, but the text TypeScript points at is code
    // ttc wrote — so ttc says what the construct meant, at the construct
    // (TASK-104), with TypeScript's own sentence alongside for checking.
    let dir = project(&[(
        "src/ts_enum.tt",
        "export enum Plain { A, B }\n\
         export function f(p: Plain): number {\n\
         \x20 return match (p) { A => 1 };\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("--> src/ts_enum.tt:3:10"),
        "reported at the `match` keyword in the .tt file: {out}"
    );
    assert!(
        out.contains("match on a tag pattern needs a value with a `kind` discriminant"),
        "in tt's words: {out}"
    );
    assert!(
        out.contains("ts2339: Property 'kind' does not exist on type 'Plain'."),
        "with the original alongside: {out}"
    );
}

#[test]
fn a_restated_diagnostic_calls_a_case_by_its_declared_name() {
    require_tsgo!();
    // TypeScript has no word for a tt case, so a narrowed one prints as
    // the object type it lowers to. tt names both sides from declarations.
    let dir = project(&[(
        "src/named.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         variant Wire { OutOfRange(value: number), Missing }\n\
         variant ParseError { NotANumber(text: string) }\n\
         function inner(w: Wire) {\n\
         \x20 if (w.kind === \"OutOfRange\") { return Result.Err(w); }\n\
         \x20 return Result.Ok(1);\n\
         }\n\
         export function outer(w: Wire): TResult<number, ParseError> {\n\
         \x20 const n = try inner(w);\n\
         \x20 return Result.Ok(n);\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("type mismatch: expected `ParseError`, found `Wire.OutOfRange`")
            && out.contains("required type: `TResult<number, ParseError>`"),
        "the case and surrounding obligation use tt declaration names: {out}"
    );
    assert!(
        !out.contains("{ kind: \"OutOfRange\"; value: number; }") && !out.contains("in tt's names"),
        "the lowered representation and duplicate prose stay hidden: {out}"
    );
}

#[test]
fn assignability_diagnostics_report_the_minimal_type_difference() {
    require_tsgo!();
    let dir = project(&[(
        "src/mismatch.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         variant InputError { Empty, NotANumber(raw: string) }\n\
         variant RangeError { TooLarge(value: number, max: number) }\n\
         export function port(value: number): TResult<number, InputError> {\n\
         \x20 return value > 65535\n\
         \x20   ? Result.Err(RangeError.TooLarge(value, 65535))\n\
         \x20   : Result.Err(InputError.Empty);\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("type mismatch: expected `InputError`, found `RangeError`"),
        "minimal incompatible leaf: {out}"
    );
    assert!(
        out.contains("required type: `TResult<number, InputError>`"),
        "the surrounding obligation remains visible: {out}"
    );
    assert!(
        !out.contains("Property 'raw' is missing") && !out.contains("in tt's names"),
        "the nested checker prose is not duplicated: {out}"
    );
}

#[test]
fn structured_type_mismatches_are_not_tied_to_an_tt_construct() {
    require_tsgo!();
    let dir = project(&[(
        "src/plain.tt",
        "const annotated: string = 1;\n\
         function takesString(value: string): void {}\n\
         takesString(2);\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("type mismatch: expected `string`, found `1`")
            && out.contains("type mismatch: expected `string`, found `2`"),
        "annotation and call argument use the same relation: {out}"
    );
}

#[test]
fn one_structured_cause_replaces_try_lowering_consequences() {
    require_tsgo!();
    let dir = project(&[(
        "src/try.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         const a = () => Result.Err(10);\n\
         function test(): TResult<string, string> {\n\
         \x20 const value = try a();\n\
         \x20 return value;\n\
         }\n",
    )]);
    let out = check(&dir);
    assert_eq!(
        out.matches("type mismatch:").count(),
        1,
        "one failed type obligation is one diagnostic: {out}"
    );
    assert!(
        out.contains("expected `string`, found `number`")
            && out.contains("required type: `TResult<string, string>`"),
        "the checker-proven incompatible types are reported: {out}"
    );
    assert!(
        !out.contains("`try` needs a Result") && !out.contains("no overlap"),
        "property and comparison consequences from lowering are suppressed: {out}"
    );
}

#[test]
fn a_precise_tt_error_owns_an_overlapping_type_consequence() {
    require_tsgo!();
    let dir = project(&[(
        "src/field.tt",
        "variant Shape { Circle(radius: number), Point }\n\
         export const radiusOf = (shape: Shape): number =>\n\
         \x20 match (shape) { Circle(radiuz) => radiuz, Point => 0 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("case `Circle` has no field `radiuz`") && !out.contains("type mismatch:"),
        "the direct tt cause replaces its broader checker consequence: {out}"
    );
}

#[test]
fn proven_statement_and_tuple_errors_own_only_their_checker_cascades() {
    require_tsgo!();
    let source = "variant PaymentMethod { Card(brand: string, last4: string), Cash }\n\
        variant Fulfillment { Pending, Picked, Cancelled }\n\
        export function card(method: PaymentMethod): string {\n\
        \x20 const Card(brand, last4) = method else { console.log(\"other\"); };\n\
        \x20 return brand + last4;\n\
        }\n\
        export function label(state: Fulfillment, method: PaymentMethod): string {\n\
        \x20 return match (state, method) {\n\
        \x20   (Picked, Card) => \"picked card\",\n\
        \x20   (Picked, Cash) => \"picked cash\",\n\
        \x20   (Picked) => \"picked\",\n\
        \x20   _ => \"other\",\n\
        \x20 };\n\
        }\n\
        const independent: string = 1;\n";
    let dir = project(&[("src/cascades.tt", source)]);

    let out = check(&dir);
    assert!(out.contains("error[let-else-not-diverging]"), "{out}");
    assert!(out.contains("error[match-tuple-arity]"), "{out}");
    assert!(out.contains("tuple pattern has 1 element"), "{out}");
    assert!(
        !out.contains("error[ts2339]") && !out.contains("error[ts2367]"),
        "checker consequences owned by the invalid constructs remain: {out}"
    );
    assert!(
        out.contains("type mismatch: expected `string`, found `1`"),
        "the independent source error must remain: {out}"
    );

    let answer = typed_server(&dir, "src/cascades.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "let-else-not-diverging"),
        "{answer}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "match-tuple-arity"),
        "{answer}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| { diagnostic["code"] != "ts2339" && diagnostic["code"] != "ts2367" })
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "ts2322"),
        "{answer}"
    );
}

#[test]
fn an_imported_field_error_is_identical_on_typed_cli_and_server_paths() {
    require_tsgo!();
    let source = "import { PaymentMethod } from \"./domain.tt\";\n\
        export function brand(method: PaymentMethod): string {\n\
        \x20 return match (method) { Card(brnad) => brnad, _ => \"n/a\" };\n\
        }\n";
    let dir = project(&[
        (
            "src/domain.tt",
            "export variant PaymentMethod { Card(brand: string, last4: string) }\n",
        ),
        ("src/payment.tt", source),
    ]);

    let out = check(&dir);
    assert!(
        out.contains("case `Card` has no field `brnad`")
            && out.contains("a field with a similar name exists: `brand`")
            && !out.contains("type mismatch:"),
        "the typed CLI reports the source cause only: {out}"
    );

    let answer = typed_server(&dir, "src/payment.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let field = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "unknown-field")
        .unwrap_or_else(|| panic!("missing imported field diagnostic: {answer}"));
    assert_eq!(source_slice(source, field), "brnad");
    assert_eq!(field["suggestions"][0]["edit"]["replacement"], "brand");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["code"] != "ts2339"),
        "the source error owns its generated consequence: {answer}"
    );
}

#[test]
fn a_nested_imported_field_error_uses_checker_evidence_and_source_span() {
    require_tsgo!();
    let source = "import type { TResult } from \"@tt/std\";\n\
        import { PaymentMethod } from \"./domain.tt\";\n\
        export function brand(r: TResult<PaymentMethod, string>): string {\n\
        \x20 return match (r) {\n\
        \x20   Ok(value: Card(brnd)) => brnd,\n\
        \x20   Ok(value) => \"other\",\n\
        \x20   Err(error) => \"error\",\n\
        \x20 };\n\
        }\n";
    let dir = project(&[
        (
            "src/domain.tt",
            "export variant PaymentMethod { Card(brand: string), Cash }\n",
        ),
        ("src/nested.tt", source),
    ]);

    let out = check(&dir);
    assert!(
        out.contains("error[ts2339]: Property 'brnd' does not exist"),
        "{out}"
    );
    assert!(out.contains("--> src/nested.tt:5:20"), "{out}");
    assert!(
        !out.contains("expected `{ brnd: any; }`") && !out.contains("type mismatch:"),
        "the direct property fact replaces the generated structural mismatch: {out}"
    );

    let answer = typed_server(&dir, "src/nested.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let field = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "ts2339")
        .unwrap_or_else(|| panic!("missing nested field diagnostic: {answer}"));
    assert_eq!(source_slice(source, field), "brnd");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["code"] != "unknown-field"),
        "{answer}"
    );
}

#[test]
fn deep_expression_try_is_accepted_on_typed_cli_and_server_paths() {
    require_tsgo!();
    let source = "import type { TResult } from \"@tt/std\";\n\
        import * as Result from \"@tt/std/result\";\n\
        declare function total(): TResult<number, string>;\n\
        export function amount(): TResult<number, string> {\n\
        \x20 return Result.Ok(Math.round(try total() * 1.1));\n\
        }\n";
    let dir = project(&[("src/deep-try.tt", source)]);

    let out = check(&dir);
    assert!(!out.contains("error["), "{out}");

    let answer = typed_server(&dir, "src/deep-try.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    assert!(diagnostics.is_empty(), "{answer}");
}

#[test]
fn a_pipeline_mismatch_names_the_step_that_rejects_the_value() {
    require_tsgo!();
    // The mismatch is at the second boundary, where the checker blames the
    // accumulated helper call — compiler glue. The per-step anchor re-homes
    // it onto the step that rejected the value instead of underlining the
    // whole pipeline (TASK-263).
    let dir = project(&[(
        "src/pipe.tt",
        "const inc = (n: number): number => n + 1;\n\
         const shout = (s: string): string => s.toUpperCase();\n\
         const a = 1 |> inc |> shout;\n",
    )]);
    let out = check(&dir);
    let step = block(&out, "ts2345");
    assert!(
        step.contains("this pipeline step expects `string`, but receives `number`"),
        "the boundary is said in pipeline vocabulary: {out}"
    );
    assert!(
        step.contains("--> src/pipe.tt:3:23"),
        "reported at the rejecting step, not over the whole pipeline: {out}"
    );
}

#[test]
fn each_failing_pipeline_boundary_gets_its_own_step_diagnostic() {
    require_tsgo!();
    let dir = project(&[(
        "src/chain.tt",
        "const inc = (n: number): number => n + 1;\n\
         const shout = (s: string): string => s.toUpperCase();\n\
         const g = 10\n\
         \x20 |> inc\n\
         \x20 |> shout\n\
         \x20 |> inc;\n",
    )]);
    let out = check(&dir);
    let first = block(&out, "src/chain.tt:5:6");
    assert!(
        first.contains("this pipeline step expects `string`, but receives `number`"),
        "the step rejecting the number is the first boundary: {out}"
    );
    let second = block(&out, "src/chain.tt:6:6");
    assert!(
        second.contains("this pipeline step expects `number`, but receives `string`"),
        "the step after the failed one reports its own boundary: {out}"
    );
}

#[test]
fn a_flow_mismatch_names_the_composed_step_and_the_boundary_types() {
    require_tsgo!();
    // A `flow` boundary mismatches as two function types; the diagnostic
    // descends to the value types of the boundary and keeps the complete
    // function obligation as context.
    let dir = project(&[(
        "src/flow.tt",
        "const inc = (n: number): number => n + 1;\n\
         const shout = (s: string): string => s.toUpperCase();\n\
         const label = flow |> inc |> inc |> shout;\n",
    )]);
    let out = check(&dir);
    let step = block(&out, "ts2345");
    assert!(
        step.contains("this pipeline step expects `string`, but receives `number`"),
        "the boundary's value types, not the whole function types: {out}"
    );
    assert!(
        step.contains("required type: `(n: number) => string`"),
        "the complete obligation remains visible: {out}"
    );
    assert!(
        step.contains("--> src/flow.tt:3:37"),
        "reported at the composed step that rejects the chain: {out}"
    );
}

#[test]
fn a_curried_combinator_chain_blames_the_step_with_the_wrong_argument() {
    require_tsgo!();
    // The report's original shape: std combinator steps whose error used to
    // underline the whole chain. `unwrapOrP(0)` fixes `T = number` while
    // the previous step produced `TOption<string>`.
    let dir = project(&[(
        "src/labels.tt",
        "import type { TOption } from \"@tt/std\";\n\
         import * as Option from \"@tt/std/option\";\n\
         declare function half(n: number): TOption<number>;\n\
         export function halfLabel(n: number): string {\n\
         \x20 return half(n)\n\
         \x20   |> Option.mapP((x: number) => String(x))\n\
         \x20   |> Option.unwrapOrP(0)\n\
         \x20   |> .toUpperCase();\n\
         }\n",
    )]);
    let out = check(&dir);
    let step = block(&out, "ts2345");
    assert!(
        step.contains("this pipeline step expects `number`, but receives `string`"),
        "the incompatible payloads, not the lowered object types: {out}"
    );
    assert!(
        step.contains("required type: `TOption<number>`"),
        "the step's complete obligation remains visible: {out}"
    );
    assert!(
        step.contains("|> Option.unwrapOrP(0)"),
        "the snippet shows the rejecting step's own line: {out}"
    );
    // The healthy step before it appears only as the producer label —
    // dashes, never the primary carets.
    let caret_rows = step.lines().filter(|line| line.contains('^')).count();
    assert_eq!(caret_rows, 1, "one primary underline: {out}");
    assert!(
        step.contains("--- the piped value is produced here"),
        "the producing step is labeled: {out}"
    );
}

#[test]
fn a_whole_pipeline_mismatch_keeps_the_generic_wording() {
    require_tsgo!();
    // Every boundary of this pipeline is fine; its *result* does not fit
    // the call it sits in. That diagnostic lands on the whole-pipeline
    // anchor (no producer context) and must not claim a step rejected
    // anything (PR #85 review).
    let dir = project(&[(
        "src/arg.tt",
        "const inc = (n: number): number => n + 1;\n\
         declare function takesString(s: string): void;\n\
         takesString(1 |> inc);\n",
    )]);
    let out = check(&dir);
    let mismatch = block(&out, "ts2345");
    assert!(
        mismatch.contains("type mismatch: expected `string`, found `number`"),
        "the pipeline's result is an ordinary mismatch: {out}"
    );
    assert!(
        !out.contains("this pipeline step"),
        "no step is blamed when no boundary failed: {out}"
    );
}

#[test]
fn a_pipeline_mismatch_labels_the_producing_step() {
    require_tsgo!();
    // Rust-style secondary span: the primary carets sit on the rejecting
    // step, and a `-` label points back at the step that produced the
    // value.
    let dir = project(&[(
        "src/pipe.tt",
        "const inc = (n: number): number => n + 1;\n\
         const shout = (s: string): string => s.toUpperCase();\n\
         const a = 1 |> inc |> shout;\n",
    )]);
    let out = check(&dir);
    let step = block(&out, "ts2345");
    assert!(
        step.contains("--- the piped value is produced here"),
        "the producer is labeled under the snippet: {out}"
    );
}

#[test]
fn a_checker_related_place_becomes_a_labeled_span() {
    require_tsgo!();
    // The checker's own related information — here the property whose
    // declared type the literal violates — is mapped back to `.tt`
    // coordinates and drawn as a label, the way rustc labels "expected
    // because of this".
    let dir = project(&[(
        "src/opts.tt",
        "type Opts = { name: string };\n\
         export const o: Opts = { name: 1 };\n",
    )]);
    let out = check(&dir);
    let mismatch = block(&out, "ts2322");
    assert!(
        mismatch.contains("---- The expected type comes from property 'name'"),
        "the declaration is labeled: {out}"
    );
    assert!(
        mismatch.contains("1 | type Opts = { name: string };"),
        "the labeled line is quoted in the same snippet: {out}"
    );
}

#[test]
fn the_server_carries_pipeline_labels() {
    require_tsgo!();
    let source = "const inc = (n: number): number => n + 1;\n\
        const shout = (s: string): string => s.toUpperCase();\n\
        const a = 1 |> inc |> shout;\n";
    let dir = project(&[("src/pipe.tt", source)]);
    let answer = typed_server(&dir, "src/pipe.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let mismatch = diagnostics
        .iter()
        .find(|d| d["code"] == "ts2345")
        .unwrap_or_else(|| panic!("no ts2345 diagnostic: {diagnostics:?}"));
    let labels = mismatch["labels"].as_array().unwrap_or_else(|| {
        panic!("the wire diagnostic carries its labels: {mismatch:?}");
    });
    assert_eq!(
        labels[0]["message"], "the piped value is produced here",
        "{labels:?}"
    );
    // 1-based coordinates, like the diagnostic itself: `inc` on line 3.
    assert_eq!(labels[0]["line"], 3, "{labels:?}");
}

#[test]
fn the_server_reports_a_pipeline_mismatch_over_the_step_text() {
    require_tsgo!();
    let source = "const inc = (n: number): number => n + 1;\n\
        const shout = (s: string): string => s.toUpperCase();\n\
        const a = 1 |> inc |> shout;\n";
    let dir = project(&[("src/pipe.tt", source)]);
    let answer = typed_server(&dir, "src/pipe.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let mismatch = diagnostics
        .iter()
        .find(|d| d["code"] == "ts2345")
        .unwrap_or_else(|| panic!("no ts2345 diagnostic: {diagnostics:?}"));
    assert_eq!(
        source_slice(source, mismatch),
        "shout",
        "the range covers exactly the rejecting step: {mismatch:?}"
    );
    assert!(
        mismatch["message"]
            .as_str()
            .unwrap()
            .contains("this pipeline step expects `string`, but receives `number`"),
        "the server carries the same wording as the CLI: {mismatch:?}"
    );
}
