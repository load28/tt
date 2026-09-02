//! Structured diagnostics — the compiler's report vocabulary.
//!
//! Every tt-level problem is reported as a [`Diagnostic`]: a stable
//! [`DiagnosticCode`], a [`Severity`], a message, and a byte span in the
//! source. One file can carry many of them — the semantic passes accumulate
//! and keep checking the next independent node instead of stopping at the
//! first violation (`docs/design/compiler-core.md` §8, TASK-117).
//!
//! The code is the diagnostic's identity across every consumer: the CLI,
//! `--server`, the engine and the editor all see the same code for the same
//! rule, and the typed and untyped pipelines share one message renderer
//! ([`non_exhaustive_message`]) so their wording cannot drift apart.

mod suggestions;

#[cfg(test)]
mod tests;

use crate::error::{TtError, line_col};

pub(crate) use suggestions::{
    MatchSite, NON_EXHAUSTIVE_HELP, NON_EXHAUSTIVE_WILDCARD_HELP, non_exhaustive_message,
    non_exhaustive_suggestions,
};

/// How serious a [`Diagnostic`] is.
///
/// Every current tt rule is an error; the variant space leaves room for
/// warnings (e.g. unreachable arms, today an editor hint) without another
/// migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Severity {
    /// The program violates a tt rule; compilation does not produce output.
    Error,
    /// Suspicious but not fatal; compilation proceeds.
    Warning,
}

/// The stable identity of a tt rule — what makes the same violation the
/// same diagnostic in every pipeline and every consumer.
///
/// Codes are per *rule*, not per message: the wording may name the variant or
/// the binding, the code says which rule fired. [`DiagnosticCode::as_str`]
/// is the wire form (`--server`, editors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// A `|>` the parser could not claim — the surrounding text is not
    /// emittable TypeScript.
    StrayPipe,
    /// An optional postfix pipeline step committed at `?.` but its complete
    /// tail is not in the supported grammar.
    MalformedPipelinePostfix,
    /// A pipeline head cannot be the base receiver of an optional chain.
    InvalidOptionalReceiver,
    /// An `if let` the parser could not claim.
    StrayIfLet,
    /// A `result` block the parser could not claim.
    StrayResult,
    /// A `variant` committed to tt syntax but not fully parsed.
    MalformedVariant,
    /// A `match` committed to tt syntax but not fully parsed.
    MalformedMatch,
    /// A claimed statement-bodied Result block can fall through without an
    /// explicit successful completion.
    ResultNoSuccessValue,
    /// A Result expression is evaluated only for its side effects, which
    /// would discard a possible Err result.
    ResultValueDiscarded,
    /// A Result block return would wrap an already-Result value.
    ResultReturnNested,
    /// A `break` would leave a ResultRegion.
    ResultBreakCrossing,
    /// A `continue` would leave a ResultRegion.
    ResultContinueCrossing,
    /// A `yield` would leave a ResultRegion.
    ResultYieldCrossing,
    /// A labeled jump would leave a ResultRegion.
    ResultLabelCrossing,
    /// A `flow` composition whose first step is a method step.
    FlowFirstStepMethod,
    /// A `try` outside the top-level statement stream.
    TryPlacement,
    /// A function-targeted `try` whose future nearest Result scope would be
    /// separated by an isolated value region.
    TryCrossesValueRegion,
    /// A let-else outside the top-level statement stream.
    LetElsePlacement,
    /// A let-else whose `else` block does not end in a diverging statement.
    LetElseNotDiverging,
    /// An `if let` in expression position.
    IfLetPlacement,
    /// A variant declaring the same case tag twice.
    VariantDuplicateCase,
    /// A variant field whose type annotation does not parse as TypeScript.
    VariantInvalidFieldType,
    /// A pattern binding the same name twice.
    PatternDuplicateBinding,
    /// A match mixing tag patterns with literal or `is` patterns.
    MatchMixedPatterns,
    /// A wildcard `_` arm that is not the last arm.
    MatchWildcardNotLast,
    /// Or-pattern alternatives of different literal kinds.
    MatchOrLiteralKindMismatch,
    /// An arm repeating a tag or literal an earlier arm already covers.
    MatchDuplicateArm,
    /// An `is` match has no final wildcard for its open class hierarchy.
    MatchIsWildcardRequired,
    /// An `is Type {}` pattern writes an empty property binding list.
    MatchIsEmptyBindings,
    /// An `is` or-pattern attempts to bind properties.
    MatchIsOrBindings,
    /// A match sits in a host context that cannot own its statement region.
    MatchPlacement,
    /// A `break` or `continue` would leave a match arm's completion region.
    MatchControlCrossing,
    /// A nested pattern inside an or-pattern.
    MatchNestedInOrPattern,
    /// Or-pattern alternatives that do not bind the same names.
    MatchOrBindingMismatch,
    /// A tuple pattern whose element count differs from the scrutinees'.
    MatchTupleArity,
    /// A case tag that resolves to no declaration (with a suggestion).
    UnknownCase,
    /// A payload field that resolves to no declaration (with a suggestion).
    UnknownField,
    /// A match that does not cover every case of its subject.
    MatchNotExhaustive,
    /// A mutation through a `val` binding.
    ValMutation,
    /// A `val` binding passed to a parameter not declared `val`.
    ValPass,
    /// The emitted output failed the TypeScript self-check.
    VerifyFailed,
    /// The TypeScript written in the file does not parse, so there is no
    /// TypeScript owner model to lower tt values against.
    SourceNotTypeScript,
    /// Host lowering could not produce an evaluation plan for a tt construct.
    LoweringPlanFailed,
    /// A diagnostic no specific rule claims. Reporting sites should not
    /// produce this — it exists so an unclassified error still has a code.
    Other,
}

impl DiagnosticCode {
    /// The code's stable wire form, e.g. `"match-not-exhaustive"`.
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticCode::StrayPipe => "stray-pipe",
            DiagnosticCode::MalformedPipelinePostfix => "malformed-pipeline-postfix",
            DiagnosticCode::InvalidOptionalReceiver => "invalid-optional-receiver",
            DiagnosticCode::StrayIfLet => "stray-if-let",
            DiagnosticCode::StrayResult => "stray-result",
            DiagnosticCode::MalformedVariant => "malformed-variant",
            DiagnosticCode::MalformedMatch => "malformed-match",
            DiagnosticCode::ResultNoSuccessValue => "result-no-success-value",
            DiagnosticCode::ResultValueDiscarded => "result-value-discarded",
            DiagnosticCode::ResultReturnNested => "result-return-nested",
            DiagnosticCode::ResultBreakCrossing => "result-break-crossing",
            DiagnosticCode::ResultContinueCrossing => "result-continue-crossing",
            DiagnosticCode::ResultYieldCrossing => "result-yield-crossing",
            DiagnosticCode::ResultLabelCrossing => "result-label-crossing",
            DiagnosticCode::FlowFirstStepMethod => "flow-first-step-method",
            DiagnosticCode::TryPlacement => "try-placement",
            DiagnosticCode::TryCrossesValueRegion => "try-crosses-value-region",
            DiagnosticCode::LetElsePlacement => "let-else-placement",
            DiagnosticCode::LetElseNotDiverging => "let-else-not-diverging",
            DiagnosticCode::IfLetPlacement => "if-let-placement",
            DiagnosticCode::VariantDuplicateCase => "variant-duplicate-case",
            DiagnosticCode::VariantInvalidFieldType => "variant-invalid-field-type",
            DiagnosticCode::PatternDuplicateBinding => "pattern-duplicate-binding",
            DiagnosticCode::MatchMixedPatterns => "match-mixed-patterns",
            DiagnosticCode::MatchWildcardNotLast => "match-wildcard-not-last",
            DiagnosticCode::MatchOrLiteralKindMismatch => "match-or-literal-kind-mismatch",
            DiagnosticCode::MatchDuplicateArm => "match-duplicate-arm",
            DiagnosticCode::MatchIsWildcardRequired => "match-is-wildcard-required",
            DiagnosticCode::MatchIsEmptyBindings => "match-is-empty-bindings",
            DiagnosticCode::MatchIsOrBindings => "match-is-or-bindings",
            DiagnosticCode::MatchPlacement => "match-placement",
            DiagnosticCode::MatchControlCrossing => "match-control-crossing",
            DiagnosticCode::MatchNestedInOrPattern => "match-nested-in-or-pattern",
            DiagnosticCode::MatchOrBindingMismatch => "match-or-binding-mismatch",
            DiagnosticCode::MatchTupleArity => "match-tuple-arity",
            DiagnosticCode::UnknownCase => "unknown-case",
            DiagnosticCode::UnknownField => "unknown-field",
            DiagnosticCode::MatchNotExhaustive => "match-not-exhaustive",
            DiagnosticCode::ValMutation => "val-mutation",
            DiagnosticCode::ValPass => "val-pass",
            DiagnosticCode::VerifyFailed => "verify-failed",
            DiagnosticCode::SourceNotTypeScript => "source-not-typescript",
            DiagnosticCode::LoweringPlanFailed => "lowering-plan-failed",
            DiagnosticCode::Other => "other",
        }
    }

    /// Every code, in the order `ttc explain` lists them.
    ///
    /// A rule is only real once it can be explained, so this array and
    /// [`DiagnosticCode::explanation`] are the same list read two ways —
    /// a new variant that forgets either one fails the test that walks it.
    pub const ALL: &[DiagnosticCode] = &[
        DiagnosticCode::StrayPipe,
        DiagnosticCode::MalformedPipelinePostfix,
        DiagnosticCode::InvalidOptionalReceiver,
        DiagnosticCode::StrayIfLet,
        DiagnosticCode::StrayResult,
        DiagnosticCode::MalformedVariant,
        DiagnosticCode::MalformedMatch,
        DiagnosticCode::ResultNoSuccessValue,
        DiagnosticCode::ResultValueDiscarded,
        DiagnosticCode::ResultReturnNested,
        DiagnosticCode::ResultBreakCrossing,
        DiagnosticCode::ResultContinueCrossing,
        DiagnosticCode::ResultYieldCrossing,
        DiagnosticCode::ResultLabelCrossing,
        DiagnosticCode::FlowFirstStepMethod,
        DiagnosticCode::TryPlacement,
        DiagnosticCode::TryCrossesValueRegion,
        DiagnosticCode::LetElsePlacement,
        DiagnosticCode::LetElseNotDiverging,
        DiagnosticCode::IfLetPlacement,
        DiagnosticCode::VariantDuplicateCase,
        DiagnosticCode::VariantInvalidFieldType,
        DiagnosticCode::PatternDuplicateBinding,
        DiagnosticCode::MatchMixedPatterns,
        DiagnosticCode::MatchWildcardNotLast,
        DiagnosticCode::MatchOrLiteralKindMismatch,
        DiagnosticCode::MatchDuplicateArm,
        DiagnosticCode::MatchIsWildcardRequired,
        DiagnosticCode::MatchIsEmptyBindings,
        DiagnosticCode::MatchIsOrBindings,
        DiagnosticCode::MatchPlacement,
        DiagnosticCode::MatchControlCrossing,
        DiagnosticCode::MatchNestedInOrPattern,
        DiagnosticCode::MatchOrBindingMismatch,
        DiagnosticCode::MatchTupleArity,
        DiagnosticCode::UnknownCase,
        DiagnosticCode::UnknownField,
        DiagnosticCode::MatchNotExhaustive,
        DiagnosticCode::ValMutation,
        DiagnosticCode::ValPass,
        DiagnosticCode::VerifyFailed,
        DiagnosticCode::SourceNotTypeScript,
        DiagnosticCode::LoweringPlanFailed,
        DiagnosticCode::Other,
    ];

    /// The code named by its wire form, e.g. `"match-not-exhaustive"`.
    pub fn parse(text: &str) -> Option<DiagnosticCode> {
        DiagnosticCode::ALL
            .iter()
            .copied()
            .find(|code| code.as_str() == text)
    }

    /// What the rule is, why tt has it, and what to write instead — the
    /// text `ttc explain <code>` prints.
    ///
    /// A message has to fit on a line above the reader's code; an
    /// explanation does not. This is where a rule says the part that does
    /// not fit: the reason it exists, which is what turns "the compiler
    /// rejected this" into "I see why".
    pub fn explanation(self) -> &'static str {
        match self {
            DiagnosticCode::StrayPipe => {
                "\
A `|>` was written where the pipeline parser could not claim the text
around it, so the file is neither a valid pipeline nor valid TypeScript.

The usual causes are a head or step that needs parentheses. A ternary or
an arrow function at the top level of either side has to be wrapped:

    (ready ? a : b) |> f
    x |> (n => n + 1)

A step may not be empty. An ambiguous head — no-semicolon style, or one
containing `in` / `instanceof` — is resolved by parenthesizing the head."
            }

            DiagnosticCode::MalformedPipelinePostfix => {
                "\
An optional postfix pipeline step started with `?.`, so tt owns the whole
step, but its tail is incomplete or outside the supported postfix grammar.

The first operation is `?.name`, `?.[key]`, or `?.(args)`. It may continue
with ordinary or optional member, index, and call operations. Tagged
templates, private fields, optional construction, and partial operations are
not supported. The whole pipeline is rejected rather than partially emitted."
            }

            DiagnosticCode::InvalidOptionalReceiver => {
                "\
The value before an optional postfix pipeline step cannot be used as an
optional-chain receiver. Parentheses only control precedence; they cannot make
a syntactically forbidden receiver valid.

For example, bare `super` may only appear in the member and call forms the
JavaScript grammar permits, and cannot become the base of `?.`. Use an
ordinary `super.member` expression before the pipeline or restructure the
access."
            }

            DiagnosticCode::StrayIfLet => {
                "\
An `if let` was written that the parser could not claim.

The pattern's parentheses are mandatory (`if let Some(value: u) = f()`),
and the `else` may only be a block or another `if let` — a plain
`else if (cond)` has to go inside an `else { ... }` block. Unlike a
lookalike that is valid TypeScript, a claimed `if let` is reported here
rather than passed through, because its text cannot be emitted as TS."
            }

            DiagnosticCode::StrayResult => {
                "\
A `result` block was claimed but could not be parsed.

`result` is contextual: a block is claimed only when it contains a direct
`try` expression. Its body is a statement list: write `const value = try expr;`
to unwrap a success value, and use an explicit `return value;` or `return;` to
complete the block successfully. Text that is meant to be an ordinary
identifier followed by a block passes through untouched."
            }

            DiagnosticCode::MalformedVariant => {
                "\
The text committed to tt's `variant` syntax but did not parse as one.

Every `enum` declaration belongs to TypeScript and passes through untouched.
A `variant` belongs to tt and each case must be
`Tag`, `Tag()`, or `Tag(field: Type, ...)`, separated by commas."
            }

            DiagnosticCode::MalformedMatch => {
                "\
The text committed to tt's `match` syntax but did not parse as one.

The scrutinee parentheses are mandatory and may not be empty. Each arm is
`pattern => expression,` or `pattern => { ... }`. An object literal body
needs its own parentheses (`Tag => ({ a: 1 })`), and scrutinees containing
a top-level `<` or `>` comparison need parenthesizing so they cannot be
read as type arguments."
            }

            DiagnosticCode::FlowFirstStepMethod => {
                "\
A `flow` composition's first step is a method step (one starting with
`.`).

`flow` composes functions rather than piping a value, so the first step is
what fixes the composed function's input type — and a method step has no
input type of its own to give. Put a named or parenthesized function
first:

    const label = flow |> half |> .toFixed(1);"
            }

            DiagnosticCode::TryPlacement => {
                "\
A `try` was written where its propagation could not go anywhere.

`try` compiles to an early exit of the nearest Result scope: the innermost
`result` block, or the enclosing function when no such block is open. It is
a value only where the TypeScript host can preserve that exit and the
original evaluation order. It is rejected at module or namespace top level
and at expression boundaries with no equivalent statement position, such as
loop headers, parameter defaults, and class field initializers.

Move the propagation into the nearest Result scope when the surrounding
expression cannot carry it."
            }

            DiagnosticCode::ResultNoSuccessValue => {
                "\
A `result` block can finish without producing an `Ok` value.

A statement-bodied Result block completes successfully only with `return value;`
or `return;`. Add a return on every reachable path."
            }

            DiagnosticCode::ResultValueDiscarded => {
                "\
A `result` expression was used as a discarded statement value.

Store, return, or otherwise consume the Result so its `Err` remains observable."
            }
            DiagnosticCode::ResultReturnNested => {
                "\
A `result` block return already has a Result value.

`return value;` completes the block with `Ok(value)`, so returning a Result
there creates a nested Result. Write `return try value;` when the inner Err
should complete the enclosing block instead. This diagnostic is emitted only
when the TypeScript checker proves the returned value has the Result shape."
            }
            DiagnosticCode::ResultBreakCrossing => {
                "\
A `break` in a `result` block would leave the block's generated completion region.

Break only a loop or switch written inside the `result` block, or move the
control transfer outside the block."
            }
            DiagnosticCode::ResultContinueCrossing => {
                "\
A `continue` in a `result` block would leave the block's generated completion region.

Continue only a loop written inside the `result` block, or move the control
transfer outside the block."
            }
            DiagnosticCode::ResultYieldCrossing => {
                "\
A `yield` in a `result` block would cross the block's completion region.

Yield outside the block, or return a Result value from the generator instead."
            }
            DiagnosticCode::ResultLabelCrossing => {
                "\
A labeled control transfer in a `result` block would leave the block's completion region.

Keep the label and its target inside the `result` block, or move the transfer
outside the block."
            }

            DiagnosticCode::TryCrossesValueRegion => {
                "\
A `try` crosses an isolated value region inside a `result` block.

The nearest Result scope is outside a value region that owns its own exits, so
the failure cannot reach that Result scope without changing the region's value
semantics. Extract the affected expression into a nested function only when
that preserves its captures and evaluation order; otherwise handle the Result
explicitly."
            }

            DiagnosticCode::LetElsePlacement => {
                "\
A let-else was written outside the statement stream it needs.

Like `try`, its `else` block leaves the enclosing function, so it belongs
to a statement list — not to a `match` arm, a `result` block, or another
construct's value region. Module top level is allowed here, because a
let-else has no `return` of its own to place."
            }

            DiagnosticCode::LetElseNotDiverging => {
                "\
A let-else `else` block can fall out of its bottom.

The binding is only in scope afterwards because the `else` never reaches
that point, so every path through it has to leave via `return`, `throw`,
`break` or `continue`. A `break` or `continue` naming a loop or switch
*inside* the block does not leave the block, and neither does a `return`
inside a nested function — that returns from the function.

The check runs on a real control-flow graph, so an `if`/`else` chain whose
branches all diverge, a `while (true)` with no `break`, or a `try`/`catch`
where both halves diverge all count."
            }

            DiagnosticCode::IfLetPlacement => {
                "\
An `if let` was written in expression position.

`if let` is a statement — it lowers to an `if` with a narrowing test.
Inside an expression region it is allowed only within a function you write
there, which is the same control-flow rule `try` and let-else follow."
            }

            DiagnosticCode::VariantDuplicateCase => {
                "\
A variant declares the same case tag twice.

The tag is the emitted union's `kind` discriminant, so two cases with one
tag would be indistinguishable at runtime and unmatchable in a pattern.
Rename one of them."
            }

            DiagnosticCode::VariantInvalidFieldType => {
                "\
A variant field's type annotation does not parse as TypeScript.

Field types are emitted into the generated union verbatim, so they are
checked as TypeScript type syntax where they are written — that way the
error points at your declaration rather than at generated code."
            }

            DiagnosticCode::PatternDuplicateBinding => {
                "\
A pattern binds the same name twice.

Two fields cannot both introduce one name; alias one of them with
`field: alias`:

    Rect(width: w, height: h) => w * h"
            }

            DiagnosticCode::MatchMixedPatterns => {
                "\
A `match` mixes tag patterns with literal or `is` patterns.

They inspect different representations: a tag match switches on `.kind`,
while literal and `is` patterns inspect the value itself. One `match` must
therefore use either the tag family or the value family. `_` belongs to both.
Split the arms into two matches, or match on a value the arms agree about."
            }

            DiagnosticCode::MatchWildcardNotLast => {
                "\
A `_` arm is followed by another arm.

`_` matches everything, so any arm after it is unreachable. Move it to the
end."
            }

            DiagnosticCode::MatchOrLiteralKindMismatch => {
                "\
An or-pattern's alternatives are literals of different kinds.

`\"a\" | 1` cannot be one comparison. Every alternative of one or-pattern
has to be the same kind of literal — all strings, all numbers, all
bigints, or all booleans. `1n` and `1` are different kinds, not one number
written two ways."
            }

            DiagnosticCode::MatchDuplicateArm => {
                "\
An arm repeats a tag or a literal an earlier arm already covers.

The later arm can never run. Literals are compared by value, so `200` and
`0xc8` are the same arm. Guarded arms may repeat a tag — the guard can
fail — but an arm that repeats a tag an *unguarded* arm already took is
still dead."
            }

            DiagnosticCode::MatchIsWildcardRequired => {
                "\
An `is` match has no final wildcard arm.

JavaScript class hierarchies are open, so ttc cannot prove that a list of
`instanceof` tests is exhaustive. Add a final `_` arm."
            }

            DiagnosticCode::MatchIsEmptyBindings => {
                "\
An `is` pattern writes an empty property binding list.

`is Error {}` has the same runtime test as `is Error` but suggests that a
value is materialized. Remove the braces."
            }

            DiagnosticCode::MatchIsOrBindings => {
                "\
An `is` or-pattern binds properties.

ttc does not answer whether different JavaScript classes share a property.
Use type-only alternatives (`is A | is B`) or split the alternatives into
separate arms before binding properties."
            }

            DiagnosticCode::MatchPlacement => {
                "\
A `match` is used in a TypeScript host that cannot own its control flow.

Expression matches lower to host-owned statements and a result slot. They
never use an IIFE, an immediately invoked callback, or `$tt_expr`. Move the
match to a function-body statement whose evaluation count and conditional
reachability are explicit."
            }
            DiagnosticCode::MatchControlCrossing => {
                r#"A `break`, `continue`, or `yield` in a match arm may target only control flow written inside that arm.

Each arm is an isolated completion region. Allowing a jump to an enclosing host construct would bypass the match result delivery and make the generated control-flow target depend on unrelated outer syntax."#
            }

            DiagnosticCode::MatchNestedInOrPattern => {
                "\
A nested pattern appears inside an or-pattern.

`A(x: Some(v)) | B(y)` would have to bind different shapes on different
alternatives. Use element-level alternation instead — `Ok(value: Some(v) |
None())` is not this rule — or write the arms separately."
            }

            DiagnosticCode::MatchOrBindingMismatch => {
                "\
An or-pattern's alternatives do not bind the same names.

The arm body is one piece of code, so every alternative has to leave it
the same bindings. Alias the fields so the sets agree:

    Circle(radius: r) | Square(side: r) => r"
            }

            DiagnosticCode::MatchTupleArity => {
                "\
A tuple pattern has a different number of elements than the match has
scrutinees.

`match (a, b)` matches pairs, so every arm is a two-element tuple pattern
(or a final bare `_`). A one-element side is still claimed as a tuple when
the other arms prove tuple intent, so the reported arity is the real
mismatch rather than a guess."
            }

            DiagnosticCode::UnknownCase => {
                "\
A pattern names a case the variant does not declare.

tt only reports this when it can name what you meant — a near-miss or a
case difference — because tag patterns also match hand-written `kind`
unions whose tags are in no declaration table. A name that is simply wrong
rather than misspelled needs types, and is left to the checker.

The suggested replacement travels with the diagnostic, so an editor can
apply it directly."
            }

            DiagnosticCode::UnknownField => {
                "\
A pattern names a field the case does not declare.

Fields are bound by name, never by position, so the name has to exist on
that case. As with an unknown case, this is only reported when the
declaration table can name the field you meant."
            }

            DiagnosticCode::MatchNotExhaustive => {
                "\
A `match` without a `_` arm does not cover every case of its subject.

Exhaustiveness is what makes adding a case to a variant a compile error at
every place that handles it, rather than a runtime surprise. Add the
missing arms, or a final `_` arm to opt out.

Two rules decide what counts as covered: a *guarded* arm never covers its
tag, because the guard can fail; a *nested* pattern does cover, because
the check descends into payloads. A hole is reported as a pattern you can
paste back — `missing \"Ok(value: None)\"`.

For a tuple match the answer is the product of the positions, so a hole is
a combination: `missing (North, Slow)`."
            }

            DiagnosticCode::ValMutation => {
                "\
A value reached through a `val` binding is mutated.

`val` makes the binding *and every path from it* read-only, at any depth:
`x.a = v` and its compound forms, `x[i] = v`, `x.a++`, `delete x.a`.

Rebinding is a different axis and is not this rule — `val let state` may
still be assigned. Reads, comparisons and spreads (`{ ...x }`) are fine;
copy and replace rather than mutate in place."
            }

            DiagnosticCode::ValPass => {
                "\
A `val` binding is passed to a parameter that is not declared `val`.

The guarantee would end at the call otherwise: the callee could mutate
what the caller promised not to. Declare the parameter `val`, or pass a
copy.

A `val` binding may only be passed to a `val` parameter of a same-file
named function, and only as a plain path argument — that is the extent of
what tt can check without types."
            }

            DiagnosticCode::VerifyFailed => {
                "\
The TypeScript the compiler emitted did not parse.

This is the compiler checking its own output, so seeing it means either
the passthrough text was not valid TypeScript to begin with, or ttc has a
bug. Check the file for a syntax error outside any tt construct first; if
there is none, this is worth reporting.

`--no-verify` turns the self-check off."
            }

            DiagnosticCode::SourceNotTypeScript => {
                "\
The TypeScript inside a claimed tt construct does not parse.

Lowering models the file's TypeScript — that is how a construct knows what
it is nested in and what evaluates when — so a `match` arm body or a
`result` block that is not TypeScript leaves nothing to lower against, and
nothing is emitted.

This is not the output self-check, so `--no-verify` does not skip it. A
file with no tt construct in it is reported through `verify-failed`
instead."
            }

            DiagnosticCode::LoweringPlanFailed => {
                "\
tt recognized a construct, but its host-lowering plan could not be built.

The diagnostic is reported at the tt construct instead of exposing an
internal compiler failure. This code preserves the failure at every compiler
entry point while the specific host rule is repaired."
            }

            DiagnosticCode::Other => {
                "\
A diagnostic no specific rule claims.

Every reporting site should carry its own code, so this one appearing on a
tt-level diagnostic is a gap in the compiler rather than a rule you can
look up."
            }
        }
    }

    /// Whether a diagnostic with this code leaves the file impossible to
    /// project into the TypeScript program.
    ///
    /// Most tt errors are *recoverable*: the emitter still produces plain
    /// TypeScript for the file (codegen is infallible), so the typed pass
    /// can run and its diagnostics are reported **alongside** the tt ones —
    /// a duplicate arm no longer hides the file's other exhaustiveness
    /// holes, type errors and `val` violations (TASK-117 symptom 3).
    ///
    /// The exceptions are the diagnostics whose presence means the output
    /// cannot be valid TypeScript at all: text the parser could not claim
    /// (a stray `|>` passes through verbatim and is not TS), a field type
    /// that would be emitted verbatim into a type position, and the output
    /// self-check itself.
    pub fn blocks_projection(self) -> bool {
        matches!(
            self,
            DiagnosticCode::StrayPipe
                | DiagnosticCode::InvalidOptionalReceiver
                | DiagnosticCode::StrayIfLet
                | DiagnosticCode::StrayResult
                | DiagnosticCode::MalformedVariant
                | DiagnosticCode::MalformedMatch
                | DiagnosticCode::MatchControlCrossing
                | DiagnosticCode::VariantInvalidFieldType
                | DiagnosticCode::VerifyFailed
                | DiagnosticCode::SourceNotTypeScript
        )
    }
}

/// One reported problem, with a byte span in the source it was found in.
///
/// This is the multi-diagnostic counterpart of [`crate::CompileError`]:
/// positions are byte offsets (convert with [`crate::line_col`], or
/// [`Diagnostic::to_compile_error`] for the CLI's line/column form), and a
/// file produces a `Vec` of them in source order rather than the first one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    /// Which rule fired.
    pub code: DiagnosticCode,
    /// How serious it is.
    pub severity: Severity,
    /// The full message, as it is shown.
    pub message: String,
    /// Byte offset of the diagnostic's position in the source, or `None`
    /// for positionless diagnostics (a failed output self-check).
    pub start: Option<usize>,
    /// Byte offset just past the range the diagnostic covers — the
    /// construct as written. `None` leaves the width to the consumer.
    pub end: Option<usize>,
    /// Complete syntax node that owns checker consequences of this cause.
    pub owner: Option<DiagnosticOwner>,
    /// How to resolve the cause, as the reporter knows it.
    ///
    /// This is the fix held apart from the problem: [`Diagnostic::message`]
    /// says what is wrong, a suggestion says what to do about it. A
    /// reporting site that can name the fix therefore leaves it out of the
    /// message — otherwise a consumer would have to recognise it by the
    /// sentence's shape to show it separately, and reading diagnostics back
    /// out of their own wording is the heuristic this split exists to
    /// remove.
    pub suggestions: Vec<Suggestion>,
}

/// One way to resolve a [`Diagnostic`].
///
/// The [`edit`](Suggestion::edit) is what separates advice a machine can
/// apply from advice only a person can: a misspelled case tag has one
/// replacement and an editor can offer it as a code action, while "add the
/// missing arms" names no text to write. Both are suggestions — the CLI
/// renders `= help:` for either — and a consumer that applies edits simply
/// skips the entries that carry none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// What to do, as a phrase ("a case with a similar name exists").
    /// Rendered after `help:`.
    pub message: String,
    /// The source change that carries it out, when there is one.
    pub edit: Option<Edit>,
}

/// A source change a machine can apply: replace `[start, end)` with
/// `replacement`.
///
/// Offsets are byte offsets into the same source the diagnostic was found
/// in, so a consumer can apply a file's edits back to front and rewrite it
/// without re-parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Byte offset where the replaced range starts.
    pub start: usize,
    /// Byte offset just past the replaced range.
    pub end: usize,
    /// The text that replaces `[start, end)`.
    pub replacement: String,
}

/// Source identity of the syntax node that owns a diagnostic cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticOwner {
    /// Byte offset where the owning syntax node starts.
    pub start: usize,
    /// Byte offset just past the owning syntax node.
    pub end: usize,
}

impl Diagnostic {
    /// The diagnostic in [`crate::CompileError`]'s form: 1-based line and
    /// column in `source`, carrying `filename` — what the CLI prints.
    pub fn to_compile_error(&self, source: &str, filename: Option<&str>) -> crate::CompileError {
        let (line, col) = match self.start {
            Some(offset) => line_col(source, offset),
            None => (0, 0),
        };
        let (end_line, end_col) = match self.end {
            Some(offset) => line_col(source, offset),
            None => (0, 0),
        };
        crate::CompileError {
            message: self.message.clone(),
            filename: filename.map(String::from),
            line,
            col,
            end_line,
            end_col,
        }
    }

    pub(crate) fn from_tt(error: TtError) -> Diagnostic {
        Diagnostic {
            code: error.code,
            severity: Severity::Error,
            message: error.message,
            start: error.offset,
            end: error.end,
            owner: error.owner,
            suggestions: error.suggestions,
        }
    }
}
