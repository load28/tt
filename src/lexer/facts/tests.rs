//! The facts machine against its oracle: for TypeScript, the statement
//! spans the machine recognizes are the statement spans SWC parses, and
//! the lexer reads a regular expression or a JSX element exactly where SWC
//! parses one — every other `/` is division and every other `<` an
//! operator or a type bracket.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use swc_common::Spanned;
use swc_ecma_ast::{Expr, ModuleDecl, Regex, Stmt};
use swc_ecma_visit::{Visit, VisitWith};

use crate::SourceKind;
use crate::host_input::HostInput;
use crate::lexer::{Token, TokenKind, lex_with_kind, trace};

/// One reading of a text: statement spans, and the starts of regular
/// expression literals and of JSX elements in expression position.
#[derive(Default, PartialEq, Eq)]
struct Reading {
    statements: BTreeSet<(usize, usize)>,
    regexes: BTreeSet<usize>,
    elements: BTreeSet<usize>,
}

struct Swc<'a> {
    input: &'a HostInput,
    reading: Reading,
}

impl Visit for Swc<'_> {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        let span = stmt.span();
        self.reading
            .statements
            .insert((self.input.byte(span.lo), self.input.byte(span.hi)));
        stmt.visit_children_with(self);
    }

    fn visit_module_decl(&mut self, decl: &ModuleDecl) {
        let span = decl.span();
        self.reading
            .statements
            .insert((self.input.byte(span.lo), self.input.byte(span.hi)));
        decl.visit_children_with(self);
    }

    fn visit_regex(&mut self, regex: &Regex) {
        self.reading.regexes.insert(self.input.byte(regex.span.lo));
    }

    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::JSXElement(_) | Expr::JSXFragment(_) = expr {
            self.reading
                .elements
                .insert(self.input.byte(expr.span().lo));
        }
        expr.visit_children_with(self);
    }
}

/// SWC's reading, or `None` when SWC does not parse the text.
fn swc_reading(src: &str, kind: SourceKind) -> Option<Reading> {
    let input = HostInput::new(src);
    let mut parser = input.parser(kind);
    let module = parser.parse_module().ok()?;
    if !parser.take_errors().is_empty() {
        return None;
    }
    let mut visitor = Swc {
        input: &input,
        reading: Reading::default(),
    };
    module.visit_with(&mut visitor);
    Some(visitor.reading)
}

fn machine_reading(src: &str, kind: SourceKind) -> Reading {
    let trace = trace(src, kind);
    Reading {
        statements: trace
            .statements
            .into_iter()
            .map(|span| (span.start, span.end))
            .collect(),
        regexes: trace.regexes.into_iter().collect(),
        elements: trace.elements.into_iter().collect(),
    }
}

/// The differences between the two readings of `src`, rendered, or `None`
/// when they agree or SWC rejects the text.
fn disagreement(src: &str, kind: SourceKind) -> Option<String> {
    let expected = swc_reading(src, kind)?;
    let found = machine_reading(src, kind);
    if expected == found {
        return None;
    }
    let show = |start: usize, end: usize| {
        let text = src.get(start..end).unwrap_or("<not a char boundary>");
        let text: String = text.chars().take(80).collect();
        format!("{start}..{end} {text:?}")
    };
    let mut out = String::new();
    let mut differ = |what: &str, swc: Vec<(usize, usize)>, machine: Vec<(usize, usize)>| {
        for (start, end) in swc.into_iter().take(6) {
            out.push_str(&format!("  {what} swc only:     {}\n", show(start, end)));
        }
        for (start, end) in machine.into_iter().take(6) {
            out.push_str(&format!("  {what} machine only: {}\n", show(start, end)));
        }
    };
    let line = |start: usize| {
        (
            start,
            src[start..].find('\n').map_or(src.len(), |n| start + n),
        )
    };
    differ(
        "statement",
        expected
            .statements
            .difference(&found.statements)
            .copied()
            .collect(),
        found
            .statements
            .difference(&expected.statements)
            .copied()
            .collect(),
    );
    differ(
        "regex",
        expected
            .regexes
            .difference(&found.regexes)
            .map(|&at| line(at))
            .collect(),
        found
            .regexes
            .difference(&expected.regexes)
            .map(|&at| line(at))
            .collect(),
    );
    differ(
        "jsx",
        expected
            .elements
            .difference(&found.elements)
            .map(|&at| line(at))
            .collect(),
        found
            .elements
            .difference(&expected.elements)
            .map(|&at| line(at))
            .collect(),
    );
    Some(out)
}

fn assert_agrees(src: &str, kind: SourceKind) {
    assert!(
        swc_reading(src, kind).is_some(),
        "SWC rejects the case, so it checks nothing:\n{src}"
    );
    if let Some(diff) = disagreement(src, kind) {
        panic!("the readings differ for:\n{src}\n{diff}");
    }
}

/// Shapes the old token-list predicates misread: a type ending in `>` or
/// `void` hid the line break after it, contextual keywords used as names,
/// and line terminators other than LF.
const KNOWN: &[&str] = &[
    "declare const o: unknown\nconst a = o as Array<number>\nconsole.log(a)\n",
    "let f: () => void\nf = () => {}\n",
    "const f = () => {}\n/x/g.exec(\"x\")\nconst g = async (): Promise<void> => {}\n-1\n",
    "declare function async(x: number): (y: number) => void\nasync(1)\n(2)\nasync (y: number): Promise<void> => {}\n",
    "let p: Promise<void>\n(p)\n",
    "type K = string\ndeclare const x: object\nx satisfies Record<K, unknown>\nthrow x\n",
    "const of = 1\nconst async = 2\nconst let_ = of\nasync\nlet_\n",
    "const x = { of: 1, let: 2, await: 3, yield: 4, async: 5 }\nx.of\nx.await\nx.yield\n",
    "const a = 1\rconst b = 2\u{2028}const c = 3\u{2029}const d = 4\r\nconst e = 5\n",
    "let a = 1 /* multi\nline */ let b = 2\n",
    "declare let q: number\nq++\nq--\nconst g = [1] as const\n",
    "function h() {\n  return\n  1\n}\nfunction* y() {\n  yield\n  1\n}\n",
    "outer: for (;;) {\n  break outer\n  continue\n}\n",
    "const m = new Map<string, number>()\nconst n = m.get('a')\n",
    "declare function g<T>(x: T): T\nconst r = g<number>(1)\nconst s = r < 2\n",
    "class A<T> extends Array<T> implements Iterable<T> {\n  x: number = 1\n  y?: string\n  static z = 2\n  get w() { return 1 }\n  m<U>(u: U): U { return u }\n  [Symbol.iterator]() { return super[Symbol.iterator]() }\n}\n",
    "interface I<T> extends Array<T> {\n  a: T\n  b(): void\n  readonly [k: string]: unknown\n  new (x: number): I<T>\n}\n",
    "type M<T> = { [K in keyof T]?: T[K] extends string ? K : never }\ntype C = T extends [infer U, ...infer R] ? U : never\n",
    "const f2 = (a: number, b?: string): number => a\nconst f3 = async <T,>(x: T) => x\n",
    "if (a) b\nelse c\ndo d()\nwhile (e)\nf()\n",
    "switch (x) {\n  case 1:\n  case 2: y()\n    break\n  default: {\n    z()\n  }\n}\n",
    "try { a() } catch { b() } finally { c() }\ntry { a() } catch (e: unknown) { b() }\n",
    "export default class {}\nexport const x = 1\nexport { x as y }\nexport * from './a'\nimport z, { w } from './b'\nimport type { T } from './c'\n",
    "declare module 'm' {\n  export const a: number\n}\ndeclare global {\n  interface Window { x: number }\n}\nnamespace N.M { export const b = 1 }\n",
    "enum E { A = 1, B, C = A | B }\nconst enum F { X }\n",
    "@dec class D {\n  @dec() m() {}\n  constructor(private readonly p: number, @inject q: string) {}\n}\n",
    "const re = /ab+c/g.test('abc')\nconst div = 4 / 2 / 1\nconst t = `a${1 / 2}b${/x/.source}`\n",
    "for (const [k, v] of Object.entries({})) console.log(k, v)\nfor (let i = 0; i < 2; i++) {}\nfor (const k in {}) {}\n",
    "label: {\n  break label\n}\nlet u = a\n(b)\nlet v = a\n[0]\n",
    "const o2 = {\n  get x() { return 1 },\n  set x(v) {},\n  async *gen() {},\n  [k]: 1,\n  ...rest,\n}\n",
    "x = y\n++z\nw = y\n!v\n",
    "abstract class Q { abstract m(): void\n  protected abstract n: number }\n",
    "function over(a: string): void\nfunction over(a: number): void\nfunction over(a: unknown) {}\n",
    "const cond = a ? (b) : c\nconst arrow = (b): number => b\n",
    "let bigint = 10n\nlet hex = 0xFF\nlet exp = 1e-3\nlet frac = .5\nlet sep = 1_000\n",
    "const obj = { a: 1 }\n[1, 2].forEach(n => n)\n",
    "class K { x = 1\n  [k] = 2\n  y = 3\n  z\n  w() {}\n}\n",
    "let a1 = b\n++c\nlet a2 = b\n/re/g.exec(c)\nlet a3 = b\n`t`\n",
    "const f4 = function () {}\n(x)\nconst f5 = () => {}\n(x)\nconst f6 = () => 1\n(x)\n",
    "const g1 = async function* () { yield* g2(); await 1 }\nconst c1 = class extends Base {}\n",
    "if (a) q()\nwhile (b) c: d()\n",
    "for await (const x of xs) {}\nfor (;;) break\nfor (x.y of z);\n",
    "let n1 = a ? b : c\nlet n2 = a?.b\nlet n3 = a ?? b\nlet n4 = a?.[0]\nlet n5 = a?.(1)\nlet n6 = x ?.5 : 1\n",
    "type F = (a: string) => void\ntype G = new (...args: any[]) => object\ntype H = typeof import('./x')\ntype L = `a${string}`\n",
    "function assertIsString(v: unknown): asserts v is string {}\nfunction isS(v: unknown): v is string { return true }\n",
    "let tup: [a: string, b?: number, ...rest: boolean[]] = ['']\nlet idx: T['k'][]\nlet u: | A | B\n",
    "export default function () {}\nexport default abstract class {}\n",
    "import json from './x.json' with { type: 'json' }\nexport = foo\nimport fs = require('fs')\nexport as namespace NS\n",
    "declare function df(): void\ndeclare const dc: number\ndeclare namespace dn { const x: number }\n",
    "const nested = a ? b ? c : d : e\nconst o3 = { a: b ? c : d, e }\n",
    "var v1 = x\n-1\nvar v2 = x\n+1\nvar v3 = x\n!y\n",
    "yield1()\nfunction* g3() { const s = yield\n  s }\n",
    "type A = number\nconst r1 = (): (A | undefined) => {}\n/x/g.exec(\"x\")\nconst r2 = (): (() => void) => { return () => {} }\n/x/g.exec(\"x\")\nconst r3 = (): (typeof r1) => r1\nconst r4 = (): (a: A) => void => { return () => {} }\n/x/g.exec(\"x\")\nconst r5 = (): ([\"a\"]) => { return [\"a\"] }\n-1\nconst r6 = (): ({ a }: { a: A }) => void => { return () => {} }\n/x/g.exec(\"x\")\n",
    "namespace B { export namespace C {} }\nimport A = B.C\n/x/g.exec(\"x\")\nimport fs = require(\"fs\")\n[1].forEach(n => n)\nexport import D = B.\n  C\n(1)\nimport type R = require(\"fs\")\n-1\nimport E = B\n`t`\n",
    "type asserts = number\nlet a1: asserts\n/x/.test(\"\")\ntype abstract = number\ntype A2 = abstract\n/x/.test(\"\")\ntype N = abstract new () => object\n/x/.test(\"\")\nfunction f1(x: unknown): asserts x {}\n/x/.test(\"\")\nfunction f2(this: unknown): asserts this {}\n-1\n",
    "let ng: new <T>(x: T) => T\n/x/.test(\"\")\ntype G = <T>(x: T) => T\n/x/.test(\"\")\ntype U = unique symbol\ntype K = keyof typeof globalThis\n/x/.test(\"\")\ntype Inf<T> = T extends Array<infer U extends string> ? U : never\n/x/.test(\"\")\ntype V<in I, out O> = (i: I) => O\n/x/.test(\"\")\n",
    "declare let namespace: any, module: any, declare: any\nnamespace instanceof Object;\nmodule instanceof Object;\nnamespace in Object;\ndeclare instanceof Object;\ndeclare in Object;\ndeclare as any;\n",
    "class C1 { declare\n x: number\n static\n y = 1\n accessor\n z = 2\n readonly\n w = 3\n get\n v() { return 1 }\n}\n/x/.test(\"\")\ntype L = { readonly\n x: number\n get\n y(): number }\n/x/.test(\"\")\nclass C2 { constructor(readonly\n p: number) {} }\n/x/.test(\"\")\n",
    "let get = 1, set = 2, of = 3, type = 4, declare = 5, abstract = 6, module = 7, namespace = 8\ntype\nFoo\ndeclare\nfoo\n",
];

#[test]
fn the_machine_reads_known_shapes_as_swc_does() {
    for case in KNOWN {
        assert_agrees(case, SourceKind::TypeScript);
    }
}

/// Malformed text — every prefix and suffix of the known shapes, and
/// tokens no frame expects — always makes progress (the machine asserts
/// it in debug builds) and yields one fact set per token.
#[test]
fn the_machine_makes_progress_on_malformed_text() {
    let garbage = [
        "for (;; , :) }",
        "f<@#, => ?.>(x)",
        ") ] } , : ;",
        "a<b<c",
        "if let",
        "let x: A<@ => # || ?? |> ~>;",
    ];
    for case in KNOWN.iter().chain(garbage.iter()) {
        for (at, _) in case.char_indices() {
            for piece in [&case[..at], &case[at..]] {
                for kind in [SourceKind::TypeScript, SourceKind::Tsx] {
                    let tokens = lex_with_kind(piece, 0, piece.len(), kind);
                    assert!(tokens.iter().all(|token| token.span.end <= piece.len()));
                }
            }
        }
    }
}

/// Statements the grammar has completed before the next token: a `/` or
/// `<` after one begins the next statement's operand.
const FINISHED: &[&str] = &[
    "if (1) a;\n",
    "if (1) ;\n",
    "if (1) a; else b;\n",
    "if (1) {}\n",
    "if (1) {} else {}\n",
    "while (0) a;\n",
    "for (;;) a;\n",
    "for (;;) {}\n",
    "for (const k of a) a;\n",
    "L: a;\n",
    "L: {}\n",
    "function f() {}\n",
    "function* g() {}\n",
    "async function h() {}\n",
    "class C {}\n",
    "abstract class K {}\n",
    "interface I {}\n",
    "enum E {}\n",
    "namespace N {}\n",
    "declare module \"m\" {}\n",
    "declare global {}\n",
    "try {} catch {}\n",
    "try {} finally {}\n",
    "switch (1) {}\n",
    "export default class {}\n",
    "export default function () {}\n",
    "export function x() {}\n",
    "export class X {}\n",
    "export {}\n",
    "export * from \"a\"\n",
    "let c: number\n",
    "type T = number\n",
    "declare const d: number\n",
    "declare function df(): void\n",
    "import \"a\"\n",
    "do {} while (0) ",
    "{ a; } ",
    "function* gy() {\n  yield\n  ",
    "L1: for (;;) {\n  break L1\n  ",
    "L2: for (;;) {\n  continue L2\n  ",
    "for (;;) {\n  break\n  ",
    "for (;;) {\n  continue\n  ",
    "function db() {\n  debugger\n  ",
    "function r(s: string) {\n  if (!s) return;\n  ",
];

/// The same text after each finished statement: its close, when the
/// statement opened a body that is still open.
fn after_finished(prefix: &str, operand: &str) -> String {
    let close = if prefix.ends_with("  ") {
        "\n}\n"
    } else {
        "\n"
    };
    format!("declare const a: any, b: any\n{prefix}{operand}{close}")
}

#[test]
fn an_operand_begins_after_a_finished_statement() {
    for prefix in FINISHED {
        assert_agrees(
            &after_finished(prefix, "/ a /.test(\"\") / 2"),
            SourceKind::TypeScript,
        );
        assert_agrees(&after_finished(prefix, "<b>/ a /</b>"), SourceKind::Tsx);
    }
    for (src, kind) in [
        (
            "const f = function () {}\n/ 2 / 1\n",
            SourceKind::TypeScript,
        ),
        ("const c = class {}\n/ 2 / 1\n", SourceKind::TypeScript),
        (
            "function* y() {\n  yield / a /\n}\n",
            SourceKind::TypeScript,
        ),
        (
            "function* y() {\n  const f = function () { return yield1 }\n  yield\n  (a)\n}\nconst yield1 = 1\n",
            SourceKind::TypeScript,
        ),
        ("let t: Array<number>\n<b>x</b>\n", SourceKind::Tsx),
    ] {
        assert_agrees(src, kind);
    }
}

#[test]
fn the_machine_reads_jsx_containers_as_swc_does() {
    for case in [
        "const a = <div onClick={() => {\n  f()\n  g()\n}}>{x}</div>\nfoo()\n",
        "const id = <T,>(x: T) => x\nconst b = <>{[1].map(n => <i key={n}>{n / 2}</i>)}</>\nbar()\n",
        "function C() {\n  return <p>{`${a}`}</p>\n}\n",
    ] {
        assert_agrees(case, SourceKind::Tsx);
    }
}

#[test]
fn line_breaks_are_every_ecma_line_terminator() {
    let src = "a\rb\u{2028}c\u{2029}d\r\ne /* \u{2028} */ f // x\rg";
    let tokens = lex_with_kind(src, 0, src.len(), SourceKind::TypeScript);
    let breaks: Vec<bool> = tokens
        .iter()
        .map(|token| token.facts.line_break_before())
        .collect();
    assert_eq!(breaks, [false, true, true, true, true, true, true]);
}

fn word_facts<'a>(src: &'a str, tokens: &'a [Token]) -> Vec<(&'a str, super::TokenFacts)> {
    tokens
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Ident))
        .map(|token| (&src[token.span.start..token.span.end], token.facts))
        .collect()
}

#[test]
fn statement_starts_and_automatic_semicolons_are_recorded_per_token() {
    let src = "let p: Promise<void>\nif (p) q\nelse r\nlabel: s\n";
    let tokens = lex_with_kind(src, 0, src.len(), SourceKind::TypeScript);
    let words = word_facts(src, &tokens);
    let find = |word: &str| words.iter().find(|(w, _)| *w == word).unwrap().1;
    assert!(find("let").statement_start());
    assert!(find("if").statement_start() && find("if").asi_before());
    assert!(find("q").statement_start() && !find("q").asi_before());
    assert!(find("else").asi_before() && !find("else").statement_start());
    assert!(find("label").statement_start() && find("label").label());
    assert!(find("void").ends_expression());
}

/// Each `{` of the source, with the kind of function body it opens.
fn braces(src: &str) -> Vec<&'static str> {
    lex_with_kind(src, 0, src.len(), SourceKind::TypeScript)
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Punct(b'{')))
        .map(|token| match token.facts {
            facts if facts.constructor_body() => "constructor",
            facts if facts.generator_body() => "generator",
            facts if facts.function_body() => "function",
            _ => "-",
        })
        .collect()
}

#[test]
fn a_brace_records_the_function_body_it_opens() {
    assert_eq!(
        braces("function* g(): Iterator<number> {}\nif (x) {}\nconst f = (a): void => {}\n"),
        ["generator", "-", "function"]
    );
    assert_eq!(
        braces(
            "class C extends mix(function () {}) {\n  constructor() {}\n  *m() {}\n  static async *n() {}\n  get x(): { a: number } { return { a: 1 } }\n  static {}\n}\n"
        ),
        [
            "function",
            "-",
            "constructor",
            "generator",
            "generator",
            "-",
            "function",
            "-",
            "-",
        ]
    );
    assert_eq!(
        braces("const o = { m() {}, *g() {}, k: {} }\nfor (;;) {}\nswitch (x) {}\n"),
        ["-", "function", "generator", "-", "-", "-"]
    );
}

/// Each `<` and `>` of the source: `(` and `)` where the facts record a
/// type-argument or type-parameter bracket, `<` and `>` where they do not.
fn angles(src: &str) -> String {
    lex_with_kind(src, 0, src.len(), SourceKind::TypeScript)
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::Punct(b'<') if token.opens_bracket() => Some('('),
            TokenKind::Punct(b'>') if token.closes_bracket() => Some(')'),
            TokenKind::Punct(byte @ (b'<' | b'>')) => Some(byte as char),
            _ => None,
        })
        .collect()
}

#[test]
fn type_argument_brackets_are_recorded_on_their_angles() {
    assert_eq!(angles("f<A, B>(x)\nnew Map<A, Array<B>>()\n"), "()(())");
    assert_eq!(angles("a < b, c > d\nx << 2 > y\n"), "<><<>");
    assert_eq!(
        angles("function g<T>(v: Map<T, T>): Set<T> {}\nclass C<T> extends D<T> {}\n"),
        "()()()()()"
    );
    assert_eq!(angles("let v: Array<number>\nconst e = f<A>\n"), "()()");
    assert_eq!(
        angles("match (x) { 1 if f<A, B>(x) => new Map<A, B>(), _ => g<A, B> }\n"),
        "()()()"
    );
}

#[test]
fn a_regular_expression_is_lexed_where_an_operand_is_expected() {
    for (src, regex) in [
        ("if (x) /a/.test(y)", true),
        ("(x) / a / 2", false),
        ("x = {} / 2", false),
        ("{} /a/.test(y)", true),
        ("return /a/", true),
        ("const of = 1; of / 2", false),
        ("f(a) / 2", false),
        ("x++ / 2", false),
        ("case /a/:", true),
        ("a.return / 2", false),
    ] {
        let tokens = lex_with_kind(src, 0, src.len(), SourceKind::TypeScript);
        let found = tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Regex));
        assert_eq!(found, regex, "{src}");
    }
}

fn corpus_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !matches!(name.as_ref(), "node_modules" | "target" | ".git") {
                corpus_files(&path, out);
            }
        } else if [
            ".ts", ".tsx", ".mts", ".cts", ".tt", ".ttx", ".js", ".mjs", ".cjs",
        ]
        .iter()
        .any(|extension| name.ends_with(extension))
        {
            out.push(path);
        }
    }
}

fn platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("{os}-{arch}")
}

/// The repository's own TypeScript and tt fixtures, and the pinned
/// TypeScript package (its JavaScript and its standard library
/// declarations) when it is installed. `TTC_FACTS_CORPUS` adds more
/// trees, separated like `PATH`. A file SWC cannot parse — tt syntax, or
/// JavaScript that is not TypeScript — has no oracle; every file SWC parses
/// must be read the same way.
#[test]
fn the_machine_reads_the_corpus_as_swc_does() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut roots: Vec<PathBuf> = [
        "tests",
        "src/stdlib",
        "editors",
        "website/src",
        "website/scripts",
        "integrations",
        "packages",
        "docs",
        "node_modules/typescript",
    ]
    .iter()
    .map(|dir| root.join(dir))
    .collect();
    roots.push(root.join(format!(
        "node_modules/@typescript/typescript-{}/lib",
        platform()
    )));
    if let Some(extra) = std::env::var_os("TTC_FACTS_CORPUS") {
        roots.extend(std::env::split_paths(&extra));
    }
    let mut files = Vec::new();
    for dir in &roots {
        corpus_files(dir, &mut files);
    }
    files.sort();
    let mut checked = 0usize;
    let mut failures = Vec::new();
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        let name = path.to_string_lossy();
        let kind = if name.ends_with('x') {
            SourceKind::Tsx
        } else {
            SourceKind::TypeScript
        };
        if swc_reading(&src, kind).is_none() {
            continue;
        }
        checked += 1;
        if let Some(diff) = disagreement(&src, kind) {
            failures.push(format!("{}\n{diff}", path.display()));
        }
    }
    assert!(checked > 50, "only {checked} corpus files parsed");
    eprintln!(
        "statement spans and regex and JSX positions agree on {checked} of {} corpus files",
        files.len()
    );
    assert!(
        failures.is_empty(),
        "{} of {checked} files differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
