/// A diagnostic is rendered at the line `tsc` would name — ECMA-262's line
/// terminators, CR LF as one — whichever of them the file uses (TASK-498).
#[test]
fn a_diagnostic_is_rendered_on_the_line_every_terminator_starts() {
    let dir = tmpdir();
    let body = |breaks: [&str; 3], prefix: &str| {
        format!(
            "export {{}};{}val const b = {{ c: 1 }};{}{prefix}b.c = 2;{}",
            breaks[0], breaks[1], breaks[2]
        )
    };
    let cases = [
        ("lf", body(["\n"; 3], ""), "3:1", "b.c = 2;"),
        ("crlf", body(["\r\n"; 3], ""), "3:1", "b.c = 2;"),
        ("cr", body(["\r"; 3], ""), "3:1", "b.c = 2;"),
        ("ls", body(["\u{2028}"; 3], ""), "3:1", "b.c = 2;"),
        ("ps", body(["\u{2029}"; 3], ""), "3:1", "b.c = 2;"),
        ("mixed", body(["\r\n", "\u{2028}", "\r"], ""), "3:1", "b.c = 2;"),
        (
            "bom",
            format!("\u{feff}{}", body(["\r"; 3], "")),
            "3:1",
            "b.c = 2;",
        ),
        (
            "astral",
            body(["\r"; 3], "\"\u{1F389}\"; "),
            "3:6",
            "\"\u{1F389}\"; b.c = 2;",
        ),
    ];
    for (name, source, position, shown) in cases {
        let file = dir.join(format!("{name}.tt"));
        fs::write(&file, &source).unwrap();
        let output = ttc(&["-p", file.to_str().unwrap()]);
        assert!(!output.status.success(), "{name}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!("{name}.tt:{position}")),
            "{name}: {stderr}"
        );
        assert!(
            stderr.contains(&format!("3 | {shown}\n")),
            "{name}: {stderr}"
        );
    }
}

/// A `#!` line ends at any line terminator, so the banner follows a
/// shebang that ends in CR rather than being pushed to the end of the file,
/// and the map keeps the shebang's own line unshifted (TASK-498).
#[test]
fn the_banner_follows_a_shebang_that_ends_in_any_terminator() {
    let dir = tmpdir();
    let out_dir = dir.join("out");
    for (name, terminator) in [("cr", "\r"), ("ls", "\u{2028}"), ("crlf", "\r\n")] {
        fs::write(
            dir.join(format!("{name}.tt")),
            format!("#!/usr/bin/env node{terminator}export const a = 1 |> String;{terminator}"),
        )
        .unwrap();
    }
    let output = ttc(&[
        "--source-map",
        "file",
        "-o",
        out_dir.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (name, terminator) in [("cr", "\r"), ("ls", "\u{2028}"), ("crlf", "\r\n")] {
        let emitted = fs::read_to_string(out_dir.join(format!("{name}.ts"))).unwrap();
        assert!(
            emitted.starts_with(&format!("#!/usr/bin/env node{terminator}// @generated")),
            "{name}: {emitted:?}"
        );
        let map = fs::read_to_string(out_dir.join(format!("{name}.ts.map"))).unwrap();
        let mappings = map
            .split("\"mappings\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("a mappings field");
        assert!(
            mappings.starts_with("AAAA;;"),
            "{name}: the shebang keeps line 1 and the banner takes line 2: {mappings}"
        );
    }
}
