//! Apply checker-provided type annotations at explicit declaration sites.

use crate::MappedEmit;

pub(crate) fn insert_annotations(emit: &mut MappedEmit, edits: &[(usize, String)]) {
    let shifted = |position: usize, inclusive: bool| {
        position
            + edits
                .iter()
                .filter(|(at, _)| *at < position || inclusive && *at == position)
                .map(|(_, text)| text.len() + 2)
                .sum::<usize>()
    };
    for mapping in &mut emit.mappings {
        mapping.out = shifted(mapping.out, true);
    }
    for mark in &mut emit.scrutinee_temps {
        mark.out = shifted(mark.out, true);
    }
    for mark in &mut emit.payload_temps {
        mark.out = shifted(mark.out, true);
    }
    for mark in &mut emit.result_return_temps {
        mark.out = shifted(mark.out, true);
        mark.out_end = shifted(mark.out_end, false);
    }
    for anchor in &mut emit.anchors {
        anchor.out = shifted(anchor.out, true);
        anchor.end = shifted(anchor.end, false);
    }
    emit.contextual_slots
        .retain(|position| !edits.iter().any(|(at, _)| at == position));
    for position in &mut emit.contextual_slots {
        *position = shifted(*position, true);
    }
    for (position, annotation) in edits.iter().rev() {
        emit.code.insert_str(*position, &format!(": {annotation}"));
    }
}
