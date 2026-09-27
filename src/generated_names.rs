use std::collections::{HashMap, HashSet};

use crate::SourceKind;

pub(crate) const PREFIX: &str = "$tt_";

#[derive(Debug, Clone, Default)]
pub(crate) struct GeneratedNames {
    occupied: HashSet<String>,
    assigned: HashMap<String, String>,
    allocated: HashSet<String>,
}

impl GeneratedNames {
    pub(crate) fn for_source(source: &str, source_kind: SourceKind) -> Self {
        Self::from_occupied(source_names(source, source_kind), HashSet::new())
    }

    pub(crate) fn from_occupied(occupied: HashSet<String>, allocated: HashSet<String>) -> Self {
        Self {
            occupied,
            assigned: HashMap::new(),
            allocated,
        }
    }

    pub(crate) fn into_allocated(self) -> HashSet<String> {
        self.allocated
    }

    pub(crate) fn stable(&mut self, base: &str) -> String {
        if let Some(name) = self.assigned.get(base) {
            return name.clone();
        }
        let name = allocate(base, &mut self.occupied)
            .unwrap_or_else(|| crate::ice::bug!("no free generated name remains for {base}"));
        self.assigned.insert(base.to_owned(), name.clone());
        self.allocated.insert(name.clone());
        name
    }
}

pub(crate) fn source_names(source: &str, source_kind: SourceKind) -> HashSet<String> {
    crate::lexer::identifier_names_with_prefix(source, source_kind, PREFIX)
}

pub(crate) fn allocate(base: &str, occupied: &mut HashSet<String>) -> Option<String> {
    allocate_after(base, occupied, &mut 0)
}

pub(crate) fn allocate_after(
    base: &str,
    occupied: &mut HashSet<String>,
    taken: &mut u32,
) -> Option<String> {
    let mut suffix = *taken;
    loop {
        crate::work::tick("generated name probes");
        let candidate = if suffix == 0 {
            base.to_owned()
        } else {
            format!("{base}_{suffix}")
        };
        if occupied.insert(candidate.clone()) {
            *taken = suffix.saturating_add(1);
            return Some(candidate);
        }
        suffix = suffix.checked_add(1)?;
    }
}
