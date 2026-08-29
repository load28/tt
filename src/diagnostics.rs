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

use crate::error::{TtError, line_col};

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
    /// Removed `<-` Result binding syntax with a migration edit.
    ResultLegacyBinding,
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
    /// A match mixing tag patterns and literal patterns.
    MatchMixedPatterns,
    /// A wildcard `_` arm that is not the last arm.
    MatchWildcardNotLast,
    /// Or-pattern alternatives of different literal kinds.
    MatchOrLiteralKindMismatch,
    /// An arm repeating a tag or literal an earlier arm already covers.
    MatchDuplicateArm,
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
            DiagnosticCode::ResultLegacyBinding => "result-legacy-binding",
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
        DiagnosticCode::ResultLegacyBinding,
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

`result` is contextual: a block is claimed only when it contains at least
one `<-` binding. Once claimed, every binding needs its declaration
keyword and a trailing `;`, and the block's final value expression must
have no `;`. Text that is meant to be an ordinary identifier followed by a
block passes through untouched; text with a `<-` in it does not."
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

`try` compiles to an early `return` from the enclosing function, so it is
a value only where the TypeScript host can preserve that return and the
original evaluation order. It is rejected at module or namespace top level
and at expression boundaries with no equivalent statement position, such as
loop headers, parameter defaults, and class field initializers.

Move the propagation into a function-body statement when the surrounding
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
            DiagnosticCode::ResultLegacyBinding => {
                "\
`<-` is no longer Result binding syntax.

Write an ordinary declaration with `= try expression;`, then complete the
Result block with `return value;`."
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

The current compiler propagates this failure to the enclosing function, but
the upcoming lexical Result scopes would make that target ambiguous. Extract
the affected expression into a nested function only when that preserves its
captures and evaluation order; otherwise handle the Result explicitly."
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
A `match` mixes tag patterns and literal patterns.

The two lower differently — a tag match switches on `.kind`, a literal
match switches on the value itself — so one `match` has to be one or the
other. `_` belongs to both. Split the arms into two matches, or match on a
value the arms agree about."
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

/// The one wording of "this match has holes", shared by the untyped pass
/// (sema, over the declaration table) and the typed pass (the engine, over
/// the checker's alphabet) — one renderer, so the two pipelines cannot
/// drift apart on phrasing (TASK-117 symptom 2).
///
/// `subject` names what the match is over when the reporter knows it
/// (`variant Shape`, `(Token, _)`, `literal union`); the typed pass, which
/// knows the alphabet but not the declaration, passes `None`. `missing`
/// entries arrive fully rendered (quoted tags, `(a, b)` combinations);
/// `tuple` picks the truncation unit.
pub(crate) fn non_exhaustive_message(
    subject: Option<&str>,
    missing: &[String],
    tuple: bool,
) -> String {
    let shown = if missing.len() > 4 {
        let unit = if tuple {
            "combinations in total"
        } else {
            "in total"
        };
        format!("{}, … ({} {unit})", missing[..3].join(", "), missing.len())
    } else {
        missing.join(", ")
    };
    let on = match subject {
        Some(subject) => format!(" on {subject}"),
        None => String::new(),
    };
    format!("match{on} is not exhaustive: missing {shown}")
}

/// The one wording of how to close a match's holes, by writing the arms.
///
/// It rides with the diagnostic as a [`Suggestion`] rather than inside
/// [`non_exhaustive_message`]: the missing tags are the *problem*, and
/// what to write instead is the *fix*. Both pipelines attach this same
/// constant, so the advice cannot drift apart either.
pub(crate) const NON_EXHAUSTIVE_HELP: &str = "add the missing arms";

/// The other way to close them: one arm that covers whatever is left.
pub(crate) const NON_EXHAUSTIVE_WILDCARD_HELP: &str = "or add a final `_` arm";

/// The body a compiler-authored arm gets. It is a placeholder on purpose —
/// what the case should evaluate to is the one thing the compiler cannot
/// know — and `undefined` is the value TypeScript will complain about if
/// the reader forgets to replace it, which is the right kind of reminder.
const ARM_BODY: &str = "=> undefined,";

/// Where a match is written: what a diagnostic about the match as a whole
/// underlines, and the braces an arm-insertion edit writes between.
///
/// Both exhaustiveness pipelines carry these four offsets — the default
/// one off the parsed match, the typed one off the probe the emission
/// recorded — so the edits below have one implementation rather than one
/// per pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatchSite {
    /// Byte offset of the `match` keyword.
    pub keyword_off: usize,
    /// Byte offset of the body's opening `{`.
    pub body_open: usize,
    /// Byte offset of the body's closing `}`.
    pub body_close: usize,
}

/// How to close a non-exhaustive match, as the two edits that do it: write
/// the missing arms, or write a final `_`.
///
/// The compiler authors the text because it is the only party that knows
/// all three of what is missing, what each case's payload is called, and
/// where the body's braces are. A consumer that reads the arms back out of
/// the rendered message would be recognizing a sentence by its shape —
/// which is what this replaces (TASK-216).
pub(crate) fn non_exhaustive_suggestions(
    source: &str,
    site: MatchSite,
    arms: &[String],
) -> Vec<Suggestion> {
    let mut out = Vec::new();
    if !arms.is_empty()
        && let Some(edit) = insert_arms(
            source,
            site,
            &arms
                .iter()
                .map(|pattern| format!("{pattern} {ARM_BODY}"))
                .collect::<Vec<_>>(),
        )
    {
        out.push(Suggestion {
            message: NON_EXHAUSTIVE_HELP.to_string(),
            edit: Some(edit),
        });
    }
    if let Some(edit) = insert_arms(source, site, &[format!("_ {ARM_BODY}")]) {
        out.push(Suggestion {
            message: NON_EXHAUSTIVE_WILDCARD_HELP.to_string(),
            edit: Some(edit),
        });
    }
    // A site whose braces do not line up with the text (a stale buffer, a
    // recovered parse) yields no edit rather than a wrong one — the advice
    // is still worth saying.
    if out.is_empty() {
        out.push(Suggestion {
            message: format!("{NON_EXHAUSTIVE_HELP} {NON_EXHAUSTIVE_WILDCARD_HELP}"),
            edit: None,
        });
    }
    out
}

/// The edit that writes `arms` into a match body, matching how the body is
/// already laid out: above the closing brace when it stands on its own
/// line, spliced in before it when the whole match is on one line.
fn insert_arms(source: &str, site: MatchSite, arms: &[String]) -> Option<Edit> {
    let bytes = source.as_bytes();
    if site.keyword_off > site.body_open
        || site.body_open >= site.body_close
        || site.body_close >= bytes.len()
        || bytes[site.body_open] != b'{'
        || bytes[site.body_close] != b'}'
    {
        return None;
    }
    let line_start = |at: usize| source[..at].rfind('\n').map_or(0, |nl| nl + 1);
    let close_line = line_start(site.body_close);
    if source[close_line..site.body_close].trim().is_empty() {
        // `}` on its own line: whole arm lines above it, indented one step
        // in from the `match` keyword's own line.
        let keyword_line = line_start(site.keyword_off);
        let indent: String = source[keyword_line..site.keyword_off]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let text: String = arms
            .iter()
            .map(|arm| format!("{indent}  {arm}\n"))
            .collect();
        return Some(Edit {
            start: close_line,
            end: close_line,
            replacement: text,
        });
    }
    // One-line match: splice the arms in after the last written arm,
    // adding the comma that arm may be missing. The range starts where the
    // body's text ends rather than at the `}`, so the padding before the
    // brace is rewritten instead of being left in the middle.
    let body = &source[site.body_open + 1..site.body_close];
    let written = body.trim_end();
    let separator = if written.is_empty() || written.ends_with(',') {
        " "
    } else {
        ", "
    };
    Some(Edit {
        start: site.body_open + 1 + written.len(),
        end: site.body_close,
        replacement: format!("{separator}{} ", arms.join(" ")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_typed_and_untyped_wordings_are_one_renderer() {
        let missing = vec!["\"Square\"".to_string(), "\"Tri\"".to_string()];
        assert_eq!(
            non_exhaustive_message(Some("variant Shape"), &missing, false),
            "match on variant Shape is not exhaustive: missing \"Square\", \"Tri\"",
        );
        assert_eq!(
            non_exhaustive_message(None, &missing, false),
            "match is not exhaustive: missing \"Square\", \"Tri\"",
        );
    }

    #[test]
    fn long_lists_truncate_the_same_way_on_both_paths() {
        let missing: Vec<String> = (0..6).map(|i| format!("\"C{i}\"")).collect();
        let said = non_exhaustive_message(None, &missing, false);
        assert!(
            said.contains("\"C0\", \"C1\", \"C2\", … (6 in total)"),
            "{said}"
        );
        let combos: Vec<String> = (0..6).map(|i| format!("(A, B{i})")).collect();
        let said = non_exhaustive_message(None, &combos, true);
        assert!(said.contains("… (6 combinations in total)"), "{said}");
    }

    #[test]
    fn every_rule_is_listed_once_and_explained() {
        // `as_str` and `explanation` are exhaustive matches, so the
        // compiler catches a new variant in both. `ALL` it cannot check:
        // this count is the prompt to list a new rule there too.
        assert_eq!(DiagnosticCode::ALL.len(), 40);
        let mut seen = std::collections::HashSet::new();
        for code in DiagnosticCode::ALL {
            let wire = code.as_str();
            assert!(seen.insert(wire), "two rules share the code {wire}");
            assert_eq!(
                DiagnosticCode::parse(wire),
                Some(*code),
                "{wire} does not round-trip through its wire form",
            );
            let explanation = code.explanation();
            assert!(
                explanation.lines().count() >= 2,
                "{wire} needs an explanation longer than its message",
            );
            assert!(
                !explanation.ends_with('\n'),
                "{wire}: the caller adds the trailing newline",
            );
        }
    }

    #[test]
    fn an_unknown_code_has_no_rule() {
        assert_eq!(DiagnosticCode::parse("no-such-rule"), None);
        assert_eq!(DiagnosticCode::parse(""), None);
    }

    #[test]
    fn a_diagnostic_converts_to_the_cli_error_form() {
        let d = Diagnostic {
            code: DiagnosticCode::MatchDuplicateArm,
            severity: Severity::Error,
            message: "match: duplicate arm \"A\"".to_string(),
            start: Some(5),
            end: Some(6),
            owner: None,
            suggestions: Vec::new(),
        };
        let e = d.to_compile_error("abc\ndef\n", Some("x.tt"));
        assert_eq!((e.line, e.col), (2, 2));
        assert_eq!(e.to_string(), "x.tt:2:2: match: duplicate arm \"A\"");
    }
}
