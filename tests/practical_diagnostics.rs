//! Application-shaped diagnostic fixtures, exercised through the real CLI.
//!
//! Unit tests pin individual rules. These fixtures deliberately combine
//! independent parser, semantic, and TypeScript failures in the same project
//! so the user-facing batch command has to preserve all of them.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::Workspace;

struct Manifest {
    entry: PathBuf,
    diagnostics: Vec<ExpectedDiagnostic>,
}

struct ExpectedDiagnostic {
    code: String,
    text: String,
    line: usize,
    message: String,
    help: Vec<String>,
    cli_help: Option<Vec<String>>,
    labels: Vec<ExpectedLabel>,
}

struct ExpectedLabel {
    text: String,
    line: usize,
    message: String,
}

struct ErrorAnnotation {
    code: String,
    line: usize,
    message: String,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/practical-diagnostics")
}

fn cases() -> Vec<PathBuf> {
    let mut cases: Vec<_> = fs::read_dir(fixtures())
        .expect("practical diagnostic fixtures exist")
        .map(|entry| entry.expect("readable fixture entry").path())
        .filter(|path| path.is_dir())
        .collect();
    cases.sort();
    assert!(
        !cases.is_empty(),
        "the practical diagnostic matrix is empty"
    );
    cases
}

fn manifest(case: &Path) -> Manifest {
    let value: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(case.join("manifest.json")).expect("readable manifest"),
    )
    .expect("valid manifest");
    let entry = value["entry"].as_str().expect("manifest entry is a string");
    let diagnostics = value["diagnostics"]
        .as_array()
        .expect("manifest diagnostics is an array")
        .iter()
        .map(|diagnostic| ExpectedDiagnostic {
            code: diagnostic["code"]
                .as_str()
                .expect("diagnostic code is a string")
                .to_string(),
            text: diagnostic["text"]
                .as_str()
                .expect("diagnostic text is a string")
                .to_string(),
            line: diagnostic["line"]
                .as_u64()
                .expect("diagnostic line is an integer") as usize,
            message: diagnostic["message"]
                .as_str()
                .expect("diagnostic message is a string")
                .to_string(),
            help: diagnostic["help"]
                .as_array()
                .expect("diagnostic help is an array")
                .iter()
                .map(|help| help.as_str().expect("help is a string").to_string())
                .collect(),
            cli_help: diagnostic.get("cliHelp").map(|help| {
                help.as_array()
                    .expect("diagnostic cliHelp is an array")
                    .iter()
                    .map(|help| help.as_str().expect("CLI help is a string").to_string())
                    .collect()
            }),
            labels: diagnostic["labels"]
                .as_array()
                .expect("diagnostic labels is an array")
                .iter()
                .map(|label| ExpectedLabel {
                    text: label["text"]
                        .as_str()
                        .expect("label text is a string")
                        .to_string(),
                    line: label["line"].as_u64().expect("label line is an integer") as usize,
                    message: label["message"]
                        .as_str()
                        .expect("label message is a string")
                        .to_string(),
                })
                .collect(),
        })
        .collect();
    Manifest {
        entry: PathBuf::from(entry),
        diagnostics,
    }
}

fn copy_project(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source).expect("readable fixture directory") {
        let entry = entry.expect("readable fixture entry");
        if entry.file_name() == "node_modules" {
            continue;
        }
        let destination = destination.join(entry.file_name());
        if entry.file_type().expect("fixture entry type").is_dir() {
            fs::create_dir_all(&destination).expect("writable test project");
            copy_project(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).expect("copyable fixture file");
        }
    }
}

fn annotated_source(path: &Path) -> (String, Vec<ErrorAnnotation>) {
    let annotated = fs::read_to_string(path).expect("readable annotated entry");
    let mut source = String::new();
    let mut annotations = Vec::new();
    for (index, line) in annotated.split_inclusive('\n').enumerate() {
        let has_newline = line.ends_with('\n');
        let content = line.strip_suffix('\n').unwrap_or(line);
        if let Some(marker) = content.find("//~") {
            source.push_str(content[..marker].trim_end());
            let annotation = content[marker + 3..].trim();
            let annotation = annotation
                .strip_prefix("ERROR[")
                .expect("practical annotations use `//~ ERROR[code] message`");
            let (code, message) = annotation
                .split_once("] ")
                .expect("practical annotation has a code and message");
            annotations.push(ErrorAnnotation {
                code: code.to_string(),
                line: index + 1,
                message: message.to_string(),
            });
        } else {
            source.push_str(content);
        }
        if has_newline {
            source.push('\n');
        }
    }
    (source, annotations)
}

fn expect_baseline(path: &Path, actual: &str) {
    if std::env::var_os("UPDATE_EXPECT").is_some() {
        fs::write(path, actual).expect("writable practical diagnostic baseline");
        return;
    }
    let expected = fs::read_to_string(path).unwrap_or_else(|_| {
        panic!(
            "{} does not exist — run `UPDATE_EXPECT=1 cargo test --test practical_diagnostics`",
            path.display()
        )
    });
    assert_eq!(
        actual,
        expected,
        "{} is out of date; regenerate it with UPDATE_EXPECT=1 and review the diff",
        path.display()
    );
}

fn toolchain_installed() -> bool {
    let mut dir = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    while let Some(current) = dir {
        for client in ["typescript", "@typescript/native-preview"] {
            if current
                .join("node_modules")
                .join(client)
                .join("dist/api/sync/api.js")
                .exists()
            {
                return true;
            }
        }
        dir = current.parent().map(Path::to_path_buf);
    }
    false
}

fn codes(stderr: &str) -> Vec<&str> {
    stderr
        .lines()
        .filter_map(|line| line.strip_prefix("error[")?.split_once("]: ").map(|x| x.0))
        .collect()
}

#[test]
fn cli_reports_every_practical_diagnostic_at_its_source() {
    if !toolchain_installed() {
        assert!(
            std::env::var_os("TTC_REQUIRE_TSGO").is_none(),
            "TTC_REQUIRE_TSGO is set but no TypeScript API is installed"
        );
        return;
    }

    for fixture in cases() {
        let project = Workspace::in_repo("practical-diagnostics");
        copy_project(&fixture, project.path());
        let manifest = manifest(project.path());
        let (source, annotations) = annotated_source(&fixture.join(&manifest.entry));
        fs::write(project.path().join(&manifest.entry), &source)
            .expect("writable stripped test entry");
        let output = Command::new(env!("CARGO_BIN_EXE_ttc"))
            .args(["--check-types", "--project", "tsconfig.json", "src"])
            .current_dir(project.path())
            .output()
            .expect("ttc runs");
        let stderr = String::from_utf8(output.stderr).expect("diagnostics are UTF-8");
        let normalized_stderr = stderr
            .replace("\r\n", "\n")
            .replace(project.path().to_string_lossy().as_ref(), "$DIR")
            .replace('\\', "/");
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}\n{stderr}",
            fixture.display()
        );

        let actual_codes = codes(&stderr);
        let expected_codes: Vec<_> = manifest
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();
        assert_eq!(
            actual_codes,
            expected_codes,
            "{}\n{stderr}",
            fixture.display()
        );

        let annotated_codes: Vec<_> = annotations
            .iter()
            .map(|annotation| annotation.code.as_str())
            .collect();
        assert_eq!(
            annotated_codes,
            expected_codes,
            "{} annotations must exhaustively name every error",
            fixture.display()
        );
        for (annotation, diagnostic) in annotations.iter().zip(&manifest.diagnostics) {
            assert_eq!(annotation.line, diagnostic.line);
            assert!(
                diagnostic.message.contains(&annotation.message),
                "{} annotation is not a message substring: {:?}",
                diagnostic.code,
                annotation.message
            );
        }
        expect_baseline(&fixture.join("expected.stderr"), &normalized_stderr);

        for diagnostic in &manifest.diagnostics {
            let line = source.lines().nth(diagnostic.line - 1).unwrap_or_else(|| {
                panic!(
                    "{} has no line {}",
                    manifest.entry.display(),
                    diagnostic.line
                )
            });
            let first_expected_line = diagnostic.text.lines().next().expect("diagnostic text");
            assert!(
                line.contains(first_expected_line),
                "{}:{} does not contain {:?}",
                manifest.entry.display(),
                diagnostic.line,
                first_expected_line
            );

            let marker = format!("error[{}]:", diagnostic.code);
            let block = stderr
                .split(&marker)
                .nth(1)
                .unwrap_or_else(|| panic!("missing {marker} in\n{stderr}"))
                .split("\nerror[")
                .next()
                .expect("diagnostic block");
            assert_eq!(
                block.lines().next().map(str::trim),
                Some(diagnostic.message.as_str()),
                "{} has the wrong CLI message:\n{block}",
                diagnostic.code
            );
            assert!(
                block.contains(&format!(
                    "{}:{}:",
                    manifest.entry.display(),
                    diagnostic.line
                )),
                "{} is at the wrong CLI location:\n{block}",
                diagnostic.code
            );
            assert!(
                block.contains(line.trim()),
                "{} does not quote its source line:\n{block}",
                diagnostic.code
            );
            let actual_help: Vec<_> = block
                .lines()
                .filter_map(|line| line.trim().strip_prefix("= help: "))
                .collect();
            let expected_help: Vec<_> = diagnostic
                .cli_help
                .as_ref()
                .unwrap_or(&diagnostic.help)
                .iter()
                .map(String::as_str)
                .collect();
            assert_eq!(
                actual_help, expected_help,
                "{} has the wrong CLI help:\n{block}",
                diagnostic.code
            );
            for label in &diagnostic.labels {
                let label_line = source.lines().nth(label.line - 1).unwrap_or_else(|| {
                    panic!("{} has no line {}", manifest.entry.display(), label.line)
                });
                assert!(label_line.contains(&label.text));
                assert!(
                    block.contains(&format!("{} | {}", label.line, label_line)),
                    "{} does not quote label line {}:\n{block}",
                    diagnostic.code,
                    label.line
                );
                assert!(
                    block.contains(&label.message),
                    "{} is missing label {:?}:\n{block}",
                    diagnostic.code,
                    label.message
                );
            }
        }
    }
}
