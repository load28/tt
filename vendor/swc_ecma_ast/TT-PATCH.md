# SWC AST dependency patch

Source: crates.io `swc_ecma_ast` 29.0.0, directory `crates/swc_ecma_ast` of
the swc repository. The dependency is Apache-2.0 licensed; see LICENSE.

Local change: `src/stmt.rs`, `VarDeclOrExpr` has a `UsingDecl(Box<UsingDecl>)`
variant (tagged `UsingDeclaration`, as `ForHead::UsingDecl` is), so a `for`
statement's initializer can hold a `using` or `await using` declaration
(ECMAScript explicit resource management, ES2026; TypeScript 5.2). Upstream's
`VarDeclOrExpr` has only `VarDecl` and `Expr`, so no parser could represent
`for (using r = open(); ; )`. The vendored `swc_ecma_parser` produces the
variant (`vendor/swc_ecma_parser/TT-PATCH.md`), and the vendored
`swc_ecma_visit` visits it (`vendor/swc_ecma_visit/TT-PATCH.md`).

The vendored `swc_ecma_parser` and `swc_ecma_visit` depend on this copy by
path, so one `swc_ecma_ast` is in the build. Remove this vendored copy only
after an upstream version represents the initializer and passes
`tests/swc_for_using_and_if_function.rs` in the parent repository.
