//! Structured internal compiler errors for tt → TypeScript lowering.
//!
//! A lowering validator does not report a user error. It reports that this
//! compiler broke one of its own contracts (`docs/design/program-lowering.md`
//! §11), and the only useful form of that report is the one a compiler
//! engineer can act on: which stage failed, which named invariant it
//! violated, which owner / Core root / operation / value / slot / block it
//! failed on, and where in the source that subject lives.
//!
//! Those are the fields of [`InternalCompilerError`]. The message is derived
//! from them; it is never the place the facts live. A validator that wants to
//! say something new adds an [`Invariant`], not a string.

use std::fmt;

use crate::evaluation_ir::{EvalBlockId, OperationId, ValueId, ValueSlotId};
use crate::program_syntax::{CoreRoot, HostOwner, SourceSpan};

/// The lowering validator whose contract was violated.
///
/// The order is the pipeline order, so a failure names how far the file
/// got. Stages that report through their own typed error enums
/// ([`crate::program_syntax::ProgramSyntaxError`],
/// [`crate::evaluation_ir::EvaluationError`]) are not repeated here; a
/// variant exists exactly when a validator raises through this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoweringStage {
    /// `validate_order` — evaluation count, order, and region containment.
    EvaluationOrder,
    /// `validate_reference` — operands that need a JavaScript reference.
    EvaluationReference,
    /// `validate_origin` — provenance and structure of every target piece.
    TargetOrigin,
    /// `validate_source_preservation` — non-tt source bytes in the target.
    TargetSourcePreservation,
}

impl fmt::Display for LoweringStage {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            LoweringStage::EvaluationOrder => "validate_order",
            LoweringStage::EvaluationReference => "validate_reference",
            LoweringStage::TargetOrigin => "validate_origin",
            LoweringStage::TargetSourcePreservation => "validate_source_preservation",
        })
    }
}

/// The named contract a validator found broken.
///
/// Each variant is one sentence of `docs/design/program-lowering.md`. A new
/// check states which of these it enforces; a new contract adds a variant
/// here first, so the set of things lowering promises stays enumerable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Invariant {
    // ---------------------------------------------------------- order --
    /// A source expression evaluated once must be evaluated once.
    EvaluationCountChanged,
    /// Source expressions keep their relative evaluation order.
    EvaluationOrderChanged,
    /// A value read before it is produced.
    ValueReadBeforeItIsProduced,
    /// A value that runs more than once relative to its host owner was
    /// planned into statements hoisted to that owner.
    RepetitionRegionLeft,
    /// A capture generated inside a conditional region is read outside it.
    ConditionalRegionLeft,

    // ------------------------------------------------------ reference --
    /// An operand that needs a JavaScript reference became a plain value.
    ReferenceDemoted,
    /// A member reference lost the receiver its call binds `this` from.
    ReceiverLost,
    /// A reference was captured under a mode its schedule cannot honour.
    ReferenceModeUnsupported,

    // --------------------------------------------------------- origin --
    /// An origin names a range outside the source file.
    OriginOutOfBounds,
    /// Anchors that open and close out of order.
    OriginNesting,
    /// A generated line break with no layout scope to indent from.
    LayoutScopeMissing,
    /// The target's length does not match the pieces it was built from.
    TargetLengthMismatch,

    // --------------------------------------------- source preservation --
    /// A source byte reaches the target more than once.
    SourceEmittedTwice,
    /// Source spans reach the target out of order without a registered
    /// relocation or a tt-owned range saying so.
    SourceReordered,
    /// A non-whitespace source byte outside every tt-owned range never
    /// reached the target.
    SourceOmitted,
}

impl Invariant {
    /// The contract, as the design document states it.
    fn contract(self) -> &'static str {
        match self {
            Invariant::EvaluationCountChanged => {
                "a source expression evaluated once is evaluated once"
            }
            Invariant::EvaluationOrderChanged => {
                "source expressions keep their relative evaluation order"
            }
            Invariant::ValueReadBeforeItIsProduced => {
                "a value is produced before the evaluation that reads it"
            }
            Invariant::RepetitionRegionLeft => {
                "an expression in a repeated region stays inside that region"
            }
            Invariant::ConditionalRegionLeft => {
                "a capture made inside a conditional region is only read inside it"
            }
            Invariant::ReferenceDemoted => {
                "an operand that needs a reference is not demoted to a value"
            }
            Invariant::ReceiverLost => "a member reference keeps the receiver its call binds",
            Invariant::ReferenceModeUnsupported => {
                "every reference mode a schedule carries can be honoured by the target"
            }
            Invariant::OriginOutOfBounds => "every origin names a range inside the source file",
            Invariant::OriginNesting => "anchors close in the order they opened",
            Invariant::LayoutScopeMissing => "a generated line break has a layout scope",
            Invariant::TargetLengthMismatch => "the target's length is the length of its pieces",
            Invariant::SourceEmittedTwice => "a source byte reaches the target at most once",
            Invariant::SourceReordered => {
                "source spans reach the target in source order unless a relocation says otherwise"
            }
            Invariant::SourceOmitted => {
                "every non-whitespace source byte outside tt-owned ranges reaches the target"
            }
        }
    }
}

/// The identities a lowering failure is about.
///
/// Every field is optional because the stages know different things: an
/// order failure names a value and a slot, a target failure names a span.
/// What a stage does know, it records — the display never invents one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct LoweringSubject {
    pub(crate) owner: Option<HostOwner>,
    pub(crate) root: Option<CoreRoot>,
    pub(crate) operation: Option<OperationId>,
    pub(crate) value: Option<ValueId>,
    pub(crate) slot: Option<ValueSlotId>,
    pub(crate) block: Option<EvalBlockId>,
}

impl LoweringSubject {
    pub(crate) fn owner(owner: HostOwner) -> Self {
        Self {
            owner: Some(owner),
            ..Self::default()
        }
    }

    pub(crate) fn with_root(mut self, root: CoreRoot) -> Self {
        self.root = Some(root);
        self
    }

    pub(crate) fn with_slot(mut self, slot: ValueSlotId) -> Self {
        self.slot = Some(slot);
        self
    }
}

/// A broken lowering contract.
///
/// This is not a diagnostic. It never reaches a `.tt` position, is never
/// suppressed, and is never downgraded to a missed optimization: a release
/// build fails on it exactly like a debug build, so a wrong lowering can
/// never be shipped silently (`docs/design/program-lowering.md` §11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalCompilerError {
    pub(crate) stage: LoweringStage,
    pub(crate) invariant: Invariant,
    pub(crate) subject: LoweringSubject,
    /// Where the subject sits in the original source.
    pub(crate) span: Option<SourceSpan>,
    /// Source spans between the subject and the construct it came from,
    /// innermost first — the provenance chain a synthetic piece is traced
    /// through.
    pub(crate) origin: Vec<SourceSpan>,
}

impl InternalCompilerError {
    pub(crate) fn new(
        stage: LoweringStage,
        invariant: Invariant,
        subject: LoweringSubject,
    ) -> Self {
        Self {
            stage,
            invariant,
            subject,
            span: None,
            origin: Vec::new(),
        }
    }

    pub(crate) fn at(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub(crate) fn with_origin(mut self, origin: Vec<SourceSpan>) -> Self {
        self.origin = origin;
        self
    }

    /// Fails the build. Emission runs only after every validator has passed,
    /// so there is no partial output to fall back to and nothing to report
    /// to the user about their own file.
    pub(crate) fn raise(self) -> ! {
        panic!("{self}")
    }
}

impl fmt::Display for InternalCompilerError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "internal compiler error: {} broke the contract that {} ({:?})",
            self.stage,
            self.invariant.contract(),
            self.invariant,
        )?;
        if let Some(span) = self.span {
            write!(out, "\n  at source bytes {}..{}", span.start, span.end)?;
        }
        let subject = &self.subject;
        if let Some(owner) = subject.owner {
            write!(
                out,
                "\n  host owner {:?} {:?} at {}..{}",
                owner.id, owner.kind, owner.span.start, owner.span.end
            )?;
        }
        if let Some(root) = subject.root {
            write!(out, "\n  core root {root:?}")?;
        }
        if let Some(operation) = subject.operation {
            write!(out, "\n  operation {operation:?}")?;
        }
        if let Some(value) = subject.value {
            write!(out, "\n  value {value:?}")?;
        }
        if let Some(slot) = subject.slot {
            write!(out, "\n  slot {slot:?}")?;
        }
        if let Some(block) = subject.block {
            write!(out, "\n  block {block:?}")?;
        }
        for span in &self.origin {
            write!(out, "\n  from source bytes {}..{}", span.start, span.end)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_names_its_stage_invariant_and_subject() {
        let error = InternalCompilerError::new(
            LoweringStage::EvaluationOrder,
            Invariant::EvaluationOrderChanged,
            LoweringSubject::default(),
        )
        .at(SourceSpan { start: 10, end: 20 })
        .with_origin(vec![SourceSpan { start: 0, end: 40 }]);
        let text = error.to_string();
        assert!(text.contains("validate_order"), "{text}");
        assert!(text.contains("relative evaluation order"), "{text}");
        assert!(text.contains("10..20"), "{text}");
        assert!(text.contains("from source bytes 0..40"), "{text}");
    }
}
