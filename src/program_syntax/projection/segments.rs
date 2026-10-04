//! Projection segment records and their interval index.

use super::*;

#[derive(Debug, Clone, Copy)]
pub(in super::super) struct ProjectionSourceSegment {
    pub(in super::super) projected: ProjectedSpan,
    pub(in super::super) source: SourceSpan,
    pub(in super::super) kind: ProjectionSegmentKind,
}

pub(in super::super) struct ProjectionSegments {
    segments: Vec<ProjectionSourceSegment>,
    index: crate::span_index::SpanIndex,
}

impl ProjectionSegments {
    pub(in super::super) fn new(segments: Vec<ProjectionSourceSegment>) -> Self {
        let index = crate::span_index::SpanIndex::new(
            segments
                .iter()
                .map(|segment| (segment.projected.start.0, segment.projected.end.0)),
        );
        Self { segments, index }
    }

    pub(in super::super) fn starting_at(&self, at: ProjectedByte) -> Vec<usize> {
        self.index.starting_in(at.0, at.0.saturating_add(1))
    }

    pub(in super::super) fn ending_at(&self, at: ProjectedByte) -> Vec<usize> {
        self.index.ending_in(at.0, at.0.saturating_add(1))
    }

    pub(in super::super) fn containing(&self, at: ProjectedByte) -> Vec<usize> {
        self.index.containing(at.0)
    }
}

impl std::ops::Deref for ProjectionSegments {
    type Target = [ProjectionSourceSegment];

    fn deref(&self) -> &Self::Target {
        &self.segments
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum ProjectionSegmentKind {
    Copied,
    /// Compiler-written delimiter that closes a copied source fragment.
    /// A parser stopping here proves that the fragment immediately before
    /// it was incomplete; the delimiter itself is fixed syntax. Its source
    /// is the first significant source byte after the fragment, the token
    /// that ends the fragment in the source and where TypeScript's parser
    /// stops on the same text.
    SourceBoundary,
    Placeholder,
    AutomaticSemicolon,
}
