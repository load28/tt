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
use common::toolchain_required as required;

/// The pinned TypeScript's `lib/tsc.js` (`common::typescript`).
fn tsc_entry() -> Option<PathBuf> {
    common::typescript()
        .map(|dir| dir.join("lib/tsc.js"))
        .filter(|entry| entry.exists())
}

fn have_node() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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
    let workspace = Workspace::in_repo_with_subdir("content-mapper", "src");
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
fn check(tsc: &Path, project: &Workspace) -> (bool, String) {
    let mut command = Command::new("node");
    command.args([
        tsc.as_os_str().to_str().unwrap(),
        "-p",
        project.to_str().unwrap(),
        "--runExternalCode",
    ]);
    // TypeScript closes mapper stdin and immediately kills the process. EOF
    // can start LLVM finalization before that kill arrives, so these descendants
    // have the same non-finalizing lifecycle as directly terminated watch tests.
    // The direct protocol test below still measures a normally exiting mapper.
    project.isolate_unfinalized_child_profile(&mut command);
    let output = command.output().expect("tsc runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

const SHAPE_TT: &str = "export variant Shape {\n  Circle(radius: number),\n  Rect(width: number, height: number),\n}\n\nexport function area(shape: Shape): number {\n  return match (shape) {\n    Circle(radius) => Math.PI * radius * radius,\n    Rect(width, height) => width * height,\n  };\n}\n";

#[test]
fn recovery_reports_the_original_host_error_once_through_the_real_mapper() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(project.path().join("src/provider.tt"),
        "declare const flag: boolean;\nconst broken = ;\nexport const later = match (flag) { true => 1, false => 2 };\n").unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { later } from './provider.tt';\nconst ok: number = later;\n",
    )
    .unwrap();
    let (ok, text) = check(&tsc, &project);
    assert!(!ok, "incomplete source still fails a build");
    assert!(text.contains("TS1109"), "{text}");
    assert_eq!(
        text.matches("error ").count(),
        1,
        "one original syntax cause, no duplicate mapper or missing-export error:\n{text}"
    );
}

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

    let (ok, text) = check(&tsc, &project);
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

    let (ok, text) = check(&tsc, &project);
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

    let (ok, text) = check(&tsc, &project);
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
fn a_deep_expression_try_typechecks_through_the_content_mapper() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(
        project.path().join("src/deep-try.tt"),
        "type TResult<T, E> = { kind: \"Ok\"; value: T } | { kind: \"Err\"; error: E };\n\
         declare const Result: { Ok<T>(value: T): TResult<T, never> };\n\
         declare function total(): TResult<number, string>;\n\
         export function amount(): TResult<{ amount: number }, string> {\n\
         \x20 return Result.Ok({ amount: try total() });\n\
         }\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, &project);
    assert!(ok, "content mapper rejected expression try:\n{text}");
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

    let (ok, text) = check(&tsc, &project);
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

    let (ok, text) = check(&tsc, &project);
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

    let (ok, text) = check(&tsc, &project);
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

    let (ok, text) = check(&tsc, &project);
    assert!(!ok);
    assert!(
        text.contains("wrong.tt(4,10): error TS2322"),
        "expected the checker's error mapped to the match, got:\n{text}"
    );
}

#[test]
fn recovered_syntax_reports_its_tt_diagnostic_instead_of_a_rejected_mapping() {
    let tsc = require_mapper_toolchain!();
    let cases = [
        (
            "declare const s: any;\nconst v = match (s) {\n  Circle { r } => r,\n};\nexport const a = 1;\n",
            "b.tt(2,11): error tt7",
        ),
        (
            "export variant Shape {\n  Circle(r: number\n}\nexport const a = 1;\n",
            "b.tt(1,8): error tt6",
        ),
        (
            "declare function f(): any;\nconst v = try f();\nexport const a = 1;\n",
            "b.tt(2,11): error tt11",
        ),
        (
            "declare const o: any;\nconst v = if let Some(value) = o { value };\nexport const a = 1;\n",
            "b.tt(2,11): error tt14",
        ),
    ];
    for (source, expected) in cases {
        let project = mapper_project(false);
        fs::write(project.path().join("src/b.tt"), source).unwrap();
        fs::write(
            project.path().join("src/main.ts"),
            "import { a } from \"./b.tt\";\nconst x: string = a;\n",
        )
        .unwrap();

        let (ok, text) = check(&tsc, &project);
        assert!(!ok);
        assert!(
            !text.contains("TS100029"),
            "TypeScript rejected the mapping:\n{text}"
        );
        assert!(
            text.contains(expected),
            "expected `{expected}` at its source, got:\n{text}"
        );
    }
}

#[test]
fn generated_slots_preserve_contextual_literal_types_for_the_checker() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    fs::write(
        project.path().join("src/context.tt"),
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\n\
         type Toggle = \"on\" | \"off\";\n\
         declare const next: () => TResult<number, string>;\n\
         export const flip = (value: Toggle): Toggle => match (value) {\n\
         \x20 \"on\" => \"off\", \"off\" => \"on\",\n\
         };\n\
         export const values = (): TResult<readonly number[], string> => result {\n\
         \x20 const value = try next();\n\
         \x20 if (value === 0) return [];\n\
         \x20 return [value];\n\
         };\n\
         void Result.Ok;\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { flip, values } from \"./context.tt\";\nvoid flip;\nvoid values;\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, &project);
    assert!(
        ok,
        "expected contextual literals to type-check, got:\n{text}"
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

    let (ok, text) = check(&tsc, &project);
    assert!(ok, "expected a clean check, got:\n{text}");
    // The mapper put the standard library where module resolution looks.
    assert!(
        project
            .path()
            .join("node_modules/@tt/std/index.ts")
            .exists()
    );
}

fn node_module_project(package_type: &str, module: &str, verbatim: bool) -> Workspace {
    let project = mapper_project(false);
    fs::write(
        project.path().join("package.json"),
        format!("{{ \"private\": true, \"type\": \"{package_type}\" }}\n"),
    )
    .unwrap();
    let config = project.path().join("tsconfig.json");
    let text = fs::read_to_string(&config)
        .unwrap()
        .replace("\"esnext\"", &format!("\"{module}\""))
        .replace("\"bundler\"", &format!("\"{module}\""))
        .replace(
            "\"skipLibCheck\": true",
            &format!("\"skipLibCheck\": true,\n    \"verbatimModuleSyntax\": {verbatim}"),
        );
    fs::write(&config, text).unwrap();
    fs::write(
        project.path().join("src/opt.tt"),
        "import type { TOption } from \"@tt/std\";\nimport * as Option from \"@tt/std/option\";\nimport * as Result from \"@tt/std/result\";\n\nexport function first(values: readonly number[]): TOption<number> {\n  return values.length > 0 ? Option.Some(values[0]) : Option.None;\n}\n\nexport const ok = Result.Ok(1);\nexport const count = [1, 2] |> ((xs) => xs.length);\n",
    )
    .unwrap();
    project
}

#[test]
fn std_imports_resolve_under_node_esm_with_verbatim_module_syntax() {
    let tsc = require_mapper_toolchain!();
    let project = node_module_project("module", "nodenext", true);
    let legacy = project.path().join("node_modules/@tt/std");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(
        legacy.join("package.json"),
        "{\n  \"name\": \"@tt/std\",\n  \"version\": \"0.0.0\",\n  \"types\": \"index.ts\"\n}\n",
    )
    .unwrap();
    for module in ttc::StdPackage::Std.modules() {
        fs::write(
            legacy.join(ttc::StdPackage::file_name(*module)),
            module.source(),
        )
        .unwrap();
    }
    fs::write(
        project.path().join("src/main.ts"),
        "import { first, count } from \"./opt.tt\";\nconst value = first([1]);\nexport const n: number = count;\nexport { value };\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, &project);
    assert!(ok, "expected a clean check, got:\n{text}");
    assert_eq!(
        fs::read_to_string(legacy.join("package.json")).unwrap(),
        ttc::StdPackage::Std.manifest()
    );
}

#[test]
fn std_imports_resolve_from_commonjs_and_esm_files_under_node16() {
    let tsc = require_mapper_toolchain!();
    for package_type in ["commonjs", "module"] {
        let project = node_module_project(package_type, "node16", false);
        fs::write(
            project.path().join("src/legacy.cts"),
            "import Option = require(\"@tt/std/option\");\nimport type { TOption } from \"@tt/std\";\nexport const legacy: TOption<number> = Option.Some(2);\n",
        )
        .unwrap();
        fs::write(
            project.path().join("src/modern.mts"),
            "import { legacy } from \"./legacy.cjs\";\nimport * as Option from \"@tt/std/option\";\nexport const next: typeof legacy = Option.None;\n",
        )
        .unwrap();
        check(&tsc, &project);
        let (ok, text) = check(&tsc, &project);
        assert!(ok, "{package_type}: expected a clean check, got:\n{text}");
    }
}

#[test]
fn a_commonjs_file_requires_the_std_package_cleanly_under_verbatim_module_syntax() {
    let tsc = require_mapper_toolchain!();
    for skip_lib_check in [true, false] {
        let project = node_module_project("module", "nodenext", true);
        let config = project.path().join("tsconfig.json");
        let text = fs::read_to_string(&config).unwrap().replace(
            "\"skipLibCheck\": true",
            &format!("\"skipLibCheck\": {skip_lib_check}"),
        );
        fs::write(&config, text).unwrap();
        let copied = project.path().join("node_modules/@tt/std");
        fs::create_dir_all(copied.join(ttc::STD_PACKAGE_COMMONJS_DIR)).unwrap();
        let mut entries = Vec::new();
        for module in ttc::StdPackage::Std.modules() {
            let file = ttc::StdPackage::file_name(*module);
            let generated = format!("{}{}", ttc::GENERATED_BANNER, module.source());
            fs::write(copied.join(file), &generated).unwrap();
            fs::write(
                copied.join(ttc::STD_PACKAGE_COMMONJS_DIR).join(file),
                &generated,
            )
            .unwrap();
            let subpath = &module.specifier()[ttc::StdPackage::Std.name().len()..];
            entries.push(format!(
                "    \".{subpath}\": {{\n      \"import\": {{ \"types\": \"./{file}\", \"default\": \"./{file}\" }},\n      \"require\": {{ \"types\": \"./cjs/{file}\", \"default\": \"./cjs/{file}\" }}\n    }}"
            ));
        }
        fs::write(
            copied.join("cjs/package.json"),
            "{\n  \"type\": \"commonjs\"\n}\n",
        )
        .unwrap();
        fs::write(
            copied.join("package.json"),
            format!(
                "{{\n  \"name\": \"@tt/std\",\n  \"version\": \"0.0.0\",\n  \"type\": \"module\",\n  \"types\": \"./index.ts\",\n  \"exports\": {{\n{}\n  }}\n}}\n",
                entries.join(",\n")
            ),
        )
        .unwrap();
        fs::write(
            project.path().join("src/b.cts"),
            "import Option = require(\"@tt/std/option\");\nimport type { TOption } from \"@tt/std\" with { \"resolution-mode\": \"require\" };\nconst legacy: TOption<number> = Option.None;\nexport = { legacy };\n",
        )
        .unwrap();
        check(&tsc, &project);
        let (ok, text) = check(&tsc, &project);
        assert!(
            ok,
            "skipLibCheck {skip_lib_check}: expected a clean check, got:\n{text}"
        );
        assert_eq!(
            fs::read_to_string(copied.join("package.json")).unwrap(),
            ttc::StdPackage::Std.manifest()
        );
        assert!(!copied.join("cjs/option.ts").exists());
    }
}

#[test]
fn std_commonjs_declarations_are_the_compilers_declaration_emit() {
    let tsc = require_mapper_toolchain!();
    let project = Workspace::new("std-declarations");
    let modules = [
        ("types", ttc::StdModule::Types),
        ("option", ttc::StdModule::Option),
        ("result", ttc::StdModule::Result),
        ("runtime", ttc::StdModule::Runtime),
    ];
    for (name, module) in modules {
        fs::write(project.path().join(format!("{name}.ts")), module.source()).unwrap();
    }
    let mut command = Command::new("node");
    command
        .arg(&tsc)
        .args([
            "--declaration",
            "--emitDeclarationOnly",
            "--strict",
            "--target",
            "es2022",
            "--module",
            "esnext",
            "--moduleResolution",
            "bundler",
            "--outDir",
            "out",
        ])
        .args(modules.map(|(name, _)| format!("{name}.ts")))
        .current_dir(project.path());
    let output = command.output().expect("tsc runs");
    assert!(output.status.success(), "{output:?}");
    for (name, module) in modules {
        assert_eq!(
            fs::read_to_string(project.path().join(format!("out/{name}.d.ts"))).unwrap(),
            module.declaration(),
            "{name}"
        );
    }
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

    let (ok, text) = check(&tsc, &project);
    assert!(
        ok,
        "expected a clean check of the .ttx project, got:\n{text}"
    );
}

#[test]
fn a_tsx_consumer_typechecks_tt_and_ttx_imports_together() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(true);
    fs::write(
        project.path().join("src/plain.tt"),
        "export const fromTt: string = \"tt\";\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/view.ttx"),
        "export const fromTtx: string = \"ttx\";\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.tsx"),
        "import { fromTt } from \"./plain.tt\";\nimport { fromTtx } from \"./view.ttx\";\ndeclare global { namespace JSX { interface IntrinsicElements { section: { children?: unknown }; } } }\nexport const view = <section>{fromTt}{fromTtx}</section>;\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, &project);
    assert!(ok, "expected a clean .tsx mixed-import check, got:\n{text}");
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

/// The mapper speaks byte offsets, so the checker counts the lines itself —
/// ECMA-262's, whichever terminator a `.tt` file uses, and UTF-16 columns
/// after a byte-order mark (TASK-498).
#[test]
fn a_tt_diagnostic_lands_on_the_line_every_terminator_starts() {
    let tsc = require_mapper_toolchain!();
    for (name, separator) in [
        ("lf", "\n"),
        ("crlf", "\r\n"),
        ("cr", "\r"),
        ("ls", "\u{2028}"),
        ("ps", "\u{2029}"),
    ] {
        let project = mapper_project(false);
        let source = format!(
            "\u{feff}declare const s: any;{separator}{separator}\"\u{1F389}\"; const v = try f();{separator}export const a = 1;{separator}"
        );
        fs::write(project.path().join("src/b.tt"), source).unwrap();
        fs::write(
            project.path().join("src/main.ts"),
            "import { a } from \"./b.tt\";\nconst x: number = a;\n",
        )
        .unwrap();
        let (ok, text) = check(&tsc, &project);
        assert!(!ok, "{name}");
        assert!(
            text.contains("b.tt(3,17): error tt11"),
            "{name}: expected the diagnostic at its source line, got:\n{text}"
        );
    }
}

#[test]
fn a_type_error_in_a_variant_field_reports_at_the_field_type() {
    let tsc = require_mapper_toolchain!();
    let project = mapper_project(false);
    // The field's type is written in the union and in the constructor; both
    // copies map to the one place the user wrote it.
    fs::write(
        project.path().join("src/price.tt"),
        "export interface Money { cents: number }\nexport variant Price { Fixed(amount: Mony), Free }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.ts"),
        "import { Price } from \"./price.tt\";\nexport const p: Price = Price.Free;\n",
    )
    .unwrap();

    let (ok, text) = check(&tsc, &project);
    assert!(!ok);
    assert!(
        text.contains("price.tt(2,38): error TS2552"),
        "expected the checker's error at the field type, got:\n{text}"
    );
    assert!(!text.contains("price.tt(2,16)"), "{text}");
}

#[test]
fn mapper_serves_only_the_exact_published_projection() {
    use std::io::{Read, Write};
    use ttc::content_projection::{ProjectionExchange, ProjectionRecord};

    let workspace = Workspace::new("mapper-published-projection");
    let path = workspace.join("api.tt");
    let source = "export const n = 1;";
    let mut exchange = ProjectionExchange::new().unwrap();
    let response = serde_json::json!({
        "text": "export const n: number = 1;", "extension": ".ts",
        "mappings": [], "diagnostics": []
    });
    exchange
        .publish(&ProjectionRecord {
            protocol: 1,
            revision: 1,
            path: path.clone(),
            source: source.into(),
            response: response.clone(),
        })
        .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--content-mapper")
        .env(ttc::content_projection::ENVIRONMENT, exchange.directory())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for (id, method, params) in [
        (
            1,
            "initialize",
            serde_json::json!({"positionEncodings": ["utf-8"]}),
        ),
        (
            2,
            "transform",
            serde_json::json!({"fileName": path, "content": source}),
        ),
        (
            3,
            "transform",
            serde_json::json!({"fileName": path, "content": "export const n = 2;"}),
        ),
    ] {
        let body =
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
                .to_string();
        write!(input, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    }
    drop(input);
    let mut wire = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut wire)
        .unwrap();
    assert!(child.wait().unwrap().success());
    let mut answers: Vec<serde_json::Value> = Vec::new();
    let mut rest = wire.as_str();
    while let Some(header) = rest.find("\r\n\r\n") {
        let length: usize = rest[..header]
            .trim_start_matches("Content-Length:")
            .trim()
            .parse()
            .unwrap();
        answers.push(serde_json::from_str(&rest[header + 4..header + 4 + length]).unwrap());
        rest = &rest[header + 4 + length..];
    }
    assert_eq!(answers[1]["result"], response);
    assert_eq!(answers[2]["result"]["text"], "export const n = 2;");
    exchange.verify_acknowledged().unwrap();
}
