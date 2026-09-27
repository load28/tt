use std::collections::{HashMap, HashSet};

use crate::SourceKind;

pub(crate) const PREFIX: &str = "$tt_";

#[derive(Debug, Clone, Default)]
pub(crate) struct GeneratedNames {
    occupied: HashSet<String>,
    assigned: HashMap<String, String>,
}

impl GeneratedNames {
    pub(crate) fn for_source(source: &str, source_kind: SourceKind) -> Self {
        Self::from_occupied(source_names(source, source_kind))
    }

    pub(crate) fn from_occupied(occupied: HashSet<String>) -> Self {
        Self {
            occupied,
            assigned: HashMap::new(),
        }
    }

    pub(crate) fn stable(&mut self, base: &str) -> String {
        if let Some(name) = self.assigned.get(base) {
            return name.clone();
        }
        let name = allocate(base, &mut self.occupied)
            .unwrap_or_else(|| crate::ice::bug!("no free generated name remains for {base}"));
        self.assigned.insert(base.to_owned(), name.clone());
        name
    }
}

pub(crate) fn source_names(source: &str, source_kind: SourceKind) -> HashSet<String> {
    crate::lexer::identifier_names_with_prefix(source, source_kind, PREFIX)
}

pub(crate) fn allocate(base: &str, occupied: &mut HashSet<String>) -> Option<String> {
    if occupied.insert(base.to_owned()) {
        return Some(base.to_owned());
    }
    let mut suffix = 1u32;
    loop {
        let candidate = format!("{base}_{suffix}");
        if occupied.insert(candidate.clone()) {
            return Some(candidate);
        }
        suffix = suffix.checked_add(1)?;
    }
}
