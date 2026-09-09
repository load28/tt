use super::*;

#[test]
fn inline_decisions_keep_the_enclosing_match_return_owner() {
    require_toolchain!();
    let output = run(r#"
variant Opt { Some(value: number), None }
function pick(o: Opt): number {
  const v = match (o) {
    Some(value) => {
      for (let i = 0; i < 1; i++) {
        if let Some(value: v2) = o {
          const nested = () => { return v2 + 1; };
          try { return nested(); } finally { console.log("cleanup"); }
        }
      }
      return value;
    },
    None => 0,
  };
  return v + 100;
}
function fallback(o: Opt): number {
  const v = match (Boolean(o)) {
    true => {
      if let Some(value: n) = o { return n; } else { return 7; }
      return 9;
    },
    false => 0,
  };
  return v + 100;
}
console.log(pick(Opt.Some(5)), pick(Opt.None));
console.log(fallback(Opt.None), fallback(Opt.Some(3)));
"#);
    assert_eq!(output, ["cleanup", "106 100", "107 103"]);
}

#[test]
fn template_captures_preserve_delimiters_and_evaluation_order() {
    require_toolchain!();
    let output = run(r#"
variant Shape { Circle(radius: number), Point }
const s = Shape.Circle(5);
const trace: string[] = [];
function before() { trace.push("before"); return 2; }
function arm(n: number) { trace.push("arm"); return n; }
const values = [1].map(x => `${before()}${x}` + match(s) { Circle(radius) => arm(radius), Point => 0 });
console.log(values[0], trace.join(","));
console.log(`${1}${match(s) { Circle(radius) => radius, Point => 0 }}${2}`);
"#);
    assert_eq!(output, ["215 before,arm", "152"]);
}

#[test]
fn result_expression_propagation_uses_its_lexical_failure_edge() {
    require_toolchain!();
    let output = run(r#"
type R<T> = {kind: "Ok"; value: T} | {kind: "Err"; error: string};
const trace: string[] = [];
function read(ok: boolean): R<number> { trace.push("read"); return ok ? {kind: "Ok", value: 3} : {kind: "Err", error: "bad"}; }
function before() { trace.push("before"); return 2; }
function compute(ok: boolean) {
  const r = result {
    let company;
    company = before() + try read(ok);
    trace.push("after");
    return company;
  };
  trace.push("outside");
  return r;
}
for (const ok of [true, false]) {
  trace.length = 0;
  console.log(JSON.stringify(compute(ok)), trace.join(","));
}
const nested = result {
  const a = try read(true);
  const inner = result { let b; b = try read(false); return b; };
  return [a, inner.kind];
};
console.log(JSON.stringify(nested));
const skipped = result { let n = 0; n = false ? try read(false) : 4; return n; };
console.log(JSON.stringify(skipped));
"#);
    assert_eq!(
        output,
        [
            r#"{"kind":"Ok","value":5} before,read,after,outside"#,
            r#"{"kind":"Err","error":"bad"} before,read,outside"#,
            r#"{"kind":"Ok","value":[3,"Err"]}"#,
            r#"{"kind":"Ok","value":4}"#,
        ]
    );
}

#[test]
fn join_storage_preserves_array_inference_under_strict_checking() {
    require_toolchain!();
    let (valid, diagnostics) = typecheck(
        r#"
variant Shape { Circle(radius: number), Point }
declare const s: Shape;
export const spread = match(s) { Circle(radius) => [radius], Point => [] };
spread.push(1);
export const n: number = spread.length;
export const block = match(s) { Circle(radius) => { return [radius]; }, Point => { return []; } };
block.push(2);
const invalid: string[] = spread;
"#,
    );
    assert!(!valid, "the inferred element type must reject string[]");
    assert!(diagnostics.contains("TS2322"), "{diagnostics}");
    assert!(
        !diagnostics.contains("TS7034")
            && !diagnostics.contains("TS7005")
            && !diagnostics.contains("TS2345"),
        "{diagnostics}"
    );
}
