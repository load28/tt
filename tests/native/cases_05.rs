/// One source per line-break style, each with the same four statements:
/// a declaration, a `val` binding, a type error on its third line, and a
/// mutation through the binding on its fourth.
fn line_break_sources() -> Vec<(&'static str, String)> {
    let statements = |prefix: &str| {
        [
            "export const value = 1;".to_string(),
            "val const b = { c: 1 };".to_string(),
            format!("{prefix}export const wrong: string = value;"),
            "b.c = 2;".to_string(),
        ]
    };
    let joined = |separators: [&str; 4], prefix: &str| {
        statements(prefix)
            .iter()
            .zip(separators)
            .map(|(statement, separator)| format!("{statement}{separator}"))
            .collect::<String>()
    };
    vec![
        ("lf", joined(["\n"; 4], "")),
        ("crlf", joined(["\r\n"; 4], "")),
        ("cr", joined(["\r"; 4], "")),
        ("ls", joined(["\u{2028}", "\u{2028}", "\u{2028}", "\n"], "")),
        ("ps", joined(["\u{2029}", "\u{2029}", "\u{2029}", "\n"], "")),
        ("mixed", joined(["\r\n", "\r", "\u{2028}", "\n"], "")),
        ("bom", format!("\u{feff}{}", joined(["\n"; 4], ""))),
        ("astral", joined(["\r"; 4], "/*\u{1F389}*/ ")),
    ]
}

/// The zero-based line and UTF-16 character of `byte` under the editor
/// protocol's line breaks — LF, CR LF and CR — after any byte-order mark.
fn protocol_position(text: &str, byte: usize) -> (usize, usize) {
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let byte = byte - (text.len() - body.len());
    let (mut line, mut character) = (0, 0);
    let mut chars = body[..byte].chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' if chars.peek() == Some(&'\n') => {}
            '\r' | '\n' => (line, character) = (line + 1, 0),
            ch => character += ch.len_utf16(),
        }
    }
    (line, character)
}

/// The 1-based line and code-point column `tsc` renders `byte` at: ECMA-262
/// line terminators, CR LF as one.
fn rendered_position(text: &str, byte: usize) -> (usize, usize) {
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let byte = byte - (text.len() - body.len());
    let (mut line, mut column) = (1, 1);
    let mut chars = body[..byte].chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' if chars.peek() == Some(&'\n') => {}
            '\r' | '\n' | '\u{2028}' | '\u{2029}' => (line, column) = (line + 1, 1),
            _ => column += 1,
        }
    }
    (line, column)
}

#[test]
fn every_line_break_style_reports_positions_in_each_consumers_lines() {
    use std::io::Write;
    require_tsgo!();
    let sources = line_break_sources();
    let files: Vec<(String, &String)> = sources
        .iter()
        .map(|(name, source)| (format!("src/{name}.tt"), source))
        .collect();
    let dir = project(
        &files
            .iter()
            .map(|(path, source)| (path.as_str(), source.as_str()))
            .collect::<Vec<_>>(),
    );

    let rendered = check(&dir);
    for (name, source) in &sources {
        for (needle, code) in [("wrong", "ts2322"), ("b.c", "val-mutation")] {
            let (line, column) = rendered_position(source, source.rfind(needle).unwrap());
            let location = format!("src/{name}.tt:{line}:{column}");
            let block = block(&rendered, &location);
            assert!(block.contains(code), "{name}: {block}");
        }
        assert_eq!(
            rendered_position(source, source.find("b.c").unwrap()),
            (4, 1),
            "{name}"
        );
    }

    let mut requests = Vec::new();
    for (index, (name, source)) in sources.iter().enumerate() {
        let path = dir.join(format!("src/{name}.tt")).canonicalize().unwrap();
        let id = index as u64 * 10;
        let (line, character) = protocol_position(source, source.rfind("b.c").unwrap());
        let (value_line, value_character) =
            protocol_position(source, source.rfind("value;").unwrap());
        requests.extend([
            serde_json::json!({ "id": id, "method": "typedCheck",
                "params": { "path": path, "text": source, "includeTypes": true } }),
            serde_json::json!({ "id": id + 1, "method": "definition",
                "params": { "path": path, "position": { "line": line, "character": character } } }),
            serde_json::json!({ "id": id + 2, "method": "hover",
                "params": { "path": path,
                    "position": { "line": value_line, "character": value_character } } }),
            serde_json::json!({ "id": id + 3, "method": "check",
                "params": { "text": source, "filename": format!("{name}.tt") } }),
        ]);
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("server starts");
    for request in &requests {
        writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("server answers");
    let answers: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON response"))
        .collect();
    let answer = |id: u64| {
        answers
            .iter()
            .find(|answer| answer["id"] == id)
            .unwrap_or_else(|| panic!("no answer {id}: {answers:?}"))
    };

    for (index, (name, source)) in sources.iter().enumerate() {
        let id = index as u64 * 10;
        let one_based = |byte: usize| {
            let (line, character) = protocol_position(source, byte);
            (line as u64 + 1, character as u64 + 1)
        };
        let at = |diagnostic: &serde_json::Value| {
            (
                diagnostic["line"].as_u64().unwrap(),
                diagnostic["col"].as_u64().unwrap(),
            )
        };
        let typed = answer(id)["result"]["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("{name}: {}", answer(id)))
            .iter()
            .filter(|diagnostic| {
                diagnostic["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with(&format!("{name}.tt")))
            })
            .cloned()
            .collect::<Vec<_>>();
        let of = |diagnostics: &[serde_json::Value], code: &str| {
            diagnostics
                .iter()
                .find(|diagnostic| {
                    diagnostic["code"] == code
                        || diagnostic["code"].as_u64().map(|n| format!("ts{n}")).as_deref()
                            == Some(code)
                })
                .unwrap_or_else(|| panic!("{name}: no {code} in {diagnostics:?}"))
                .clone()
        };
        assert_eq!(
            at(&of(&typed, "ts2322")),
            one_based(source.rfind("wrong").unwrap()),
            "{name}: {typed:?}"
        );
        assert_eq!(
            at(&of(&typed, "val-mutation")),
            one_based(source.find("b.c").unwrap()),
            "{name}: {typed:?}"
        );
        let untyped = answer(id + 3)["result"]["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("{name}: {}", answer(id + 3)))
            .clone();
        assert_eq!(
            at(&of(&untyped, "val-mutation")),
            one_based(source.find("b.c").unwrap()),
            "{name}: {untyped:?}"
        );
        let mutation = of(&typed, "val-mutation");
        let removal = &mutation["suggestions"][0]["edit"];
        assert_eq!(
            at(removal),
            one_based(source.find("val ").unwrap()),
            "{name}: {mutation}"
        );
        assert_eq!(
            (
                removal["endLine"].as_u64().unwrap(),
                removal["endCol"].as_u64().unwrap()
            ),
            one_based(source.find("const b").unwrap()),
            "{name}: {mutation}"
        );

        let locations = answer(id + 1)["result"]["locations"]
            .as_array()
            .unwrap_or_else(|| panic!("{name}: {}", answer(id + 1)))
            .clone();
        let declared = protocol_position(source, source.find("b =").unwrap());
        assert!(
            locations.iter().any(|location| {
                location["range"]["start"]
                    == serde_json::json!({ "line": declared.0, "character": declared.1 })
            }),
            "{name}: {locations:?}"
        );

        let hover = &answer(id + 2)["result"];
        assert!(
            hover["signature"]
                .as_str()
                .is_some_and(|signature| signature.contains("value")),
            "{name}: {hover}"
        );
        let used = protocol_position(source, source.rfind("value;").unwrap());
        assert_eq!(
            hover["range"]["start"],
            serde_json::json!({ "line": used.0, "character": used.1 }),
            "{name}: {hover}"
        );
    }
}
