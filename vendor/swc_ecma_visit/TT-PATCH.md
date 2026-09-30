# SWC visitor dependency patch

Source: crates.io `swc_ecma_visit` 29.0.0, directory `crates/swc_ecma_visit`
of the swc repository. The dependency is Apache-2.0 licensed; see LICENSE.

Local change: `src/generated.rs`, every visitor (`Visit`, `VisitMut`, `Fold`,
and their `_ast_path` forms) visits the `VarDeclOrExpr::UsingDecl` variant the
vendored `swc_ecma_ast` adds (`vendor/swc_ecma_ast/TT-PATCH.md`), in the same
form the generator writes for `ForHead::UsingDecl`; `fields::VarDeclOrExprField`
has a `UsingDecl` field and the node iterator yields the declaration. The file
is upstream's generated code with exactly these arms added.

Remove this vendored copy together with `vendor/swc_ecma_ast`.
