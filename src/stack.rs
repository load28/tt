//! The stack policy every compiler thread runs under.

use std::thread;

/// Stack reserved for every thread that runs the compiler's recursive
/// descent, so nesting depth is bounded by this policy rather than by the
/// platform default of the thread that happened to call in.
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
}
