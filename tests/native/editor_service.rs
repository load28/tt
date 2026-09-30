#[test]
fn an_unfinished_if_let_leaves_the_rest_of_the_file_served() {
    require_tsgo!();
    let decl = "import type { TOption } from \"@tt/std\";\n\
declare function find(id: string): TOption<{ name: string }>;\n";
    for head in [
        "if let Some(value: w) = find(id)",
        "if let Some(",
        "if let Some(v) = find(id.)",
        "if let Some(v) =",
    ] {
        let source = format!(
            "{decl}export function f(id: string) {{\n  {head}\n  const b = find(id);\n  return b.kind;\n}}\n\
export function g(n: number) {{ return n.toFixed(); }}\n"
        );
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);

        let later = utf16_position(&source, "toFixed()");
        let completion = project
            .completion(&file, ttc::engine::Position { character: later.character + 3, ..later }, true)
            .unwrap();
        assert!(
            completion.items.iter().any(|item| item.label == "toFixed"),
            "{source}"
        );
        let hover = project
            .hover(&file, utf16_position(&source, "kind;"))
            .unwrap()
            .expect("hover after the unfinished if let");
        assert!(hover.signature.contains("kind"), "{source}");
        let names: Vec<_> = project
            .document_symbols(&file)
            .unwrap()
            .into_iter()
            .map(|symbol| symbol.name)
            .collect();
        assert!(names.contains(&"g".to_string()), "{source}: {names:?}");
        let diagnostics = listed(&project.service_diagnostics(&file).unwrap());
        assert!(
            diagnostics.iter().all(|d| d.3 != "'}' expected."),
            "{source}: {diagnostics:?}"
        );
    }
}

#[test]
fn the_operand_of_an_unfinished_if_let_is_served() {
    require_tsgo!();
    let (source, position) = at_cursor(
        "import type { TOption } from \"@tt/std\";\n\
declare function find(id: string): TOption<{ name: string }>;\n\
export function f(id: string) {\n  if let Some(v) = find(id.@@)\n  return 1;\n}\n",
    );
    let dir = project(&[("src/main.tt", &source)]);
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let mut project = open_service(&file);
    let completion = project.completion(&file, position, true).unwrap();
    assert!(
        completion.items.iter().any(|item| item.label == "charAt"),
        "{:?}",
        completion.items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    let help = project
        .signature_help(&file, position)
        .unwrap()
        .expect("signature help in the operand");
    assert_eq!(
        help.signatures[0].label,
        "find(id: string): TOption<{ name: string; }>"
    );
    assert_eq!(
        listed(&project.service_diagnostics(&file).unwrap()),
        vec![(3, 27, 1003, "Identifier expected.".to_string())]
    );
}

#[test]
fn signature_help_answers_for_the_source_call_around_generated_calls() {
    require_tsgo!();
    let decl = "const half = (n: number) => n / 2;\n\
const obj = { twice(n: number) { return n * 2; } };\n";
    let cases: [(&str, Option<(&str, u32)>); 7] = [
        ("console.log(m |> ha@@lf, m);", Some(("log(...data: any[]): void", 0))),
        ("console.log(m |> half@@);", Some(("log(...data: any[]): void", 0))),
        (
            "Math.max(1, m |> half |> Str@@ing);",
            Some(("max(...values: number[]): number", 0)),
        ),
        ("half(m |> ha@@lf);", Some(("half(n: number): number", 0))),
        ("const x = 4 |> obj.tw@@ice;", None),
        ("const g = flow |> half |> Str@@ing;", None),
        ("const y = m |> (v => v@@ + 1);", None),
    ];
    for (statement, expected) in cases {
        let (source, position) = at_cursor(&format!(
            "{decl}export function f(m: number) {{\n  {statement}\n}}\n"
        ));
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        let help = project.signature_help(&file, position).unwrap();
        let answer = help.map(|help| {
            (
                help.signatures[help.active_signature as usize].label.clone(),
                help.active_parameter,
            )
        });
        assert_eq!(
            answer,
            expected.map(|(label, parameter)| (label.to_string(), parameter)),
            "{statement}"
        );
    }
}
