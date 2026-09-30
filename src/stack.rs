//! The stack policy every compiler thread runs under.

use std::thread;

/// Initial stack reserved for compiler threads. Every recursive traversal
/// of syntax, tt's and the host AST's, grows its stack at its recursion
/// boundary, so this reservation is not a syntax-depth limit. It bounds only
/// the recursion that cannot grow: dropping a host AST, whose destructors
/// the vendored SWC crates generate.
pub const COMPILER_STACK_SIZE: usize = 256 * 1024 * 1024;

/// Runs `work` on a thread whose stack is [`COMPILER_STACK_SIZE`]; a panic
/// in `work` resumes on the caller's thread.
pub fn on_compiler_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    thread::scope(|scope| {
        let handle = thread::Builder::new()
            .stack_size(COMPILER_STACK_SIZE)
            .spawn_scoped(scope, work)
            .unwrap_or_else(|error| panic!("the compiler thread could not be created: {error}"));
        handle
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    })
}

/// Grow at recursive compiler traversal boundaries rather than imposing a
/// syntax-depth limit or relying on the caller's remaining stack.
pub(crate) fn grow<T>(work: impl FnOnce() -> T) -> T {
    stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, work)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deeply_nested_expression_parses_on_the_compiler_stack() {
        let depth = 20_000;
        let source = format!(
            "const x = {}1{};
",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let report =
            on_compiler_stack(|| crate::compile_report(&source, &crate::Options::default()));
        assert!(report.diagnostics.is_empty());
    }

    fn nested(depth: usize, open: &str, inner: &str, close: &str) -> String {
        format!("{}{inner}{}", open.repeat(depth), close.repeat(depth))
    }

    #[test]
    fn a_deeply_nested_match_is_read_on_the_compiler_stack() {
        let depth = 10_000;
        let source = format!(
            "export const x = {};\n",
            nested(depth, "match (a) { A => ", "1", " }")
        );
        let path = std::path::Path::new("/deep/main.tt");
        on_compiler_stack(|| {
            assert!(!crate::engine::semantic_tokens(&source).is_empty());
            crate::engine::tt_hints(path, &source);
            crate::engine::tt_declarations(path, &source);
        });
    }

    #[test]
    fn every_nested_tt_construct_compiles_without_the_callers_stack() {
        const V: &str =
            "export variant V { A(v: V), B }\ndeclare const a: V;\ndeclare const b: V;\n";
        const R: &str = "import { Ok, type Result } from \"@tt/std/result\";\n\
                         declare const r: Result<number, string>;\n\
                         declare function h(x: unknown): Result<number, string>;\n";
        let depth = 300;
        let arms = |open: &str, inner: &str, close: &str| {
            format!(
                "{V}export const x = {};\n",
                nested(depth, open, inner, close)
            )
        };
        let statements = |body: String| format!("{V}export function f() {{ {body} }}\n");
        let sources = [
            ("match", arms("match (a) { A(v) => ", "1", ", B => 2 }")),
            (
                "block arm",
                arms("match (a) { A(v) => { return ", "1", "; }, B => 2 }"),
            ),
            (
                "arrow arm",
                arms("match (a) { A(v) => () => ", "1", ", B => 2 }"),
            ),
            ("scrutinee", arms("match (", "a", ") { A(v) => a, B => b }")),
            (
                "guard",
                arms("match (a) { A(v) if ", "true", " => 1, _ => 2 }"),
            ),
            (
                "tuple",
                arms("match (a, b) { (A(v), B) => ", "1", ", _ => 2 }"),
            ),
            (
                "pattern",
                format!(
                    "{V}export const x = match (a) {{ {} => 1, _ => 2 }};\n",
                    nested(depth, "A(v: ", "B()", ")")
                ),
            ),
            (
                "template",
                arms("`${match (a) { A(v) => ", "1", ", B => 2 }}`"),
            ),
            (
                "if let",
                statements(nested(depth, "if let A(v) = a { ", "g();", " }")),
            ),
            (
                "else if let",
                statements(format!(
                    "if let A(v) = a {{ g(); }}{}",
                    " else if let A(v) = a { g(); }".repeat(depth)
                )),
            ),
            (
                "let else",
                statements(nested(
                    depth,
                    "const A(v) = a else { ",
                    "return;",
                    " return; };",
                )),
            ),
            (
                "result",
                format!(
                    "{R}export const x = {};\n",
                    nested(depth, "result { const v = try r; return ", "Ok(1)", "; }")
                ),
            ),
            (
                "try",
                format!(
                    "{R}export function f(): Result<number, string> {{ return Ok({}); }}\n",
                    nested(depth, "try h(", "1", ")")
                ),
            ),
            (
                "pipeline",
                format!("export const x = {};\n", nested(depth, "a |> f(", "1", ")")),
            ),
        ];
        thread::scope(|scope| {
            thread::Builder::new()
                .stack_size(1024 * 1024)
                .spawn_scoped(scope, || {
                    let path = std::path::Path::new("/nested/main.tt");
                    for (name, source) in &sources {
                        let report = crate::compile_report(source, &crate::Options::default());
                        assert!(report.emit.is_some(), "{name}: {:?}", report.diagnostics);
                        crate::emit_mapped(source);
                        crate::engine::semantic_tokens(source);
                        crate::engine::tt_hints(path, source);
                        crate::engine::tt_declarations(path, source);
                    }
                })
                .unwrap()
                .join()
                .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
        });
    }
}
