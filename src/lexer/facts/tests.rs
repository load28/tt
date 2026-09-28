//! The facts machine against its oracle: for TypeScript, the statement
//! spans the machine recognizes are the statement spans SWC parses.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use swc_common::Spanned;
use swc_ecma_ast::{ModuleDecl, Stmt};
use swc_ecma_visit::{Visit, VisitWith};

use crate::SourceKind;
use crate::host_input::HostInput;
use crate::lexer::{Token, TokenKind, lex_with_kind, statement_spans};

struct Statements<'a> {
    input: &'a HostInput,
    spans: BTreeSet<(usize, usize)>,
}

impl Visit for Statements<'_> {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        let span = stmt.span();
        self.spans
            .insert((self.input.byte(span.lo), self.input.byte(span.hi)));
        stmt.visit_children_with(self);
    }

    fn visit_module_decl(&mut self, decl: &ModuleDecl) {
        let span = decl.span();
        self.spans
            .insert((self.input.byte(span.lo), self.input.byte(span.hi)));
        decl.visit_children_with(self);
    }
}

/// SWC's statement spans, or `None` when SWC does not parse the text.
fn swc_statements(src: &str, kind: SourceKind) -> Option<BTreeSet<(usize, usize)>> {
    let input = HostInput::new(src);
    let mut parser = input.parser(kind);
    let module = parser.parse_module().ok()?;
    if !parser.take_errors().is_empty() {
        return None;
    }
    let mut visitor = Statements {
        input: &input,
        spans: BTreeSet::new(),
    };
    module.visit_with(&mut visitor);
    Some(visitor.spans)
}

fn machine_statements(src: &str, kind: SourceKind) -> BTreeSet<(usize, usize)> {
    statement_spans(src, kind)
        .into_iter()
        .map(|span| (span.start, span.end))
        .collect()
}

/// The differences between the two readings of `src`, rendered, or `None`
/// when they agree or SWC rejects the text.
fn disagreement(src: &str, kind: SourceKind) -> Option<String> {
    let expected = swc_statements(src, kind)?;
    let found = machine_statements(src, kind);
    if expected == found {
        return None;
    }
    let show = |(start, end): (usize, usize)| {
        let text = src.get(start..end).unwrap_or("<not a char boundary>");
        let text: String = text.chars().take(80).collect();
        format!("{start}..{end} {text:?}")
    };
    let mut out = String::new();
    for missing in expected.difference(&found).take(6) {
        out.push_str(&format!("  swc only:     {}\n", show(*missing)));
    }
    for extra in found.difference(&expected).take(6) {
        out.push_str(&format!("  machine only: {}\n", show(*extra)));
    }
    Some(out)
}

fn assert_agrees(src: &str, kind: SourceKind) {
    assert!(
        swc_statements(src, kind).is_some(),
        "SWC rejects the case, so it checks nothing:\n{src}"
    );
    if let Some(diff) = disagreement(src, kind) {
        panic!("statement spans differ for:\n{src}\n{diff}");
    }
}

/// Shapes the old token-list predicates misread: a type ending in `>` or
/// `void` hid the line break after it, contextual keywords used as names,
/// and line terminators other than LF.
const KNOWN: &[&str] = &[
    "declare const o: unknown\nconst a = o as Array<number>\nconsole.log(a)\n",
    "let f: () => void\nf = () => {}\n",
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
    "let get = 1, set = 2, of = 3, type = 4, declare = 5, abstract = 6, module = 7, namespace = 8\ntype\nFoo\ndeclare\nfoo\n",
];

#[test]
fn the_machine_reads_known_shapes_as_swc_does() {
    for case in KNOWN {
        assert_agrees(case, SourceKind::TypeScript);
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
        if swc_statements(&src, kind).is_none() {
            continue;
        }
        checked += 1;
        if let Some(diff) = disagreement(&src, kind) {
            failures.push(format!("{}\n{diff}", path.display()));
        }
    }
    assert!(checked > 50, "only {checked} corpus files parsed");
    eprintln!(
        "statement spans agree on {checked} of {} corpus files",
        files.len()
    );
    assert!(
        failures.is_empty(),
        "{} of {checked} files differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
