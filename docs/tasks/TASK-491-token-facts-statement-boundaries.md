# TASK-491: Model statement boundaries once, as token facts owned by the lexer

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

An architecture audit found that statement boundaries and automatic semicolon insertion (ASI) were answered by local predicates over token lists in several places at once, and that each answered the question differently. Inputs that TypeScript reads unambiguously were misread: a type ending in `>` or `void` hid the line break after it, contextual keywords used as names misfired, and a CR or U+2028 line break was not a line break.

## Scope

- Included: One shared model in the lexer layer (`src/lexer/facts.rs`) that records per-token facts while lexing, and every consumer that answered the same question on its own: the lexer's regular-expression and JSX decisions, flow statement splitting and function-body classification (`src/flow/`), the parser's statement starts, `try`/`if let` expression positions, pipeline heads and steps, and `match` host ambiguity (`src/parser/`), the program-syntax projection's boundary semicolon, `val`'s line-break test, and the byte scanner's regular-expression guess (`src/scanner.rs`). An SWC oracle test for the model.
- Excluded: Moving the let-else divergence check onto an SWC control-flow graph (a later phase). Its statement splitting reads the new facts. The pipeline's own precedence rules (`is_pipe_boundary_word`, the ternary taint) and tt's reserved-name rule (`is_reserved`) are tt grammar, not statement boundaries, and are unchanged.

## Decisions

### Decision 1: Verify the audit before relying on it

- **Context**: The audit listed the duplicate predicates and a set of wrong inputs.
- **Alternatives considered**: Take the list as given.
- **Decision and rationale**: Each claim was checked against the tree at `d19a479`. The predicates were where the audit said, with two updates: the `val` line-break test had moved from `src/val.rs` to `src/parser/vals.rs` (TASK-490), and there was a fourth line-terminator definition, the lexer's own `line_end`, which ended a `//` comment only at LF, so a file with CR line endings lexed as one comment. The lexer also produced three one-byte `Punct` tokens for a U+2028 or U+2029 (and for a byte-order mark) instead of reading them as white space. Every listed wrong input reproduced (see the work log).

### Decision 2: A push-down recognizer driven by the lexer, not a second pass

- **Context**: The facts must be computed once per token stream, and the lexer's regular-expression decision must read them. A `/` is lexed before the tokens after it exist, so the model has to answer "is an operand expected here" from the tokens before it.
- **Alternatives considered**: (a) A second pass over the finished token stream. The lexer would still need its own regex rule, which is one of the duplicates. (b) Parsing each stream with SWC. tt source is not TypeScript, and a region SWC cannot parse would have no facts at all. (c) A push-down recognizer the lexer drives token by token.
- **Decision and rationale**: (c). `Machine` keeps a stack of grammar frames — statement lists, statements, expressions, bracketed groups (arguments, control heads, parameter lists), object, class, and interface bodies, and types — and each token is offered to the top frame, which consumes it or finishes and lets the frame below take it. Where TypeScript needs lookahead to classify a contextual keyword (`let`, `async`, `type`, `declare`, a member modifier), the machine peeks at the next significant byte and word with the scanner's trivia skipper. The one ambiguity that needs more — whether `<` after an operand opens type arguments — is settled by a small type-grammar skipper over the bytes (`type_arguments_end`) and TypeScript's `canFollowTypeArgumentsInExpression` rule. The same skipper replaces the byte bracket matcher the JSX scanner used for `<Comp<T> />`. Types are entered after annotations, `as`/`satisfies`, type arguments and parameters, heritage clauses, and a type alias's `=`, and end where the next token cannot continue them; `[`, `<`, `extends`, and `is` continue a type only on the same line, as in TypeScript's parser. The machine is tolerant rather than validating: a token no frame expects is absorbed where it stands, so malformed or tt-only text never disturbs the facts around it.

### Decision 3: The facts recorded on every token

- **Context**: The consumers asked a small number of questions.
- **Decision and rationale**: `TokenFacts` is a 16-bit set on each `Token`: `line_break_before` (the full ECMA-262 §12.3 `LineTerminator` set, inside comments too), `ends_expression` (an operand or type is complete after the token), `asi_before` (§12.10.1, including the restricted productions `return`, `yield`, `break`, `continue`, and postfix `++`/`--`), `statement_start`, `label`, `member` (a class, interface, type literal, or object literal member name), and, on a `{`, `function_body`, `generator_body`, and `constructor_body`. `boundary_before` (an automatic semicolon or a statement start) is the "the expression before this token has ended" question most consumers ask. The `Token` struct grows from 40 to 48 bytes.

### Decision 4: tt's statement-shaped constructs are part of the model

- **Context**: The streams the machine reads are tt source. `if let`, let-else, `match` bodies, `variant` bodies, and `result` blocks put braces and keywords where TypeScript has none.
- **Alternatives considered**: Leave tt constructs to tolerance only. A `match` guard's `if`, a let-else `else`, and a `match` arm's JSX value would then be misread.
- **Decision and rationale**: The machine knows their skeletons: an `if` not followed by `(` is an `if let` head (pattern, `=`, scrutinee, block, `else`); a binding followed by `(` is a let-else pattern whose declaration may take an `else` block; `match (…) {` opens arms (`pattern [if guard] => body`); `variant Name {` opens cases; an operand followed on the same line by `{` opens a construct body. None of these shapes is valid TypeScript, so reading them this way cannot change the facts of a TypeScript file.

### Decision 5: SWC is the oracle

- **Context**: Contract 1 says every valid TypeScript file is a valid tt file; the facts must read TypeScript the way TypeScript does.
- **Decision and rationale**: For TypeScript input, the statement spans the machine recognizes must equal SWC's statement and module-declaration spans. `lexer::statement_spans` collects the machine's spans (nested template interpolations and JSX containers included) for the test, and `src/lexer/facts/tests.rs` compares them with SWC's on the known-wrong inputs, on semicolon-free and TSX shapes, and on a corpus: the repository's own TypeScript and tt fixtures and snapshots, and the pinned TypeScript package's JavaScript and library declarations. `TTC_FACTS_CORPUS` adds trees.

### Decision 6: One keyword table, owned by the lexer

- **Context**: `NON_VALUE_WORDS`, `NON_LABEL_WORDS`, `TYPE_OPERATOR_WORDS`, `NON_TYPE_WORDS`, `BLOCK_STMT_WORDS`, `EXPR_BRACE_WORDS`, `CONTROL_PAREN_WORDS`, and the parser's `STMT_ONLY_WORDS` each encoded part of the keyword grammar for one caller.
- **Decision and rationale**: The machine owns the keyword grammar: `reserved` (labels, bindings) and `statement_only_keyword` (the keywords that cannot stand in an operand). The flow lists are deleted with the predicates that used them. The tt sub-grammars that reject a statement keyword inside a `try` operand, a pipeline step, a let-else initializer, or an `if let` scrutinee now ask `crate::lexer::statement_only_keyword`, so there is one table.

### Decision 7: Which brace opens a function body is a fact too

- **Context**: Flow's function-scope queries (`in_function_body`, `function_depth_at`, `function_target_at` and their tt-aware variants) classified each `{` by walking back from it: `annotated_paren` skipped a return type with `NON_TYPE_WORDS`, and `paren_heads_function` excluded control heads with `CONTROL_PAREN_WORDS` and a heritage call by its `extends`. The walk read any `) {` as a function body, so the block after an `if let` scrutinee ending in a call, and a `match` body, counted as functions.
- **Alternatives considered**: Keep the walk. It is the same kind of local token-list predicate the task removes, and its answer differed from the grammar.
- **Decision and rationale**: The machine knows where a body follows a parameter list, a return type, or `=>`, so it records `function_body` on that `{`, with `generator_body` for `function*`, `*m()`, and `async *m()`, and `constructor_body` for a class `constructor`. A tt `match` arm's block (`pattern => {`) keeps the arrow-body reading the queries relied on; their tt-aware variants still exclude it by the parser's ownership set. `annotated_paren`, `paren_heads_function`, `find_open`, `NON_TYPE_WORDS`, and `CONTROL_PAREN_WORDS` are deleted, and the queries no longer take the source text. A `try` in an `if let` body at a module's top level is now the `try-placement` error it always should have been.

### Decision 8: The byte scanner stops guessing regular expressions

- **Context**: `scanner::regex_allowed` was a second regex-vs-division rule (the previous token's last byte or word) behind `find_matching`, `skip_template`, and `has_top_level_comma`, which `is_primary_expression`, `contains_await`, and `scan_type_end` used on text codegen emits.
- **Alternatives considered**: Feed the byte scans a machine. They produce no tokens, so each would need its own driver.
- **Decision and rationale**: The questions are answered over the lexer's tokens (`src/lexer/queries.rs`: `has_top_level_comma`, `is_primary_expression`, `contains_await`, `type_parameter_names`), lexed with the file's `SourceKind` so JSX in emitted `.tsx` is read as JSX. `regex_allowed`, `find_matching`, `skip_template`, `scan_type_end`, and the byte versions are deleted; `scanner.rs` keeps only byte primitives. The parser carries its `SourceKind` for the one query it asks (`try` operand ends).

### Decision 9: Lexical corrections the model depends on

- **Context**: Line-break facts are only as right as the trivia the lexer skips.
- **Decision and rationale**: `scanner::skip_trivia` is the one trivia skipper and reports whether it crossed a line terminator (`line_terminator_len`: LF, CR, U+2028, U+2029). A `//` comment ends at any of them (`line_end`), so CR-only files no longer lex as one comment. Non-ASCII white space and the byte-order mark are trivia rather than three one-byte `Punct` tokens. A hashbang line at a file's start is trivia (ECMA-262 `HashbangComment`). `?.` followed by a digit is `?` and a numeric literal (`a?.5:1`), as ECMA-262's `OptionalChainingPunctuator` lookahead requires. A regular expression literal stops at any line terminator.

## Work log

- 2026-09-28: Verified the audit. Reproduced with `ttc --check`/`ttc -p`: a let-else `else` block whose line before `throw e` ends in `o as Array<number>`, `let g: () => void`, `let p: Promise<void>`, `x satisfies Record<K, unknown>`, or ends with a CR or U+2028 line break reports `let-else-not-diverging`; `let p: Promise<void>` followed by an `if let` reports `if-let-placement`; a pipeline on the line after such a type claims the previous line as its head (`generated TypeScript failed to parse`); a `try` statement after one compiles as a value-form `try`.
- 2026-09-28: Added the scanner's trivia primitives (`line_terminator_len`, `skip_trivia`, a `line_end` that stops at every terminator, non-ASCII white space and the byte-order mark as trivia, a hashbang line at a file's start as trivia) and the machine (`src/lexer/facts.rs`, `facts/{statements,expressions,types}.rs`). The lexer drives it and records the facts on every token; numeric literals, which the lexer still emits as byte-sized pieces, are pushed as one operand. Added the SWC oracle (`facts/tests.rs`). The first corpus run found `x << 24` read as a type assertion after the second `<`; a shift operator's second `<` is now part of the operator.
- 2026-09-28: Moved every consumer onto the facts: flow statement splitting and labels (`src/flow/scanner.rs`), concise-arrow ends (`src/flow/syntax.rs`), the parser's statement starts, `try`/`if let` expression positions, pipeline heads and steps, `match` host ambiguity, and the brace-continuation test (`src/parser/`), the projection's boundary semicolon (`src/program_syntax/projection.rs`), and `val`'s same-line rule (`src/parser/vals.rs`). Deleted `asi_boundary_at` and its helpers, `ConciseArrowBoundaries`, `brace_opens_statement`, `brace_starts_statement`, `head_owes_body`, `NON_VALUE_WORDS`, `NON_LABEL_WORDS`, `TYPE_OPERATOR_WORDS`, `BLOCK_STMT_WORDS`, `EXPR_BRACE_WORDS`, `starts_statement`, `in_for_update`, `follows_object_member_colon`, `STMT_ONLY_WORDS`, and the cursor's `line_break_before`/`ends_expression`. Added `tests/compile/cases_12.rs`.
- 2026-09-28: Recorded function-body facts (`function_body`, `generator_body`, `constructor_body`) and moved flow's function-scope queries onto them (`src/flow/syntax.rs`; callers in `src/parser/`, `src/sema/checker.rs`, `src/core_ir/lower.rs`). Deleted the return-type walk and its word lists. Added `a_brace_records_the_function_body_it_opens` and `a_block_after_a_call_opens_no_function`.
- 2026-09-28: Moved codegen's text queries onto tokens (`src/lexer/queries.rs`), threaded the `SourceKind` through `push_grouped`, `push_receiver`, `needs_grouping`, and `grouping_required` (`src/codegen/core/emitter/`), and deleted the scanner's regex guess and bracket matcher. Two emit snapshots changed (Issue 3); reviewed with `UPDATE_EXPECT=1 cargo test --test snapshot`.
- 2026-09-28: Reviewed the machine for frames that hand a token back and forth without consuming it (Issue 4) and added `the_machine_makes_progress_on_malformed_text`. The line-comment test in `src/codegen/rope/builder.rs` now ends a comment at every line terminator (`scanner::line_end`). Updated `docs/design/compiler-architecture.md` (in English) and noted the superseded predicates at the top of the TASK-391, TASK-451, TASK-480, and TASK-482 records.
- 2026-09-28: Ran the oracle on a wider corpus: `TTC_FACTS_CORPUS=/opt/node22/lib/node_modules:/opt/node22/lib/node_modules/npm/node_modules:/opt/node22/lib/node_modules/eslint/node_modules:/opt/node22/lib/node_modules/ts-node/node_modules cargo test --lib the_machine_reads_the_corpus` — statement spans agree on all 2931 files SWC parses (3013 files; npm's semicolon-free JavaScript, TypeScript 5's compiler and library declarations, eslint, prettier, pnpm, yarn, playwright). The default corpus agrees on all 510 of 570 files SWC parses.

## Issues and resolutions

### Issue 1: A `match` arm's JSX value was lexed as operators

- **Symptom**: `sibling_jsx_tt_values_share_one_owner_rewrite` and two JSX snapshot tests failed: `generated TypeScript failed to parse: Expected '</', got ','`.
- **Cause**: A `match` body frame pushes its arm expression when the arm's first token arrives, so while the lexer decided whether `<b>` after `=>` opens JSX, the top frame was the `match` body, which did not report an expected operand.
- **Resolution**: A `match` body in its arm-body state, and `export default`, report an expected operand, as a statement start does.

### Issue 2: Members named `try` were claimed as the value-form `try`

- **Symptom**: `interface X { try(x); }`, `{ try(x) { … } }`, and `{ try: 1 }` failed to pass through (`untyped_try_methods_survive_next_to_tt_constructs` and three passthrough tests).
- **Cause**: The old statement-start test answered "yes" after any `{`, which sent a member named `try` to the statement parser, whose member-shape rejection kept it TypeScript. The facts correctly say a member name does not start a statement, so the parser tried the value form.
- **Resolution**: The machine records `member` on a class, interface, type literal, or object literal member name, and the parser never reads a member name as the value-form `try`. The same fact joins statement starts as a position where a `match` is delegated to the host grammar, which the old test had covered by the same accident.

### Issue 3: Two JSX emit snapshots lost a pair of parentheses

- **Symptom**: `workflow-jsx-block-callback` and `workflow-jsx-nested-arrow` now emit `$tt_v0 = <ul>…</ul>;` where the fixtures had `$tt_v0 = (<ul>…</ul>);`.
- **Cause**: The byte scanner read the `/` of a JSX closing tag (`</button>`) as the start of a regular expression, lost the bracket balance, and answered "top-level comma" for a value that has none. The rule (`docs/ai/tt.md`) keeps parentheses around a lowered value only where they group it.
- **Resolution**: A real fix; the fixtures were updated and the diff is exactly the two parenthesis pairs.

### Issue 4: Two frames could hand a malformed token back and forth

- **Symptom**: None observed; found by review. A type-argument list meeting a token no type can start (`A<@>`), and a `for` head's test or update meeting a stray `,` or `:`, pushed a child that returned the token unconsumed, then pushed the child again. Only the machine's retry bound ended the loop.
- **Cause**: Their fallback pushed a child for every token, not only for tokens the child can start with.
- **Resolution**: Both frames consume a token their child cannot start with. The retry bound is now a debug assertion as well, so a regression fails the tests instead of degrading silently; `the_machine_makes_progress_on_malformed_text` lexes every prefix and suffix of the known shapes and of malformed text in both source kinds.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0; 40 suites, 1470 tests, 0 failures. The TASK-391, TASK-451, TASK-480, TASK-482, and TASK-490 tests are unchanged and green.
- [x] `./scripts/ci extension`: exit 0; 208 extension tests passed, none skipped.
- [x] `scripts/check-task-index`: the index and the records agree.
- [x] Snapshots: two emit fixtures changed, each by one redundant pair of parentheses (Issue 3); no other snapshot changed.
- [x] The oracle (`src/lexer/facts/tests.rs`): statement spans equal SWC's on every known shape, the TSX shapes, all 510 corpus files SWC parses by default, and all 2931 files of the wider corpus in the work log.
- [x] The regressions in `tests/compile/cases_12.rs` are the shapes reproduced with the previous `ttc` in the first work-log entry (a let-else, `if let`, pipeline head, `try` statement, and statement `match` after a line ending in a type or a contextual name, and CR, CR LF, U+2028, U+2029, and a CR-terminated comment as line terminators), plus the module-level `try` in an `if let` body (Decision 7).

## Result

Added `src/lexer/facts.rs`, `src/lexer/facts/{statements,expressions,types,tests}.rs`, `src/lexer/queries.rs`, and `tests/compile/cases_12.rs`. Changed `src/lexer.rs`, `src/scanner.rs`, `src/flow/{mod,scanner,syntax,tests}.rs`, `src/parser/{cursor,iflets,lets,parse,pipes,results,tries,vals}.rs`, `src/program_syntax/projection.rs`, `src/sema/checker.rs`, `src/core_ir/lower.rs`, `src/codegen/core/mod.rs`, `src/codegen/core/emitter/{expression,helpers,host,pattern,result,source}.rs`, `src/codegen/rope/builder.rs`, `tests/compile.rs`, two fixtures under `tests/fixtures/emit/`, `docs/design/compiler-architecture.md`, the TASK-391, TASK-451, TASK-480, and TASK-482 records, `docs/tasks/INDEX.md`, and this record.

Statement boundaries, automatic semicolons, labels, member names, and function-body braces are decided once per token stream by the lexer and read everywhere else; the flow word lists, the lexer's and the scanner's previous-token rules, and the predicates they served are gone, and the one keyword table the parser still consults (`statement_only_keyword`) lives in the lexer. Follow-ups: measure the lexing cost with `scripts/bench-compare` against `d19a479` (a `Token` is 8 bytes larger and every token passes through the machine), and move the let-else divergence check onto an SWC control-flow graph (the later phase this task excluded).
