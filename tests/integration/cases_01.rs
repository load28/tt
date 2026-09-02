#[test]
fn runtime_or_patterns_share_one_body() {
    require_toolchain!();
    let lines = run(r#"
variant Key {
  Enter(),
  Escape,
  Tab,
  Char(ch: string),
}

function action(k: Key): string {
  return match (k) {
    Enter => "submit",
    Escape | Tab => "cancel",
    Char(ch) => "type:" + ch,
  };
}

console.log(action(Key.Enter()));
console.log(action(Key.Escape));
console.log(action(Key.Tab));
console.log(action(Key.Char("z")));
"#);
    assert_eq!(lines, vec!["submit", "cancel", "cancel", "type:z"]);
}

#[test]
fn runtime_match_guards_fall_through_top_to_bottom() {
    require_toolchain!();
    let lines = run(r#"
variant Score {
  Graded(points: number),
  Pending,
}

function grade(s: Score): string {
  return match (s) {
    Graded(points) if points >= 90 => "A",
    Graded(points) if points >= 80 => "B",
    Graded(points) => "F",
    Pending => "-",
  };
}

function tally(s: Score): number {
  return match (s) {
    Graded(points) if points > 0 => {
      const doubled = points * 2;
      return doubled;
    },
    _ => 0,
  };
}

console.log(grade(Score.Graded(95)));
console.log(grade(Score.Graded(85)));
console.log(grade(Score.Graded(10)));
console.log(grade(Score.Pending));
console.log(tally(Score.Graded(3)));
console.log(tally(Score.Graded(-1)));
"#);
    assert_eq!(lines, vec!["A", "B", "F", "-", "6", "0"]);
}

#[test]
fn runtime_generic_variant() {
    require_toolchain!();
    let lines = run(r#"
variant TOption<T> {
  Some(value: T),
  None,
}

function unwrapOr<T>(o: TOption<T>, fallback: T): T {
  return match (o) {
    Some(value) => value,
    None => fallback,
  };
}

console.log(unwrapOr(TOption.Some(7), 0));
console.log(unwrapOr<number>(TOption.None, 42));
"#);
    assert_eq!(lines, vec!["7", "42"]);
}

#[test]
fn runtime_async_match_with_await() {
    require_toolchain!();
    let lines = run(r#"
variant Job {
  Fetch(n: number),
  Idle,
}

async function double(n: number): Promise<number> {
  return n * 2;
}

async function runJob(j: Job): Promise<number> {
  return match (j) {
    Fetch(n) => await double(n),
    Idle => 0,
  };
}

runJob(Job.Fetch(21)).then((a) => {
  console.log(a);
  return runJob(Job.Idle);
}).then((b) => {
  console.log(b);
});
"#);
    assert_eq!(lines, vec!["42", "0"]);
}

#[test]
fn runtime_unexpected_case_throws() {
    require_toolchain!();
    // The emitted default branch is a plain runtime guard — it protects when
    // the type system was bypassed (e.g. data from the outside world).
    let lines = run(r#"
variant AB { A(n: number), B }
function f(x: AB): number {
  return match (x) {
    A(n) => n,
    B => 2,
  };
}
const g = f as unknown as (x: { kind: string }) => number;
try {
  g({ kind: "C" });
} catch (e) {
  console.log("threw: " + (e as Error).message);
}
"#);
    assert_eq!(
        lines,
        vec![r#"threw: tt match: unexpected case {"kind":"C"}"#]
    );
}

#[test]
fn runtime_plain_typescript_enum_coexists() {
    require_toolchain!();
    // TypeScript enum stays untouched while a unit-only tt variant lowers.
    let lines = run(r#"
enum Color { Red, Green, Blue }
variant Shape { Circle(radius: number), Point }

console.log(Color.Green);
console.log(Color[Color.Blue]);
console.log(JSON.stringify(Shape.Circle(1)));
"#);
    assert_eq!(lines, vec!["1", "Blue", r#"{"kind":"Circle","radius":1}"#]);
}

#[test]
fn runtime_std_option_result_functional_pipeline() {
    require_toolchain!();
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("not a number: " + raw) : Result.Ok(n);
}

const half = (n: number): TOption<number> =>
  n % 2 === 0 ? Option.Some(n / 2) : Option.None;

const describe = (raw: string): string =>
  match (parseNum(raw)) {
    Ok(value) => match (half(value)) {
      Some(value: h) => "half=" + h,
      None => "odd:" + value,
    },
    Err(error) => "error:" + error,
  };

console.log(describe("42"));
console.log(describe("7"));
console.log(describe("x"));
console.log(Option.unwrapOr(Option.map(Option.fromNullable([1, 2].find((n) => n > 1)), (n) => n * 2), -1));
console.log(Result.unwrapOr(Result.andThen(parseNum("10"), (n): TResult<number, string> => n > 5 ? Result.Ok(n * 2) : Result.Err("small")), -1));
console.log(Result.isErr(Result.fromThrowable(() => JSON.parse("{"))));
"#,
    );
    assert_eq!(
        lines,
        vec![
            "half=21",
            "odd:7",
            "error:not a number: x",
            "4",
            "20",
            "true"
        ]
    );
}

#[test]
fn runtime_std_new_combinators() {
    require_toolchain!();
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

console.log(JSON.stringify(Option.zip(Option.Some(1), Option.Some("a"))));
console.log(JSON.stringify(Option.zip(Option.Some(1), Option.None)));
console.log(JSON.stringify(Option.flatten(Option.Some(Option.Some(2)))));
console.log(JSON.stringify(Option.collect([Option.Some(1), Option.Some(2)])));
console.log(JSON.stringify(Option.collect([Option.Some(1), Option.None])));
console.log(JSON.stringify(Option.transpose(Option.Some(Result.Ok<number>(3)))));
console.log(JSON.stringify(Result.collect([Result.Ok(1), Result.Ok(2)])));
console.log(JSON.stringify(Result.collect<number, string>([Result.Ok(1), Result.Err("x")])));
console.log(JSON.stringify(Result.flatten<number, string>(Result.Ok(Result.Ok(4)))));
const nested: TResult<TOption<number>, string> = Result.Ok(Option.None);
console.log(JSON.stringify(Result.transpose(nested)));
Result.fromPromise(Promise.resolve(5))
  .then((r) => console.log(JSON.stringify(r)))
  .then(() => Result.fromPromise(Promise.reject("boom")))
  .then((r) => console.log(JSON.stringify(r)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Some","value":[1,"a"]}"#,
            r#"{"kind":"None"}"#,
            r#"{"kind":"Some","value":2}"#,
            r#"{"kind":"Some","value":[1,2]}"#,
            r#"{"kind":"None"}"#,
            r#"{"kind":"Ok","value":{"kind":"Some","value":3}}"#,
            r#"{"kind":"Ok","value":[1,2]}"#,
            r#"{"kind":"Err","error":"x"}"#,
            r#"{"kind":"Ok","value":4}"#,
            r#"{"kind":"None"}"#,
            r#"{"kind":"Ok","value":5}"#,
            r#"{"kind":"Err","error":"boom"}"#,
        ]
    );
}

#[test]
fn runtime_try_error_propagation() {
    require_toolchain!();
    let lines = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("not a number: " + raw) : Result.Ok(n);
}

function sumList(raws: string[]): TResult<number, string> {
  let total = 0;
  for (const raw of raws) {
    const n = try parseNum(raw);
    total += n;
  }
  return Result.Ok(total);
}

function checked(raw: string): TResult<number, string> {
  try parseNum(raw);
  let big: number = try parseNum(raw);
  return Result.Ok(big * 10);
}

function adjusted(raw: string): TResult<number, string> {
  return Result.Ok(Math.round(try parseNum(raw) * 1.1));
}

console.log(JSON.stringify(sumList(["1", "2", "3"])));
console.log(JSON.stringify(sumList(["1", "x"])));
console.log(JSON.stringify(checked("4")));
console.log(JSON.stringify(adjusted("5")));
console.log(JSON.stringify(adjusted("x")));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":6}"#,
            r#"{"kind":"Err","error":"not a number: x"}"#,
            r#"{"kind":"Ok","value":40}"#,
            r#"{"kind":"Ok","value":6}"#,
            r#"{"kind":"Err","error":"not a number: x"}"#,
        ]
    );
}

#[test]
fn runtime_or_patterns_in_let_else_and_if_let() {
    require_toolchain!();
    // tsc --strict must accept both shapes: the let-else guard narrows the
    // temporary to the alternatives' union for the shared destructuring,
    // and the if-let disjunction narrows inside the then-block.
    let lines = run(r#"
variant Shape { Circle(r: number), Square(r: number), Dot }

function side(s: Shape): number {
  const Circle(r) | Square(r) = s else { return 0; };
  return r;
}

function tell(s: Shape): string {
  if let Circle(r) | Square(r) = s {
    return "sized " + r;
  } else {
    return "dot";
  }
}

console.log(side(Shape.Circle(3)));
console.log(side(Shape.Square(4)));
console.log(side(Shape.Dot));
console.log(tell(Shape.Square(5)));
console.log(tell(Shape.Dot));
"#);
    assert_eq!(lines, vec!["3", "4", "0", "sized 5", "dot"]);
}

#[test]
fn runtime_try_inside_an_if_let_body_propagates_from_the_function() {
    require_toolchain!();
    // The if-let body is inline in the enclosing function, so the `try`
    // propagates from `f` — not from any construct in between.
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("not a number: " + raw) : Result.Ok(n);
}

function f(o: TOption<string>): TResult<number, string> {
  if let Some(value) = o {
    const n = try parseNum(value);
    return Result.Ok(n * 10);
  }
  return Result.Ok(-1);
}

console.log(JSON.stringify(f(Option.Some("7"))));
console.log(JSON.stringify(f(Option.Some("x"))));
console.log(JSON.stringify(f(Option.None)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":70}"#,
            r#"{"kind":"Err","error":"not a number: x"}"#,
            r#"{"kind":"Ok","value":-1}"#,
        ]
    );
}

#[test]
fn runtime_try_inside_a_closure_propagates_from_the_closure() {
    require_toolchain!();
    // Rust's `?` inside a closure: the `try` inside the arrow written in a
    // match scrutinee returns from the *arrow*, and the match sees the
    // Result it produced.
    let lines = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("not a number: " + raw) : Result.Ok(n);
}

function describe(raw: string): string {
  return match (((): TResult<number, string> => {
    const n = try parseNum(raw);
    return Result.Ok(n * 2);
  })()) {
    Ok(value) => "doubled: " + value,
    Err(error) => "failed: " + error,
  };
}

console.log(describe("21"));
console.log(describe("x"));
"#,
    );
    assert_eq!(lines, vec!["doubled: 42", "failed: not a number: x"]);
}

#[test]
fn runtime_let_else_narrows_and_diverges() {
    require_toolchain!();
    // tsc --strict must accept the emitted destructuring: the diverging
    // else block narrows the temporary to the matched case.
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function findUser(id: number): TOption<string> {
  return id === 1 ? Option.Some("amy") : Option.None;
}

function greet(id: number): string {
  const Some(value: user) = findUser(id) else { return "who?"; };
  return "hello, " + user;
}

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("bad") : Result.Ok(n);
}

function double(raw: string): number {
  const Ok(value) = parseNum(raw) else { return -1; };
  return value * 2;
}

console.log(greet(1));
console.log(greet(2));
console.log(double("21"));
console.log(double("x"));
"#,
    );
    assert_eq!(lines, vec!["hello, amy", "who?", "42", "-1"]);
}

#[test]
fn runtime_let_else_diverges_through_every_statement_form() {
    require_toolchain!();
    // TASK-172: the flow graph accepts a `switch`, a loop with no normal
    // exit, a `try`/`catch`, and a labeled `break` as diverging. Each
    // else block here really does leave the function on every path, so
    // the emitted narrowing must hold for `tsc --strict` and the values
    // must come out right at run time.
    let lines = run_with_std(
        r#"
import type { TOption } from "./tt/index.js";
import * as Option from "./tt/option.js";

function findUser(id: number): TOption<string> {
  return id === 1 ? Option.Some("amy") : Option.None;
}

// Every clause leaves, and a `default` catches what no case matched.
function bySwitch(id: number, kind: string): string {
  const Some(value: user) = findUser(id) else {
    switch (kind) {
      case "quiet": return "";
      default: return "who?";
    }
  };
  return "hello, " + user;
}

// A guarded block and its handler both leave.
function byTry(id: number): string {
  const Some(value: user) = findUser(id) else {
    try {
      return "missing " + id;
    } catch (e) {
      throw e;
    }
  };
  return "hello, " + user;
}

// Everything leaving normally runs the `finally` first.
function byFinally(id: number): string {
  const Some(value: user) = findUser(id) else {
    try {
      log("looking");
    } finally {
      return "gone";
    }
  };
  return "hello, " + user;
}

// A labeled `break` lands after the block, on the `return`.
function byLabel(id: number): string {
  const Some(value: user) = findUser(id) else {
    search: {
      if (id < 0) { break search; }
      return "unknown " + id;
    }
    return "negative";
  };
  return "hello, " + user;
}

// A loop with no normal exit is left only by `return`.
function byLoop(id: number): string {
  const Some(value: user) = findUser(id) else {
    while (true) {
      return "spun " + id;
    }
  };
  return "hello, " + user;
}

function log(_m: string): void {}

console.log(bySwitch(1, "loud"));
console.log(bySwitch(2, "loud"));
console.log(bySwitch(2, "quiet"));
console.log(byTry(1));
console.log(byTry(2));
console.log(byFinally(2));
console.log(byLabel(2));
console.log(byLabel(-1));
console.log(byLoop(2));
"#,
    );
    assert_eq!(
        lines,
        vec![
            "hello, amy",
            "who?",
            "",
            "hello, amy",
            "missing 2",
            "gone",
            "unknown 2",
            "negative",
            "spun 2",
        ]
    );
}

#[test]
fn runtime_let_else_diverges_through_an_inline_if_let() {
    require_toolchain!();
    // TASK-172: `if let` is the one tt construct that can carry a block's
    // divergence — its body and `else` are inline, so an exit written in
    // either leaves `classify`, not the construct. tsc --strict must
    // accept the narrowing that follows, and the values must come out
    // right at run time.
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function findUser(id: number): TOption<string> {
  return id === 1 ? Option.Some("amy") : Option.None;
}

function backup(id: number): TResult<string, string> {
  return id === 2 ? Result.Ok("bob") : Result.Err("none for " + id);
}

function classify(id: number): string {
  const Some(value: user) = findUser(id) else {
    if let Ok(value: fallback) = backup(id) {
      return "backup " + fallback;
    } else {
      return "nobody " + id;
    }
  };
  return "hello, " + user;
}

// A chained `else if let`, and a nested one in the then-half.
function chained(id: number): string {
  const Some(value: user) = findUser(id) else {
    if let Ok(value: fallback) = backup(id) {
      if let Some(value: again) = findUser(id) {
        return "both " + again;
      } else {
        return "backup " + fallback;
      }
    } else if let Err(error) = backup(id) {
      throw new Error(error);
    } else {
      return "unreachable";
    }
  };
  return "hello, " + user;
}

console.log(classify(1));
console.log(classify(2));
console.log(classify(3));
console.log(chained(2));
try {
  chained(3);
} catch (e) {
  console.log("threw " + (e as Error).message);
}
"#,
    );
    assert_eq!(
        lines,
        vec![
            "hello, amy",
            "backup bob",
            "nobody 3",
            "backup bob",
            "threw none for 3",
        ]
    );
}

#[test]
fn runtime_let_else_else_block_returns_an_object_literal() {
    require_toolchain!();
    // The natural shape for a `Result`-returning function: the else block
    // propagates an `Err` as an object literal. Its `}` ends no statement,
    // so the divergence check still sees a `return`.
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function findUser(id: number): TOption<string> {
  return id === 1 ? Option.Some("amy") : Option.None;
}

function greet(id: number): TResult<string, string> {
  const Some(value: user) = findUser(id) else { return { kind: "Err", error: "no user " + id }; };
  return { kind: "Ok", value: "hello, " + user };
}

console.log(JSON.stringify(greet(1)));
console.log(JSON.stringify(greet(2)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":"hello, amy"}"#,
            r#"{"kind":"Err","error":"no user 2"}"#,
        ]
    );
}

/* ------------------------------------------------------------------ */
/* the generated output is plain TypeScript: tsc accepts it            */
/* ------------------------------------------------------------------ */
