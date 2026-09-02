/// The overlay is what gets checked: a mutation that exists only in the
/// buffer is reported, and one that exists only on disk is not. This is the
/// whole point — an editor asks about the text it is showing.
#[test]
fn overlay_checks_the_buffer_rather_than_the_saved_file() {
    require_types_toolchain!();
    let saved = "val const saved = new Map<string, number>();\n\
                 saved.set(\"gone\", 1);\n\
                 export const n = saved.size;\n";
    let buffer = "val const edited = new Map<string, number>();\n\
                  edited.delete(\"new\");\n\
                  export const n = edited.size;\n";

    let err = types_stderr_overlay(saved, buffer, true);
    assert!(
        err.contains("cannot call mutating method `delete` through val binding `edited`"),
        "the buffer's mutation should be reported:\n{err}"
    );
    assert!(
        !err.contains("saved"),
        "the saved file's text should not be checked:\n{err}"
    );
    // The position is the buffer's, and the file is named as the user knows
    // it — not as a temporary.
    assert!(err.contains("--> src/main.tt:2:1"), "{err}");
}

/// `--tt-only` drops TypeScript's layer and keeps tt's. The editor uses this
/// form when its type-diagnostic setting is disabled.
#[test]
fn tt_only_keeps_the_tt_layer_and_drops_the_type_layer() {
    require_types_toolchain!();
    let source = "val const scores = new Map<string, number>();\n\
                  scores.set(\"a\", 1);\n\
                  const wrong: number = \"not a number\";\n\
                  export const n = scores.size + wrong;\n";

    let full = types_stderr_overlay(source, source, false);
    assert!(
        full.contains("type mismatch: expected `number`, found `\"not a number\"`"),
        "{full}"
    );
    assert!(full.contains("cannot call mutating method `set`"), "{full}");

    let tt_only = types_stderr_overlay(source, source, true);
    assert!(
        !tt_only.contains("type mismatch:"),
        "no type error should survive --tt-only:\n{tt_only}"
    );
    assert!(
        tt_only.contains("cannot call mutating method `set`"),
        "{tt_only}"
    );
}

/// A `val` mutation is judged by what the receiver *is*, and the overlay
/// keeps the buffer in its own project — so a type that comes from another
/// module of the project still resolves.
#[test]
fn overlay_keeps_the_buffer_in_its_project() {
    require_types_toolchain!();
    let dir = tmpdir();
    let src = dir.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("store.ts"),
        "export const scores = new Map<string, number>();\n\
         export class Query { set(k: string): Query { return this; } }\n",
    )
    .unwrap();
    let file = src.join("main.tt");
    fs::write(&file, "export const nothing = 0;\n").unwrap();

    let buffer = "import { scores, Query } from \"./store\";\n\
                  val const shared = scores;\n\
                  shared.set(\"a\", 1);\n\
                  val const query = new Query();\n\
                  query.set(\"b\");\n\
                  export const n = shared.size;\n";

    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args([
            "--check-types",
            "--tt-only",
            "--overlay",
            file.to_str().unwrap(),
            "src",
        ])
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run ttc");
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(buffer.as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("failed to run ttc");
    let err = String::from_utf8_lossy(&out.stderr).into_owned();

    // `Map#set` through the imported binding is a built-in mutation …
    assert!(
        err.contains("cannot call mutating method `set` through val binding `shared`"),
        "{err}"
    );
    // … and `Query#set`, which only shares the name, is not.
    assert!(!err.contains("`query`"), "{err}");
}

/// `ttc --server` answers `ttSymbol` without a project or a toolchain: the
/// names it resolves exist only in `.tt` source, so nothing else can.
#[test]
fn the_server_resolves_tt_names_without_a_toolchain() {
    use std::io::Write;
    let dir = tmpdir();
    let file = dir.join("shape.tt");
    let source = "variant Shape { Circle(radius: number), Point }\n\
                  const a = match (s) { Circle(radius) => radius, Point => 0 };\n";
    fs::write(&file, source).unwrap();

    let request = serde_json::json!({
        "id": 1,
        "method": "ttSymbol",
        "params": {
            "path": file.to_string_lossy(),
            "text": source,
            // line 1, on the `Circle` of the match arm
            "position": { "line": 1, "character": 22 },
        },
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("server starts");
    writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("server answers");
    let answer: serde_json::Value =
        serde_json::from_slice(String::from_utf8_lossy(&out.stdout).trim().as_bytes())
            .expect("one JSON line");
    assert_eq!(answer["result"]["kind"], "case");
    assert_eq!(answer["result"]["variantName"], "Shape");
    assert!(answer["result"].get("enumName").is_none());
    assert_eq!(
        answer["result"]["signature"],
        "Shape.Circle(radius: number)"
    );
    // ...and points at the declaration on line 0.
    assert_eq!(answer["result"]["definition"]["range"]["start"]["line"], 0);
}

/* ------------------------------------------------------------------ */
/* typed check without a backend (TASK-124)                            */
/* ------------------------------------------------------------------ */

#[test]
fn a_missing_backend_still_reports_tt_diagnostics() {
    // The TypeScript layer failing to run removes the typed facts, not
    // the pass: tt's own diagnostics are still reported in full, the
    // failure is named, and the exit code stays "could not check" (2).
    let dir = tmpdir();
    fs::write(
        dir.join("a.tt"),
        "variant E { A(x: number), B }\n\
         const v = match (E.A(1)) { A(x) => x, A(x) => 0, B => 1 };\n",
    )
    .unwrap();
    // A temporary directory has no `node_modules` above it, so the project
    // has no TypeScript and the backend cannot run — the one way to say
    // that now that a toolchain comes from the project and nowhere else.
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", dir.to_str().unwrap()])
        .output()
        .expect("failed to run ttc");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("duplicate arm"), "{stderr}");
    assert!(
        stderr.contains("only tt-level diagnostics are shown"),
        "{stderr}"
    );
}

#[test]
fn a_build_writes_no_source_map_unless_it_is_asked_for() {
    // TASK-200: emitting a map appends a `sourceMappingURL` line, and a
    // hand-written `.ts` passes through byte for byte by contract — so the
    // default has to be off.
    let dir = tmpdir();
    let source = dir.join("a.tt");
    let out_dir = dir.join("out");
    fs::write(
        &source,
        "variant E { A(v: number), B }\nexport const n = match (E.B) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let out = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        "--no-banner",
        source.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{out:?}");
    let code = fs::read_to_string(out_dir.join("a.ts")).unwrap();
    assert!(!code.contains("sourceMappingURL"), "{code}");
    assert!(!out_dir.join("a.ts.map").exists());
}

#[test]
fn a_source_map_file_lands_beside_its_output_and_names_the_tt_source() {
    let dir = tmpdir();
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let source = src_dir.join("a.tt");
    let out_dir = dir.join("out");
    fs::write(
        &source,
        "variant E { A(v: number), B }\nexport const n = match (E.B) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let out = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        "--source-map",
        "file",
        "--no-banner",
        source.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{out:?}");
    let code = fs::read_to_string(out_dir.join("a.ts")).unwrap();
    assert!(code.ends_with("//# sourceMappingURL=a.ts.map\n"), "{code}");
    let map = fs::read_to_string(out_dir.join("a.ts.map")).unwrap();
    assert!(map.contains("\"version\":3"), "{map}");
    assert!(map.contains("\"file\":\"a.ts\""), "{map}");
    // The map sits in `out/`, the source in `src/`; the name it records has
    // to resolve from the map's own directory even on a first build, when
    // `out/` did not exist while the map was being built.
    assert!(map.contains("\"sources\":[\"../src/a.tt\"]"), "{map}");
    assert!(map.contains("\"sourcesContent\""), "{map}");
}

#[test]
fn an_inline_source_map_travels_with_printed_output() {
    let dir = tmpdir();
    let source = dir.join("a.tt");
    fs::write(
        &source,
        "variant E { A(v: number), B }\nexport const n = match (E.B) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let out = ttc(&[
        "-p",
        "--no-banner",
        "--source-map",
        "inline",
        source.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{out:?}");
    let code = String::from_utf8(out.stdout).unwrap();
    let marker = "//# sourceMappingURL=data:application/json;charset=utf-8;base64,";
    let at = code.find(marker).expect("inline map");
    let encoded = code[at + marker.len()..].trim_end();
    let json = String::from_utf8(base64_decode(encoded)).expect("utf-8 map");
    assert!(json.contains("\"version\":3"), "{json}");
    assert!(json.contains("a.tt"), "{json}");
}

/// Minimal Base64 decoder for the inline-map test — the test decodes what
/// the compiler encoded rather than comparing encoded text.
fn base64_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits = 0u32;
    let mut count = 0u32;
    let mut out = Vec::new();
    for byte in text.bytes().filter(|b| *b != b'=') {
        let value = ALPHABET
            .iter()
            .position(|c| *c == byte)
            .unwrap_or_else(|| panic!("not base64: {byte}")) as u32;
        bits = (bits << 6) | value;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    out
}

#[test]
fn a_pass_through_file_keeps_its_bytes_even_when_maps_are_on() {
    // Invariant 1: a valid `.ts` is copied byte for byte. There is no
    // translation for a map to describe, so none is written.
    let dir = tmpdir();
    let source = dir.join("plain.ts");
    let out_dir = dir.join("out");
    let text = "export const x: number = 1;\n";
    fs::write(&source, text).unwrap();
    let out = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        "--source-map",
        "file",
        "--no-banner",
        source.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(fs::read_to_string(out_dir.join("plain.ts")).unwrap(), text);
    assert!(!out_dir.join("plain.ts.map").exists());
}

#[test]
fn a_banner_shifts_the_map_so_positions_still_line_up() {
    let dir = tmpdir();
    let source = dir.join("a.tt");
    let out_dir = dir.join("out");
    fs::write(
        &source,
        "variant E { A(v: number), B }\nexport const n = match (E.B) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let with_banner = ttc(&[
        "-o",
        out_dir.to_str().unwrap(),
        "--source-map",
        "file",
        source.to_str().unwrap(),
    ]);
    assert!(with_banner.status.success(), "{with_banner:?}");
    let map = fs::read_to_string(out_dir.join("a.ts.map")).unwrap();
    let mappings = map
        .split("\"mappings\":\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("mappings");
    // The banner is one generated line with nothing behind it.
    assert!(mappings.starts_with(';'), "{mappings}");
    assert!(!mappings.starts_with(";;"), "{mappings}");
}

/* ------------------------------------------------------------------ */
/* ttc explain                                                        */
/* ------------------------------------------------------------------ */

#[test]
fn explain_prints_the_rule_behind_a_code() {
    let out = ttc(&["explain", "match-not-exhaustive"]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("error[match-not-exhaustive]"), "{text}");
    assert!(text.contains("does not cover every case"), "{text}");
    // Longer than the message it explains — that is the point of it.
    assert!(text.lines().count() > 4, "{text}");
}

#[test]
fn explain_accepts_a_code_pasted_from_a_build_log() {
    // What a reader copies is `error[val-mutation]`, brackets and all.
    let out = ttc(&["explain", "error[val-mutation]"]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("error[val-mutation]"), "{text}");
}

#[test]
fn explain_with_no_code_lists_every_rule() {
    let out = ttc(&["explain"]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    for code in ttc::DiagnosticCode::ALL {
        assert!(
            text.contains(code.as_str()),
            "{} missing:\n{text}",
            code.as_str()
        );
    }
}

#[test]
fn explain_names_the_list_when_the_code_is_unknown() {
    let out = ttc(&["explain", "no-such-rule"]);
    assert!(!out.status.success(), "{out:?}");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("unknown diagnostic code"), "{err}");
    assert!(err.contains("ttc explain"), "{err}");
}

/* ------------------------------------------------------------------ */
/* rendered diagnostics                                               */
/* ------------------------------------------------------------------ */

#[test]
fn a_diagnostic_is_rendered_with_its_rule_position_snippet_and_fix() {
    // The whole user-visible contract of a tt error, in one place: the
    // rule's code names it, `-->` places it, the snippet quotes the line
    // the reader has to change, the carets cover the construct as written,
    // and `= help:` says what to write instead.
    let dir = tmpdir();
    let source = dir.join("shapes.tt");
    fs::write(
        &source,
        "variant Shape { Circle(radius: number), Empty }\nconst a = match (s) { Circel(radius) => radius, Empty => 0 };\n",
    )
    .unwrap();
    let out = ttc(&["--check", source.to_str().unwrap()]);
    assert!(!out.status.success(), "expected a failing exit code");
    let err = String::from_utf8(out.stderr).unwrap();
    let rendered: Vec<&str> = err.lines().map(|line| line.trim_end()).collect();

    assert_eq!(
        rendered[0],
        "error[unknown-case]: variant Shape has no case `Circel`",
    );
    assert!(rendered[1].ends_with("shapes.tt:2:23"), "{err}");
    assert!(rendered[1].trim_start().starts_with("-->"), "{err}");
    assert_eq!(rendered[2], "  |");
    assert_eq!(
        rendered[3],
        "2 | const a = match (s) { Circel(radius) => radius, Empty => 0 };",
    );
    assert_eq!(
        rendered[4], "  |                       ^^^^^^",
        "the carets cover the tag as written\n{err}",
    );
    assert_eq!(rendered[5], "  |");
    assert_eq!(
        rendered[6],
        "  = help: a case with a similar name exists: `Circle`",
    );
}

#[test]
fn the_rendered_code_is_the_one_explain_answers_to() {
    // A reader's path out of a diagnostic: read the code off the header,
    // paste it into `ttc explain`. That only works if they are the same
    // string, so this pins the round trip rather than each half.
    let dir = tmpdir();
    let source = dir.join("holes.tt");
    fs::write(
        &source,
        "variant Shape { Circle(r: number), Empty }\nconst a = match (s) { Circle(r) => r };\n",
    )
    .unwrap();
    let out = ttc(&["--check", source.to_str().unwrap()]);
    let err = String::from_utf8(out.stderr).unwrap();
    let code = err
        .split_once("error[")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(code, _)| code.to_string())
        .unwrap_or_else(|| panic!("no rendered code:\n{err}"));
    assert_eq!(code, "match-not-exhaustive");

    let explained = ttc(&["explain", &code]);
    assert!(explained.status.success(), "`ttc explain {code}` failed");
}

/* ------------------------------------------------------------------ */
/* the panic safety net (TASK-214)                                    */
/* ------------------------------------------------------------------ */

/// `ttc` with the environment a test needs. `TTC_PANIC_FOR_TEST` makes a
/// debug build fail at a named point, which is the only way to observe
/// what the compiler does when the compiler itself is wrong.
fn ttc_failing_at(point: &str, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(args)
        .env("TTC_PANIC_FOR_TEST", point)
        // The report offers a backtrace; a test reads the report.
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("failed to run ttc")
}

#[test]
fn a_compiler_bug_is_reported_as_a_bug_and_names_the_file() {
    let dir = tmpdir();
    let source = dir.join("main.tt");
    fs::write(&source, "const a = 1;\n").unwrap();
    let out = ttc_failing_at("compile", &["--check", source.to_str().unwrap()]);

    // 101 is what a Rust panic exits with; keeping it means a caller that
    // already distinguishes "crashed" from "your code is wrong" still can.
    assert_eq!(out.status.code(), Some(101), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.starts_with("error: internal compiler error:"),
        "the report has to lead, not a backtrace: {err}"
    );
    assert!(
        err.contains(&format!("while compiling: {}", source.display())),
        "the report names the file it was working on: {err}"
    );
    assert!(
        err.contains("This is a bug in ttc, not in the code it was given"),
        "a reader must not think their own file is at fault: {err}"
    );
    assert!(err.contains("github.com/load28/tt/issues"), "{err}");
    assert!(
        err.contains("RUST_BACKTRACE=1"),
        "and must be told how to attach a backtrace: {err}"
    );
}

#[test]
fn the_server_answers_a_failed_request_and_keeps_the_session() {
    // The protocol promises a failed request never ends the session. A
    // panic is a failed request, so it may not be the exception.
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .env("TTC_PANIC_FOR_TEST", "server")
        .env_remove("RUST_BACKTRACE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn the server");
    let requests = "{\"id\":1,\"method\":\"check\",\"params\":{\"text\":\"const a = 1;\\n\"}}\n\
                    {\"id\":2,\"method\":\"check\",\"params\":{\"text\":\"const b = 2;\\n\"}}\n";
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(requests.as_bytes())
        .unwrap();
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();

    // Both questions were answered, in order, each with its own id — the
    // second one is the whole point: the session outlived the first panic.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let answers: Vec<&str> = stdout.lines().collect();
    assert_eq!(answers.len(), 2, "both requests answered: {stdout}");
    for (index, answer) in answers.iter().enumerate() {
        let value: serde_json::Value = serde_json::from_str(answer).unwrap();
        assert_eq!(value["id"], serde_json::json!(index + 1), "{answer}");
        assert!(
            value["error"]
                .as_str()
                .unwrap_or_default()
                .contains("internal compiler error"),
            "{answer}"
        );
    }
    // Stdin closing is the only thing that ends the session.
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    // And the bug still reached a human, on the stream that is not the
    // protocol's.
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        err.matches("This is a bug in ttc").count(),
        2,
        "one report per panic: {err}"
    );
}
