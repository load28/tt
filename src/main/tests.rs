use super::*;

#[test]
fn every_topic_resolves_to_a_nonempty_section() {
    for (name, _, heading) in HELP_TOPICS {
        let section = guide_section(heading);
        assert!(
            !section.trim().is_empty(),
            "topic {name}: heading {heading:?} not found in docs/ai/tt.md"
        );
        if !heading.is_empty() {
            assert!(section.starts_with(heading), "topic {name}: wrong slice");
        }
    }
}

#[test]
fn sections_stop_at_the_next_heading() {
    let section = guide_section("## match");
    assert!(section.contains("or-pattern"));
    assert!(!section.contains("\n## try"), "section leaked past its end");
    let preamble = guide_section("");
    assert!(preamble.contains("CONTRACTS"));
    assert!(!preamble.contains("\n## "));
}

#[test]
fn topic_names_and_aliases_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for (name, aliases, _) in HELP_TOPICS {
        assert!(seen.insert(*name), "duplicate topic {name}");
        for alias in *aliases {
            assert!(seen.insert(*alias), "duplicate alias {alias}");
        }
    }
    assert!(!seen.contains("all") && !seen.contains("guide"));
}
