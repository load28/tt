//! Internal compiler errors — what the compiler says when *it* is wrong.
//!
//! Two things live here, and they are the same idea at two scales.
//!
//! A lowering validator does not report a user error. It reports that this
//! compiler broke one of its own contracts (`docs/design/program-lowering.md`
//! §11), and the only useful form of that report is the one a compiler
//! engineer can act on: which stage failed, which named invariant it
//! violated, which owner / Core root / operation / value / slot / block it
//! failed on, and where in the source that subject lives. Those are the
//! fields of [`InternalCompilerError`]. The message is derived from them; it
//! is never the place the facts live. A validator that wants to say
//! something new adds an [`Invariant`], not a string.
//!
//! A validator's report reaches the user by panicking, and so does every
//! `unwrap` this compiler has not yet earned the right to. Both are the same
//! event — *ttc is broken here* — so both get the same report:
//! [`install_reporter`] replaces the runtime's panic message with one that
//! says what the compiler was doing, that the user's code is not at fault,
//! and where to send it. [`catching`] then decides what surviving a panic
//! means for a given entry point: the CLI stops with a deliberate exit code,
//! while `--server` answers that one request with an error and keeps the
//! session — which is what its protocol already promises for every other
//! kind of failure.
//!
//! The report never copies the user's source anywhere. It names the file it
//! was working on ([`working_on`]) and leaves sharing it to the person who
//! owns it.

use std::cell::RefCell;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

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

thread_local! {
    /// The file this thread is working on, for a panic report to name.
    /// Empty outside [`working_on`] — a panic in project setup or argument
    /// parsing belongs to no one file, and the report says so rather than
    /// naming the last one it happened to see.
    static WORKING_ON: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
}

/// Runs `work` with `path` recorded as the file being compiled.
///
/// Nesting is a stack, so a panic names the innermost file — the one whose
/// text the compiler actually had in hand — and the outer frames stay
/// intact for the caller that set them.
pub fn working_on<T>(path: &Path, work: impl FnOnce() -> T) -> T {
    /// Pops the frame however `work` ends.
    ///
    /// A panic must not simply leak its frame: the hook runs *before*
    /// unwinding drops locals, so the report still sees this frame, and
    /// then this drop removes it. Without that, a caught panic would leave
    /// the path behind and the next unrelated failure — the server keeps
    /// running, so there is a next one — would name a file it never
    /// touched.
    struct Frame;
    impl Drop for Frame {
        fn drop(&mut self) {
            let _ = WORKING_ON.try_with(|stack| stack.borrow_mut().pop());
        }
    }

    WORKING_ON.with(|stack| stack.borrow_mut().push(path.to_path_buf()));
    let _frame = Frame;
    work()
}

/// The file [`working_on`] last named on this thread, if any.
fn working_file() -> Option<PathBuf> {
    WORKING_ON
        .try_with(|stack| stack.borrow().last().cloned())
        .ok()
        .flatten()
}

/// What a panic payload says, for the payloads a Rust panic can carry.
fn payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&'static str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "a panic with no message".to_string()
    }
}

/// The report a panic is turned into.
///
/// Split out from the hook so it can be tested without panicking: the hook
/// is one call, and this is everything it decides.
fn report(message: &str, location: Option<String>, file: Option<&Path>) -> String {
    let mut out = format!("error: internal compiler error: {message}\n");
    if let Some(file) = file {
        out.push_str(&format!("\n  while compiling: {}", file.display()));
    }
    if let Some(location) = location {
        out.push_str(&format!("\n  at: {location}"));
    }
    out.push_str(&format!("\n  ttc {}\n", env!("CARGO_PKG_VERSION")));
    out.push_str(
        "\nThis is a bug in ttc, not in the code it was given. Please report it at\n\
         https://github.com/load28/tt/issues — include what you ran and, if you can\n\
         share it, the file named above.\n",
    );
    if std::env::var_os("RUST_BACKTRACE").is_none() {
        out.push_str("\nRe-run with RUST_BACKTRACE=1 for a backtrace to attach.\n");
    }
    out
}

/// Replaces the runtime's panic message with the compiler's own report.
///
/// Call once, before any work. Every panic — a validator's
/// [`InternalCompilerError::raise`], an `unwrap` that should not have been
/// there — reaches the user through this, so "the compiler crashed" and
/// "the compiler told me it is broken and how to report it" stop being the
/// same experience.
///
/// The hook itself must never panic: it runs while the thread is already
/// unwinding, and a panic there aborts the process. Everything it touches
/// is therefore fallible-and-ignored.
pub fn install_reporter() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let location = info.location().map(|at| at.to_string());
        let file = working_file();
        eprint!(
            "{}",
            report(&payload_message(info.payload()), location, file.as_deref())
        );
        // The default hook is what honours `RUST_BACKTRACE`, so a reader
        // who asked for a backtrace still gets one, under the report.
        if std::env::var_os("RUST_BACKTRACE").is_some() {
            previous(info);
        }
    }));
}

/// Panics when this build was asked to, at the named point.
///
/// A safety net is only a net if something has been dropped into it, and
/// the two guarantees worth testing — the CLI reports a bug as a bug, and
/// `--server` survives one — can only be observed by making the compiler
/// actually fail. `TTC_PANIC_FOR_TEST=<point>` is that failure, and it
/// exists **only in debug builds**: a release compiler has no path to it,
/// so this cannot be reached by anything a user runs.
#[cfg(debug_assertions)]
pub fn panic_for_test(point: &str) {
    if std::env::var("TTC_PANIC_FOR_TEST").is_ok_and(|asked| asked == point) {
        panic!("TTC_PANIC_FOR_TEST asked this build to fail at {point}");
    }
}

/// The release build's [`panic_for_test`]: nothing, and no way to ask.
#[cfg(not(debug_assertions))]
pub fn panic_for_test(_point: &str) {}

/// Runs `work`, turning a panic into an `Err` carrying its message.
///
/// The report has already been printed by [`install_reporter`] at the point
/// the panic happened — nearest to the facts. What this adds is the
/// *decision* about what happens next, which only the entry point can make.
///
/// `work` is asserted unwind-safe: a caller that keeps state across this
/// call is promising it can live with that state as the panic left it, and
/// the entry points here say in their own comments why they can.
pub fn catching<T>(work: impl FnOnce() -> T) -> Result<T, String> {
    panic::catch_unwind(AssertUnwindSafe(work)).map_err(|payload| payload_message(payload.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_says_what_broke_where_and_whose_fault_it_is_not() {
        let text = report(
            "index out of bounds",
            Some("src/codegen/core.rs:412:9".to_string()),
            Some(Path::new("src/shapes.tt")),
        );
        assert!(text.starts_with("error: internal compiler error: index out of bounds\n"));
        assert!(text.contains("while compiling: src/shapes.tt"), "{text}");
        assert!(text.contains("at: src/codegen/core.rs:412:9"), "{text}");
        assert!(text.contains(env!("CARGO_PKG_VERSION")), "{text}");
        // The one sentence that turns "it crashed" into "report it".
        assert!(text.contains("This is a bug in ttc"), "{text}");
        assert!(text.contains("github.com/load28/tt/issues"), "{text}");
    }

    #[test]
    fn a_report_with_nothing_to_name_still_reports() {
        let text = report("a panic with no message", None, None);
        assert!(!text.contains("while compiling"), "{text}");
        assert!(!text.contains("at:"), "{text}");
        assert!(text.contains("This is a bug in ttc"), "{text}");
    }

    #[test]
    fn the_working_file_is_the_innermost_one() {
        assert_eq!(working_file(), None);
        working_on(Path::new("/a.tt"), || {
            assert_eq!(working_file(), Some(PathBuf::from("/a.tt")));
            working_on(Path::new("/b.tt"), || {
                assert_eq!(working_file(), Some(PathBuf::from("/b.tt")));
            });
            // The outer frame survives the inner one.
            assert_eq!(working_file(), Some(PathBuf::from("/a.tt")));
        });
        assert_eq!(working_file(), None);
    }

    #[test]
    fn a_caught_panic_does_not_leave_its_file_behind() {
        // The server survives panics, so there is always a next failure —
        // and it must not be reported against the file the last one was in.
        let previous = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let caught = catching(|| working_on(Path::new("/a.tt"), || panic!("boom")));
        panic::set_hook(previous);

        assert!(caught.is_err());
        assert_eq!(working_file(), None, "the frame outlived its panic");
    }

    #[test]
    fn catching_turns_a_panic_into_its_message() {
        // The hook is not installed here, so the runtime would print its
        // own message; silence it for the length of the test.
        let previous = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let ok = catching(|| 1 + 1);
        let panicked = catching(|| -> i32 { panic!("a slot with no value") });
        let unreachable = catching(|| -> i32 { unreachable!() });
        panic::set_hook(previous);

        assert_eq!(ok, Ok(2));
        assert_eq!(panicked, Err("a slot with no value".to_string()));
        assert!(unreachable.is_err());
    }

    #[test]
    fn a_lowering_failure_reports_through_the_same_path() {
        // `raise()` panics, so the reporter covers a broken invariant and a
        // stray `unwrap` alike — one report for one kind of event.
        let previous = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let raised = catching(|| {
            InternalCompilerError::new(
                LoweringStage::TargetOrigin,
                Invariant::OriginOutOfBounds,
                LoweringSubject::default(),
            )
            .raise()
        });
        panic::set_hook(previous);
        let message = raised.expect_err("raise panics");
        assert!(message.contains("validate_origin"), "{message}");
        assert!(message.contains("inside the source file"), "{message}");
    }

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
