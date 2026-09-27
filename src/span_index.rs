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
    fn an_empty_index_answers_nothing() {
        let index = SpanIndex::new([]);
        assert!(index.containing(0).is_empty());
        assert!(!index.any_containing(0));
        assert!(index.covering(0, 0).is_empty());
        assert_eq!(index.starting_after(0).count(), 0);
    }
}
