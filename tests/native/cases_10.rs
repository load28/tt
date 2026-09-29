#[test]
fn completion_never_offers_the_pipeline_runtime_helpers() {
    require_tsgo!();
    // Each file imports one runtime helper; the other is an export of the
    // runtime TypeScript would offer to import.
    for marked in [
        "const trimmed = flow |> String |> .trim();\nconst t = @@;\n",
        "function cat(a: string, b: string): string { return a + b; }\n\
const v = 1 |> String |> cat(@@",
    ] {
        let (source, position) = at_cursor(marked);
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        let answer = project.completion(&file, position, false).unwrap();
        let generated: Vec<_> = answer
            .items
            .iter()
            .filter(|item| item.label.starts_with("$tt_"))
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(generated, Vec::<&str>::new(), "{source}");
        assert!(
            answer.items.iter().any(|item| item.label == "String"),
            "{source}"
        );
    }
}
