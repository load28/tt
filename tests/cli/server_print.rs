//! `--server`'s `print` and `dependencies` answer what `-p` and
//! `--dependencies` print, from one session (TASK-567).

use super::*;

/// One session asked every request in order; its answers, by request.
fn server_answers(requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let input: String = requests
        .iter()
        .enumerate()
        .map(|(id, request)| {
            let mut request = request.clone();
            request["id"] = serde_json::json!(id);
            format!("{request}\n")
        })
        .collect();
    let (lines, status) = server_lines(input.as_bytes());
    assert!(status.success(), "{lines:?}");
    assert_eq!(lines.len(), requests.len(), "{lines:#?}");
    lines
        .iter()
        .enumerate()
        .map(|(id, line)| {
            let answer: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(answer["id"], id, "{answer}");
            answer
        })
        .collect()
}

fn project() -> Workspace {
    let dir = tmpdir();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        dir.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"target":"esnext","module":"esnext","moduleResolution":"bundler","noEmit":true,"jsx":"preserve"},"include":["src"]}"#,
    )
    .unwrap();
    fs::write(
        src.join("state.tt"),
        "export variant State { Ready(value: number), Empty }\n\
         type Item = { kind: \"item\"; run: (x: number) => number };\n\
         declare function consume(item: Item): void;\n\
         export function use(state: State) {\n\
         \x20 consume(match (state) {\n\
         \x20   Ready(value) => { const doubled = value * 2; return { kind: \"item\", run: x => x + doubled }; },\n\
         \x20   Empty => ({ kind: \"item\", run: x => x }),\n\
         \x20 });\n\
         \x20 return match (state) { Ready(value) => [value], Empty => [] };\n\
         }\n",
    )
    .unwrap();
    fs::write(
        src.join("uses.tt"),
        "#!/usr/bin/env node\n\
         import { State } from \"./state.tt\";\n\
         import { Some, None } from \"@tt/std/option\";\n\
         export const label = (state: State) => match (state) { Ready(value) => Some(value), Empty => None };\n",
    )
    .unwrap();
    fs::write(
        src.join("view.ttx"),
        "variant Tone { Loud, Quiet }\n\
         export const view = (tone: Tone) => <main>{match (tone) { Loud => <b>!</b>, Quiet => null }}</main>;\n",
    )
    .unwrap();
    fs::write(
        src.join("crlf.tt"),
        "variant O { Some(value: number), None }\r\nexport const f = (o: O) => match (o) {\r\n  Some(value) => value,\r\n  None => 0,\r\n};\r\n",
    )
    .unwrap();
    fs::write(
        src.join("plain.ts"),
        "import { label } from \"./uses.tt\";\nexport const again = label;\n",
    )
    .unwrap();
    fs::write(
        src.join("bad.tt"),
        "variant State { Ready, Empty }\ndeclare const state: State;\nexport const value = match (state) { Ready => 1 };\n",
    )
    .unwrap();
    dir
}

#[test]
fn server_print_answers_exactly_what_print_prints() {
    let dir = project();
    let files = [
        "state.tt",
        "uses.tt",
        "view.ttx",
        "crlf.tt",
        "plain.ts",
        "bad.tt",
        "missing.tt",
    ];
    let option_sets: [(&[&str], serde_json::Value); 4] = [
        (&[], serde_json::json!({})),
        (
            &["--rewrite-imports", "off", "--source-map", "inline"],
            serde_json::json!({ "rewriteImports": "off", "sourceMap": "inline" }),
        ),
        (
            &["--rewrite-imports", "ts", "--no-banner", "--no-verify"],
            serde_json::json!({ "rewriteImports": "ts", "banner": false, "verify": false }),
        ),
        (
            &["--source-map", "off", "--rewrite-imports", "js"],
            serde_json::json!({ "sourceMap": "off", "rewriteImports": "js" }),
        ),
    ];
    let mut requests = Vec::new();
    let mut expected = Vec::new();
    for file in files {
        let path = dir.join("src").join(file);
        let path = path.to_str().unwrap();
        for (args, params) in &option_sets {
            let mut params = params.clone();
            params["path"] = serde_json::json!(path);
            requests.push(serde_json::json!({ "method": "print", "params": params }));
            let mut command = vec!["-p"];
            command.extend_from_slice(args);
            command.push(path);
            expected.push((command.join(" "), ttc(&command)));
        }
    }
    let answers = server_answers(&requests);
    for ((command, output), answer) in expected.iter().zip(&answers) {
        let result = &answer["result"];
        let stderr: String = result["messages"]
            .as_array()
            .unwrap_or_else(|| panic!("{command}: {answer}"))
            .iter()
            .map(|message| format!("{}\n", message.as_str().unwrap()))
            .collect();
        assert_eq!(stderr, String::from_utf8_lossy(&output.stderr), "{command}");
        if output.status.success() {
            assert_eq!(
                result["code"].as_str(),
                Some(String::from_utf8(output.stdout.clone()).unwrap().as_str()),
                "{command}"
            );
        } else {
            assert!(output.stdout.is_empty(), "{command}");
            assert_eq!(result["code"], serde_json::Value::Null, "{command}");
        }
    }
    assert!(
        expected.iter().any(|(_, output)| !output.status.success()),
        "no failing case was compared"
    );
    if common::toolchain() {
        let refined = answers[0]["result"]["code"].as_str().unwrap();
        assert!(refined.contains(": number[];"), "{refined}");
    }
}

#[test]
fn server_print_rejects_what_print_rejects() {
    let answers = server_answers(&[
        serde_json::json!({ "method": "print", "params": { "path": "x.tt", "sourceMap": "file" } }),
        serde_json::json!({ "method": "print", "params": { "path": "x.tt", "rewriteImports": "cjs" } }),
        serde_json::json!({ "method": "print", "params": {} }),
    ]);
    for answer in answers {
        assert!(answer["error"].is_string(), "{answer}");
    }
}

#[test]
fn server_dependencies_answer_what_dependencies_prints() {
    let dir = project();
    fs::remove_file(dir.join("src/bad.tt")).unwrap();
    let files = ["state.tt", "uses.tt", "view.ttx", "state.tt"];
    let requests: Vec<_> = files
        .iter()
        .map(|file| {
            let path = dir.join("src").join(file);
            serde_json::json!({ "method": "dependencies", "params": { "path": path } })
        })
        .collect();
    let answers = server_answers(&requests);
    for (file, answer) in files.iter().zip(&answers) {
        let path = dir.join("src").join(file);
        let output = ttc(&["--dependencies", path.to_str().unwrap()]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let printed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(answer["result"], printed, "{file}");
    }
}

/// `dependencies` of `files`, asked of one session in order, against what
/// `--dependencies` prints for each on its own.
fn assert_dependencies_match(files: &[std::path::PathBuf]) {
    let requests: Vec<_> = files
        .iter()
        .map(|path| serde_json::json!({ "method": "dependencies", "params": { "path": path } }))
        .collect();
    for (path, answer) in files.iter().zip(server_answers(&requests)) {
        let output = ttc(&["--dependencies", path.to_str().unwrap()]);
        if output.status.success() {
            let printed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(answer["result"], printed, "{}", path.display());
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let message = stderr.trim_end().strip_prefix("ttc: ").unwrap_or(&stderr);
            assert_eq!(answer["error"], message, "{}", path.display());
        }
    }
}

#[test]
fn server_dependencies_check_a_file_its_configuration_leaves_out_as_a_root() {
    if !common::toolchain() {
        return;
    }
    // A configuration TypeScript cannot read admits nothing; a file it
    // leaves out still depends on what its own program reads.
    let broken = Workspace::in_repo("server-dependencies-broken");
    fs::write(
        broken.join("tsconfig.json"),
        r#"{ "compilerOptions": { "strict": true, }"#,
    )
    .unwrap();
    fs::write(
        broken.join("a.tt"),
        "declare const flag: boolean;\nexport const v = match (flag) { true => [1], false => [] };\n",
    )
    .unwrap();
    assert_dependencies_match(&[broken.join("a.tt"), broken.join("a.tt")]);

    // A file outside `include`, asked after one inside it: the check that
    // answered the first did not cover the second.
    let dir = Workspace::in_repo("server-dependencies-excluded");
    fs::create_dir(dir.join("shared")).unwrap();
    fs::write(
        dir.join("shared/helper.ts"),
        "export const helper = (x: number) => x + 1;\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("app/src")).unwrap();
    fs::write(
        dir.join("app/tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"module":"preserve","moduleResolution":"bundler","allowImportingTsExtensions":true,"noEmit":true},"include":["src"]}"#,
    )
    .unwrap();
    fs::write(dir.join("app/src/in.tt"), "export const one = 1;\n").unwrap();
    fs::write(
        dir.join("app/out.tt"),
        "import { helper } from \"../shared/helper.ts\";\nexport const two = helper(1);\n",
    )
    .unwrap();
    assert_dependencies_match(&[dir.join("app/src/in.tt"), dir.join("app/out.tt")]);
}

#[test]
fn server_dependencies_refuse_what_dependencies_refuses_in_its_words() {
    let dir = project();
    assert_dependencies_match(&[
        dir.join("src/gone.tt"),
        dir.join("missing/gone.tt"),
        dir.join("src/plain.ts"),
    ]);
}
