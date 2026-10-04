# Recovery source projection extraction

Task: [TASK-744](../tasks/TASK-744-recovery-projection.md).
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).
Base: `85729d01a6cc169d26bce81b9ee25e7b7734f9ae` (TASK-743, PR #138).

## Boundary

`compile_projection_report_parsed` compiles normally, collects parser and
diagnostic recovery nodes, sorts them, selects outermost nodes, masks their
source ranges, and retries compilation. Once no further nodes are found, it
returns the ordinary diagnostics and recovered spans, adding declarations for
recovered variants to emitted or withheld output.

Move these complete definitions from `src/lib/compile.rs` into a private
`src/lib/compile/recovery.rs` child:

- `overwrite_recovery` (private to the child).
- `recoverable_constructs`, `outermost_recoveries`, `recover_source`, and
  `declare_recovered_variants` (`pub(super)`).

The parent imports those four entry points. The child imports only the existing
AST/parser and diagnostic/emission types. It neither invokes compilation nor
owns retry state, snapshots, services, or filesystem access. Keep the explicit
path attribute required by the path-attributed compile module. Public exports,
signatures, caller arguments, and evaluation order remain unchanged.

## Preserved observations

Preserve diagnostic filtering and order, missing-span handling, the caller's
stable start/descending-end sort, nested-node suppression, byte-range clamping,
ASCII space filling, placeholder width thresholds, and UTF-8 assumptions.
Preserve the unreachable MatchArms branch, reserved-name exclusion, generic
parameters, exports, newline insertion, declaration order, and every anchor
field and byte coordinate. Keep withheld emission and all retries in the parent.

## Scope and stop conditions

Allowed production files are only `src/lib/compile.rs` and its new child.
Allowed documentation is this brief, TASK-744, the index, and the program's
progress notes. Do not change expectations, tests, dependency versions,
algorithms, error behavior, sorting, or visibility beyond the four entry points.
Unexpected output differences block completion; fixes require a separate task.

## Evidence and acceptance

On the unchanged base, run `cargo test --offline --test compile --test
editor_cases --test native`, then repeat on the extracted head. Existing public
compile cases exercise projection/withheld results and stray syntax recovery;
native cases exercise discarded-result recovery across successive retries;
editor cases cover the service behavior. No helper-only tests are needed for a
literal move. Rare synthetic clipping and missing-span inputs receive structural
evidence rather than a claim of exhaustive public-case coverage.

Compare all five complete definitions to the base after removing the four
visibility prefixes and reversing rustfmt's single wrapped declaration signature;
reverse the extraction and compare the entire parent file
byte for byte. Read all call sites and the full production diff independently
of that comparison. Run `./scripts/ci agents rust`, recording tool versions,
exit statuses, skips, unavailable prerequisites, and baseline-tracking results.
Do not regenerate baselines. Keep logs outside tracked sources. Rust 1.98.0 and
the repository's TypeScript 7.1.0-dev.20260826.1 remain pinned.

Acceptance requires a coherent private responsibility, unchanged definitions and
orchestration, unchanged output expectations, and successful required gates.
