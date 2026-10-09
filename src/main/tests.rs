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

#[test]
fn expanding_inputs_measures_each_path_a_bounded_number_of_times() {
    let measured = |files: usize| {
        let dir =
            std::env::temp_dir().join(format!("ttc-identities-{}-{files}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for index in 0..files {
            std::fs::write(dir.join(format!("f{index}.tt")), "export const v = 1;\n").unwrap();
        }
        build::IDENTITIES_MEASURED.with(|count| count.set(0));
        let jobs = build::build_jobs(&[dir.to_string_lossy().into_owned()], None, true).unwrap();
        let count = build::IDENTITIES_MEASURED.with(std::cell::Cell::get);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(jobs.len(), files);
        count
    };
    let small = measured(40);
    let large = measured(80);
    assert!(
        large <= 2 * small + 8,
        "{small} path identities for 40 files but {large} for 80"
    );
}
