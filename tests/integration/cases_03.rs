#[test]
fn cli_build_emits_a_complete_tree_that_runs() {
    require_toolchain!();
    let dir = tmpdir();
    write_consumer_tree(&dir);

    let (ok, err) = run_ttc(&dir, &["-o", "build", "--no-banner", "src"]);
    assert!(ok, "build failed:\n{err}");

    // Hand-written TypeScript rides along byte-for-byte except for its
    // relative `.tt` (and `@tt/std`) specifiers.
    let main_ts = fs::read_to_string(dir.join("build/main.ts")).unwrap();
    assert_eq!(
        main_ts,
        CONSUMER_MAIN_TS
            .replace("./notice.tt", "./notice.js")
            .replace("@tt/std/option", "./tt/option.js")
    );
    for module in ttc::StdModule::STANDARD {
        assert!(dir.join("build/tt").join(module.file_name()).exists());
    }

    // The emitted tree stands on its own: tsc compiles it, node runs it.
    fs::write(dir.join("build/package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = Command::new("tsc")
        .current_dir(&dir)
        .args(["build/main.ts", "--outDir", "build"])
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "tsc failed:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = Command::new("node")
        .current_dir(&dir)
        .arg("build/main.js")
        .output()
        .expect("failed to run node");
    assert!(
        out.status.success(),
        "node failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        ["info: hello", "warn[7]: careful", "true"]
    );
}

#[test]
fn cli_refuses_to_overwrite_a_pass_through_input() {
    let dir = tmpdir();
    fs::write(dir.join("main.ts"), "export const x = 1;\n").unwrap();

    // In place, a pass-through `.ts` would land on top of itself.
    let (ok, err) = run_ttc(&dir, &["main.ts"]);
    assert!(!ok, "expected failure:\n{err}");
    assert!(err.contains("output would overwrite the input"), "{err}");
    let untouched = fs::read_to_string(dir.join("main.ts")).unwrap();
    assert_eq!(untouched, "export const x = 1;\n");

    // A separate output tree is fine.
    let (ok, err) = run_ttc(&dir, &["-o", "out", "main.ts"]);
    assert!(ok, "build failed:\n{err}");
}

#[test]
fn cli_types_leaves_nothing_but_the_sidecars() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(ok, "--types failed:\n{err}");

    // Declaration emit runs in memory: no cache tree, and above all no
    // copy of the hand-written TypeScript anywhere.
    assert!(!dir.join(".tt-build").exists(), "a cache tree was created");
    let copies: Vec<String> = walk(&dir)
        .into_iter()
        .filter(|path| {
            path.file_name().is_some_and(|name| name == "main.ts")
                && !path.starts_with(dir.join("src"))
        })
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        copies.is_empty(),
        "hand-written source was copied: {copies:?}"
    );

    // What it does leave: one sidecar pair per .tt, plus the std types.
    assert!(dir.join(".tt-types/notice.tt.d.ts").exists());
    assert!(dir.join(".tt-types/notice.tt.d.ts.map").exists());
    assert!(dir.join(".tt-types/level.tt.d.ts").exists());
    for module in ttc::StdModule::STANDARD {
        assert!(
            dir.join(".tt-types/tt")
                .join(module.file_name())
                .with_extension("d.ts")
                .exists()
        );
    }
}

#[test]
fn cli_types_reports_type_errors_but_keeps_the_sidecars_fresh() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);
    // A type error in the consumer, not a tt-level one: declarations are
    // still emitted, so the sidecars must be written and the run must fail.
    fs::write(
        dir.join("src/main.ts"),
        format!("{CONSUMER_MAIN_TS}\nconst wrong: number = \"text\";\n"),
    )
    .unwrap();

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(!ok, "expected a failing exit code:\n{err}");
    assert!(
        err.contains("main.ts"),
        "diagnostic should name the file: {err}"
    );
    assert!(
        dir.join(".tt-types/notice.tt.d.ts").exists(),
        "sidecars should still be written: {err}"
    );
}

#[test]
fn cli_types_reports_tt_type_errors_at_the_source_position() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);
    // A type error *inside* tt syntax. The emitted TypeScript is a switch
    // IIFE that moves the offending expression far from where it was
    // written, and the file it lives in is never written to disk — the
    // diagnostic has to name `bad.tt` and the source line/column anyway.
    let bad = "import type { TResult } from \"@tt/std\";\n\
               import * as Result from \"@tt/std/result\";\n\
               \n\
               declare function evaluate(): TResult<number, string>;\n\
               \n\
               export const bad = evaluate() |> Result.mapP((n) => n.length);\n";
    fs::write(dir.join("src/bad.tt"), bad).unwrap();

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(!ok, "expected a failing exit code:\n{err}");

    // `length` sits at column 55 of line 5 of the source. The emitted code
    // puts it elsewhere entirely, and there is no `bad.ts` to open. The
    // message and the position have to belong to the *same* diagnostic, so
    // this reads the rendered block rather than two independent lines.
    let reported = err
        .split("error[")
        .find(|block| block.contains("does not exist on type"))
        .unwrap_or_else(|| panic!("no type error reported:\n{err}"));
    assert!(
        reported.contains("--> src/bad.tt:6:55"),
        "diagnostic should point into the .tt source: {reported}"
    );
    assert!(
        !err.contains("bad.ts"),
        "named a file that does not exist: {err}"
    );
}

#[test]
fn cli_types_without_typescript_says_so() {
    require_toolchain!();
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/level.tt"), LEVEL_TT).unwrap();
    // No TypeScript on purpose: a project's TypeScript comes from its own
    // `node_modules` and nowhere else, and a temporary directory has none
    // above it. So this runs everywhere, rather than skipping on any
    // machine that happens to have a compiler installed somewhere.
    let (ok, err) = run_ttc_env(&dir, &["--types", "src"]);
    assert!(!ok, "expected failure:\n{err}");
    assert!(err.contains("no TypeScript compiler found"), "{err}");
}

#[test]
fn cli_types_sidecars_typecheck_the_source_tree() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(ok, "--types failed:\n{err}");

    // The declarations keep the *source* specifiers — that is what resolves
    // in the consumer's merged view.
    let sidecar = fs::read_to_string(dir.join(".tt-types/notice.tt.d.ts")).unwrap();
    assert!(sidecar.contains("from \"@tt/std\""), "{sidecar}");
    assert!(sidecar.contains("from \"./level.tt\""), "{sidecar}");
    assert!(
        sidecar.contains("export declare function render"),
        "{sidecar}"
    );
    assert!(dir.join(".tt-types/notice.tt.d.ts.map").exists());
    assert!(dir.join(".tt-types/level.tt.d.ts").exists());
    for module in ttc::StdModule::STANDARD {
        assert!(
            dir.join(".tt-types/tt")
                .join(module.file_name())
                .with_extension("d.ts")
                .exists(),
            "std declaration missing: {:?}",
            module
        );
    }

    // Round trip: the untouched source tree typechecks once the sidecars
    // are merged in (`rootDirs`) and `@tt/std` is mapped (`paths`).
    fs::write(
        dir.join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "rootDirs": ["./src", "./.tt-types"],
    "paths": {
      "@tt/std": ["./.tt-types/tt/index.d.ts"],
      "@tt/std/*": ["./.tt-types/tt/*.d.ts"]
    }
  },
  "include": ["src"]
}
"#,
    )
    .unwrap();
    let out = Command::new("tsc")
        .current_dir(&dir)
        .args(["-p", "tsconfig.json"])
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "consumer typecheck failed:\n{}\n---sidecar---\n{sidecar}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/* ------------------------------------------------------------------ */
/* pipeline                                                            */
/* ------------------------------------------------------------------ */

// Inline the curried std combinators so the snippets need no module
// resolution (the std source itself is covered by tests/stdlib.rs).
const PIPE_PRELUDE: &str = r#"
type TOption<T> = { kind: "Some"; value: T } | { kind: "None" };
const Option = {
  Some: <T>(value: T): TOption<T> => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
  mapP:
    <T, U>(f: (value: T) => U) =>
    (o: TOption<T>): TOption<U> =>
      o.kind === "Some" ? { kind: "Some", value: f(o.value) } : { kind: "None" },
  unwrapOrP:
    <T>(fallback: T) =>
    (o: TOption<T>): T =>
      o.kind === "Some" ? o.value : fallback,
};
const half = (n: number): TOption<number> =>
  n % 2 === 0 ? Option.Some(n / 2) : Option.None;
"#;

#[test]
fn pipeline_curried_combinator_steps_infer_without_annotations() {
    require_toolchain!();
    // The whole point of the $tt_ap emission: `x` in the curried step must
    // infer as number (a direct-application emission collapses it to
    // `unknown` — TS18046).
    let (ok, out) = typecheck(&format!(
        "{PIPE_PRELUDE}\nconst label: string = half(4) |> Option.mapP(x => x + 1) |> Option.unwrapOrP(0) |> .toFixed(1);\n"
    ));
    assert!(ok, "{out}");
}

#[test]
fn pipeline_generic_user_functions_instantiate() {
    require_toolchain!();
    // Composing generic functions is where pipe() libraries lose inference;
    // step-by-step application must keep it.
    let (ok, out) = typecheck(
        "const wrap = <T,>(v: T): T[] => [v];\nconst arr: number[][] = 3 |> wrap |> wrap;\n",
    );
    assert!(ok, "{out}");
}

#[test]
fn pipeline_files_import_one_shared_runtime() {
    require_toolchain!();
    let dir = tmpdir();
    let mut files = Vec::new();
    for suffix in ["a", "b"] {
        let source = format!(
            "declare function input_{suffix}(): number;\n\
             declare const step_{suffix}: (value: number) => number;\n\
             const value_{suffix} = input_{suffix}() |> step_{suffix};\n\
             const flow_{suffix} = flow |> step_{suffix} |> step_{suffix};\n"
        );
        let code =
            compile(&source, &options_with_runtime("./runtime.js")).expect("tt compile failed");
        assert!(!code.lines().any(|line| line.starts_with("function $tt_")));
        assert!(!code.lines().any(|line| line.starts_with("var $tt_")));
        assert!(code.contains("from \"./runtime.js\""));
        let file = dir.join(format!("{suffix}.ts"));
        fs::write(&file, code).unwrap();
        files.push(file);
    }
    write_runtime(&dir);
    files.push(dir.join("runtime.ts"));

    let out = Command::new("tsc")
        .args(&files)
        .arg("--noEmit")
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn pipeline_type_error_in_a_step_is_reported_on_user_text() {
    require_toolchain!();
    // A step that is not a unary function is the user's type error — tsc
    // must reject it (ttc emits it untouched).
    let (ok, out) = typecheck("const n: number = 1 |> ((a: string) => a.length);\n");
    assert!(!ok, "{out}");
}

#[test]
fn a_direct_pipeline_call_keeps_contextual_typing() {
    require_toolchain!();
    let (ok, out) = typecheck("const value: number = 1 |> (x => x + 1);\n");
    assert!(ok, "{out}");
}

#[test]
fn pipeline_runs_left_to_right() {
    require_toolchain!();
    let lines = run(r#"
const order: string[] = [];
const tap = <T,>(name: string) => (v: T): T => { order.push(name); return v; };
const out = (order.push("head"), 10) |> tap("s1") |> .toFixed(0) |> tap("s2");
console.log(order.join(","), out);
"#);
    assert_eq!(lines, ["head,s1,s2 10"]);
}

#[test]
fn optional_postfix_preserves_short_circuit_order_and_method_receiver() {
    require_toolchain!();
    let lines = run(r#"
const order: string[] = [];
const mark = (name: string, value: number): number => { order.push(name); return value; };
const key = (): "method" => { order.push("key"); return "method"; };
const live = {
  base: 10,
  method(value: number): number {
    order.push(this === live ? "this" : "lost-this");
    return this.base + value;
  },
};
const absent = (() => undefined as typeof live | undefined)();
const hit = (order.push("head-hit"), live) |> ?.[key()]?.(mark("arg", 2));
const miss = (order.push("head-miss"), absent) |> ?.[key()]?.(mark("skipped", 3));
const after = miss |> (value => { order.push("after"); return value ?? -1; });
console.log(hit, miss, after, order.join(","));
"#);
    assert_eq!(
        lines,
        ["12 undefined -1 head-hit,key,arg,this,head-miss,after"],
        "{lines:?}"
    );
}

#[test]
fn optional_postfix_keeps_nested_tt_values_inside_the_conditional_tail() {
    require_toolchain!();
    let lines = run(r#"
variant E { A(value: number), B }
const order: string[] = [];
const subject = (): E => { order.push("subject"); return E.A(4); };
const live = { method(value: number): number { order.push("method"); return value; } };
const absent = (() => undefined as typeof live | undefined)();
const miss = absent |> ?.method(match (subject()) { A(value) => value, B => 0 });
const hit = live |> ?.method(match (subject()) { A(value) => value, B => 0 });
console.log(miss, hit, order.join(","));
"#);
    assert_eq!(lines, ["undefined 4 subject,method"], "{lines:?}");
}

#[test]
fn optional_postfix_types_are_checked_as_plain_typescript() {
    require_toolchain!();
    let (ok, out) = typecheck(
        "declare const value: { n: number } | undefined;\n\
         const maybe: number | undefined = value |> ?.n;\n\
         const project = flow |> ((v: { n: number } | undefined) => v) |> ?.n;\n\
         const also: number | undefined = project(value);\n",
    );
    assert!(ok, "{out}");

    let (ok, out) = typecheck(
        "declare const value: { n: number } | undefined;\n\
         const bad = value |> ?.n |> ((n: number) => n + 1);\n",
    );
    assert!(!ok, "{out}");
}

#[test]
fn a_materialized_pipeline_keeps_head_before_callee() {
    require_toolchain!();
    let lines = run(r#"
variant E { A(value: number), B }
const order: string[] = [];
const head = (): E => { order.push("head"); return E.A(2); };
const step = () => { order.push("step"); return (value: number) => {
  order.push("call");
  return value + 1;
}; };
const value = match (head()) { A(value) => value, B => 0 } |> step();
console.log(order.join(","), value);
"#);
    assert_eq!(lines, ["head,step,call 3"]);
}

#[test]
fn flow_composition_infers_input_from_its_first_step() {
    require_toolchain!();
    // The composed function's parameter type comes from the first step,
    // and every later step (curried combinator, method step) infers from
    // the previous step's return type — no annotations anywhere.
    let (ok, out) = typecheck(&format!(
        "{PIPE_PRELUDE}\nconst label = flow |> half |> Option.mapP(x => x + 1) \
         |> Option.unwrapOrP(0) |> .toFixed(1);\nconst s: string = label(4);\n"
    ));
    assert!(ok, "{out}");
}

#[test]
fn flow_composition_keeps_the_first_step_arity() {
    require_toolchain!();
    // Composition is emitted with a rest-tuple parameter, so a multi-argument
    // first step stays multi-argument (a unary `flow` type would lose this).
    let (ok, out) = typecheck(
        "const add = (a: number, b: number) => a + b;\nconst f = flow |> add |> ((n: number) => n * 2);\nconst v: number = f(1, 2);\n",
    );
    assert!(ok, "{out}");
}

#[test]
fn flow_composition_input_mismatch_is_a_type_error_on_user_text() {
    require_toolchain!();
    // Calling the composed function with the wrong argument type is the
    // user's error — ttc emits no type tricks that could hide it.
    let (ok, out) = typecheck(
        "const parse = (s: string) => s.length;\nconst f = flow |> parse |> ((n: number) => n + 1);\nconst v = f(3);\n",
    );
    assert!(!ok, "{out}");
}

#[test]
fn flow_composition_runs_left_to_right_when_called() {
    require_toolchain!();
    let lines = run(r#"
const order: string[] = [];
const tap = <T,>(name: string) => (v: T): T => { order.push(name); return v; };
const f = flow |> tap<number>("s1") |> .toFixed(0) |> tap("s2");
console.log(order.join(","), "|", f(10), "|", order.join(","));
"#);
    assert_eq!(lines, [" | 10 | s1,s2"]); // nothing ran until the call
}

#[test]
fn pipeline_await_in_head_runs_in_the_surrounding_async_context() {
    require_toolchain!();
    let lines = run(r#"
const upper = (s: string) => s.toUpperCase();
async function main() {
  const v = await Promise.resolve("ok") |> upper |> .concat("!");
  console.log(v);
}
await main();
"#);
    assert_eq!(lines, ["OK!"]);
}

/* ------------------------------------------------------------------ */
/* tuple match                                                         */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_tuple_match_dispatches_on_the_combination() {
    require_toolchain!();
    let lines = run(r#"
variant Conn { Online(latency: number), Offline }
variant Mode { Auto(), Manual(level: number) }

function decide(c: Conn, m: Mode): number {
  return match (c, m) {
    (Online(latency), Auto) if latency < 50 => 10,
    (Online, Auto) => 5,
    (Online, Manual(level)) => level,
    (Offline, _) => 0,
  };
}

console.log(decide(Conn.Online(10), Mode.Auto()));
console.log(decide(Conn.Online(80), Mode.Auto()));
console.log(decide(Conn.Online(10), Mode.Manual(7)));
console.log(decide(Conn.Offline, Mode.Auto()));
"#);
    assert_eq!(lines, vec!["10", "5", "7", "0"]);
}

#[test]
fn tuple_match_bindings_typecheck_per_position() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
variant Left { A(n: number), B }
variant Right { C(s: string), D }
function f(l: Left, r: Right): string {
  return match (l, r) {
    (A(n), C(s)) => s.repeat(n),
    (A(n), D) => n.toFixed(0),
    (B, C(s)) => s,
    (B, D) => "",
  };
}
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn tuple_match_scrutinees_evaluate_once_each_left_to_right() {
    require_toolchain!();
    let lines = run(r#"
variant Coin { Heads(), Tails }
const order: string[] = [];
function heads(name: string): Coin { order.push(name); return Coin.Heads(); }
const r = match (heads("a"), heads("b")) {
  (Heads, Heads) => 1,
  _ => 0,
};
console.log(order.join(","), r);
"#);
    assert_eq!(lines, vec!["a,b 1"]);
}

/* ------------------------------------------------------------------ */
/* nested patterns                                                     */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_nested_pattern_falls_through_on_inner_mismatch() {
    require_toolchain!();
    let lines = run(r#"
variant Opt { Some(value: number), None }
variant Res { Ok(value: Opt), Err(error: string) }

function grade(r: Res): string {
  return match (r) {
    Ok(value: Some(value: v)) if v > 9000 => "over",
    Ok(value: Some(value: v)) => "num:" + v,
    Ok(value: None()) => "empty",
    Err(error) => "err:" + error,
    // v1 exhaustiveness: nested arms cover nothing, so `Ok` counts as
    // uncovered without a final wildcard (documented, like guards).
    _ => "unreachable",
  };
}

console.log(grade(Res.Ok(Opt.Some(9001))));
console.log(grade(Res.Ok(Opt.Some(3))));
console.log(grade(Res.Ok(Opt.None)));
console.log(grade(Res.Err("boom")));
"#);
    assert_eq!(lines, vec!["over", "num:3", "empty", "err:boom"]);
}

#[test]
fn nested_pattern_bindings_typecheck_through_the_paths() {
    require_toolchain!();
    // The emitted condition chain must narrow $tt_m.value for the
    // destructuring — no type tricks, plain control-flow analysis.
    let (ok, out) = typecheck(
        r#"
variant Opt { Some(value: number), None }
variant Res { Ok(value: Opt), Err(error: string) }
function f(r: Res): number {
  return match (r) {
    Ok(value: Some(value: v)) => v + 1,
    _ => 0,
  };
}
"#,
    );
    assert!(ok, "{out}");
}

/* ------------------------------------------------------------------ */
/* if let                                                              */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_if_let_chains_and_falls_back() {
    require_toolchain!();
    let lines = run(r#"
variant Opt { Some(value: number), None }

function pick(a: Opt, b: Opt): number {
  let out = -1;
  if let Some(value) = a {
    out = value;
  } else if let Some(value) = b {
    out = value * 10;
  } else {
    out = 0;
  }
  return out;
}

console.log(pick(Opt.Some(1), Opt.Some(2)));
console.log(pick(Opt.None, Opt.Some(2)));
console.log(pick(Opt.None, Opt.None));
"#);
    assert_eq!(lines, vec!["1", "20", "0"]);
}
