#[test]
fn if_let_bindings_stay_narrowed_inside_closures() {
    require_toolchain!();
    // The binding materializes as a const, so the narrowed type survives
    // closure boundaries — the gap that motivated the feature (TASK-042 G5).
    let (ok, out) = typecheck(
        r#"
variant Opt { Some(value: string), None }
function f(o: Opt, xs: number[]): string[] {
  const collected: string[] = [];
  if let Some(value) = o {
    xs.forEach(() => collected.push(value.toUpperCase()));
  }
  return collected;
}
"#,
    );
    assert!(ok, "{out}");
}

/* ------------------------------------------------------------------ */
/* result computation block                                            */
/* ------------------------------------------------------------------ */

/// tt variants in exactly the shape `@tt/std`'s `Result` has, so the block
/// tests need no module setup.
const RESULT_PRELUDE: &str = r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
variant UserError { NoUser() }
variant CompanyError { NoCompany(id: number) }
type User = { id: number; name: string; companyId: number };
type Company = { id: number; name: string };
declare function getUser(id: number): Res<User, UserError>;
declare function getCompany(id: number): Res<Company, CompanyError>;
"#;

#[test]
fn result_block_unions_the_error_types_of_its_bindings() {
    require_toolchain!();
    // The whole error-type question: two bindings with different error
    // types must produce `Res<_, UserError | CompanyError>` with no help
    // from ttc and no change to the combinators' signatures.
    let (ok, out) = typecheck(&format!(
        r#"{RESULT_PRELUDE}
const view = (id: number): Res<string, UserError | CompanyError> => result {{
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  return user.name + "@" + company.name;
}};
"#
    ));
    assert!(ok, "{out}");
}

#[test]
fn result_block_missing_an_error_type_is_a_type_error() {
    require_toolchain!();
    // The other half: an annotation that forgets one binding's error type
    // is tsc's error, reported on the user's own annotation.
    let (ok, out) = typecheck(&format!(
        r#"{RESULT_PRELUDE}
const view = (id: number): Res<string, UserError> => result {{
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  return user.name + "@" + company.name;
}};
"#
    ));
    assert!(!ok, "{out}");
}

#[test]
fn result_block_bindings_are_narrowed_success_values() {
    require_toolchain!();
    // No annotations anywhere: each binding must be the `Ok` payload type,
    // and the block's value type must flow out of the block.
    let (ok, out) = typecheck(&format!(
        r#"{RESULT_PRELUDE}
const view = (id: number) => result {{
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  const label: string = user.name.toUpperCase() + company.name;
  return {{ user, company, label }};
}};
const check = (id: number): string => match (view(id)) {{
  Ok(value) => value.label,
  Err(error) => match (error) {{
    NoUser => "no user",
    NoCompany(id: missing) => "no company " + missing,
  }},
}};
"#
    ));
    assert!(ok, "{out}");
}

#[test]
fn runtime_result_block_short_circuits_on_the_first_err() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }

const steps: string[] = [];
const step = (name: string, ok: boolean): Res<string, string> => {
  steps.push(name);
  return ok ? Res.Ok(name) : Res.Err("failed:" + name);
};

const chain = (secondOk: boolean) => result {
  const a = try step("a", true);
  const b = try step("b", secondOk);
  const c = try step("c", true);
  return a + b + c;
};

console.log(JSON.stringify(chain(true)), steps.join(","));
steps.length = 0;
console.log(JSON.stringify(chain(false)), steps.join(","));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":"abc"} a,b,c"#,
            r#"{"kind":"Err","error":"failed:b"} a,b"#,
        ]
    );
}

#[test]
fn runtime_using_disposes_when_try_propagates_err() {
    require_toolchain!();
    let lines = run_with_tsc_flags(
        r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }

const events: string[] = [];
const fail = (): Res<number, string> => Res.Err("boom");

const sync = () => {
  using resource = {
    [Symbol.dispose]() { events.push("sync-dispose"); },
  };
  const value = try fail();
  return Res.Ok(value);
};

const asyncRun = async () => {
  await using resource = {
    async [Symbol.asyncDispose]() { events.push("async-dispose"); },
  };
  const value = try fail();
  return Res.Ok(value);
};

console.log(JSON.stringify(sync()), events.join(","));
events.length = 0;
asyncRun().then((value) => console.log(JSON.stringify(value), events.join(",")));
"#,
        &["--lib", "es2022,dom,esnext.disposable"],
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Err","error":"boom"} sync-dispose"#,
            r#"{"kind":"Err","error":"boom"} async-dispose"#,
        ]
    );
}

#[test]
fn runtime_nested_results_preserve_constructor_and_generator_protocols() {
    require_toolchain!();
    for source in [
        "class C { constructor() { try fail(); } }\n",
        "function* values() { yield try fail(); }\n",
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{source}\n{diagnostics:#?}");
        assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::TryPlacement);
    }

    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
const fail = (): Res<number, string> => Res.Err("boom");

class C {
  outcome;
  constructor() {
    this.outcome = result { return try fail(); };
  }
}

function* values() {
  yield result { return try fail(); };
  yield "after";
}

const instance = new C();
console.log(instance instanceof C, JSON.stringify(instance.outcome));
const iterator = values();
console.log(JSON.stringify(iterator.next()));
console.log(JSON.stringify(iterator.next()));
console.log(Array.from(values()).map((value) => JSON.stringify(value)).join(","));
"#);
    assert_eq!(
        lines,
        vec![
            r#"true {"kind":"Err","error":"boom"}"#,
            r#"{"value":{"kind":"Err","error":"boom"},"done":false}"#,
            r#"{"value":"after","done":false}"#,
            r#"{"kind":"Err","error":"boom"},"after""#,
        ]
    );
}

#[test]
fn runtime_result_block_with_await_resolves_to_a_result() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }

const fetchNum = async (n: number): Promise<Res<number, string>> =>
  n > 0 ? Res.Ok(n) : Res.Err("not positive");

const total = async (a: number, b: number) => result {
  const x = try await fetchNum(a);
  const y = try await fetchNum(b);
  return x + y;
};

total(2, 3).then((r) => console.log(JSON.stringify(r)));
total(2, -1).then((r) => console.log(JSON.stringify(r)));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":5}"#,
            r#"{"kind":"Err","error":"not positive"}"#,
        ]
    );
}

#[test]
fn runtime_result_exits_cross_user_breakable_statements() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
const events: string[] = [];
const step = (ok: boolean, name: string): Res<number, string> =>
  ok ? Res.Ok(name.length) : Res.Err(name);

const fromFor = (ok: boolean) => result {
  for (const name of ["for"]) { return try step(ok, name); }
  events.push("for-tail");
  return 99;
};
const fromWhile = (ok: boolean) => result {
  while (true) { return try step(ok, "while"); }
  events.push("while-tail");
  return 99;
};
const fromDo = (ok: boolean) => result {
  do { return try step(ok, "do"); } while (false);
  events.push("do-tail");
  return 99;
};
const fromSwitch = (ok: boolean) => result {
  switch (ok) { default: return try step(ok, "switch"); }
  events.push("switch-tail");
  return 99;
};

for (const run of [fromFor, fromWhile, fromDo, fromSwitch]) {
  console.log(JSON.stringify(run(true)), JSON.stringify(run(false)));
}
console.log(events.join(","));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":3} {"kind":"Err","error":"for"}"#,
            r#"{"kind":"Ok","value":5} {"kind":"Err","error":"while"}"#,
            r#"{"kind":"Ok","value":2} {"kind":"Err","error":"do"}"#,
            r#"{"kind":"Ok","value":6} {"kind":"Err","error":"switch"}"#,
            "",
        ]
    );
}

#[test]
fn runtime_result_preserves_statement_match_effect_order() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
const events: string[] = [];
const read = (): Res<number, string> => Res.Ok(7);
const subject = (tag: number) => { events.push("subject-" + tag); return tag; };

const run = (tag: number) => result {
  const value = try read();
  match (subject(tag)) {
    1 => { events.push("one"); },
    _ => { events.push("other"); },
  }
  events.push("after");
  return value;
};

console.log(JSON.stringify(run(1)), events.join(","));
events.length = 0;
console.log(JSON.stringify(run(2)), events.join(","));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":7} subject-1,one,after"#,
            r#"{"kind":"Ok","value":7} subject-2,other,after"#,
        ]
    );
}

#[test]
fn runtime_ordinary_result_success_preserves_expression_host_protocols() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
const read = (value: number): Res<number, string> => Res.Ok(value);

class FieldBox {
  field = result { const value = try read(1); return value; };
}
class ConstructorBox {
  outcome;
  constructor() {
    this.outcome = result { const value = try read(2); return value; };
  }
}
function withDefault(value = result { const item = try read(3); return item; }) {
  return value;
}
function* values() {
  yield result { const item = try read(4); return item; };
  yield "after";
}
const text = `value=${result { const item = try read(5); return item; }}`;

console.log(JSON.stringify(new FieldBox().field));
console.log(JSON.stringify(new ConstructorBox().outcome));
console.log(JSON.stringify(withDefault()));
console.log(Array.from(values()).map((value) => JSON.stringify(value)).join(","));
console.log(text);
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":1}"#,
            r#"{"kind":"Ok","value":2}"#,
            r#"{"kind":"Ok","value":3}"#,
            r#"{"kind":"Ok","value":4},"after""#,
            "value=[object Object]",
        ]
    );
}

#[test]
fn strict_typescript_accepts_all_result_discriminator_shapes() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
type Alias<T, E> = Res<T, E>;

const directErr = () => {
  const value = try Res.Err("direct");
  return Res.Ok(value);
};
const directOk = () => {
  const value = try Res.Ok(1);
  return Res.Ok(value + 1);
};
const widened = (input: Res<number, string>): Res<number, string> => {
  const value = try input;
  return Res.Ok(value + 1);
};
const aliased = (input: Alias<number, string>): Alias<number, string> => {
  const value = try input;
  return Res.Ok(value + 1);
};
function generic<T, E>(input: Res<T, E>): Res<T, E> {
  const value = try input;
  return Res.Ok(value);
}

console.log(JSON.stringify(directErr()));
console.log(JSON.stringify(directOk()));
console.log(JSON.stringify(widened(Res.Err("wide"))));
console.log(JSON.stringify(aliased(Res.Ok(2))));
console.log(JSON.stringify(generic(Res.Ok("generic"))));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Err","error":"direct"}"#,
            r#"{"kind":"Ok","value":2}"#,
            r#"{"kind":"Err","error":"wide"}"#,
            r#"{"kind":"Ok","value":3}"#,
            r#"{"kind":"Ok","value":"generic"}"#,
        ]
    );
}

#[test]
fn runtime_result_block_replaces_nested_combinator_callbacks() {
    require_toolchain!();
    // The motivating shape: three dependent steps that all stay in scope,
    // written flat, against the real standard library.
    let lines = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type User = { id: number; companyId: number; name: string };
type Company = { id: number; name: string };

const getUser = (id: number): TResult<User, string> =>
  id === 1 ? Result.Ok({ id, companyId: 7, name: " Ada " }) : Result.Err("no user " + id);
const getCompany = (id: number): TResult<Company, string> =>
  Result.Ok({ id, name: "Acme" });
const getPermission = (u: User, c: Company): TResult<string, string> =>
  Result.Ok(u.name.trim() + "@" + c.name);

const view = (id: number) => result {
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  const normalized = user.name |> .trim() |> .toLowerCase();
  const permission = try getPermission(user, company);
  return { user, company, permission, normalized };
};

console.log(JSON.stringify(view(1)));
console.log(JSON.stringify(view(2)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":{"user":{"id":1,"companyId":7,"name":" Ada "},"company":{"id":7,"name":"Acme"},"permission":"Ada@Acme","normalized":"ada"}}"#,
            r#"{"kind":"Err","error":"no user 2"}"#,
        ]
    );
}

/* ------------------------------------------------------------------ */
/* literal match patterns                                              */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_literal_string_match() {
    require_toolchain!();
    let lines = run(r#"
type Direction = "north" | "south" | "east" | "west";

function short(dir: Direction) {
  return match (dir) {
    "north" => "N",
    "south" => "S",
    "east" => "E",
    "west" => "W",
  };
}

console.log(short("north"), short("south"), short("east"), short("west"));
"#);
    assert_eq!(lines, ["N S E W"]);
}

#[test]
fn runtime_literal_number_match_with_or_patterns() {
    require_toolchain!();
    let lines = run(r#"
function status(code: 200 | 201 | 404 | 500) {
  return match (code) {
    200 | 201 => "success",
    404 => "not found",
    500 => "server error",
  };
}

console.log(status(200), status(201), status(404), status(500));
"#);
    assert_eq!(lines, ["success success not found server error"]);
}

#[test]
fn runtime_literal_boolean_match() {
    require_toolchain!();
    let lines = run(r#"
function label(flag: boolean) {
  return match (flag) {
    true => "yes",
    false => "no",
  };
}

console.log(label(true), label(false));
"#);
    assert_eq!(lines, ["yes no"]);
}

#[test]
fn runtime_literal_match_keeps_number_spellings() {
    require_toolchain!();
    let lines = run(r#"
function pick(n: number) {
  return match (n) {
    0xff => "hex",
    1_000 => "sep",
    1.5e2 => "exp",
    -1 => "neg",
    _ => "other",
  };
}

console.log(pick(255), pick(1000), pick(150), pick(-1), pick(0));
"#);
    assert_eq!(lines, ["hex sep exp neg other"]);
}

#[test]
fn runtime_literal_match_evaluates_the_scrutinee_once() {
    require_toolchain!();
    let lines = run(r#"
let calls = 0;
function getValue(): string {
  calls += 1;
  return "b";
}

const picked = match (getValue()) {
  "a" => 1,
  "b" => 2,
  _ => 3,
};
console.log(picked, calls);
"#);
    assert_eq!(lines, ["2 1"]);
}

#[test]
fn runtime_literal_match_runtime_guard_throws() {
    require_toolchain!();
    let lines = run(r#"
function label(dir: string) {
  return match (dir as "a" | "b") {
    "a" => 1,
    "b" => 2,
  };
}

try {
  label("zzz");
  console.log("no throw");
} catch (e) {
  console.log((e as Error).message);
}
"#);
    assert_eq!(lines, [r#"tt match: unexpected literal "zzz""#]);
}

#[test]
fn runtime_literal_match_with_guard() {
    require_toolchain!();
    let lines = run(r#"
function classify(code: number, retry: boolean) {
  return match (code) {
    500 if retry => "retrying",
    500 => "failed",
    _ => "ok",
  };
}

console.log(classify(500, true), classify(500, false), classify(200, true));
"#);
    assert_eq!(lines, ["retrying failed ok"]);
}

#[test]
fn typecheck_literal_match_narrows_each_arm() {
    require_toolchain!();
    // The switch discriminates on the value itself, so tsc narrows the
    // scrutinee inside each arm with no type tricks.
    let (ok, out) = typecheck(
        r#"
type Size = "sm" | "md" | "lg";
const px: number = match ("sm" as Size) {
  "sm" => 12,
  "md" => 16,
  "lg" => 20,
};
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn typecheck_literal_match_block_bodies() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
const label: string = match ("a" as "a" | "b") {
  "a" => { return "first"; },
  "b" => { return "second"; },
};
"#,
    );
    assert!(ok, "{out}");
}

/* ------------------------------------------------------------------ */
/* val — binding modifier                                             */
/* ------------------------------------------------------------------ */

#[test]
fn typecheck_val_bindings_are_plain_typescript() {
    require_toolchain!();
    // `val` is compile-time only: what reaches tsc is an ordinary
    // declaration and an ordinary parameter, with no readonly types and
    // no runtime helper.
    let (ok, out) = typecheck(
        r#"
type User = { name: string; tags: string[] };

val const user: User = { name: "Kim", tags: ["dev"] };

function inspect(val u: User): string {
  return u.name + u.tags.length;
}

val let state = { count: 0 };
state = { ...state, count: state.count + 1 };

const label = inspect(user) + state.count;
"#,
    );
    assert!(ok, "{out}");
    assert!(
        !out.contains("val "),
        "the modifier leaked into the output: {out}"
    );
    assert!(!out.contains("readonly"), "{out}");
}
