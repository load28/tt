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

#[test]
fn an_unfinished_pipeline_step_leaves_the_rest_of_the_function_checked() {
    require_tsgo!();
    // TypeScript on `const n = xs.length +` reports only the missing
    // operand; the head, the statement after it, and the function's
    // return stay what they are.
    for next in ["return n;", "const m = n + 1; return m;"] {
        let source = format!(
            "export function run(xs: number[]): number {{\n  const n = xs |> .length |> \n  {next}\n}}\n"
        );
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        assert_eq!(
            listed(&project.service_diagnostics(&file).unwrap()),
            vec![],
            "{source}"
        );
        let hover = project
            .hover(&file, utf16_position(&source, "xs |>"))
            .unwrap()
            .expect("hover on the head");
        assert_eq!(hover.signature, "(parameter) xs: number[]");
    }
}

#[test]
fn a_syntax_error_in_a_match_arm_is_reported_where_typescript_puts_it() {
    require_tsgo!();
    // In the `.ts` twin TypeScript reports at the token after the arm body,
    // the `,`; the glue `;` stands where that token was.
    let decl = "variant Shape { Circle(radius: number), Square(side: number) }\n\
declare const s: Shape;\n";
    for (name, body, code, message) in [
        ("main.tt", "radius.", 1003, "Identifier expected."),
        ("main.tt", "radius *", 1109, "Expression expected."),
        ("main.ttx", "radius.", 1003, "Identifier expected."),
    ] {
        let source = format!(
            "{decl}export const a = match (s) {{\n  Circle(radius) => {body},\n  Square(side) => side,\n}};\n"
        );
        let dir = project(&[(&format!("src/{name}"), &source)]);
        let file = dir.join("src").join(name).canonicalize().unwrap();
        let mut project = open_service(&file);
        let comma = utf16_position(&source, &format!("{body},")).character + body.len() as u32;
        assert_eq!(
            listed(&project.service_diagnostics(&file).unwrap()),
            vec![(3, comma, code, message.to_string())],
            "{source}"
        );
    }
}

#[test]
fn an_unfinished_pipeline_call_step_answers_signature_help() {
    require_tsgo!();
    let cat = "function cat(a: string, b: string): string { return a + b; }\n";
    let add = "function add(a: number, b: number): number { return a + b; }\n";
    for (marked, label) in [
        (format!("{cat}const v = 1 |> String |> cat(\"x\", @@"), "cat(a: string, b: string)"),
        (format!("{add}const v = 1 |> add(2, @@"), "add(a: number, b: number)"),
        (format!("{add}const v = 1 |> add(2, @@\nconst w = 1;\n"), "add(a: number, b: number)"),
        (
            format!("{add}export function f() {{\n  const v = 1 |> add(2, @@\n}}\n"),
            "add(a: number, b: number)",
        ),
    ] {
        let (source, position) = at_cursor(&marked);
        let dir = project(&[("src/main.tt", &source)]);
        let file = dir.join("src/main.tt").canonicalize().unwrap();
        let mut project = open_service(&file);
        let help = project
            .signature_help(&file, position)
            .unwrap()
            .unwrap_or_else(|| panic!("no signature help: {source}"));
        assert!(
            help.signatures[help.active_signature as usize]
                .label
                .starts_with(label),
            "{source}: {help:?}"
        );
        assert_eq!(help.active_parameter, 1, "{source}");
    }
}

#[test]
fn a_type_error_keeps_its_rendering_while_an_open_document_does_not_parse() {
    require_tsgo!();
    // The typed layer is the one an editor shows once it answers; while
    // line 2 does not parse, it still checks the buffer and says the same
    // thing about line 1.
    let clean = "export const bad: number = \"x\";\nexport const y = 1 + 2;\n";
    let edited = "export const bad: number = \"x\";\nexport const y = 1 + ;\n";
    let dir = project(&[("src/main.tt", clean)]);
    let mismatch = |answer: &serde_json::Value| {
        answer["result"]["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("{answer}"))
            .iter()
            .filter(|d| d["code"] == "ts2322")
            .map(|d| (d["line"].clone(), d["col"].clone(), d["message"].clone()))
            .collect::<Vec<_>>()
    };
    let before = typed_server(&dir, "src/main.tt", clean);
    let during = typed_server(&dir, "src/main.tt", edited);
    assert_eq!(during["result"]["blocked"], false, "{during}");
    assert_eq!(mismatch(&during), mismatch(&before), "{during}");
    assert_eq!(
        mismatch(&before),
        vec![(
            serde_json::json!(1),
            serde_json::json!(28),
            serde_json::json!("type mismatch: expected `number`, found `\"x\"`"),
        )]
    );
    assert!(
        during["result"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "ts1109" && d["line"] == 2),
        "{during}"
    );

    // A file read from disk is checked as `tsc` checks it: blocked while
    // it does not parse.
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    fs::write(&file, edited).unwrap();
    let mut project = ttc::engine::Engine::new(None)
        .open_project(
            &[file.to_string_lossy().into_owned()],
            &ttc::engine::ProjectOptions::default(),
        )
        .unwrap();
    let snapshot = project.update(std::slice::from_ref(&file)).unwrap();
    assert!(snapshot.is_blocked(&file));
    project.open_document(file.clone(), edited.to_string());
    let snapshot = project.update(std::slice::from_ref(&file)).unwrap();
    assert!(!snapshot.is_blocked(&file));
}

/// The parse-only surfaces read an imported `.tt` file as the session holds
/// it open: a case added to the open `st.tt` buffer is completed, hovered
/// and defined in `use.tt` before it is saved, as TypeScript's surfaces
/// already see it.
#[test]
fn tt_names_read_an_imported_declaration_from_its_open_buffer() {
    let dir = tmpdir();
    let st = dir.join("src/st.tt");
    let use_tt = dir.join("src/use.tt");
    write(&dir, "src/st.tt", "export variant St { A, B }\n");
    let edited = "export variant St { A, B, C }\n";
    let typing = "import { St } from \"./st.tt\";\nexport function g(s: St): number {\n  return match (s) {\n    A => 1,\n    \n  };\n}\n";
    let written = typing.replace("    \n  };", "    C => 3,\n    A => 4,\n  };");
    write(&dir, "src/use.tt", typing);
    let at = |text: &str, needle: &str, delta: usize| {
        let position = source_position(text, needle, delta);
        serde_json::json!({ "line": position.line, "character": position.character })
    };
    let answers = server_answers(
        &dir,
        &[
            serde_json::json!({ "id": 1, "method": "openDocument",
                "params": { "path": st, "text": edited } }),
            serde_json::json!({ "id": 2, "method": "ttCompletions",
                "params": { "path": use_tt, "text": typing,
                    "position": at(typing, "    \n  };", 4) } }),
            serde_json::json!({ "id": 3, "method": "ttSymbol",
                "params": { "path": use_tt, "text": written,
                    "position": at(&written, "C => 3", 0) } }),
            serde_json::json!({ "id": 4, "method": "declarations",
                "params": { "path": use_tt, "text": typing } }),
            serde_json::json!({ "id": 5, "method": "ttHints",
                "params": { "path": use_tt, "text": written } }),
            serde_json::json!({ "id": 6, "method": "closeDocument",
                "params": { "path": st } }),
            serde_json::json!({ "id": 7, "method": "ttSymbol",
                "params": { "path": use_tt, "text": written,
                    "position": at(&written, "C => 3", 0) } }),
        ],
    );
    let labels: Vec<_> = answers[1]["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(labels, ["A", "B", "C", "_"], "{answers:?}");
    let symbol = &answers[2]["result"];
    assert_eq!(symbol["kind"], "case", "{answers:?}");
    assert_eq!(
        symbol["definition"]["path"],
        st.canonicalize().unwrap().to_string_lossy().as_ref(),
        "{answers:?}"
    );
    assert_eq!(
        symbol["definition"]["range"]["start"],
        serde_json::json!({ "line": 0, "character": 26 }),
        "{answers:?}"
    );
    let cases: Vec<_> = answers[3]["result"]["variants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|variant| variant["name"] == "St")
        .unwrap()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["tag"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(cases, ["A", "B", "C"], "{answers:?}");
    assert_eq!(
        answers[4]["result"]["hints"].as_array().unwrap().len(),
        1,
        "the second `A` arm is unreachable: {answers:?}"
    );
    assert_eq!(
        answers[6]["result"],
        serde_json::Value::Null,
        "closed, `st.tt` is its saved text again: {answers:?}"
    );
}

/// A variant field's type is the user's TypeScript, and every service
/// feature answers there as it does in a hand-written union: hover,
/// definition, references, rename, completion and the checker's errors.
#[test]
fn a_variant_field_type_is_typescript_to_every_service_feature() {
    require_tsgo!();
    let source = "export interface Money { cents: number }\nexport variant Price { Fixed(amount: Money), Free }\nexport const toMoney = (n: number): Money => ({ cents: n });\n";
    let typo = source.replace("amount: Money", "amount: Mony");
    let typing = source.replace("amount: Money", "amount: Mo");
    let dir = project(&[("src/price.tt", source)]);
    let path = dir.join("src/price.tt");
    let at = |text: &str, needle: &str, delta: usize| {
        let position = source_position(text, needle, delta);
        serde_json::json!({ "line": position.line, "character": position.character })
    };
    let field = at(source, "amount: Money", 8);
    let answers = server_answers(
        &dir,
        &[
            serde_json::json!({ "id": 1, "method": "openDocument",
                "params": { "path": path, "text": source } }),
            serde_json::json!({ "id": 2, "method": "hover",
                "params": { "path": path, "position": field } }),
            serde_json::json!({ "id": 3, "method": "definition",
                "params": { "path": path, "position": field } }),
            serde_json::json!({ "id": 4, "method": "references",
                "params": { "path": path, "position": at(source, "Money {", 0) } }),
            serde_json::json!({ "id": 5, "method": "rename",
                "params": { "path": path, "position": field } }),
            serde_json::json!({ "id": 6, "method": "updateDocument",
                "params": { "path": path, "text": typing } }),
            serde_json::json!({ "id": 7, "method": "completion",
                "params": { "path": path, "position": at(&typing, "amount: Mo", 10) } }),
            serde_json::json!({ "id": 8, "method": "updateDocument",
                "params": { "path": path, "text": typo } }),
            serde_json::json!({ "id": 9, "method": "tsDiagnostics",
                "params": { "path": path } }),
        ],
    );
    let start = |value: &serde_json::Value| {
        (
            value["range"]["start"]["line"].as_u64().unwrap(),
            value["range"]["start"]["character"].as_u64().unwrap(),
        )
    };
    assert!(
        answers[1]["result"]["signature"]
            .as_str()
            .is_some_and(|signature| signature.contains("interface Money")),
        "{answers:?}"
    );
    let definitions = answers[2]["result"]["locations"].as_array().unwrap();
    assert_eq!(
        definitions.iter().map(start).collect::<Vec<_>>(),
        [(0, 17)],
        "{answers:?}"
    );
    let mut references: Vec<_> = answers[3]["result"]["locations"]
        .as_array()
        .unwrap()
        .iter()
        .map(start)
        .collect();
    references.sort();
    assert_eq!(references, [(0, 17), (1, 37), (2, 36)], "{answers:?}");
    let mut renamed: Vec<_> = answers[4]["result"]["edits"]
        .as_array()
        .unwrap_or_else(|| panic!("{answers:?}"))
        .iter()
        .map(start)
        .collect();
    renamed.sort();
    assert_eq!(renamed, [(0, 17), (1, 37), (2, 36)], "{answers:?}");
    let labels: Vec<_> = answers[6]["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["label"].as_str().unwrap())
        .collect();
    for expected in ["Money", "Date", "string"] {
        assert!(labels.contains(&expected), "{expected}: {labels:?}");
    }
    let diagnostics = answers[8]["result"]["diagnostics"].as_array().unwrap();
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| (start(d), d["code"].clone()))
            .collect::<Vec<_>>(),
        [((1, 37), serde_json::json!(2552))],
        "{answers:?}"
    );
}

/// A member name typed in an interpolation whose `}` is not written yet is
/// a member access, and TypeScript's members answer it, as in a `.ts` file.
#[test]
fn a_member_in_an_unterminated_interpolation_completes_members() {
    require_tsgo!();
    let dir = project(&[]);
    let path = dir.join("src/at.tt");
    let mut answers = Vec::new();
    for source in [
        "const at = new Date();\nconst s = `returned ${at.",
        "const at = new Date();\nconst s = `returned ${at.ge",
    ] {
        write(&dir, "src/at.tt", source);
        let end = source_position(source, source, source.len());
        let end = serde_json::json!({ "line": end.line, "character": end.character });
        answers.extend(server_answers(
            &dir,
            &[
                serde_json::json!({ "id": 1, "method": "openDocument",
                    "params": { "path": path, "text": source } }),
                serde_json::json!({ "id": 2, "method": "ttCompletions",
                    "params": { "path": path, "text": source, "position": end } }),
                serde_json::json!({ "id": 3, "method": "completion",
                    "params": { "path": path, "position": end, "member": true } }),
            ],
        ));
    }
    for answer in answers.chunks(3) {
        assert_eq!(
            answer[1]["result"]["member"],
            serde_json::json!({ "receiver": "at" }),
            "{answers:?}"
        );
        let labels: Vec<_> = answer[2]["result"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["label"].as_str().unwrap())
            .collect();
        assert!(labels.contains(&"getTime"), "{labels:?}");
        assert!(!labels.contains(&"match"), "{labels:?}");
    }
}

/// TASK-571: sibling `try`s in the argument of a return that leaves a
/// `result` block lower in the return's prelude, so the block's storage
/// and the checked program see only TypeScript.
#[test]
fn sibling_tries_in_a_result_return_check_clean() {
    require_tsgo!();
    let dir = project(&[(
        "src/sum.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         const a = (): TResult<number, string> => Result.Ok(1);\n\
         declare function f(x: number, y: number): number;\n\
         export function g() {\n\
         \x20 return result { return (try a()) + (try a()); };\n\
         }\n\
         export function h(c: boolean) {\n\
         \x20 return result { if (c) return f(try a(), try a()); return [try a(), try a()].length; };\n\
         }\n\
         export const n: number = g().kind === \"Ok\" ? 1 : 0;\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("error["), "{out}");
}

/// TASK-572: an operand of a comma expression before a tt value is
/// evaluated as a statement, so the lowered comma expression holds no
/// unused capture (TS2695).
#[test]
fn a_comma_operand_before_a_value_checks_clean() {
    require_tsgo!();
    let dir = project(&[(
        "src/comma.tt",
        "variant K { A, B }\n\
         declare function tick(): void;\n\
         declare function tock(): number;\n\
         export function f(k: K) {\n\
         \x20 return (tick(), match (k) { A => 1, B => 2 });\n\
         }\n\
         export function g(k: K) {\n\
         \x20 const x = (tick(), tock(), match (k) { A => 1, B => 2 });\n\
         \x20 return x;\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("error["), "{out}");
}

/// TASK-573: a method call whose argument holds a tt value keeps
/// TypeScript's facts about the call: a generic method's inference with a
/// `this` parameter, and an optional call's narrowing of its receiver.
#[test]
fn a_method_call_around_a_value_keeps_inference_and_narrowing() {
    require_tsgo!();
    let dir = project(&[(
        "src/methods.tt",
        "variant K { A, B }\n\
         class Repo {\n\
         \x20 items = [\"x\"];\n\
         \x20 first<T>(this: Repo, fallback: T): string | T { return this.items[0] ?? fallback; }\n\
         }\n\
         type O = { name: string; id<T>(x: T): T };\n\
         export function repo(k: K, r: Repo) {\n\
         \x20 const v: string | number = r.first(match (k) { A => 1, B => 2 });\n\
         \x20 return v;\n\
         }\n\
         export function optional(obj: O | undefined, k: K) {\n\
         \x20 const s: string | undefined = obj?.id(match (k) { A => obj.name, B => \"b\" });\n\
         \x20 return s;\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("error["), "{out}");
}

/// TASK-574: a result, annotation, or argument mismatch of a pipeline
/// whose last step is a postfix tail belongs to the whole pipeline, as it
/// does for a function step; only a step's input speaks as a step.
#[test]
fn a_postfix_pipeline_result_mismatch_is_reported_on_the_pipeline() {
    require_tsgo!();
    let dir = project(&[(
        "src/postfix.tt",
        "export function g(): number {\n\
         \x20 return \"a\" |> .trim();\n\
         }\n\
         export const x: number = \"a\" |> .trim() |> .toUpperCase();\n\
         declare function f(n: number): number;\n\
         export const y = f(\"a\" |> .toUpperCase());\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("this pipeline step expects"), "{out}");
    let result = block(&out, "src/postfix.tt:2:10");
    assert!(
        result.contains("error[ts2322]: type mismatch: expected `number`, found `string`"),
        "{out}"
    );
    let annotation = block(&out, "src/postfix.tt:4:26");
    assert!(annotation.contains("error[ts2322]"), "{out}");
    let argument = block(&out, "src/postfix.tt:6:20");
    assert!(argument.contains("error[ts2345]"), "{out}");
}

#[test]
fn a_later_declarator_reading_an_earlier_one_checks_clean() {
    require_tsgo!();
    let dir = project(&[(
        "src/declarators.tt",
        "variant O { A(n: number), B }\n\
         declare const o: O;\n\
         declare function t(s: string): number;\n\
         export function f() {\n\
         \x20 const a = t(\"a\"), b = match (o) { A(n) => a + n, B => 0 };\n\
         \x20 return b;\n\
         }\n\
         export function g() {\n\
         \x20 var a = 10, b = match (o) { A(n) => a + n, B => 0 };\n\
         \x20 return b;\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("error"), "{out}");
}

/// TASK-595: a value under `? :`, `&&`, or `||` sees the narrowing its
/// operation's condition gives, as in TypeScript: the lowering tests the
/// condition where it evaluates it, so the references it narrows stay
/// narrowed in the value.
#[test]
fn a_value_under_a_conditional_operation_keeps_the_conditions_narrowing() {
    require_tsgo!();
    let dir = project(&[(
        "src/narrowing.tt",
        "import type { TResult } from \"@tt/std\";\n\
         variant O { A(n: number), B }\n\
         declare function g(n: number): TResult<number, string>;\n\
         export function h(o: O, cfg: { name?: string }) {\n\
         \x20 return cfg.name ? match (o) { A(n) => cfg.name.slice(n), B => cfg.name } : \"anon\";\n\
         }\n\
         export function k(o: O, s: string | null) {\n\
         \x20 return s && match (o) { A(n) => s.charAt(n), B => s };\n\
         }\n\
         export function l(o: O, init: { v: number } | undefined) {\n\
         \x20 let u = init;\n\
         \x20 u = init;\n\
         \x20 const a = u ? match (o) { A(n) => u.v + n, B => u.v } : 0;\n\
         \x20 const b = !u || match (o) { A(n) => u.v > n, B => u.v > 0 };\n\
         \x20 const c = u !== undefined && u.v > 0 && match (o) { A(n) => u.v + n, B => 1 };\n\
         \x20 return [a, b, c];\n\
         }\n\
         export function m(s: string | null) {\n\
         \x20 return result { const x = s && try g(s.length); return x; };\n\
         }\n\
         export const e = (Math.random() > 0.5 ? { k: 1 } : null) && match (O.B as O) { A(n) => n, B => 0 };\n\
         export const f = (Math.random() > 0.5 ? [] : null) || match (O.B as O) { A(n) => n, B => 0 };\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("error"), "{out}");
}

/// TASK-596: a value under `as T`, `<T>`, or `satisfies T` gets `T` as its
/// contextual type, but its storage keeps the value's own type: an
/// assertion is not an assignment, and `satisfies` does not change the
/// type of its operand.
#[test]
fn an_asserted_value_keeps_its_own_type() {
    require_tsgo!();
    let dir = project(&[(
        "src/asserted.tt",
        "variant O { A, B }\n\
         type Ev = { kind: \"click\"; x: number } | { kind: \"key\"; code: string };\n\
         declare function read(): string;\n\
         declare const x0: unknown;\n\
         export function f(o: O, x: unknown) {\n\
         \x20 const n = match (o) { A => x, B => 0 } as number;\n\
         \x20 const m = match (o) { A => ({ a: 1 }), B => ({ a: 2 }) } satisfies { a?: number };\n\
         \x20 return n + m.a.toFixed(1).length;\n\
         }\n\
         export function g(o: O) {\n\
         \x20 const k = match (o) { A => read(), B => \"b\" } as \"a\" | \"b\";\n\
         \x20 const e = match (o) { A => ({ kind: \"click\", x: 1 }), B => ({ kind: \"key\", code: \"z\" }) } satisfies Ev;\n\
         \x20 const l = match (o) { A => 1, B => 2 } satisfies 1 | 2;\n\
         \x20 let w = match (o) { A => 1, B => 2 } satisfies number;\n\
         \x20 w = 5;\n\
         \x20 const q: string = match (o) { A => 1, B => 2 } as unknown as string;\n\
         \x20 const c = <number>match (o) { A => x0, B => 0 };\n\
         \x20 return [k, e.kind, l, w, q, c];\n\
         }\n\
         export function h(o: O): number {\n\
         \x20 return match (o) { A => x0, B => 0 } as number;\n\
         }\n\
         export function i(o: O) {\n\
         \x20 return String(match (o) { A => ({ a: 1 }), B => ({ a: 2 }) } satisfies { a?: number });\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("error"), "{out}");
}

#[test]
fn a_values_storage_widens_only_fresh_literal_types() {
    require_tsgo!();
    let dir = project(&[(
        "src/literals.tt",
        "variant O { A, B }\n\
         type Dir = \"north\" | \"south\";\n\
         declare function take(d: Dir): void;\n\
         declare function dir(): Dir;\n\
         declare function id<T>(v: T): T;\n\
         enum E { X, Y }\n\
         export function f(o: O, a: Dir, b: Dir) {\n\
         \x20 const d = match (o) { A => a, B => b };\n\
         \x20 const c = match (o) { A => \"north\" as const, B => \"south\" as const };\n\
         \x20 const t = match (o) { A => `north` as const, B => `south` as const };\n\
         \x20 const k = match (o) { A => { const x = a; return x; }, B => b };\n\
         \x20 const g = match (o) { A => dir(), B => a! };\n\
         \x20 const s = match (o) { A => a, B => b } satisfies string;\n\
         \x20 take(d); take(c); take(t); take(k); take(g); take(s);\n\
         }\n\
         export function r(o: O, a: Dir, b: Dir) { return match (o) { A => a, B => b }; }\n\
         export function u(o: O, a: Dir) { take(r(o, a, a)); }\n\
         export function w(o: O, a: Dir) {\n\
         \x20 const cc = \"north\";\n\
         \x20 let p = match (o) { A => \"x\", B => \"y\" };\n\
         \x20 p = \"z\";\n\
         \x20 let q = match (o) { A => cc, B => id(\"p\") };\n\
         \x20 q = \"z\";\n\
         \x20 let n = match (o) { A => -1, B => (true) ? 2 : 3 };\n\
         \x20 n = 7;\n\
         \x20 let e = match (o) { A => E.X, B => E.Y };\n\
         \x20 e = E.Y;\n\
         \x20 let m = match (o) { A => false, B => a };\n\
         \x20 m = true; m = \"south\";\n\
         \x20 return [p, q, n, e, m];\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("error"), "{out}");
}
