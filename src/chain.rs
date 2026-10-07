use std::sync::Arc;

pub(crate) struct Chain<T>(Option<Arc<Link<T>>>);

struct Link<T> {
    item: T,
    rest: Chain<T>,
    len: usize,
}

impl<T> Chain<T> {
    pub(crate) const fn new() -> Self {
        Self(None)
    }

    pub(crate) fn cons(item: T, rest: Chain<T>) -> Self {
        let len = rest.len() + 1;
        Self(Some(Arc::new(Link { item, rest, len })))
    }

    pub(crate) fn len(&self) -> usize {
        self.0.as_ref().map_or(0, |link| link.len)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub(crate) fn first(&self) -> Option<&T> {
        self.0.as_deref().map(|link| &link.item)
    }

    pub(crate) fn rest(&self) -> Option<&Chain<T>> {
        self.0.as_deref().map(|link| &link.rest)
    }

    pub(crate) fn iter(&self) -> Iter<'_, T> {
        Iter(self)
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.iter().nth(index)
    }

    pub(crate) fn skip(&self, count: usize) -> &Chain<T> {
        let mut chain = self;
        for _ in 0..count {
            match chain.rest() {
                Some(rest) => chain = rest,
                None => break,
            }
        }
        chain
    }

    pub(crate) fn same(&self, other: &Chain<T>) -> bool {
        match (&self.0, &other.0) {
            (Some(left), Some(right)) => Arc::ptr_eq(left, right),
            (None, None) => true,
            _ => false,
        }
    }

    pub(crate) fn identity(&self) -> usize {
        self.0.as_ref().map_or(0, |link| Arc::as_ptr(link) as usize)
    }
}

impl<T: Clone> Chain<T> {
    pub(crate) fn from_vec(items: Vec<T>) -> Self {
        items
            .into_iter()
            .rev()
            .fold(Chain::new(), |rest, item| Chain::cons(item, rest))
    }
}

impl<T> Clone for Chain<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for Chain<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PartialEq> PartialEq for Chain<T> {
    fn eq(&self, other: &Self) -> bool {
        let (mut left, mut right) = (self, other);
        loop {
            if left.same(right) {
                return true;
            }
            match (&left.0, &right.0) {
                (Some(l), Some(r)) if l.len == r.len && l.item == r.item => {
                    left = &l.rest;
                    right = &r.rest;
                }
                _ => return false,
            }
        }
    }
}

impl<T: Eq> Eq for Chain<T> {}

impl<T: std::fmt::Debug> std::fmt::Debug for Chain<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

pub(crate) struct Iter<'a, T>(&'a Chain<T>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let link = self.0.0.as_deref()?;
        self.0 = &link.rest;
        Some(&link.item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}

impl<'a, T> IntoIterator for &'a Chain<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T> Drop for Chain<T> {
    fn drop(&mut self) {
        let mut next = self.0.take();
        while let Some(link) = next {
            match Arc::try_unwrap(link) {
                Ok(mut link) => next = link.rest.0.take(),
                Err(_) => break,
            }
        }
    }
}

pub(crate) struct ChainSlice<T> {
    chain: Chain<T>,
    len: usize,
}

impl<T> ChainSlice<T> {
    pub(crate) fn new() -> Self {
        Self {
            chain: Chain::new(),
            len: 0,
        }
    }

    pub(crate) fn whole(chain: Chain<T>) -> Self {
        let len = chain.len();
        Self { chain, len }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn iter(&self) -> std::iter::Take<Iter<'_, T>> {
        self.chain.iter().take(self.len)
    }

    pub(crate) fn last(&self) -> Option<&T> {
        self.len.checked_sub(1).and_then(|index| self.get(index))
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        (index < self.len).then(|| self.chain.get(index)).flatten()
    }

    pub(crate) fn skip(&self, count: usize) -> Self {
        let count = count.min(self.len);
        Self {
            chain: self.chain.skip(count).clone(),
            len: self.len - count,
        }
    }

    pub(crate) fn take(&self, count: usize) -> Self {
        Self {
            chain: self.chain.clone(),
            len: count.min(self.len),
        }
    }

    pub(crate) fn is_whole(&self) -> bool {
        self.len == self.chain.len()
    }
}

impl<T: Clone> ChainSlice<T> {
    pub(crate) fn to_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }
}

impl<T> Clone for ChainSlice<T> {
    fn clone(&self) -> Self {
        Self {
            chain: self.chain.clone(),
            len: self.len,
        }
    }
}

impl<T> Default for ChainSlice<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PartialEq> PartialEq for ChainSlice<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        if self.chain.same(&other.chain) {
            return true;
        }
        if self.is_whole() && other.is_whole() {
            return self.chain == other.chain;
        }
        self.iter().eq(other.iter())
    }
}

impl<T: Eq> Eq for ChainSlice<T> {}

impl<T: std::fmt::Debug> std::fmt::Debug for ChainSlice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, T> IntoIterator for &'a ChainSlice<T> {
    type Item = &'a T;
    type IntoIter = std::iter::Take<Iter<'a, T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub(crate) struct Segments<T>(Option<Arc<Segment<T>>>);

struct Segment<T> {
    earlier: Segments<T>,
    own: Vec<T>,
    len: usize,
}

impl<T> Segments<T> {
    pub(crate) const fn new() -> Self {
        Self(None)
    }

    pub(crate) fn extend(earlier: Segments<T>, own: Vec<T>) -> Self {
        if own.is_empty() {
            return earlier;
        }
        let len = earlier.len() + own.len();
        Self(Some(Arc::new(Segment { earlier, own, len })))
    }

    pub(crate) fn len(&self) -> usize {
        self.0.as_ref().map_or(0, |segment| segment.len)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub(crate) fn identity(&self) -> usize {
        self.0
            .as_ref()
            .map_or(0, |segment| Arc::as_ptr(segment) as usize)
    }

    pub(crate) fn earlier(&self) -> Option<&Segments<T>> {
        self.0.as_deref().map(|segment| &segment.earlier)
    }

    pub(crate) fn own(&self) -> &[T] {
        self.0.as_deref().map_or(&[], |segment| &segment.own)
    }

    fn segments(&self) -> Vec<&[T]> {
        let mut found = Vec::new();
        let mut at = self;
        while let Some(segment) = at.0.as_deref() {
            found.push(segment.own.as_slice());
            at = &segment.earlier;
        }
        found.reverse();
        found
    }

    pub(crate) fn iter(&self) -> std::iter::Flatten<std::vec::IntoIter<&[T]>> {
        self.segments().into_iter().flatten()
    }

    pub(crate) fn first(&self) -> Option<&T> {
        self.iter().next()
    }

    pub(crate) fn fresh(&self, seen: &mut std::collections::HashSet<usize>) -> Vec<&T> {
        let mut found = Vec::new();
        let mut at = self;
        while let Some(segment) = at.0.as_deref() {
            if !seen.insert(at.identity()) {
                break;
            }
            found.push(segment.own.as_slice());
            at = &segment.earlier;
        }
        found.into_iter().rev().flatten().collect()
    }
}

impl<T: Clone> Segments<T> {
    pub(crate) fn from_vec(items: Vec<T>) -> Self {
        Self::extend(Segments::new(), items)
    }

    pub(crate) fn to_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }

    #[cfg(test)]
    pub(crate) fn edit(&mut self, edit: impl FnOnce(&mut Vec<T>)) {
        let mut items = self.to_vec();
        edit(&mut items);
        *self = Self::from_vec(items);
    }
}

impl<T> Clone for Segments<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for Segments<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PartialEq> PartialEq for Segments<T> {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
            || (self.len() == other.len() && self.iter().eq(other.iter()))
    }
}

impl<T: Eq> Eq for Segments<T> {}

impl<T: std::fmt::Debug> std::fmt::Debug for Segments<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Drop for Segments<T> {
    fn drop(&mut self) {
        let mut next = self.0.take();
        while let Some(segment) = next {
            match Arc::try_unwrap(segment) {
                Ok(mut segment) => next = segment.earlier.0.take(),
                Err(_) => break,
            }
        }
    }
}

impl<'a, T> IntoIterator for &'a Segments<T> {
    type Item = &'a T;
    type IntoIter = std::iter::Flatten<std::vec::IntoIter<&'a [T]>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
