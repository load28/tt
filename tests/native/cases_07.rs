fn json_report(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "--json-report prints one JSON object: {error}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn reported_paths(report: &serde_json::Value, key: &str) -> Vec<PathBuf> {
    report[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} is a list: {report}"))
        .iter()
        .map(|entry| {
            PathBuf::from(
                entry
                    .as_str()
                    .or_else(|| entry["path"].as_str())
                    .unwrap_or_else(|| panic!("{key} names paths: {report}")),
            )
        })
        .collect()
}

#[test]
fn types_with_diagnostics_exits_1_and_reports_every_file_it_wrote() {
    require_emit!();
    let dir = project(&[("src/a.tt", "export const a: number = \"text\";\n")]);
    let out_dir = dir.join("out");
    let output = run(&dir, &["--types", "--json-report", "src", "-o", "out"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = json_report(&output);
    assert_eq!(report["checked"], true, "{report}");
    assert_eq!(report["diagnostics"], 1, "{report}");
    assert_eq!(reported_paths(&report, "failed"), Vec::<PathBuf>::new());
    let written = reported_paths(&report, "written");
    let declaration = fs::canonicalize(&out_dir).unwrap().join("a.tt.d.ts");
    let map = fs::canonicalize(&out_dir).unwrap().join("a.tt.d.ts.map");
    assert!(written.contains(&declaration), "{report}");
    assert!(written.contains(&map), "{report}");
    for path in &written {
        assert!(path.is_absolute() && path.is_file(), "{}", path.display());
    }
    let sources: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&map).unwrap()).unwrap();
    assert_eq!(sources["sources"], serde_json::json!(["../src/a.tt"]));
}

#[test]
fn types_that_cannot_write_every_file_exits_3_and_names_each_one() {
    require_emit!();
    let dir = project(&[
        ("src/a.tt", "export const a: number = 1;\n"),
        ("src/b.tt", "export const b: number = 2;\n"),
    ]);
    let out_dir = dir.join("out");
    fs::create_dir_all(out_dir.join("a.tt.d.ts")).unwrap();
    fs::create_dir_all(out_dir.join("b.tt.d.ts.map")).unwrap();
    let output = run(
        &dir,
        &["--types", "--json-report", "src", "-o", out_dir.to_str().unwrap()],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    let report = json_report(&output);
    assert_eq!(report["checked"], true, "{report}");
    assert_eq!(report["diagnostics"], 0, "{report}");

    let out_dir = fs::canonicalize(&out_dir).unwrap();
    let mut failed = reported_paths(&report, "failed");
    failed.sort();
    assert_eq!(
        failed,
        vec![
            out_dir.join("a.tt.d.ts"),
            out_dir.join("a.tt.d.ts.map"),
            out_dir.join("b.tt.d.ts.map"),
        ],
        "{report}"
    );
    assert_eq!(
        reported_paths(&report, "written"),
        vec![out_dir.join("b.tt.d.ts")],
        "{report}"
    );
    assert!(out_dir.join("b.tt.d.ts").is_file());
    assert!(!out_dir.join("a.tt.d.ts.map").exists());
    for name in ["a.tt.d.ts", "a.tt.d.ts.map", "b.tt.d.ts.map"] {
        assert!(
            stderr.contains(&format!("cannot write out/{name}")),
            "{name}: {stderr}"
        );
    }
    for entry in report["failed"].as_array().unwrap() {
        assert!(
            !entry["error"].as_str().unwrap_or_default().is_empty(),
            "{report}"
        );
    }
}

#[test]
fn types_that_cannot_check_exits_2_and_leaves_earlier_output() {
    require_tsgo!();
    let dir = project(&[("src/a.tt", "export const a: number = 1;\n")]);
    let out_dir = dir.join("out");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(out_dir.join("a.tt.d.ts"), "export {};\n").unwrap();
    let output = run(
        &dir,
        &[
            "--types",
            "--json-report",
            "src/missing.tt",
            "-o",
            out_dir.to_str().unwrap(),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = json_report(&output);
    assert_eq!(report["checked"], false, "{report}");
    assert_eq!(reported_paths(&report, "written"), Vec::<PathBuf>::new());
    assert_eq!(reported_paths(&report, "failed"), Vec::<PathBuf>::new());
    assert_eq!(
        fs::read_to_string(out_dir.join("a.tt.d.ts")).unwrap(),
        "export {};\n"
    );
}

#[test]
fn types_without_a_report_prints_nothing_on_stdout() {
    require_emit!();
    let dir = project(&[("src/a.tt", "export const a: number = 1;\n")]);
    let out_dir = dir.join("out");
    let output = run(&dir, &["--types", "src", "-o", out_dir.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn types_names_each_file_of_a_declaration_collision_once() {
    require_emit!();
    // Two directory inputs mirror to one output tree, so both `x.tt` files
    // claim `types/x.tt.d.ts` and its map.
    let dir = project(&[]);
    for (input, value) in [("src/a", 1), ("src/b", 2)] {
        fs::create_dir_all(dir.join(input)).unwrap();
        fs::write(
            dir.join(input).join("x.tt"),
            format!("export const x: number = {value};\n"),
        )
        .unwrap();
    }
    let output = run(
        &dir,
        &["--types", "--json-report", "-o", "types", "src/a", "src/b"],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    let report = json_report(&output);
    assert_eq!(reported_paths(&report, "written"), Vec::<PathBuf>::new());
    let types = fs::canonicalize(&dir).unwrap().join("types");
    let mut failed = reported_paths(&report, "failed");
    failed.sort();
    assert_eq!(
        failed,
        vec![types.join("x.tt.d.ts"), types.join("x.tt.d.ts.map")],
        "{report}"
    );
    for name in ["x.tt.d.ts", "x.tt.d.ts.map"] {
        let prefix = format!("ttc: cannot write types/{name}:");
        assert_eq!(
            stderr.lines().filter(|line| line.starts_with(&prefix)).count(),
            1,
            "{name}: {stderr}"
        );
    }
}

/// Reads a running watch's stderr and waits for its passes.
struct WatchPasses {
    lines: std::sync::mpsc::Receiver<String>,
    seen: String,
    passes: usize,
}

impl WatchPasses {
    fn of(child: &mut std::process::Child) -> Self {
        let (sender, lines) = std::sync::mpsc::channel();
        let stderr = child.stderr.take().expect("stderr piped");
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
            {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            lines,
            seen: String::new(),
            passes: 0,
        }
    }

    /// Waits for the next pass and returns what it printed.
    fn next(&mut self) -> String {
        self.passes += 1;
        let start = self.seen.len();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while self.seen.matches("— watching").count() < self.passes {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let line = self
                .lines
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("pass {} never finished:\n{}", self.passes, self.seen));
            self.seen.push_str(&line);
            self.seen.push('\n');
        }
        self.seen[start..].to_string()
    }
}

#[test]
fn a_typed_watch_reports_a_missing_configuration_and_recovers_when_it_returns() {
    require_tsgo!();
    let dir = project(&[("src/a.tt", "export const a: number = 1;\n")]);
    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command
        .args(["--check-types", "-w", "src"])
        .current_dir(&dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    dir.isolate_unfinalized_child_profile(&mut command);
    let mut child = command.spawn().expect("ttc runs");
    let mut watch = WatchPasses::of(&mut child);

    let first = watch.next();
    assert!(!first.contains("error["), "{first}");
    let config = fs::read_to_string(dir.join("tsconfig.json")).unwrap();
    fs::rename(dir.join("tsconfig.json"), dir.join("moved.json")).unwrap();
    let missing = watch.next();
    fs::remove_file(dir.join("moved.json")).unwrap();
    write(&dir, "tsconfig.json", &config);
    let restored = watch.next();
    write(&dir, "src/a.tt", "export const a: number = \"text\";\n");
    let edited = watch.next();
    let _ = child.kill();
    let status = child.wait().expect("ttc exits");

    assert!(
        missing.contains("error[ts5083]: Cannot read file")
            && missing.contains("--> tsconfig.json"),
        "{}",
        watch.seen
    );
    assert!(!restored.contains("error["), "{}", watch.seen);
    assert!(
        edited.contains("error[ts2322]") && edited.contains("--> src/a.tt:1:26"),
        "{}",
        watch.seen
    );
    assert!(!watch.seen.contains("internal compiler error"), "{}", watch.seen);
    assert_ne!(status.code(), Some(101), "{}", watch.seen);
}
