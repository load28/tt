//! Every valid TypeScript file is a valid .tt file and must compile to
//! itself, byte for byte.

use ttc::{Options, SourceKind, compile};

fn assert_passthrough(src: &str) {
    let out = compile(src, &Options::default()).expect("compile failed");
    assert_eq!(out, src);
}

fn assert_tsx_passthrough(src: &str) {
    let out = compile(
        src,
        &Options {
            source_kind: SourceKind::Tsx,
            ..Options::default()
        },
    )
    .expect("compile failed");
    assert_eq!(out, src);
}

#[test]
fn valid_tsx_is_byte_identical() {
    assert_tsx_passthrough(
        r#"type Props<T> = { value: T; render(value: T): React.ReactNode };
const Item = <T,>({ value, render }: Props<T>) => (
  <article data-label="match (x) { A => 1 }">
    <header>{render(value)}</header>
    <>enum Result and result blocks are ordinary JSX text</>
  </article>
);
"#,
    );
}

#[test]
fn malformed_entity_text_inside_a_string_is_still_valid_tsx() {
    assert_tsx_passthrough("const text = \"<>&#;;\\\\w\";\n");
}

#[test]
fn string_prototype_match() {
    assert_passthrough("const m = \"abc\".match(/b/);\n");
}

#[test]
fn optional_chaining_match() {
    assert_passthrough("const m = s?.match(re) ?? [];\n");
}

#[test]
fn binding_named_match() {
    // `match` is not a reserved word, so it is an ordinary name — and a
    // `for…of` binding is `match`, a token, and a block, which is the
    // silhouette of a match expression (TASK-229). The corpus differential
    // found this one in this repository's own TypeScript.
    assert_passthrough(
        "declare const xs: string[];\nfor (const match of xs) {\n  console.log(match);\n}\n",
    );
    assert_passthrough(
        "declare const xs: string[];\nfor (const match of xs) {\n  xs.map((x) => x);\n}\n",
    );
}

#[test]
fn function_and_method_named_match_with_an_arrow_in_the_body() {
    // Without a return-type annotation, `match(x) { ... }` is `match`,
    // parens, and a block. What settles it is the block: a statement list,
    // not an arm list — and the arrow inside a call belongs to the call.
    assert_passthrough(
        "declare function f(g: (n: number) => number): number;\n\
         function match(x: number) { return f((y) => y + x); }\n",
    );
    assert_passthrough(
        "declare function f(g: (n: number) => number): number;\n\
         class C { match(x: number) { return f((y) => y + x); } }\n",
    );
    assert_passthrough(
        "declare function f(g: (n: number) => number): number;\n\
         const o = { match(x: number) { return f((y) => y + x); } };\n",
    );
    // A body that opens with a statement keyword is a statement list
    // whatever else it contains.
    assert_passthrough(
        "declare const xs: number[];\n\
         class C { match(x: number) { const f = (y: number) => y; return f(x); } }\n",
    );
    // A top-level arrow expression is itself a valid method-body statement.
    // Its `=>` is not evidence that the surrounding braces are tt match arms.
    assert_passthrough("class C { match(x: number) { (foo: number) => foo + x } }\n");
    assert_passthrough("const o = { match(x: number) { (foo: number) => foo + x } };\n");
    assert_passthrough(
        "const o = { nested: { match(x: number) { (foo: number) => foo + x } } };\n",
    );
    assert_passthrough(
        "declare const flag: boolean;\n\
         const o = flag ? {} : { match(x: number) { (foo: number) => foo + x } };\n",
    );
    assert_passthrough("function match(x: number) { (foo: number) => foo + x }\n");
    assert_passthrough(
        "namespace N {\n\
           export function match(x: number) { (foo: number) => foo + x }\n\
           export interface I { match(x: number): (foo: number) => number; }\n\
         }\n",
    );
    assert_passthrough(
        "interface I { match(x: number): (foo: number) => number; }\n\
         type T = { match(x: number): (foo: number) => number };\n",
    );
    assert_passthrough("export default { match(x: number) { (foo: number) => foo + x } };\n");
    assert_passthrough(
        "type T = unknown;\n\
         const o = <T>{ match(x: number) { (foo: number) => foo + x } };\n",
    );
    assert_passthrough(
        "declare function dec(value: unknown, context: unknown): void;\n\
         class C { @dec match(x: number) { (foo: number) => foo + x } }\n",
    );
}

#[test]
fn private_method_named_match_with_an_arrow_in_the_body() {
    assert_passthrough(
        "class C1 { #match(x: number) { (foo: number) => foo + x } }\n\
         class C2 { static #match(x: number) { (foo: number) => foo + x } }\n\
         class C3 { async #match(x: number) { (foo: number) => foo + x } }\n\
         class C4 { *#match(x: number) { (foo: number) => foo + x } }\n\
         class C5 { get #match() { (foo: number) => foo } }\n\
         class C6 { set #match(x: number) { (foo: number) => foo + x } }\n",
    );

    let source = "variant Choice { Yes, No }\n\
        class C { #match(value: Choice) { return match (value) { Yes => 1, No => 0 }; } }\n";
    let output = compile(source, &Options::default()).expect("compile failed");
    assert!(
        output.contains("class C { #match(value: Choice)"),
        "{output}"
    );
    assert_eq!(output.matches("switch (").count(), 1, "{output}");
}

#[test]
fn tt_match_inside_a_method_body_is_not_a_host_member_key() {
    let source = "variant Choice { Yes, No }\n\
        class C { choose(value: Choice) { return match (value) { Yes => 1, No => 0 }; } }\n";
    let output = compile(source, &Options::default()).expect("compile failed");
    assert!(output.contains("switch ("), "{output}");
    assert!(!output.contains("return match (value)"), "{output}");
}

#[test]
fn host_match_declarations_survive_beside_tt_syntax() {
    let source = "variant Choice { Yes, No }\n\
        class C { match(x: number) { (foo: number) => foo + x } }\n\
        class D { match(x) { Foo => x } }\n\
        function match(x: number) { (foo: number) => foo + x }\n\
        const value = match (Choice.Yes) { Yes => 1, No => 0 };\n";
    let output = compile(source, &Options::default()).expect("compile failed");
    assert!(
        output.contains("class C { match(x: number) { (foo: number) => foo + x } }"),
        "{output}"
    );
    assert!(
        output.contains("function match(x: number) { (foo: number) => foo + x }"),
        "{output}"
    );
    assert!(
        output.contains("class D { match(x) { Foo => x } }"),
        "{output}"
    );
    assert_eq!(output.matches("switch (").count(), 1, "{output}");
}

#[test]
fn host_match_method_body_may_contain_a_tt_match() {
    let source = "variant Choice { Yes, No }\n\
        class C { match(value: Choice) { return match (value) { Yes => 1, No => 0 }; } }\n\
        const o = { match(value: Choice) { return match (value) { Yes => 2, No => 3 }; } };\n";
    let output = compile(source, &Options::default()).expect("compile failed");
    assert!(
        output.contains("class C { match(value: Choice)"),
        "{output}"
    );
    assert!(
        output.contains("const o = { match(value: Choice)"),
        "{output}"
    );
    assert_eq!(output.matches("switch (").count(), 2, "{output}");
}

#[test]
fn host_match_ownership_is_found_in_a_nested_parser_region() {
    assert_passthrough("const rendered = `${({ match(value) { value => value } }).match(1)}`;\n");
}

#[test]
fn host_match_ownership_survives_statement_capability_boundaries() {
    let source = "variant Choice { Yes, No }\n\
        declare const choice: Choice;\n\
        while (true) { if let Yes() = choice { break; const o = { match(x: number) { (foo: number) => foo + x } }; } }\n\
        async function asyncOwner() { if let Yes() = choice { await Promise.resolve(); const o = { match(x: number) { (foo: number) => foo + x } }; } }\n\
        function* generatorOwner() { if let Yes() = choice { yield 1; const o = { match(x: number) { (foo: number) => foo + x } }; } }\n";
    let output = compile(source, &Options::default()).expect("compile failed");
    assert_eq!(output.matches("match(x: number)").count(), 3, "{output}");
}

#[test]
fn host_match_ownership_survives_labeled_statement_boundaries() {
    let source = "variant Choice { Yes, No }\n\
        declare const choice: Choice;\n\
        outer: while (true) { if let Yes() = choice { break outer; const a = { match(x: number) { (foo: number) => foo + x } }; } }\n\
        retry: while (true) { if let Yes() = choice { continue retry; const b = { match(x: number) { (foo: number) => foo + x } }; } }\n";
    let output = compile(source, &Options::default()).expect("compile failed");
    assert_eq!(output.matches("match(x: number)").count(), 2, "{output}");
}

#[test]
fn class_method_named_match() {
    assert_passthrough(
        r#"
class Router {
  match(pathname: string): boolean {
    return this.routes.some((r) => r.test(pathname));
  }
}
"#,
    );
}

#[test]
fn object_method_named_match() {
    assert_passthrough(
        r#"
const matcher = {
  match(s: string) { return s.length > 0; },
};
"#,
    );
}

#[test]
fn interface_member_named_match() {
    assert_passthrough(
        r#"
interface Matcher {
  match(s: string): boolean;
}
"#,
    );
}

#[test]
fn function_named_match() {
    assert_passthrough(
        r#"
function match(a: number, b: number) {
  return a === b;
}
const ok = match(1, 1);
"#,
    );
}

#[test]
fn variable_named_variant() {
    assert_passthrough(
        r#"
const variant = { kind: "a" };
console.log(variant.kind, variant);
"#,
    );
}

#[test]
fn variant_followed_by_a_line_break_is_an_expression_statement() {
    assert_passthrough(
        "declare let declare: number, variant: number, Foo: number, A: number;\n\
         variant\nFoo\n{ A }\n\
         variant /* a\n */ Foo\n{ A }\n\
         declare\nvariant\nFoo\n{ A }\n\
         export {};\n",
    );
}

#[test]
fn a_default_export_of_a_value_named_variant_passes_through() {
    assert_passthrough("declare let variant: number, Foo: number;\nexport default variant\nFoo\n");
    assert_passthrough("declare let variant: number;\nexport default variant;\n");
}

#[test]
fn declare_followed_by_a_line_break_does_not_declare_the_variant() {
    let out = compile(
        "declare let declare: number;\ndeclare\nvariant Foo { A }\n",
        &Options::default(),
    )
    .expect("compile failed");
    assert!(
        out.starts_with("declare let declare: number;\ndeclare\n"),
        "{out}"
    );
    assert!(out.contains("const Foo = {"), "{out}");
}

#[test]
fn match_inside_string() {
    assert_passthrough("const s = \"match (x) { A => 1 }\";\n");
}

#[test]
fn match_inside_comment() {
    assert_passthrough("// match (x) { A => 1 }\n/* match (y) { B => 2 } */\nconst z = 1;\n");
}

#[test]
fn match_inside_template_chunk() {
    assert_passthrough("const s = `match (x) { A => 1 } and ${1 + 2}`;\n");
}

#[test]
fn regex_containing_braces() {
    assert_passthrough("const re = /match \\(x\\) \\{.*\\}/g;\n");
}

#[test]
fn generics_and_arrows() {
    assert_passthrough(
        r#"
const pick = <T,>(xs: T[], i: number): T | undefined => xs[i];
type Fn = (a: string, b: number) => Map<string, Array<number>>;
"#,
    );
}

#[test]
fn match_property_key() {
    assert_passthrough("const cfg = { match: true, mode: \"all\" };\n");
}

#[test]
fn misc_async_code() {
    assert_passthrough(
        r#"
export async function main(): Promise<void> {
  const data = await fetch("/api").then((r) => r.json());
  switch (data.kind) {
    case "a": break;
    default: break;
  }
}
"#,
    );
}

#[test]
fn ts_numeric_enum() {
    assert_passthrough("enum Direction {\n  Up = 1,\n  Down,\n  Left,\n  Right,\n}\n");
}

#[test]
fn ts_string_enum() {
    assert_passthrough("enum Level {\n  Info = \"INFO\",\n  Warn = \"WARN\",\n}\n");
}

#[test]
fn ts_unit_only_enum() {
    assert_passthrough("enum Color { Red, Green, Blue }\n");
}

#[test]
fn ts_exported_unit_only_enum() {
    assert_passthrough("export enum Color { Red, Green, Blue }\n");
}

#[test]
fn ts_const_enum() {
    assert_passthrough("const enum Flags { None, Read, Write }\n");
}

#[test]
fn ts_declare_enum() {
    assert_passthrough("declare enum Ambient { A, B }\n");
}

#[test]
fn ts_computed_member_enum() {
    assert_passthrough(
        "enum FileAccess {\n  Read = 1 << 1,\n  Write = 1 << 2,\n  ReadWrite = Read | Write,\n}\n",
    );
}

#[test]
fn multibyte_content_preserved() {
    assert_passthrough("const 인사말 = \"안녕하세요 🎉\"; // 한글 주석과 match (x) { A => 1 }\n");
}

#[test]
fn plain_ts_using_option_result_names_is_untouched() {
    // The built-in Option/Result variants must never affect pure TypeScript: a
    // file that works with these names on its own (import, constructors, a
    // switch over the tags) contains no tt syntax and passes through.
    assert_passthrough(
        r#"
import { Option, Result } from "./tt.js";
const o = Option.Some(1);
switch (o.kind) {
  case "Some":
    break;
  case "None":
    break;
}
const r: Result<number, string> = Result.Err("nope");
"#,
    );
}

#[test]
fn bitwise_or_arguments_untouched() {
    // `|` in ordinary expression positions (including a method named
    // `match`) never becomes an or-pattern.
    assert_passthrough("const m = matcher.match(a | b);\nconst flags = READ | WRITE;\n");
}

#[test]
fn ts_try_catch_finally_block() {
    assert_passthrough(
        "try {\n  risky();\n} catch (e) {\n  handle(e);\n} finally {\n  done();\n}\n",
    );
}

#[test]
fn class_field_and_method_named_try() {
    assert_passthrough(
        "class Guard {\n  try = 5;\n  run() {\n    try {\n      this.try += 1;\n    } catch {}\n  }\n}\n",
    );
}

#[test]
fn interface_members_named_try() {
    // Signatures named `try` — including generic and annotation-free ones —
    // must never be taken for a tt try statement.
    assert_passthrough(
        "interface Retryable {\n  try(times: number): void;\n  try2?: () => void;\n}\ninterface Generic {\n  try<T>(x: T);\n}\n",
    );
}

#[test]
fn object_property_and_method_named_try() {
    assert_passthrough(
        "const machine = { try(x: number) { return x + 1; } };\nconst spec = { try: 1 };\nmachine.try(spec.try);\n",
    );
}

#[test]
fn plain_if_else_statement() {
    assert_passthrough(
        "function f(a: boolean): number {\n  if (a) {\n    return 1;\n  } else {\n    return 2;\n  }\n}\n",
    );
}

#[test]
fn function_named_some_called_after_const() {
    // `const Some = ...` has no pattern parens, so it is never a let-else.
    assert_passthrough("const Some = (x: number) => x + 1;\nconst y = Some(2);\n");
}

#[test]
fn object_method_named_const() {
    // A method *named* `const` is followed by `(`, not `<ident>(`.
    assert_passthrough("const machine = { const(x: number) { return x; } };\nmachine.const(1);\n");
}

#[test]
fn import_specifiers_without_tt_extension_untouched() {
    assert_passthrough(
        r#"
import { a } from "./mod.js";
import def from "../other";
import * as ns from "pkg";
export { b } from "./re.ts";
import "polyfill";
"#,
    );
}

#[test]
fn tt_specifier_in_string_comment_and_template_untouched() {
    assert_passthrough(
        "const s = \"import x from './a.tt'\";\n// import y from \"./b.tt\";\nconst t = `from \"./c.tt\"`;\n",
    );
}

#[test]
fn computed_dynamic_import_of_tt_path_is_untouched() {
    // A literal prefix is not the complete module specifier.
    assert_passthrough("const m = import(\"./x.tt\" + suffix);\n");
}

#[test]
fn tt_paths_outside_module_references_are_untouched() {
    for source in [
        "import alias = Namespace.member;\n",
        "const fs = require(\"./legacy.tt\");\n",
        "const m = import(`./${name}.tt`);\n",
        "module\n\"./token.tt\";\n",
        "const o = { module: \"./token.tt\" };\n",
        "/// <reference path=\"./token.tt\" />\nexport {};\n",
        "/** @type {import(\"./token.tt\").Token} */\nlet token;\n",
    ] {
        assert_passthrough(source);
    }
}

#[test]
fn export_declarations_are_not_reexports() {
    // `export` followed by a declaration must never be scanned for a
    // module specifier, even if a `from` + string appears later.
    assert_passthrough("export const from = 1;\nexport function f() { return \"./x.tt\"; }\n");
}

#[test]
fn bitwise_or_and_unions_are_not_pipelines() {
    assert_passthrough("const a = x | y;\nconst b = x || y;\nconst c = x | y > z;\n");
    assert_passthrough("type U = A | B;\nlet v: string | number = 1;\n");
    assert_passthrough("function f<T extends A | B>(x: T): T | null { return x; }\n");
}

#[test]
fn flow_is_an_ordinary_identifier_in_typescript() {
    // `flow` only means composition at a pipeline head, and a pipeline
    // needs a `|>` — which valid TypeScript cannot contain.
    assert_passthrough("import { flow } from \"fp-ts/function\";\nconst f = flow(g, h);\n");
    assert_passthrough("const flow = 1;\nconst a = flow | mask;\nconst b = o.flow;\n");
    assert_passthrough("function flow<T>(x: T): T { return x; }\ntype flow = number;\n");
}

#[test]
fn pipe_bytes_in_strings_comments_regexes_and_templates_pass_through() {
    assert_passthrough(
        "const s = \"a |> b\";\n// c |> d\n/* e |> f */\nconst r = /\\|>/;\nconst t = `g |> h`;\n",
    );
}

#[test]
fn match_shaped_calls_with_two_arguments_pass_through() {
    // A real function named `match` called with a comma list — no braces
    // follow, so it can never be claimed.
    assert_passthrough("const x = match(a, b);\nobj.match(a, (b, c));\n");
}

#[test]
fn if_statements_and_if_shaped_members_pass_through() {
    assert_passthrough("if (c) { a(); } else if (d) { b(); } else { e(); }\n");
    assert_passthrough("const o = { if: 1 };\ninterface I { if: number }\nobj.if(x);\n");
}

#[test]
fn result_is_an_ordinary_identifier_in_typescript() {
    // `result { ... }` is only claimed when the block carries a Result
    // binding (`const x <- ...;`), which valid TypeScript cannot contain.
    assert_passthrough("const result = compute();\nconsole.log(result);\n");
    assert_passthrough("class result { }\ninterface result { a: number }\ntype result = number;\n");
    assert_passthrough("const o = { result: 1 };\nobj.result(x);\nfoo(result, { a: 1 });\n");
}

#[test]
fn identifier_statement_followed_by_a_block_passes_through() {
    // The ASI shape `result` + newline + block statement is valid (dead)
    // TypeScript — without a binding inside, nothing is claimed.
    assert_passthrough("result\n{\n  const y = 2;\n  console.log(y);\n}\n");
    assert_passthrough("result\n{\n  const c = a < -b;\n}\n");
}

#[test]
fn a_keyword_less_binding_shape_is_only_claimed_where_typescript_cannot_reach() {
    // `b <- f();` is the comparison `b < -f();`, so the missing-keyword
    // diagnostic must not fire anywhere valid TypeScript can put a block
    // after the identifier `result`.
    assert_passthrough(
        "type result = { ok: boolean };\nfunction f(): result {\n  a <- readNum();\n  return { ok: true };\n}\n",
    );
    // `type X = result` + a block statement on the next line.
    assert_passthrough("type X = result\n{\n  a <- readNum();\n}\n");
    // The ASI shape, with a keyword-less run inside.
    assert_passthrough("result\n{\n  a <- readNum();\n}\n");
    assert_passthrough("class result {\n  a = 1;\n}\n");
}

#[test]
fn less_than_negation_passes_through() {
    assert_passthrough("const c = a < -b;\nif (x <-1) { f(); }\nwhile (i <-n) { g(); }\n");
    assert_passthrough("const d = result.a < -1;\nconst e = (a) < (-b);\n");
}

#[test]
fn negative_literal_type_arguments_pass_through() {
    // `let x: Foo<-1>;` is the one valid-TypeScript shape that puts `<-`
    // after a declaration keyword — its tail carries the generic's closing
    // `>`, which an expression cannot, so it is never a Result binding.
    assert_passthrough(
        "type result = { ok: boolean };\nfunction f(): result {\n  let x: Foo<-1>;\n  let y: Map<-1, string>, z: number;\n  return { ok: true };\n}\n",
    );
}

/* ------------------------------------------------------------------ */
/* literal patterns must not claim ordinary TypeScript                 */
/* ------------------------------------------------------------------ */

#[test]
fn switch_over_string_literals() {
    assert_passthrough(
        r#"
function short(dir: "north" | "south") {
  switch (dir) {
    case "north":
      return "N";
    case "south":
      return "S";
  }
}
"#,
    );
}

#[test]
fn call_named_match_followed_by_a_block() {
    assert_passthrough("match(x)\n{ 1 }\n");
}

#[test]
fn call_named_match_followed_by_a_block_of_any_content() {
    // A line break before the `{` ends the call statement, whatever the
    // block holds — arrows included, which read like arms.
    let prelude = "declare function match(x: unknown): void;\ndeclare const x: unknown;\n";
    for rest in [
        "match(x)\n{ _ => 1 }\n",
        "match(x)\n{ (_: unknown) => 1 }\n",
        "match (x)\n{ A => 1, B => 2 }\n",
        "match(x) /* a\n */ { _ => 1 }\n",
        "const v = match(x)\n{ _ => 1 };\n",
        "function f() {\n  return match(x)\n  { _ => 1 }\n}\n",
    ] {
        assert_passthrough(&format!("{prelude}{rest}"));
    }
}

#[test]
fn object_literal_with_numeric_and_string_keys() {
    assert_passthrough("const table = { 200: \"ok\", \"404\": \"missing\", true: 1 };\n");
}

#[test]
fn arrow_functions_returning_literals() {
    assert_passthrough("const f = (x: number) => 1;\nconst g = () => \"a\";\n");
}

#[test]
fn numeric_literals_of_every_form() {
    assert_passthrough(
        "const a = 0xff;\nconst b = 1_000;\nconst c = 1.5e2;\nconst d = 0b1010;\n\
         const e = 0o17;\nconst f = 10n;\nconst g = -1;\nconst h = .5;\n",
    );
}

#[test]
fn boolean_literals_in_ordinary_positions() {
    assert_passthrough("const t = true;\nconst f = false;\nconst u = { a: true };\n");
}

/* ---- `val` as an ordinary identifier ---- */

#[test]
fn variable_named_val() {
    assert_passthrough("const val = { a: 1 };\nval.a = 2;\nconst n = val.a + 1;\n");
}

#[test]
fn property_and_parameter_named_val() {
    assert_passthrough("const o = { val: 1 };\no.val = 2;\n");
    assert_passthrough("function f(val: number) { return val + 1; }\n");
    assert_passthrough("const g = (val: string) => val.length;\n");
    assert_passthrough("interface I { val: string }\ntype T = { val?: number };\n");
    assert_passthrough("class C { val = 1; getVal() { return this.val; } }\n");
}

#[test]
fn val_followed_by_a_declaration_on_the_next_line() {
    // Two statements separated by ASI: an expression statement naming the
    // variable `val`, then a declaration. `val` only modifies what follows
    // it on the same line, so this keeps its meaning.
    assert_passthrough("let x = 0;\nx = val\nconst y = 1;\n");
    assert_passthrough("val\nconst y = 1;\n");
    assert_passthrough("val;\nconst y = 1;\n");
}

#[test]
fn val_in_front_of_an_operator_word() {
    assert_passthrough("const u = (val as User);\nconst v = (val satisfies User);\n");
    assert_passthrough("for (val of items) { log(val); }\n");
    assert_passthrough("if (val in obj) { log(1); }\nif (val instanceof C) { log(2); }\n");
}

#[test]
fn val_as_a_call_argument_or_element() {
    assert_passthrough("f(val, other);\nconst xs = [val, other];\n");
    assert_passthrough("const m = new Map([[val, 1]]);\n");
    assert_passthrough("arr.reduce((acc, val) => acc + val, 0);\n");
}

#[test]
fn non_ascii_identifiers_ending_in_a_tt_keyword() {
    assert_passthrough(
        "const étry = (n: number) => n;\nconsole.log(étry(2));\nconst 名try = (n: number) => n;\n名try(1);\nconst x = { étry: (n: number) => n };\nx.étry(1);\nfunction g() {\n  return étry(3);\n}\n",
    );
    assert_passthrough(
        "declare function f(...a: unknown[]): number;\nconst ématch = (n: number) => ({ n });\nconst m = ématch (1)\n{ }\nconst éval = [1];\nconst w = f(1, éval [0]);\nconst éflow = [1];\nconst q = f(1, éflow [0]);\n",
    );
    assert_passthrough(
        "const évariant = 1;\nconst v = évariant\nlet Foo = 2;\nconst éresult = 1;\nlet r = éresult\n{ }\nconst éelse = 1;\n",
    );
}

#[test]
fn non_ascii_white_space_still_separates_words() {
    assert_passthrough("const\u{00A0}a = 1;\nlet\u{3000}b = a;\u{2028}const c = b;\n");
}

#[test]
fn val_element_access_in_arguments_and_elements() {
    assert_passthrough(
        "const val = [5];\nconsole.log(val [0]);\nconst arr = [1, val [0]];\ng(1, val [1]);\n",
    );
    assert_passthrough("h(val [0], val [1]);\nnew C(val [0]);\nconst p = (val [0]);\n");
    assert_passthrough("const t = c ? f(val [0]) : { a: 1 };\n");
    assert_passthrough("const u = c ? f(val [0]) : w => w;\n");
    assert_passthrough("function k(a = g(1, val [0])) { return a; }\n");
    assert_passthrough("if (val [0]) { log(1); }\nwhile (x, val [0]) { break; }\n");
}

#[test]
fn val_element_access_never_becomes_a_parameter_by_its_surroundings() {
    assert_passthrough("const v = c ? (val [0]) : w => w;\n");
    assert_passthrough("const v = c ? (val [0]) : (w: number): number => w;\n");
    assert_passthrough("f(val [0])\n{\n  log(1);\n}\n");
    assert_passthrough("g(1, val [0])\n{ }\n");
    assert_passthrough("type T = [val [number]];\nlet t: (val [number]) | undefined;\n");
}

#[test]
fn val_decorators_stay_decorators() {
    assert_passthrough(
        "class D {\n  @val x = 1;\n  @val [k]() {}\n  constructor(@val y: number, @val private z: number) {}\n}\n",
    );
}

#[test]
fn untyped_try_method_signatures_remain_host_members() {
    assert_passthrough("interface X { try(x); }\ntype Y = { try(x); };\n");
}

/// A `/` or `<` after a statement the grammar has completed begins the next
/// statement's operand (TASK-494).
const FINISHED_STATEMENTS: &[&str] = &[
    "if (1) a;\n",
    "if (1) a; else b;\n",
    "if (1) {}\n",
    "while (0) a;\n",
    "for (;;) {}\n",
    "L: {}\n",
    "function f() {}\n",
    "class C {}\n",
    "interface I {}\n",
    "enum E {}\n",
    "namespace N {}\n",
    "try {} catch {}\n",
    "switch (1) {}\n",
    "export default function () {}\n",
    "let c: number\n",
    "type T = number\n",
    "import \"a\"\n",
    "do {} while (0) ",
    "{ a; } ",
];

#[test]
fn a_regex_or_element_after_a_finished_statement_passes_through() {
    for prefix in FINISHED_STATEMENTS {
        assert_passthrough(&format!(
            "declare const a: any, b: any;\n{prefix}/ a /.test(\"\") / 2;\n"
        ));
        assert_tsx_passthrough(&format!(
            "declare const a: any, b: any;\n{prefix}<b>/ a /</b>;\n"
        ));
    }
    assert_passthrough("function* g() {\n  yield\n  / a /.test(\"\");\n}\n");
    assert_passthrough("L: for (;;) {\n  break L\n  / a /.test(\"\");\n}\n");
    assert_passthrough("const f = function () {}\n/ 2 / 1;\n");
}

#[test]
fn type_arguments_with_commas_pass_through() {
    assert_passthrough(
        "declare function f<A, B>(v: unknown): unknown;\ntype A = 1;\ntype B = 2;\nconst m = new Map<A, B>();\nconst r = f<A, B>(m), s = f<B, A>;\nlet t = (f<A, Map<A, B>>(r), s);\n",
    );
}

#[test]
fn an_automatic_semicolon_ends_a_block_bodied_arrow_function_before_an_operator_line() {
    for arrow in [
        "export const f = () => {}",
        "export const f = async () => {}",
        "export const f = (): void => {}",
        "export const f = <T,>(a: T) => {}",
        "declare let x: unknown; x = () => {}",
    ] {
        for line in [
            "/x/g.exec(\"x\")",
            "/=x/.test(\"=x\")",
            "+1",
            "-1",
            "(1)",
            "[1]",
            "`t`",
        ] {
            let source = format!("{arrow}\n{line}\n");
            assert_passthrough(&source);
            assert_tsx_passthrough(&source);
        }
    }
}

#[test]
fn an_arrow_function_with_a_parenthesized_return_type_passes_through() {
    for ty in [
        "(A | B)",
        "(void)",
        "(() => void)",
        "(\"a\" | \"b\")",
        "(A[])",
        "(typeof x)",
        "(keyof A)",
        "(readonly A[])",
        "([\"a\"])",
        "(a: A) => void",
        "({ a }: A) => void",
        "([p, q = 1]: A[]) => void",
        "(this: A) => void",
    ] {
        let source = format!(
            "type A = {{ a: 1 }};\ntype B = 2;\ndeclare const x: number;\nexport const f = (): {ty} => {{ return null as any }}\n/x/g.exec(\"x\")\n"
        );
        assert_passthrough(&source);
        assert_tsx_passthrough(&source);
    }
}

#[test]
fn a_line_after_an_import_equals_declaration_passes_through() {
    for declaration in [
        "import fs = require(\"fs\")\n",
        "import type R = require(\"fs\")\n",
        "export import F = require(\"fs\")\n",
        "import A = B.C\n",
        "export import D = B.\n  C\n",
        "import E = B\n",
    ] {
        for line in [
            "/x/g.exec(\"x\")",
            "[1].forEach(n => n)",
            "(1)",
            "-1",
            "`t`",
        ] {
            let source = format!(
                "namespace B {{ export namespace C {{ export const q = 1 }} }}\n{declaration}{line}\n"
            );
            assert_passthrough(&source);
            assert_tsx_passthrough(&source);
        }
    }
}

#[test]
fn contextual_type_and_statement_words_pass_through() {
    for head in [
        "type asserts = number;\nexport let a: asserts\n",
        "type abstract = number;\nexport type A = abstract\n",
        "declare let namespace: any;\nnamespace instanceof Object;\n",
        "declare let module: any;\nmodule in Object;\n",
        "declare let declare: any;\ndeclare as any;\n",
        "export function g(v: unknown): asserts v is string {}\n",
        "export type C = abstract new () => object\n",
        "export type G = <T>(x: T) => T\n",
    ] {
        for line in ["/x/g.exec(\"x\")", "[1].forEach(n => n)", "(1)", "`t`"] {
            let source = format!("{head}{line}\n");
            assert_passthrough(&source);
            assert_tsx_passthrough(&source);
        }
    }
}

#[test]
fn a_line_after_an_import_type_is_not_its_type_arguments() {
    for source in [
        "declare const y: any;\nlet x: typeof import(\"x\")\n<any>y\n",
        "declare const y: any;\nlet x: import(\"x\")\n<any>y\n",
        "declare const y: any;\nlet x: import(\"x\").A\n<any>y\n",
        "let x: import(\"x\").A<any>;\n",
        "let x: typeof import(\"x\")<any>;\n",
        "let x: import(\"x\")<<T>() => T>;\n",
    ] {
        assert_passthrough(source);
    }
    for source in [
        "let x: typeof import(\"x\")\n<b>hi</b>\n",
        "let x: import(\"x\").A\n<b>hi</b>\n",
        "let x: import(\"x\").A<any>;\n",
    ] {
        assert_tsx_passthrough(source);
    }
}

#[test]
fn a_using_declaration_in_a_for_statement_and_a_function_as_an_if_clause() {
    for source in [
        "declare function res(): { [Symbol.dispose](): void };\nexport function f() {\n  for (using q = res(); ; ) { break; }\n  for (using q = res(), p = res(); q; ) { break; }\n}\n",
        "declare function res(): { [Symbol.asyncDispose](): Promise<void> };\nexport async function f() {\n  for (await using q = res(); ;) { break; }\n}\n",
        "if (Math.random()) function f() {}\nif (Math.random()) {} else function g() {}\n",
    ] {
        assert_passthrough(source);
    }
}
