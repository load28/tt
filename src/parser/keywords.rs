//! Reserved identifiers and pipeline expression boundaries.

/// Whether a word can be a tt variant tag or pattern binding.
pub(crate) fn is_reserved(word: &str) -> bool {
    matches!(
        word,
        "async"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "variant"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "let"
            | "new"
            | "null"
            | "of"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

// Words that terminate the expression currently being scanned — an
// undotted occurrence means whatever follows starts a new expression, so
// the pipeline-head tracker resets there. Prefix operators that *continue*
// an expression (`new`, `typeof`, `void`, `delete`, `await`) and
// expression-capable keywords (`function`, `class`) are deliberately
// absent; `in`/`of` reset for the sake of `for` heads (their rare binary
// use next to a pipeline needs parens, documented). A `match` for the same
// reason as [`is_reserved`] — every identifier in the file reaches it.
pub(super) fn is_pipe_boundary_word(word: &str) -> bool {
    matches!(
        word,
        "break"
            | "case"
            | "catch"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "do"
            | "else"
            | "export"
            | "finally"
            | "for"
            | "if"
            | "in"
            | "let"
            | "of"
            | "return"
            | "switch"
            | "throw"
            | "var"
            | "while"
            | "with"
            | "yield"
    )
}
