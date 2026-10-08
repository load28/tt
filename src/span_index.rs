#[derive(Debug)]
pub(crate) struct SpanIndex {
    by_start: Vec<usize>,
    starts: Vec<usize>,
    by_end: Vec<usize>,
    ends: Vec<usize>,
    max_end: Vec<usize>,
    leaves: usize,
}

impl SpanIndex {
    pub(crate) fn new(spans: impl IntoIterator<Item = (usize, usize)>) -> Self {
        let spans: Vec<(usize, usize)> = spans.into_iter().collect();
        let mut by_start: Vec<usize> = (0..spans.len()).collect();
        by_start.sort_by_key(|&index| spans[index].0);
        let mut by_end: Vec<usize> = (0..spans.len()).collect();
        by_end.sort_by_key(|&index| spans[index].1);
        let leaves = by_start.len().next_power_of_two();
        let mut max_end = vec![0; 2 * leaves];
        for (position, &index) in by_start.iter().enumerate() {
            max_end[leaves + position] = spans[index].1;
        }
        for node in (1..leaves).rev() {
            max_end[node] = max_end[2 * node].max(max_end[2 * node + 1]);
        }
        Self {
            starts: by_start.iter().map(|&index| spans[index].0).collect(),
            ends: by_end.iter().map(|&index| spans[index].1).collect(),
            by_start,
            by_end,
            max_end,
            leaves,
        }
    }

    pub(crate) fn covering(&self, start: usize, end: usize) -> Vec<usize> {
        let started = self.starts.partition_point(|&at| at <= start);
        let mut found = Vec::new();
        self.collect(1, 0, self.leaves, started, end, &mut found);
        found.sort_unstable();
        found
    }

    /// Calls `visit` with every span that contains `at`, in no particular
    /// order.
    pub(crate) fn each_containing(&self, at: usize, mut visit: impl FnMut(usize)) {
        let Some(after) = at.checked_add(1) else {
            return;
        };
        let started = self.starts.partition_point(|&start| start <= at);
        self.visit(1, 0, self.leaves, started, after, &mut visit);
    }

    fn visit(
        &self,
        node: usize,
        low: usize,
        high: usize,
        limit: usize,
        end: usize,
        visit: &mut impl FnMut(usize),
    ) {
        if low >= limit || self.max_end[node] < end {
            return;
        }
        if high - low == 1 {
            visit(self.by_start[low]);
            return;
        }
        let middle = low + (high - low) / 2;
        self.visit(2 * node, low, middle, limit, end, visit);
        self.visit(2 * node + 1, middle, high, limit, end, visit);
    }

    /// The spans that start at `at`, in no particular order.
    pub(crate) fn each_starting_at(&self, at: usize) -> impl Iterator<Item = usize> + '_ {
        let first = self.starts.partition_point(|&start| start < at);
        let last = self.starts.partition_point(|&start| start <= at);
        self.by_start[first..last].iter().copied()
    }

    /// The spans that end at `at`, in no particular order.
    pub(crate) fn each_ending_at(&self, at: usize) -> impl Iterator<Item = usize> + '_ {
        let first = self.ends.partition_point(|&end| end < at);
        let last = self.ends.partition_point(|&end| end <= at);
        self.by_end[first..last].iter().copied()
    }

    pub(crate) fn containing(&self, at: usize) -> Vec<usize> {
        match at.checked_add(1) {
            Some(after) => self.covering(at, after),
            None => Vec::new(),
        }
    }

    pub(crate) fn any_containing(&self, at: usize) -> bool {
        let Some(after) = at.checked_add(1) else {
            return false;
        };
        let started = self.starts.partition_point(|&start| start <= at);
        self.reaches(1, 0, self.leaves, started, after)
    }

    pub(crate) fn starting_after(&self, after: usize) -> impl Iterator<Item = usize> + '_ {
        let first = self.starts.partition_point(|&start| start <= after);
        self.by_start[first..].iter().copied()
    }

    pub(crate) fn starting_in(&self, low: usize, high: usize) -> Vec<usize> {
        let first = self.starts.partition_point(|&start| start < low);
        let last = self.starts.partition_point(|&start| start < high);
        let mut found = self.by_start[first..last.max(first)].to_vec();
        found.sort_unstable();
        found
    }

    pub(crate) fn ending_in(&self, low: usize, high: usize) -> Vec<usize> {
        let first = self.ends.partition_point(|&end| end < low);
        let last = self.ends.partition_point(|&end| end < high);
        let mut found = self.by_end[first..last.max(first)].to_vec();
        found.sort_unstable();
        found
    }

    fn collect(
        &self,
        node: usize,
        low: usize,
        high: usize,
        limit: usize,
        end: usize,
        found: &mut Vec<usize>,
    ) {
        if low >= limit || self.max_end[node] < end {
            return;
        }
        if high - low == 1 {
            found.push(self.by_start[low]);
            return;
        }
        let middle = low + (high - low) / 2;
        self.collect(2 * node, low, middle, limit, end, found);
        self.collect(2 * node + 1, middle, high, limit, end, found);
    }

    fn reaches(&self, node: usize, low: usize, high: usize, limit: usize, end: usize) -> bool {
        if low >= limit || self.max_end[node] < end {
            return false;
        }
        if high - low == 1 {
            return true;
        }
        let middle = low + (high - low) / 2;
        self.reaches(2 * node, low, middle, limit, end)
            || self.reaches(2 * node + 1, middle, high, limit, end)
    }
}

/// Spans ordered by start, the wider of two spans with one start first, so
/// a walk over a range meets each outermost span before the spans inside it.
#[derive(Debug, Default)]
pub(crate) struct NestedOrder {
    starts: Vec<usize>,
    ends: Vec<usize>,
    indices: Vec<usize>,
    max_end: Vec<usize>,
    min_end: Vec<usize>,
    leaves: usize,
}

impl NestedOrder {
    pub(crate) fn new(spans: impl IntoIterator<Item = (usize, usize)>) -> Self {
        let spans: Vec<(usize, usize)> = spans.into_iter().collect();
        let mut indices: Vec<usize> = (0..spans.len()).collect();
        indices.sort_by_key(|&index| (spans[index].0, std::cmp::Reverse(spans[index].1), index));
        let leaves = indices.len().next_power_of_two();
        let mut max_end = vec![0; 2 * leaves];
        for (position, &index) in indices.iter().enumerate() {
            max_end[leaves + position] = spans[index].1;
        }
        let mut min_end = vec![usize::MAX; 2 * leaves];
        for (position, &index) in indices.iter().enumerate() {
            min_end[leaves + position] = spans[index].1;
        }
        for node in (1..leaves).rev() {
            max_end[node] = max_end[2 * node].max(max_end[2 * node + 1]);
            min_end[node] = min_end[2 * node].min(min_end[2 * node + 1]);
        }
        Self {
            starts: indices.iter().map(|&index| spans[index].0).collect(),
            ends: indices.iter().map(|&index| spans[index].1).collect(),
            indices,
            max_end,
            min_end,
            leaves,
        }
    }

    /// The spans that contain `start..end`, each once: the innermost first
    /// when `innermost` is set, the outermost first otherwise.
    pub(crate) fn containing(
        &self,
        start: usize,
        end: usize,
        innermost: bool,
    ) -> impl Iterator<Item = usize> + '_ {
        self.containing_within(start, end, usize::MAX, innermost)
    }

    /// [`Self::containing`], keeping only the spans that end at or before
    /// `bound`.
    pub(crate) fn containing_within(
        &self,
        start: usize,
        end: usize,
        bound: usize,
        innermost: bool,
    ) -> impl Iterator<Item = usize> + '_ {
        let limit = self.starts.partition_point(|&at| at <= start);
        let mut stack = vec![(1usize, 0usize, self.leaves)];
        std::iter::from_fn(move || {
            while let Some((node, low, high)) = stack.pop() {
                if low >= limit || self.max_end[node] < end || self.min_end[node] > bound {
                    continue;
                }
                if high - low == 1 {
                    return Some(self.indices[low]);
                }
                let middle = low + (high - low) / 2;
                if innermost {
                    stack.push((2 * node, low, middle));
                    stack.push((2 * node + 1, middle, high));
                } else {
                    stack.push((2 * node + 1, middle, high));
                    stack.push((2 * node, low, middle));
                }
            }
            None
        })
    }

    /// The first position at or after `after` whose span starts at or after
    /// `cursor` and ends at or before `bound`.
    pub(crate) fn next_within(&self, cursor: usize, bound: usize, after: usize) -> Option<usize> {
        let mut at = after.max(self.starts.partition_point(|&start| start < cursor));
        while at < self.starts.len() && self.starts[at] <= bound {
            let start = self.starts[at];
            let group = self.starts.partition_point(|&other| other <= start);
            let within = at + self.ends[at..group].partition_point(|&end| end > bound);
            if within < group {
                return Some(within);
            }
            at = group;
        }
        None
    }

    pub(crate) fn span(&self, position: usize) -> (usize, usize) {
        (self.starts[position], self.ends[position])
    }

    pub(crate) fn index(&self, position: usize) -> usize {
        self.indices[position]
    }
}

pub(crate) fn innermost_containers(
    spans: &[(usize, usize)],
    queries: &[(usize, usize)],
) -> Vec<Option<usize>> {
    let mut ends: Vec<usize> = spans.iter().map(|&(_, end)| end).collect();
    ends.sort_unstable();
    ends.dedup();
    let mut by_start: Vec<usize> = (0..spans.len()).collect();
    by_start.sort_by_key(|&index| spans[index].0);
    let mut order: Vec<usize> = (0..queries.len()).collect();
    order.sort_by_key(|&query| queries[query].0);
    let mut shortest = ShortestByEnd::new(ends.len());
    let mut inserted = 0;
    let mut answers = vec![None; queries.len()];
    let mut group = 0;
    while group < order.len() {
        let start = queries[order[group]].0;
        let last = group + order[group..].partition_point(|&query| queries[query].0 == start);
        while inserted < by_start.len() && spans[by_start[inserted]].0 < start {
            let index = by_start[inserted];
            shortest.insert(&ends, spans[index], index);
            inserted += 1;
        }
        for &query in &order[group..last] {
            answers[query] = shortest.ending_from(&ends, queries[query].1);
        }
        while inserted < by_start.len() && spans[by_start[inserted]].0 == start {
            let index = by_start[inserted];
            shortest.insert(&ends, spans[index], index);
            inserted += 1;
        }
        for &query in &order[group..last] {
            let (_, end) = queries[query];
            let after = end
                .checked_add(1)
                .and_then(|after| shortest.ending_from(&ends, after));
            answers[query] = [answers[query], after]
                .into_iter()
                .flatten()
                .min_by_key(|&index| (spans[index].1 - spans[index].0, index));
        }
        group = last;
    }
    answers
}

struct ShortestByEnd {
    tree: Vec<Option<(usize, usize)>>,
}

impl ShortestByEnd {
    fn new(len: usize) -> Self {
        ShortestByEnd {
            tree: vec![None; len + 1],
        }
    }

    fn insert(&mut self, ends: &[usize], (start, end): (usize, usize), index: usize) {
        let key = (end - start, index);
        let mut position = ends.len() - ends.partition_point(|&at| at < end);
        while position < self.tree.len() {
            if self.tree[position].is_none_or(|current| key < current) {
                self.tree[position] = Some(key);
            }
            position += position.isolate_lowest_one();
        }
    }

    fn ending_from(&self, ends: &[usize], end: usize) -> Option<usize> {
        let mut position = ends.len() - ends.partition_point(|&at| at < end);
        let mut best: Option<(usize, usize)> = None;
        while position > 0 {
            if let Some(key) = self.tree[position]
                && best.is_none_or(|current| key < current)
            {
                best = Some(key);
            }
            position -= position.isolate_lowest_one();
        }
        best.map(|(_, index)| index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPANS: [(usize, usize); 9] = [
        (10, 40),
        (0, 5),
        (12, 14),
        (12, 30),
        (3, 3),
        (20, 21),
        (35, 60),
        (12, 14),
        (40, 40),
    ];

    fn scan(keep: impl Fn(usize, usize) -> bool) -> Vec<usize> {
        (0..SPANS.len())
            .filter(|&index| keep(SPANS[index].0, SPANS[index].1))
            .collect()
    }

    #[test]
    fn every_query_answers_what_a_scan_of_the_spans_answers() {
        let index = SpanIndex::new(SPANS);
        for at in 0..65 {
            assert_eq!(
                index.containing(at),
                scan(|start, end| start <= at && at < end)
            );
            assert_eq!(
                index.any_containing(at),
                !scan(|start, end| start <= at && at < end).is_empty()
            );
            let mut after = scan(|start, _| at < start);
            after.sort_by_key(|&index| SPANS[index].0);
            assert_eq!(index.starting_after(at).collect::<Vec<_>>(), after);
            for to in at..65 {
                assert_eq!(
                    index.covering(at, to),
                    scan(|start, end| start <= at && to <= end)
                );
                assert_eq!(
                    index.starting_in(at, to),
                    scan(|start, _| at <= start && start < to)
                );
                assert_eq!(
                    index.ending_in(at, to),
                    scan(|_, end| at <= end && end < to)
                );
            }
        }
    }

    #[test]
    fn a_containing_walk_meets_what_a_scan_of_the_spans_meets_in_order() {
        let order = NestedOrder::new(SPANS);
        for at in 0..65 {
            for to in at..65 {
                let mut outer = scan(|start, end| start <= at && to <= end);
                outer.sort_by_key(|&index| {
                    (SPANS[index].0, std::cmp::Reverse(SPANS[index].1), index)
                });
                assert_eq!(order.containing(at, to, false).collect::<Vec<_>>(), outer);
                for bound in to..65 {
                    let within: Vec<usize> = outer
                        .iter()
                        .copied()
                        .filter(|&index| SPANS[index].1 <= bound)
                        .collect();
                    assert_eq!(
                        order
                            .containing_within(at, to, bound, false)
                            .collect::<Vec<_>>(),
                        within
                    );
                }
                outer.reverse();
                assert_eq!(order.containing(at, to, true).collect::<Vec<_>>(), outer);
            }
        }
    }

    #[test]
    fn a_nested_walk_meets_what_a_scan_of_the_spans_meets_in_order() {
        let order = NestedOrder::new(SPANS);
        for cursor in 0..65 {
            for bound in cursor..65 {
                let mut expected: Vec<usize> = scan(|start, end| cursor <= start && end <= bound);
                expected.sort_by_key(|&index| {
                    (SPANS[index].0, std::cmp::Reverse(SPANS[index].1), index)
                });
                let mut walked = Vec::new();
                let mut after = 0;
                while let Some(position) = order.next_within(cursor, bound, after) {
                    walked.push(order.index(position));
                    after = position + 1;
                }
                assert_eq!(walked, expected);
            }
        }
    }

    #[test]
    fn the_innermost_container_is_what_a_scan_of_the_spans_answers() {
        let queries: Vec<_> = (0..65)
            .flat_map(|start| (start..65).map(move |end| (start, end)))
            .collect();
        let answers = innermost_containers(&SPANS, &queries);
        for (&(at, to), answer) in queries.iter().zip(answers) {
            let scanned = (0..SPANS.len())
                .filter(|&index| {
                    let (start, end) = SPANS[index];
                    start <= at && to <= end && (start, end) != (at, to)
                })
                .min_by_key(|&index| SPANS[index].1 - SPANS[index].0);
            assert_eq!(answer, scanned, "{at}..{to}");
        }
    }

    #[test]
    fn an_empty_index_answers_nothing() {
        let index = SpanIndex::new([]);
        assert!(index.containing(0).is_empty());
        assert!(!index.any_containing(0));
        assert!(index.covering(0, 0).is_empty());
        assert_eq!(index.starting_after(0).count(), 0);
    }
}
