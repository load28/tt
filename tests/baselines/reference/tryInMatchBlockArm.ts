//// [tryInMatchBlockArm.tt] ////
// A `try` in a match block arm leaves the enclosing function with its `Err`,
// as it does in an expression arm (docs/design/try-result-scopes.md 4.6:
// the function target stays legal in an isolated value region): the arm's
// own `return` still delivers the match value. This holds for the statement
// forms and the value form, and wherever the match is lowered to statements
// (a declaration, a template, a conditional branch, a concise arrow's body, a
// loop test, a nested match).
import type { TResult } from "@tt/std";
const log: string[] = [];
const on = (): boolean => true;
function read(n: number): TResult<number, string> {
  log.push(`read ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `bad ${n}` };
}
function declared(flag: boolean, n: number): TResult<number, string> {
  const b = match (flag) { true => { const v = try read(n); return v + 1; }, false => 0 };
  log.push("after declared");
  return { kind: "Ok", value: b };
}
function propagated(n: number): TResult<string, string> {
  const b = match (n > -10) { true => { try read(n); return "checked"; }, false => "skipped" };
  return { kind: "Ok", value: b };
}
function valueForm(n: number): TResult<number, string> {
  const b = match (n) { 0 => { return try read(n); }, _ => { const k = 1 + try read(n); return k; } };
  return { kind: "Ok", value: b };
}
function templated(n: number): TResult<string, string> {
  const s = `<${match (on()) { true => { const v = try read(n); return v; }, false => 0 }}>`;
  return { kind: "Ok", value: s };
}
function branch(flag: boolean, n: number): TResult<number, string> {
  const b = flag ? match (on()) { true => { const v = try read(n); return v; }, false => 0 } : -1;
  return { kind: "Ok", value: b };
}
function arrow(ns: number[]): TResult<number[], string> {
  const inner = (x: number): TResult<number, string> => ({
    kind: "Ok",
    value: match (x > -100) { true => { const v = try read(x); return v * 2; }, false => 0 },
  });
  const out: number[] = [];
  for (const x of ns) {
    const r = inner(x);
    if (r.kind === "Err") return r;
    out.push(r.value);
  }
  return { kind: "Ok", value: out };
}
function looped(n: number): TResult<number, string> {
  let i = 0;
  while (i < 2 && match (on()) { true => { const v = try read(n); return v >= 0; }, false => false }) {
    i++;
  }
  return { kind: "Ok", value: i };
}
function nested(n: number): TResult<number, string> {
  const b = match (on()) {
    true => match (n) { 7 => { const v = try read(n); return v; }, _ => { const w = try read(n); return -w; } },
    false => 0,
  };
  return { kind: "Ok", value: b };
}
for (const n of [3, -1]) {
  log.length = 0;
  console.log(JSON.stringify([declared(true, n), declared(false, n), propagated(n), valueForm(n)]), log.join(","));
  log.length = 0;
  console.log(JSON.stringify([templated(n), branch(true, n), branch(false, n), arrow([n, 1]), looped(n), nested(n)]), log.join(","));
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [tryInMatchBlockArm.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
// A `try` in a match block arm leaves the enclosing function with its `Err`,
// as it does in an expression arm (docs/design/try-result-scopes.md 4.6:
// the function target stays legal in an isolated value region): the arm's
// own `return` still delivers the match value. This holds for the statement
// forms and the value form, and wherever the match is lowered to statements
// (a declaration, a template, a conditional branch, a concise arrow's body, a
// loop test, a nested match).
import type { TResult } from "./tt/index.js";
const log: string[] = [];
const on = (): boolean => true;
function read(n: number): TResult<number, string> {
  log.push(`read ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `bad ${n}` };
}
function declared(flag: boolean, n: number): TResult<number, string> {
  let $tt_v0: number;
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        const $tt_t0 = read(n);
        if (!("value" in $tt_t0)) {
          return $tt_t0;
        }
        const v = $tt_t0.value; $tt_v0 = v + 1; break;
      }
      case false: {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const b = $tt_v0;
  log.push("after declared");
  return { kind: "Ok", value: b };
}
function propagated(n: number): TResult<string, string> {
  let $tt_v1: string;
  {
    const $tt_m = n > -10;
    switch ($tt_m) {
      case true: {
        const $tt_t1 = read(n);
        if (!("value" in $tt_t1)) {
          return $tt_t1;
        } $tt_v1 = "checked"; break;
      }
      case false: {
        $tt_v1 = "skipped";
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const b = $tt_v1;
  return { kind: "Ok", value: b };
}
function valueForm(n: number): TResult<number, string> {
  let $tt_v2: number;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 0: {
        const $tt_t2 = read(n);
        if (!("value" in $tt_t2)) {
          return $tt_t2;
        }
        $tt_v2 = $tt_t2.value;
    break;
      }
      default: {
        let $tt_v3: number;
        const $tt_t3 = read(n);
        if (!("value" in $tt_t3)) {
          return $tt_t3;
        }
        $tt_v3 = $tt_t3.value;
        const k = 1 + $tt_v3; $tt_v2 = k; break;
      }
    }
  }
  const b = $tt_v2;
  return { kind: "Ok", value: b };
}
function templated(n: number): TResult<string, string> {
  let $tt_v4: number;
  {
    const $tt_m = on();
    switch ($tt_m) {
      case true: {
        const $tt_t4 = read(n);
        if (!("value" in $tt_t4)) {
          return $tt_t4;
        }
        const v = $tt_t4.value; $tt_v4 = v; break;
      }
      case false: {
        $tt_v4 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const s = `<${$tt_v4}>`;
  return { kind: "Ok", value: s };
}
function branch(flag: boolean, n: number): TResult<number, string> {
  let $tt_v7: number;
  if (flag) {
    {
      const $tt_m = on();
      switch ($tt_m) {
        case true: {
          const $tt_t5 = read(n);
          if (!("value" in $tt_t5)) {
            return $tt_t5;
          }
          const v = $tt_t5.value; $tt_v7 = v; break;
        }
        case false: {
          $tt_v7 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v7 = -1;
  }
  
  const b = $tt_v7;
  return { kind: "Ok", value: b };
}
function arrow(ns: number[]): TResult<number[], string> {
  const inner = (x: number): TResult<number, string> => {
    let $tt_v8: number;
    {
      const $tt_m = x > -100;
      switch ($tt_m) {
        case true: {
          const $tt_t6 = read(x);
          if (!("value" in $tt_t6)) {
            return $tt_t6;
          }
          const v = $tt_t6.value; $tt_v8 = v * 2; break;
        }
        case false: {
          $tt_v8 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
        }
      }
    }
    return ({
    kind: "Ok",
    value: $tt_v8,
  });
  };
  const out: number[] = [];
  for (const x of ns) {
    const r = inner(x);
    if (r.kind === "Err") return r;
    out.push(r.value);
  }
  return { kind: "Ok", value: out };
}
function looped(n: number): TResult<number, string> {
  let i = 0;
  while (true) {
    let $tt_v11: boolean;
    let $tt_v10: boolean;
    if ($tt_v10 = i < 2) {
      let $tt_v9: boolean;
      {
        const $tt_m = on();
        switch ($tt_m) {
          case true: {
            const $tt_t7 = read(n);
            if (!("value" in $tt_t7)) {
              return $tt_t7;
            }
            const v = $tt_t7.value; $tt_v9 = v >= 0; break;
          }
          case false: {
            $tt_v9 = false;
            break;
          }
          default: {
            throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
          }
        }
      }
      $tt_v11 = $tt_v10 && $tt_v9;
    } else {
      $tt_v11 = $tt_v10;
    }
    
    if (!($tt_v11)) break; {
    i++;
  }}
  return { kind: "Ok", value: i };
}
function nested(n: number): TResult<number, string> {
  let $tt_v12: number;
  {
    const $tt_m = on();
    switch ($tt_m) {
      case true: {
        {
          const $tt_m = n;
          switch ($tt_m) {
            case 7: {
              const $tt_t8 = read(n);
              if (!("value" in $tt_t8)) {
                return $tt_t8;
              }
              const v = $tt_t8.value; $tt_v12 = v; break;
            }
            default: {
              const $tt_t9 = read(n);
              if (!("value" in $tt_t9)) {
                return $tt_t9;
              }
              const w = $tt_t9.value; $tt_v12 = -w; break;
            }
          }
        }
        break;
      }
      case false: {
        $tt_v12 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const b = $tt_v12;
  return { kind: "Ok", value: b };
}
for (const n of [3, -1]) {
  log.length = 0;
  console.log(JSON.stringify([declared(true, n), declared(false, n), propagated(n), valueForm(n)]), log.join(","));
  log.length = 0;
  console.log(JSON.stringify([templated(n), branch(true, n), branch(false, n), arrow([n, 1]), looped(n), nested(n)]), log.join(","));
}
