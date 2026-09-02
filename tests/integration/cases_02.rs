#[test]
fn typecheck_exhaustive_match_passes() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
variant Shape { Circle(radius: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  Point => 0,
};
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn typecheck_wildcard_makes_partial_match_exhaustive() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  _ => 0,
};
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn std_result_constructors_type_only_their_own_variant() {
    require_toolchain!();
    // `Ok` carries no error type and `Err` carries no success type, so each
    // constructor is typed by its own variant — and both still fit a
    // `TResult<T, E>` wherever one is expected.
    let (ok, out) = typecheck_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
import type { TOk, TErr } from "./tt/index.js";

type Exact<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

const ok = Result.Ok(123);
const err = Result.Err("bad");
const okIsOkOfNumber: Exact<typeof ok, TOk<number>> = true;
const errIsErrOfString: Exact<typeof err, TErr<string>> = true;

const fromOk: TResult<number, string> = Result.Ok(1);
const fromErr: TResult<number, string> = Result.Err("bad");

function parse(value: string): TResult<number, string> {
  if (value.length === 0) {
    return Result.Err("empty");
  }
  return Result.Ok(Number(value));
}

console.log(okIsOkOfNumber, errIsErrOfString, fromOk, fromErr, parse("1"));
"#,
    );
    assert!(ok, "tsc rejected variant-typed constructors:\n{out}");
}

#[test]
fn try_error_types_infer_as_a_union_without_an_annotation() {
    require_toolchain!();
    // Two `try`s over results with different error types: the lowered early
    // returns plus `Result.Ok(...)` give tsc `TErr<UserError> | TErr<ConfigError>
    // | TOk<Data>`, which is exactly `TResult<Data, UserError | ConfigError>`.
    // ttc collects no error types of its own — this is tsc's union inference.
    let (ok, out) = typecheck_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type User = { id: number };
type Config = { port: number };
type UserError = { tag: "user" };
type ConfigError = { tag: "config" };

declare function getUser(): TResult<User, UserError>;
declare function getConfig(): TResult<Config, ConfigError>;

function load() {
  const user = try getUser();
  const config = try getConfig();
  return Result.Ok({ user, config });
}

const loaded: TResult<{ user: User; config: Config }, UserError | ConfigError> = load();
console.log(loaded);
"#,
    );
    assert!(ok, "tsc lost the try error union:\n{out}");
}

#[test]
fn try_error_union_stays_checked_against_the_declared_return_type() {
    require_toolchain!();
    // The inference above is not a hole: an annotated function whose `Err`
    // type does not cover a propagated error is still a type error, reported
    // by tsc on the emitted early return.
    let (ok, out) = typecheck_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

declare function getUser(): TResult<number, { tag: "user" }>;

function load(): TResult<number, string> {
  const user = try getUser();
  return Result.Ok(user);
}

console.log(load());
"#,
    );
    assert!(!ok, "tsc accepted an uncovered error type:\n{out}");
}

/// Declarations shared by the `andThen` error-union tests: four steps, each
/// failing its own way, so a chain that loses an error type is visible in the
/// asserted union.
const ERROR_UNION_PRELUDE: &str = r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type Exact<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

type User = { id: number };
type Company = { name: string };
type Profile = { title: string };
type ConfigError = { tag: "config" };
type TokenError = { tag: "token" };
type FetchError = { tag: "fetch" };
type ValidationError = { tag: "validation" };

declare function loadConfig(): TResult<string, ConfigError>;
declare function loadToken(config: string): TResult<User, TokenError>;
declare function getCompany(user: User): TResult<Company, FetchError>;
declare function fetchProfile(user: User): TResult<Profile, FetchError>;
declare function validateProfile(profile: Profile): TResult<Profile, ValidationError>;
"#;

#[test]
fn std_result_and_then_unions_the_two_error_types() {
    require_toolchain!();
    // `andThen` takes the chained function's error type as its own generic,
    // so chaining a `TResult<User, TokenError>` with a step that fails with
    // `FetchError` keeps both — no `mapErr` to a common type first.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

const chained = Result.andThen(first, (user) => getCompany(user));
const exact: Exact<typeof chained, TResult<Company, TokenError | FetchError>> = true;

console.log(chained, exact);
"#
    ));
    assert!(ok, "andThen lost an error type:\n{out}");
}

#[test]
fn std_result_and_then_on_a_variant_typed_value_keeps_the_chained_error() {
    require_toolchain!();
    // A value typed as the `TOk<T>` variant alone (what `Result.Ok(...)` and a
    // never-failing function give) offers nothing to infer the incoming `E`
    // from. The `E = never` default is what keeps that case precise instead of
    // collapsing the union to `unknown`.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
const chained = Result.andThen(Result.Ok({{ id: 1 }}), (user: User) => fetchProfile(user));
const exact: Exact<typeof chained, TResult<Profile, FetchError>> = true;

console.log(chained, exact);
"#
    ));
    assert!(
        ok,
        "andThen on an Ok value lost the chained error type:\n{out}"
    );
}

#[test]
fn std_result_and_then_p_accumulates_error_types_along_a_pipeline() {
    require_toolchain!();
    // The end-to-end shape from the design: `try` collects two error types
    // into the function's inferred return type, and every `andThenP` step
    // adds its own. ttc collects nothing — this is tsc's union inference.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
function loadUser() {{
  const config = try loadConfig();
  const token = try loadToken(config);
  return Result.Ok(token);
}}

const profile = loadUser()
  |> Result.andThenP(fetchProfile)
  |> Result.andThenP(validateProfile);

const exact: Exact<
  typeof profile,
  TResult<Profile, ConfigError | TokenError | FetchError | ValidationError>
> = true;

console.log(profile, exact);
"#
    ));
    assert!(ok, "the pipeline lost an error type:\n{out}");
}

#[test]
fn std_result_map_p_keeps_the_error_type_it_was_given() {
    require_toolchain!();
    // `map`/`mapP` add no failure of their own, so they carry `E` through
    // unchanged — including a union an earlier `andThenP` accumulated.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

const title = first
  |> Result.andThenP(fetchProfile)
  |> Result.mapP((profile) => profile.title);

const exact: Exact<typeof title, TResult<string, TokenError | FetchError>> = true;

console.log(title, exact);
"#
    ));
    assert!(ok, "mapP changed the error type:\n{out}");
}

#[test]
fn std_result_and_then_p_composes_under_flow() {
    require_toolchain!();
    // `andThenP` returns a function still generic in `E`, so a `flow`
    // composition of two steps stays open at its input end: applying it to a
    // `TResult<User, TokenError>` unions that error in too.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

const pipeline = flow
  |> Result.andThenP(fetchProfile)
  |> Result.andThenP(validateProfile);

const profile = pipeline(first);
const exact: Exact<
  typeof profile,
  TResult<Profile, TokenError | FetchError | ValidationError>
> = true;

console.log(profile, exact);
"#
    ));
    assert!(ok, "flow composition lost an error type:\n{out}");
}

#[test]
fn std_result_and_then_p_takes_an_annotated_inline_callback() {
    require_toolchain!();
    // The curried form reads `T` off the chained function, so an inline
    // callback carries its own parameter annotation. A named function (every
    // other test here) needs nothing.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

const profile = first |> Result.andThenP((user: User) => fetchProfile(user));
const exact: Exact<typeof profile, TResult<Profile, TokenError | FetchError>> = true;

console.log(profile, exact);
"#
    ));
    assert!(ok, "an annotated inline callback did not typecheck:\n{out}");
}

#[test]
fn std_result_block_output_pipes_into_and_then_p() {
    require_toolchain!();
    // A `result` block infers the same shape a `try` function does — one `Ok`
    // arm plus one `Err` arm per binding — so its value chains on with its
    // error types intact.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
const user = result {{
  const config = try loadConfig();
  const loaded = try loadToken(config);
  return loaded;
}};

const profile = user |> Result.andThenP(fetchProfile);
const exact: Exact<
  typeof profile,
  TResult<Profile, ConfigError | TokenError | FetchError>
> = true;

console.log(profile, exact);
"#
    ));
    assert!(
        ok,
        "a result block lost its error types in a pipeline:\n{out}"
    );
}

#[test]
fn std_result_and_then_error_union_stays_checked_against_an_annotation() {
    require_toolchain!();
    // Accumulating errors is not a hole either: a declared return type that
    // covers only one of the two chained error types is still a tsc error.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

function chain(): TResult<Profile, TokenError> {{
  return Result.andThen(first, (user) => fetchProfile(user));
}}

console.log(chain());
"#
    ));
    assert!(
        !ok,
        "tsc accepted a return type missing an error case:\n{out}"
    );
}

#[test]
fn runtime_result_and_then_chain_short_circuits_on_the_first_err() {
    require_toolchain!();
    // The types changed; the emitted values did not. Both spellings still
    // return the first `Err` untouched and run the rest only on `Ok`.
    let lines = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type Parsed = { n: number };
type ParseError = { tag: "parse"; raw: string };
type RangeError = { tag: "range"; n: number };

const parse = (raw: string): TResult<Parsed, ParseError> =>
  Number.isNaN(Number(raw))
    ? Result.Err({ tag: "parse" as const, raw })
    : Result.Ok({ n: Number(raw) });

const inRange = (p: Parsed): TResult<number, RangeError> =>
  p.n <= 10 ? Result.Ok(p.n) : Result.Err({ tag: "range" as const, n: p.n });

const check = (raw: string) => parse(raw) |> Result.andThenP(inRange);

console.log(JSON.stringify(check("4")));
console.log(JSON.stringify(check("40")));
console.log(JSON.stringify(check("x")));
console.log(JSON.stringify(Result.andThen(parse("4"), inRange)));
console.log(JSON.stringify(Result.andThen(parse("x"), inRange)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":4}"#,
            r#"{"kind":"Err","error":{"tag":"range","n":40}}"#,
            r#"{"kind":"Err","error":{"tag":"parse","raw":"x"}}"#,
            r#"{"kind":"Ok","value":4}"#,
            r#"{"kind":"Err","error":{"tag":"parse","raw":"x"}}"#,
        ]
    );
}

#[test]
fn typecheck_match_on_handwritten_discriminated_union() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
type AppEvent =
  | { kind: "click"; x: number; y: number }
  | { kind: "key"; code: string };
const f = (e: AppEvent) => match (e) {
  click(x, y) => x + y,
  key(code) => code.length,
};
"#,
    );
    assert!(ok, "{out}");
}

/* ------------------------------------------------------------------ */
/* import specifier rewriting                                          */
/* ------------------------------------------------------------------ */

const ERROR_TT: &str = "export variant CalcError { DivByZero, Overflow(limit: number) }\n";
const MAIN_TT: &str = r#"import { CalcError } from "./error.tt";
const e = CalcError.Overflow(9);
const msg = match (e) {
  Overflow(limit) => `over ${limit}`,
  _ => "other",
};
console.log(msg);
export {};
"#;

#[test]
fn cross_file_tt_import_typechecks_and_runs() {
    require_toolchain!();
    let dir = tmpdir();
    let error_ts = compile(ERROR_TT, &Options::default()).expect("tt compile failed");
    let main_ts = compile(MAIN_TT, &Options::default()).expect("tt compile failed");
    assert!(main_ts.contains("\"./error.js\""), "{main_ts}");
    fs::write(dir.join("error.ts"), &error_ts).unwrap();
    fs::write(dir.join("main.ts"), &main_ts).unwrap();
    fs::write(dir.join("package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = Command::new("tsc")
        .arg(dir.join("main.ts"))
        .arg("--outDir")
        .arg(&dir)
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "tsc failed:\n{}\n---main.ts---\n{main_ts}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = Command::new("node")
        .arg(dir.join("main.js"))
        .output()
        .expect("failed to run node");
    assert!(
        out.status.success(),
        "node failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "over 9");
}

/* ------------------------------------------------------------------ */
/* project-wide exhaustiveness through the CLI                         */
/* ------------------------------------------------------------------ */

const TOKEN_TT: &str =
    "export variant Token {\n  Num(value: number),\n  Ident(name: string),\n  Eof,\n}\n";

/// Runs the ttc binary itself — declaration collection across files lives
/// in the CLI, not in `compile`. No tsc/node needed.
fn run_ttc(dir: &std::path::Path, args: &[&str]) -> (bool, String) {
    run_ttc_env(dir, args)
}

fn run_ttc_env(dir: &std::path::Path, args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run ttc");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Whether ttc can resolve a TypeScript to drive *and* emit declarations
/// with it. Asked by running `--types` over a trivial project and looking
/// for the sidecar: the answer is ttc's own resolution, not a guess about
/// the machine. A released `typescript@7` can check but not emit, so it
/// answers `false` here and the `--types` success tests skip.
fn usable_typescript_for_types() -> bool {
    let dir = project_dir();
    fs::write(dir.join("probe.tt"), "export const n: number = 1;\n").unwrap();
    let (ok, _) = run_ttc(&dir, &["--types", "probe.tt", "-o", "."]);
    ok && dir.join("probe.tt.d.ts").exists()
}

/// Skip a `--types` success test when no TypeScript that can emit
/// declarations is reachable.
macro_rules! require_types_typescript {
    () => {
        if !usable_typescript_for_types() {
            eprintln!("skipping: no TypeScript for ttc to drive, or it cannot emit declarations");
            return;
        }
    };
}

#[test]
fn cli_checks_exhaustiveness_across_tt_imports() {
    let dir = tmpdir();
    fs::write(dir.join("token.tt"), TOKEN_TT).unwrap();
    fs::write(
        dir.join("parser.tt"),
        "import { Token } from \"./token.tt\";\nconst show = (t: Token) =>\n  match (t) {\n    Num(value) => value,\n    Ident(name) => 0,\n  };\n",
    )
    .unwrap();
    let (ok, err) = run_ttc(&dir, &["--check", "parser.tt"]);
    assert!(!ok, "expected failure:\n{err}");
    // The rendered form: the rule and its message on the header line, the
    // file and position on the location line, the construct underlined.
    assert!(
        err.contains(
            "error[match-not-exhaustive]: match on variant Token (imported from \"./token.tt\") \
             is not exhaustive: missing \"Eof\""
        ),
        "{err}"
    );
    assert!(err.contains("--> parser.tt:3:3"), "{err}");
    assert!(err.contains("3 |   match (t) {"), "{err}");
    assert!(err.contains("  |   ^^^^^^^^^"), "{err}");

    fs::write(
        dir.join("parser.tt"),
        "import { Token } from \"./token.tt\";\nconst show = (t: Token) =>\n  match (t) {\n    Num(value) => value,\n    Ident(name) => 0,\n    Eof => -1,\n  };\n",
    )
    .unwrap();
    let (ok, err) = run_ttc(&dir, &["--check", "parser.tt"]);
    assert!(ok, "expected success:\n{err}");
}

#[test]
fn untyped_cli_does_not_infer_imported_field_ownership() {
    let dir = tmpdir();
    fs::write(
        dir.join("domain.tt"),
        "export variant PaymentMethod { Card(brand: string, last4: string) }\n",
    )
    .unwrap();
    fs::write(
        dir.join("payment.tt"),
        "import { PaymentMethod } from \"./domain.tt\";\n\
         export function brand(method: PaymentMethod): string {\n\
         \x20 return match (method) { Card(brnad) => brnad, _ => \"n/a\" };\n\
         }\n",
    )
    .unwrap();

    let (ok, err) = run_ttc(&dir, &["--check", "payment.tt"]);
    assert!(ok, "the typed checker owns imported field identity:\n{err}");
}

#[test]
fn untyped_cli_does_not_infer_a_single_imported_case_owner() {
    let dir = tmpdir();
    fs::write(
        dir.join("domain.tt"),
        "export variant PaymentMethod { Card(brand: string), BankTransfer(iban: string) }\n",
    )
    .unwrap();
    fs::write(
        dir.join("payment.tt"),
        "import { PaymentMethod } from \"./domain.tt\";\n\
         export function fee(method: PaymentMethod): number {\n\
         \x20 return match (method) { Crad(brand) => 1, _ => 0 };\n\
         }\n",
    )
    .unwrap();

    let (ok, err) = run_ttc(&dir, &["--check", "payment.tt"]);
    assert!(
        ok,
        "the typed checker owns the scrutinee's case domain:\n{err}"
    );
}

#[test]
fn untyped_cli_does_not_infer_a_generic_payload_owner() {
    let dir = tmpdir();
    fs::write(
        dir.join("domain.tt"),
        "export variant PaymentMethod { Card(brand: string), Cash }\n",
    )
    .unwrap();
    fs::write(
        dir.join("nested.tt"),
        "import type { TResult } from \"@tt/std\";\n\
         import { PaymentMethod } from \"./domain.tt\";\n\
         export function brand(r: TResult<PaymentMethod, string>): string {\n\
         \x20 return match (r) {\n\
         \x20   Ok(value: Card(brnd)) => brnd,\n\
         \x20   Ok(value) => \"other\",\n\
         \x20   Err(error) => \"error\",\n\
         \x20 };\n\
         }\n",
    )
    .unwrap();

    let (ok, err) = run_ttc(&dir, &["--check", "nested.tt"]);
    assert!(
        ok,
        "generic substitution belongs to the typed checker:\n{err}"
    );
}

#[test]
fn cli_skips_unresolvable_imports_silently() {
    // A missing module is tsc's problem (TS2307); the match simply stays
    // unchecked, as before phase 2.
    let dir = tmpdir();
    fs::write(
        dir.join("main.tt"),
        "import { Gone } from \"./missing.tt\";\nconst x = match (g) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let (ok, err) = run_ttc(&dir, &["--check", "main.tt"]);
    assert!(ok, "expected success:\n{err}");
}

#[test]
fn cli_cross_file_match_runs_end_to_end() {
    require_toolchain!();
    let dir = tmpdir();
    fs::write(dir.join("token.tt"), TOKEN_TT).unwrap();
    fs::write(
        dir.join("main.tt"),
        "import { Token } from \"./token.tt\";\nconst t = Token.Ident(\"x\");\nconsole.log(match (t) {\n  Num(value) => `n${value}`,\n  Ident(name) => `i${name}`,\n  Eof => \"eof\",\n});\nexport {};\n",
    )
    .unwrap();
    let (ok, err) = run_ttc(&dir, &["token.tt", "main.tt"]);
    assert!(ok, "ttc failed:\n{err}");
    fs::write(dir.join("package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = Command::new("tsc")
        .arg(dir.join("main.ts"))
        .arg("--outDir")
        .arg(&dir)
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "tsc failed:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = Command::new("node")
        .arg(dir.join("main.js"))
        .output()
        .expect("failed to run node");
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ix");
}

/* ------------------------------------------------------------------ */
/* symbol interface (--symbols)                                        */
/* ------------------------------------------------------------------ */

#[test]
fn symbols_reports_imports_and_positions_as_valid_json() {
    let dir = tmpdir();
    fs::write(dir.join("token.tt"), TOKEN_TT).unwrap();
    fs::write(
        dir.join("parser.tt"),
        "import { Token as Tok } from \"./token.tt\";\nimport { Gone } from \"./missing.tt\";\nvariant Local { A(x: number) }\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(&dir)
        .args(["--symbols", "parser.tt"])
        .output()
        .expect("failed to run ttc");
    assert!(out.status.success());
    let json = String::from_utf8_lossy(&out.stdout).into_owned();

    // Shape: the local variant with its position, the resolved import with the
    // referenced file's exported declarations, and the unresolvable import
    // marked null.
    assert!(json.contains("\"file\":\"parser.tt\""), "{json}");
    assert!(json.contains("\"variants\":["), "{json}");
    assert!(!json.contains("\"enums\""), "{json}");
    assert!(json.contains("\"name\":\"Local\""), "{json}");
    assert!(
        json.contains("\"entries\":[{\"name\":\"Token\",\"alias\":\"Tok\"}]"),
        "{json}"
    );
    assert!(
        json.contains(
            "\"name\":\"Token\",\"exported\":true,\"generics\":\"\",\"line\":1,\"col\":16"
        ),
        "{json}"
    );
    assert!(
        json.contains("\"tag\":\"Eof\",\"line\":4,\"col\":3,\"fields\":null"),
        "{json}"
    );
    assert!(json.contains("\"specifier\":\"./missing.tt\""), "{json}");
    assert!(json.contains("\"resolved\":null,\"variants\":[]"), "{json}");

    // And it must be JSON a real parser accepts.
    if have("node") {
        let mut child = Command::new("node")
            .args([
                "-e",
                "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>JSON.parse(d))",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("failed to run node");
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success(), "not valid JSON:\n{json}");
    }
}

/* ------------------------------------------------------------------ */
/* the unified pipeline through the CLI: build and --types             */
/* ------------------------------------------------------------------ */

const LEVEL_TT: &str = "export variant Level {\n  Low,\n  High(threshold: number),\n}\n";

const NOTICE_TT: &str = "import type { TOption } from \"@tt/std\";\nimport * as Option from \"@tt/std/option\";\nimport { Level } from \"./level.tt\";\n\nexport variant Notice {\n  Info(text: string),\n  Warn(text: string, code: number),\n}\n\nexport function render(n: Notice): string {\n  return match (n) {\n    Info(text) => `info: ${text}`,\n    Warn(text, code) => `warn[${code}]: ${text}`,\n  };\n}\n\nexport function gate(l: Level): number {\n  return match (l) {\n    Low => 0,\n    High(threshold) => threshold,\n  };\n}\n\nexport function first(list: Notice[]): TOption<Notice> {\n  return list.length > 0 ? Option.Some(list[0]) : Option.None;\n}\n";

const CONSUMER_MAIN_TS: &str = "import * as Option from \"@tt/std/option\";\nimport { Notice, render, first } from \"./notice.tt\";\n\nconst items = [Notice.Info(\"hello\"), Notice.Warn(\"careful\", 7)];\nfor (const n of items) console.log(render(n));\nconsole.log(Option.isSome(first(items)));\n";

/// A mixed source tree: two `.tt` modules (one importing the other and the
/// standard library) plus a hand-written `.ts` entry that imports `.tt`.
/// Every file under `dir`, recursively.
fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // node_modules is the linked TypeScript, not project output.
        if path.file_name().is_some_and(|name| name == "node_modules") {
            continue;
        }
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

fn write_consumer_tree(dir: &std::path::Path) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/level.tt"), LEVEL_TT).unwrap();
    fs::write(dir.join("src/notice.tt"), NOTICE_TT).unwrap();
    fs::write(dir.join("src/main.ts"), CONSUMER_MAIN_TS).unwrap();
}
