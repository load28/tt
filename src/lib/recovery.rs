//! Source-coordinate restoration for parser-owned editor insertions.

use super::*;

pub(super) struct EditorSource {
    pub text: String,
    /// Inserted byte ranges in repaired-source coordinates.
    inserted: Vec<(usize, usize)>,
}

impl EditorSource {
    pub fn new(source: &str, mut insertions: Vec<(usize, String)>) -> Self {
        insertions.sort_by_key(|(at, _)| *at);
        let mut text = String::new();
        let mut inserted = Vec::new();
        let mut from = 0;
        for (at, token) in insertions {
            text.push_str(&source[from..at]);
            let start = text.len();
            text.push_str(&token);
            text.push('\n');
            inserted.push((start, text.len()));
            from = at;
        }
        text.push_str(&source[from..]);
        Self { text, inserted }
    }

    fn original(&self, at: usize) -> usize {
        at - self
            .inserted
            .iter()
            .map(|&(start, end)| at.saturating_sub(start).min(end - start))
            .sum::<usize>()
    }

    pub fn restore(&self, mut emit: MappedEmit) -> MappedEmit {
        for glue in &mut emit.inserted {
            glue.src = self.original(glue.src);
        }
        let mut mappings = Vec::new();
        for mapping in emit.mappings {
            let end = mapping.src + mapping.len;
            let mut from = mapping.src;
            for &(start, stop) in &self.inserted {
                if stop <= from || start >= end {
                    continue;
                }
                if from < start {
                    mappings.push(EmitMapping {
                        src: self.original(from),
                        out: mapping.out + from - mapping.src,
                        len: start - from,
                    });
                }
                let glue_start = from.max(start);
                let glue_end = stop.min(end);
                emit.inserted.push(InsertedGlue {
                    src: self.original(glue_start),
                    out: mapping.out + glue_start - mapping.src,
                    out_end: mapping.out + glue_end - mapping.src,
                });
                from = glue_end;
            }
            if from < end {
                mappings.push(EmitMapping {
                    src: self.original(from),
                    out: mapping.out + from - mapping.src,
                    len: end - from,
                });
            }
        }
        emit.mappings = mappings;
        debug_assert!(crate::typescript::mapper::in_output_order(&emit.mappings));
        for mark in &mut emit.scrutinee_temps {
            mark.src = self.original(mark.src);
        }
        for mark in &mut emit.payload_temps {
            mark.src = self.original(mark.src);
        }
        for anchor in &mut emit.anchors {
            anchor.src = self.original(anchor.src);
            anchor.src_end = self.original(anchor.src_end);
            anchor.owner_end = self.original(anchor.owner_end);
            anchor.context = anchor
                .context
                .map(|(start, end)| (self.original(start), self.original(end)));
        }
        for mark in &mut emit.result_return_temps {
            mark.src = self.original(mark.src);
            mark.src_end = self.original(mark.src_end);
        }
        for name in &mut emit.declared_names {
            name.src = self.original(name.src);
            name.src_end = self.original(name.src_end);
        }
        for binding in &mut emit.shared_bindings {
            for occurrence in &mut binding.occurrences {
                occurrence.src = self.original(occurrence.src);
                occurrence.src_end = self.original(occurrence.src_end);
            }
        }
        for list in &mut emit.destructured_lists {
            list.src = self.original(list.src);
            list.src_end = self.original(list.src_end);
        }
        emit.inserted.sort_by_key(|glue| glue.out);
        for scope in &mut emit.completion_scopes {
            scope.source.start = self.original(scope.source.start);
            scope.source.end = self.original(scope.source.end);
            for host in &mut scope.hosts {
                host.start = self.original(host.start);
                host.end = self.original(host.end);
            }
        }
        emit
    }
}
