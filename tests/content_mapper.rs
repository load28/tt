//! `ttc --content-mapper` against the real TypeScript (TASK-257).
//!
//! These cases spawn the repository's installed TypeScript (`npm ci`,
//! TASK-256) with `--runExternalCode` on a project whose tsconfig names
//! `@openload28/tt-lang` as a content mapper, and the mapper it spawns is this
//! build's `ttc`. That is the whole consumer contract in one process tree:
//! TypeScript resolves the mapper package, speaks JSON-RPC to `ttc
//! --content-mapper`, holds the transformed `.tt`/`.ttx` files virtually,
//! and reports diagnostics through the span map.
//!
//! They skip silently when the install is not there or has no content
//! mapper support (that arrived in the TypeScript 7.1 line). Where CI says
//! a toolchain must be present, `TTC_REQUIRE_TSGO=1` turns either skip
//! into a failure, exactly as `tests/native.rs` does.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::Workspace;

/// The repository's installed `typescript/lib/tsc.js`, searched upwards
/// the way `toolchain.rs` searches.
fn tsc_entry() -> Option<PathBuf> {
    let mut dir = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    while let Some(current) = dir {
        let entry = current.join("node_modules/typescript/lib/tsc.js");
        if entry.exists() {
            return Some(entry);
        }
        dir = current.parent().map(Path::to_path_buf);
    }
    None
}

fn have_node() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// True when the caller has declared that a toolchain must be present.
fn required() -> bool {
    std::env::var_os("TTC_REQUIRE_TSGO").is_some_and(|v| !v.is_empty() && v != "0")
}

/// Whether the installed TypeScript knows `--runExternalCode` — the gate
/// content mappers sit behind. A 7.0 pin does not; the 7.1 line does.
fn supports_content_mappers(tsc: &Path) -> bool {
    Command::new("node")
        .args([
            tsc.as_os_str().to_str().unwrap(),
            "--runExternalCode",
            "--version",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// The installed TypeScript with content mapper support, or the reason to
/// skip. `TTC_REQUIRE_TSGO=1` turns both reasons into failures.
fn toolchain() -> Option<PathBuf> {
    if !have_node() {
        assert!(
            !required(),
            "TTC_REQUIRE_TSGO is set but node is not installed"
        );
        return None;
    }
    let Some(tsc) = tsc_entry() else {
        assert!(
            !required(),
            "TTC_REQUIRE_TSGO is set but this repository has no TypeScript \
             installed — run `npm ci` at the repository root"
        );
        return None;
    };
    if !supports_content_mappers(&tsc) {
        assert!(
            !required(),
            "TTC_REQUIRE_TSGO is set but the installed TypeScript has no \
             content mapper support — pin a 7.1 in the root package.json"
        );
        return None;
    }
    Some(tsc)
}

macro_rules! require_mapper_toolchain {
    () => {
        match toolchain() {
            Some(tsc) => tsc,
            None => return,
        }
    };
}

/// A consumer project: a tsconfig naming `@openload28/tt-lang` as the mapper
/// for `.tt`/`.ttx`, and a stub install of that package whose mapper
/// process is this build's `ttc`.
fn mapper_project(jsx: bool) -> Workspace {
    let workspace = Workspace::with_subdir("content-mapper", "src");
    fs::write(
        workspace.path().join("package.json"),
        "{ \"private\": true }\n",
    )
    .unwrap();
    let jsx_option = if jsx {
        "\"jsx\": \"preserve\",\n    "
    } else {
        ""
    };
    fs::write(
        workspace.path().join("tsconfig.json"),
        format!(
            "{{\n  \"compilerOptions\": {{\n    {jsx_option}\"strict\": true,\n    \"noEmit\": true,\n    \"target\": \"es2022\",\n    \"module\": \"esnext\",\n    \"moduleResolution\": \"bundler\",\n    \"skipLibCheck\": true\n  }},\n  \"contentMappers\": [\n    {{ \"package\": \"@openload28/tt-lang\", \"extensions\": [\".tt\", \".ttx\"] }}\n  ],\n  \"include\": [\"src\"]\n}}\n"
        ),
    )
    .unwrap();
    let package = workspace.path().join("node_modules/@openload28/tt-lang");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("package.json"),
        format!(
            "{{\n  \"name\": \"@openload28/tt-lang\",\n  \"version\": \"0.0.0-test\",\n  \"typescript\": {{\n    \"contentMapper\": {{\n      \"exec\": [{:?}, \"--content-mapper\"]\n    }}\n  }}\n}}\n",
            env!("CARGO_BIN_EXE_ttc")
        ),
    )
    .unwrap();
    workspace
}

/// One `tsc -p <project> --runExternalCode` run.
fn check(tsc: &Path, project: &Path) -> (bool, String) {
    let output = Command::new("node")
        .args([
            tsc.as_os_str().to_str().unwrap(),
            "-p",
            project.to_str().unwrap(),
            "--runExternalCode",
        ])
        .output()
        .expect("tsc runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

const SHAPE_TT: &str = "export variant Shape {\n  Circle(radius: number),\n  Rect(width: number, height: number),\n}\n\nexport function area(shape: Shape): number {\n  return match (shape) {\n    Circle(radius) => Math.PI * radius * radius,\n    Rect(width, height) => width * height,\n  };\n}\n";

#[test]
fn a_ts_file_imports_a_tt_file_with_no_sidecar_on_disk() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(project.path().join("src/shape.tt"), SHAPE_TT).unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { Shape, area } from \"./shape.tt\";\nconst ok: number = area(Shape.Circle(2));\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(ok, "expected a clean check, got:\n{text}");
    // The check held the transform virtually: nothing was written next to
    // the sources, which is the point of the mapper over the sidecar.
    assert!(!project.path().join("src/shape.tt.d.ts").exists());
    assert!(!project.path().join(".tt-types").exists());
}

#[test]
fn a_consumer_type_error_reports_at_the_consumer() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(project.path().join("src/shape.tt"), SHAPE_TT).unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { Shape, area } from \"./shape.tt\";\nconst bad: string = area(Shape.Circle(2));\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(!ok);
    assert!(
        text.contains("main.ts(2,7): error TS2322"),
        "expected TS2322 at the consumer, got:\n{text}"
    );
}

#[test]
fn a_tt_diagnostic_reports_at_its_source_with_the_tt_source() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(project.path().join("src/shape.tt"), SHAPE_TT).unwrap();
    // One-hop import: the missing `Rect` arm is knowable only by reading
    // `./shape.tt`, which is the mapper's extern collection at work.
    fs::write(
        project.path().join("src/partial.tt"),
        "import { Shape } from \"./shape.tt\";\n\nexport function tag(shape: Shape): string {\n  return match (shape) {\n    Circle(radius) => \"circle\",\n  };\n}\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { tag } from \"./partial.tt\";\nconst t: string = tag({ kind: \"Circle\", radius: 1 });\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(!ok);
    // The diagnostic is the mapper's own: tt's source name, tt's stable
    // code number, at the match's position in the original file.
    assert!(
        text.contains("partial.tt(4,10): error tt27"),
        "expected the tt exhaustiveness diagnostic at its source, got:\n{text}"
    );
    assert!(
        text.contains("not exhaustive"),
        "expected the tt message, got:\n{text}"
    );
}

#[test]
fn a_deep_expression_try_reports_the_placement_rule_at_its_source() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(
        project.path().join("src/deep-try.tt"),
        "declare function total(): TResult<number, string>;\n\
         export function amount(): TResult<number, string> {\n\
         \x20 return Result.Ok({ amount: try total() });\n\
         }\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(!ok);
    assert!(
        text.contains("deep-try.tt(3,30): error tt11")
            && text.contains("statement, not an expression"),
        "expected the source placement diagnostic, got:\n{text}"
    );
    assert!(
        !text.contains("verify-failed") && !text.contains("source-not-typescript"),
        "verification must not own the parser error:\n{text}"
    );
}

#[test]
fn an_imported_field_error_is_checker_owned_at_the_field_token() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(
        project.path().join("src/domain.tt"),
        "export variant PaymentMethod { Card(brand: string, last4: string) }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/payment.tt"),
        "import { PaymentMethod } from \"./domain.tt\";\n\n\
         export function brand(method: PaymentMethod): string {\n\
         \x20 return match (method) { Card(brnad) => brnad, _ => \"n/a\" };\n\
         }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { brand } from \"./payment.tt\";\nvoid brand;\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(!ok);
    assert!(
        text.contains("payment.tt(4,32): error TS2339")
            && text.contains("Property 'brnad' does not exist"),
        "expected the checker's source-mapped field diagnostic, got:\n{text}"
    );
    assert!(!text.contains("error tt26"), "{text}");
}

#[test]
fn an_imported_case_error_with_a_wildcard_is_checker_owned() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(
        project.path().join("src/domain.tt"),
        "export variant PaymentMethod { Card(brand: string), BankTransfer(iban: string) }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/payment.tt"),
        "import { PaymentMethod } from \"./domain.tt\";\n\n\
         export function fee(method: PaymentMethod): number {\n\
         \x20 return match (method) { Crad(brand) => 1, _ => 0 };\n\
         }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { fee } from \"./payment.tt\";\nvoid fee;\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(!ok);
    assert!(
        text.contains("payment.tt(4,10): error TS2678")
            && text.contains("Type '\"Crad\"' is not comparable"),
        "expected the checker's source-mapped case diagnostic, got:\n{text}"
    );
    assert!(!text.contains("error tt25"), "{text}");
}

#[test]
fn a_nested_imported_field_error_is_reported_at_its_token() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(
        project.path().join("src/domain.tt"),
        "export variant PaymentMethod { Card(brand: string), Cash }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/nested.tt"),
        "import type { TResult } from \"@tt/std\";\n\
         import { PaymentMethod } from \"./domain.tt\";\n\n\
         export function brand(r: TResult<PaymentMethod, string>): string {\n\
         \x20 return match (r) {\n\
         \x20   Ok(value: Card(brnd)) => brnd,\n\
         \x20   Ok(value) => \"other\",\n\
         \x20   Err(error) => \"error\",\n\
         \x20 };\n\
         }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { brand } from \"./nested.tt\";\nvoid brand;\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(!ok);
    assert!(
        text.contains("nested.tt(6,20): error TS2339")
            && text.contains("Property 'brnd' does not exist"),
        "{text}"
    );
    assert!(
        !text.contains("{ brnd: any; }") && !text.contains("error tt26"),
        "{text}"
    );
}

#[test]
fn a_type_error_inside_glue_reports_at_the_construct() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(project.path().join("src/shape.tt"), SHAPE_TT).unwrap();
    // A match whose arms disagree with the declared return type: the
    // checker sees the disagreement in compiler-written glue, and the
    // anchor span carries it back to the construct.
    fs::write(
        project.path().join("src/wrong.tt"),
        "import { Shape } from \"./shape.tt\";\n\nexport function wrong(shape: Shape): number {\n  return match (shape) {\n    Circle(radius) => radius,\n    Rect(width, height) => \"not a number\",\n  };\n}\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { wrong } from \"./wrong.tt\";\nconst n: number = wrong({ kind: \"Circle\", radius: 1 });\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(!ok);
    assert!(
        text.contains("wrong.tt(4,3): error TS2322"),
        "expected the checker's error mapped to the match, got:\n{text}"
    );
}

#[test]
fn std_imports_resolve_through_materialization() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(
        project.path().join("src/opt.tt"),
        "import type { TOption } from \"@tt/std\";\nimport * as Option from \"@tt/std/option\";\n\nexport function first(values: readonly number[]): TOption<number> {\n  return values.length > 0 ? Option.Some(values[0]) : Option.None;\n}\n\nexport function describe(values: readonly number[]): string {\n  return match (first(values)) {\n    Some(value) => `first: ${value}`,\n    None => \"empty\",\n  };\n}\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { describe } from \"./opt.tt\";\nconst text: string = describe([1, 2, 3]);\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(ok, "expected a clean check, got:\n{text}");
    // The mapper put the standard library where module resolution looks.
    assert!(
        project
            .path()
            .join("node_modules/@tt/std/index.ts")
            .exists()
    );
}

#[test]
fn a_ttx_file_serves_as_tsx() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(true);
    fs::write(
        project.path().join("src/badge.ttx"),
        "export variant State {\n  On(label: string),\n  Off,\n}\n\ndeclare global {\n  namespace JSX {\n    interface IntrinsicElements {\n      span: { className?: string; children?: unknown };\n    }\n  }\n}\n\nexport function Badge(props: { state: State }) {\n  return match (props.state) {\n    On(label) => <span className=\"on\">{label}</span>,\n    Off => <span className=\"off\">off</span>,\n  };\n}\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { State } from \"./badge.ttx\";\nconst s: State = State.Off;\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, project.path());
    assert!(
        ok,
        "expected a clean check of the .ttx project, got:\n{text}"
    );
}

/// The protocol end to end without TypeScript: this test is the peer,
/// speaking Content-Length-framed JSON-RPC to `ttc --content-mapper`
/// directly. It needs no toolchain, so the wire contract stays covered
/// even where the tsgo-driven cases above skip.
#[test]
fn the_mapper_process_answers_the_protocol_directly() {
    use std::io::{Read, Write};

    let workspace = Workspace::new("content-mapper-wire");
    fs::write(
        workspace.path().join("package.json"),
        "{ \"private\": true }\n",
    )
    .unwrap();
    let file = workspace.path().join("shape.tt");

    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--content-mapper")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the mapper process starts");

    let mut stdin = child.stdin.take().unwrap();
    let requests = [
        serde_json::json!({ "jsonrpc": "2.0", "id": "api1", "method": "initialize",
            "params": { "positionEncodings": ["utf-8", "utf-16"] } }),
        serde_json::json!({ "jsonrpc": "2.0", "id": "api2", "method": "openProject",
            "params": { "configFileName": workspace.path().join("tsconfig.json").to_str().unwrap(),
                        "projectHandle": "p:0" } }),
        serde_json::json!({ "jsonrpc": "2.0", "id": "api3", "method": "transform",
            "params": { "fileName": file.to_str().unwrap(),
                        "content": "export variant Shape { Circle(radius: number), Point }\n",
                        "projectHandle": "p:0" } }),
        serde_json::json!({ "jsonrpc": "2.0", "id": "api4", "method": "closeProject",
            "params": { "projectHandle": "p:0" } }),
    ];
    for request in &requests {
        let body = serde_json::to_string(request).unwrap();
        write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    }
    drop(stdin); // end of input ends the session with exit 0

    let mut wire = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut wire)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "clean exit at end of stdin");

    // Parse every framed response in order.
    let mut answers = Vec::new();
    let mut rest = wire.as_str();
    while let Some(start) = rest.find("\r\n\r\n") {
        let length: usize = rest[..start]
            .trim_start_matches("Content-Length:")
            .trim()
            .parse()
            .unwrap();
        let body = &rest[start + 4..start + 4 + length];
        answers.push(serde_json::from_str::<serde_json::Value>(body).unwrap());
        rest = &rest[start + 4 + length..];
    }
    assert_eq!(answers.len(), 4, "one answer per request:\n{wire}");
    assert_eq!(answers[0]["id"], "api1");
    assert_eq!(answers[0]["result"]["positionEncoding"], "utf-8");
    assert_eq!(answers[0]["result"]["diagnosticSource"], "tt");
    assert_eq!(answers[1]["result"], serde_json::json!({}));
    assert_eq!(answers[2]["result"]["extension"], ".ts");
    assert!(
        answers[2]["result"]["text"]
            .as_str()
            .unwrap()
            .contains("kind: \"Circle\"")
    );
    assert_eq!(answers[3]["result"], serde_json::json!({}));
    // openProject materialized the standard library at the config root.
    assert!(
        workspace
            .path()
            .join("node_modules/@tt/std/index.ts")
            .exists()
    );
}
