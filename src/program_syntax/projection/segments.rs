//! Projection segment records and their interval index.

use super::*;

#[derive(Debug, Clone, Copy)]
pub(in super::super) struct ProjectionSourceSegment {
    pub(in super::super) projected: ProjectedSpan,
    pub(in super::super) source: SourceSpan,
    pub(in super::super) kind: ProjectionSegmentKind,
}

/// The segments a projection records, in the order the projection keeps
/// them while it writes. A construct that wraps text already written records
/// its own segment before that text's: the segments recorded in front are
/// held apart, newest first, so recording one costs nothing however many
/// came before.
#[derive(Debug, Default)]
pub(in super::super) struct SegmentList {
    front: Vec<ProjectionSourceSegment>,
    back: Vec<ProjectionSourceSegment>,
}

impl SegmentList {
    pub(in super::super) fn len(&self) -> usize {
        self.front.len() + self.back.len()
    }

    pub(in super::super) fn push(&mut self, segment: ProjectionSourceSegment) {
        self.back.push(segment);
    }

    pub(in super::super) fn push_front(&mut self, segment: ProjectionSourceSegment) {
        self.front.push(segment);
    }

    pub(in super::super) fn insert(&mut self, index: usize, segment: ProjectionSourceSegment) {
        match index.checked_sub(self.front.len()) {
            Some(index) => self.back.insert(index, segment),
            None => self.front.insert(self.front.len() - index, segment),
        }
    }

    /// The segments from position `index` on.
    pub(in super::super) fn since(
        &self,
        index: usize,
    ) -> impl DoubleEndedIterator<Item = &ProjectionSourceSegment> {
        self.front[..self.front.len().saturating_sub(index)]
            .iter()
            .rev()
            .chain(&self.back[index.saturating_sub(self.front.len())..])
    }

    pub(in super::super) fn into_vec(self) -> Vec<ProjectionSourceSegment> {
        let mut segments = self.front;
        segments.reverse();
        segments.extend(self.back);
        segments
    }
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

    pub(in super::super) fn starting_in(
        &self,
        low: ProjectedByte,
        high: ProjectedByte,
    ) -> Vec<usize> {
        self.index.starting_in(low.0, high.0)
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
