//! Hints — what tt has to say about a file that is not an error.
//!
//! tt's diagnostics are errors, and only errors: the CLI either compiles a
//! file or reports a position and stops. That is a deliberate part of the
//! error-layer contract (`CLAUDE.md`), and it leaves no room for the one
//! thing the pattern analysis knows but must not reject — an **unreachable
//! arm**. Rust reports it as a lint; making it an error here would reject
//! programs that compile today, and tt has no warning level to put it at.
//!
//! An editor does, though. A hint is not part of the compile answer: it is
//! attached to a range, it never fails a build, and the CLI never prints
//! one. So the arms the usefulness algorithm already computes as dead
//! ([`crate::MatchAnalysis::unreachable`], TASK-103) surface here and
//! nowhere else.
//!
//! Like [`super::names`] and [`super::completions`] this is **parse-only**:
//! source in, hints out, no toolchain and no project — so an editor shows
//! them mid-edit and in a workspace with no TypeScript installed.

use std::path::Path;

use super::documents::Texts;
use super::language::{Range, span_range};

/// What a hint is about. One kind today; the enum is the seam that keeps a
/// consumer from switching on message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtHintKind {
    /// An arm that matches nothing an earlier arm has not already matched.
    UnreachableArm,
}

/// One thing tt has to say about a range that is not an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtHint {
    /// What kind of hint it is.
    pub kind: TtHintKind,
    /// The range it is about — the whole arm, so an editor can dim it.
    pub range: Range,
    /// The message, as it is shown.
    pub message: String,
}

/// Every hint tt has about `source`.
///
/// `path` resolves relative `.tt` imports, exactly as the other parse-only
/// surfaces resolve them. This is the stand-alone question: an imported
/// file is read as saved. A session asks [`super::Workspace::tt_hints`],
/// which reads its open documents.
pub fn tt_hints(path: &Path, source: &str) -> Vec<TtHint> {
    hints(path, source, Texts::Disk)
}

pub(super) fn hints(path: &Path, source: &str, texts: Texts<'_>) -> Vec<TtHint> {
    let analyses = super::language::analyses_for(path, source, texts);
    let mut out = Vec::new();
    for analysis in &analyses.matches {
        for &index in &analysis.unreachable {
            let Some(arm) = analysis.arms.get(index) else {
                continue;
            };
            // The body span runs to the arm's delimiter, so it can carry
            // trailing whitespace — a dimmed range should stop at the code.
            let end = source[..arm.body_end].trim_end().len();
            out.push(TtHint {
                kind: TtHintKind::UnreachableArm,
                range: span_range(source, arm.pattern_start, end),
                message: "unreachable arm: an earlier arm already matches every value \
                          this one would"
                    .to_string(),
            });
        }
    }
    out.sort_by_key(|hint| (hint.range.start.line, hint.range.start.character));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hints(source: &str) -> Vec<TtHint> {
        tt_hints(Path::new("/p/a.tt"), source)
    }

    #[test]
    fn an_arm_an_earlier_one_already_covers_is_hinted() {
        let src = "variant E { A(x: string), B(y: number) }\n\
                   const v = match (e) { A(x) => x, B(y) => y, A(x: z) => z };\n";
        let found = hints(src);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].kind, TtHintKind::UnreachableArm);
        // The whole third arm, from its pattern to the end of its body.
        let line = src.lines().nth(1).expect("second line");
        let start = line.find("A(x: z)").expect("the dead arm") as u32;
        assert_eq!(found[0].range.start.line, 1);
        assert_eq!(found[0].range.start.character, start);
        assert_eq!(
            found[0].range.end.character,
            start + "A(x: z) => z".len() as u32
        );
    }

    #[test]
    fn a_live_match_has_nothing_to_say() {
        let src = "variant E { A(x: string), B(y: number) }\n\
                   const v = match (e) { A(x) => x, B(y) => y };\n";
        assert!(hints(src).is_empty());
    }

    #[test]
    fn a_guarded_arm_is_never_dead() {
        // The guard may be false, so a later arm for the same case still
        // matches values the guarded one does not.
        let src = "variant E { A(x: string), B(y: number) }\n\
                   const v = match (e) { A(x) if ok => x, A(x: z) => z, B(y) => y };\n";
        assert!(hints(src).is_empty());
    }

    #[test]
    fn a_tuple_combination_an_earlier_arm_covers_is_hinted() {
        let src = "variant D { N(a: number), S(b: number) }\n\
                   variant P { F(c: number), G(d: number) }\n\
                   const v = match (x, y) { (N, _) => 1, (S, F) => 2, (S, G) => 3, (N, F) => 4 };\n";
        let found = hints(src);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].range.start.line, 2);
    }

    #[test]
    fn a_trailing_wildcard_does_not_hide_dead_arms() {
        let tuple = "variant S { Circle, Square }\n\
                     variant W { Nope, Yes }\n\
                     const v = match (s, w) { (Circle, _) => 1, (_, Nope) => 2, (Circle, Nope) => 3, _ => 4 };\n";
        let found = hints(tuple);
        assert_eq!(found.len(), 1, "{found:?}");
        let line = tuple.lines().nth(2).expect("the match line");
        assert_eq!(
            found[0].range.start.character,
            line.find("(Circle, Nope)").expect("the dead arm") as u32
        );

        let nested = "variant Shape { Circle(radius: number), Square }\n\
                      variant Box { Has(s: Shape), Empty }\n\
                      const v = match (b) { Has(s: Circle(radius: r)) => 1, Has(s: Circle(radius: q)) => 2, _ => 5 };\n";
        let found = hints(nested);
        assert_eq!(found.len(), 1, "{found:?}");
        let line = nested.lines().nth(2).expect("the match line");
        assert_eq!(
            found[0].range.start.character,
            line.find("Has(s: Circle(radius: q))")
                .expect("the dead arm") as u32
        );
    }

    #[test]
    fn a_wildcard_after_arms_that_cover_every_case_is_dead_and_a_live_one_is_not() {
        let dead = "variant E { A, B }\nconst v = match (e) { A => 1, B => 2, _ => 3 };\n";
        let found = hints(dead);
        assert_eq!(found.len(), 1, "{found:?}");
        let line = dead.lines().nth(1).expect("the match line");
        assert_eq!(
            found[0].range.start.character,
            line.find("_ =>").expect("the wildcard arm") as u32
        );

        let live = "variant E { A, B }\nconst v = match (e) { A => 1, _ => 3 };\n";
        assert!(hints(live).is_empty(), "{:?}", hints(live));
    }
}
