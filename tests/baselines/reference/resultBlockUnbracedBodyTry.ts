//// [resultBlockUnbracedBodyTry.tt] ////
// A `try` in a `result` block that sits in the unbraced body of an `if`,
// `else`, loop, or label opens its own block there, as it does in a
// function, and so does a returned `match` the block rebuilds.
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant S { A(n: number), B }
const ok = (n: number): TResult<number, string> => Result.Ok(n);
const bad = (e: string): TResult<number, string> => Result.Err(e);
function early(c: boolean, r: TResult<number, string>) {
  return result {
    if (c) return try r;
    return 0;
  };
}
function paren(c: boolean, r: TResult<number, string>) {
  return result {
    if (c) return (try r);
    return 0;
  };
}
function statement(c: boolean, r: TResult<number, string>) {
  return result {
    if (c) try r;
    return 1;
  };
}
function elseBody(c: boolean, r: TResult<number, string>) {
  return result {
    if (c) {} else return try r;
    return 2;
  };
}
function elseIf(c: boolean, d: boolean, r: TResult<number, string>) {
  return result {
    if (c) return 3;
    else if (d) try r;
    else if (!d) return try r;
    return 4;
  };
}
function nestedIf(c: boolean, d: boolean, r: TResult<number, string>) {
  return result {
    if (c) if (d) try r; else return try r;
    return 5;
  };
}
function loopBody(r: TResult<number, string>) {
  return result {
    for (;;) return try r;
  };
}
function whileBody(r: TResult<number, string>) {
  let turns = 0;
  return result {
    while (turns++ < 2) try r;
    return turns;
  };
}
function labelBody(r: TResult<number, string>) {
  return result {
    done: return try r;
  };
}
function returnedMatch(c: boolean, s: S, r: TResult<number, string>) {
  return result {
    const base = try r;
    if (c) return match (s) { A(n) => n + base, B => -1 };
    return 6;
  };
}
for (const r of [ok(5), bad("no")]) {
  console.log(JSON.stringify([
    early(true, r), early(false, r), paren(true, r), statement(true, r),
    statement(false, r), elseBody(false, r), elseBody(true, r),
  ]));
  console.log(JSON.stringify([
    elseIf(true, false, r), elseIf(false, true, r), elseIf(false, false, r),
    nestedIf(true, true, r), nestedIf(true, false, r), nestedIf(false, true, r),
  ]));
  console.log(JSON.stringify([
    loopBody(r), whileBody(r), labelBody(r),
    returnedMatch(true, S.A(7), r), returnedMatch(true, S.B, r), returnedMatch(false, S.B, r),
  ]));
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [resultBlockUnbracedBodyTry.ts]
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
// A `try` in a `result` block that sits in the unbraced body of an `if`,
// `else`, loop, or label opens its own block there, as it does in a
// function, and so does a returned `match` the block rebuilds.
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
type S =
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const ok = (n: number): TResult<number, string> => Result.Ok(n);
const bad = (e: string): TResult<number, string> => Result.Err(e);
function early(c: boolean, r: TResult<number, string>) {
  let $tt_v0: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v0: {
    if (c) {
      const $tt_t0 = r;
      if (!("value" in $tt_t0)) {
        $tt_v0 = $tt_t0;
        break $tt_v0;
      }
      const $tt_a0 = { value: { kind: "Ok" as const, value: $tt_t0.value } };
      $tt_v0 = $tt_a0.value;
      break $tt_v0;
    }
    {
      const $tt_a1 = { value: { kind: "Ok" as const, value: 0 } };
      $tt_v0 = $tt_a1.value;
      break $tt_v0;
    }
  }
  return $tt_v0;
}
function paren(c: boolean, r: TResult<number, string>) {
  let $tt_v1: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v1: {
    if (c) {
      const $tt_t1 = r;
      if (!("value" in $tt_t1)) {
        $tt_v1 = $tt_t1;
        break $tt_v1;
      }
      const $tt_a2 = { value: { kind: "Ok" as const, value: ($tt_t1.value) } };
      $tt_v1 = $tt_a2.value;
      break $tt_v1;
    }
    {
      const $tt_a3 = { value: { kind: "Ok" as const, value: 0 } };
      $tt_v1 = $tt_a3.value;
      break $tt_v1;
    }
  }
  return $tt_v1;
}
function statement(c: boolean, r: TResult<number, string>) {
  let $tt_v2: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v2: {
    if (c) {
      const $tt_t2 = r;
      if (!("value" in $tt_t2)) {
        $tt_v2 = $tt_t2;
        break $tt_v2;
      }
    }
    {
      const $tt_a4 = { value: { kind: "Ok" as const, value: 1 } };
      $tt_v2 = $tt_a4.value;
      break $tt_v2;
    }
  }
  return $tt_v2;
}
function elseBody(c: boolean, r: TResult<number, string>) {
  let $tt_v3: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v3: {
    if (c) {} else {
      const $tt_t3 = r;
      if (!("value" in $tt_t3)) {
        $tt_v3 = $tt_t3;
        break $tt_v3;
      }
      const $tt_a5 = { value: { kind: "Ok" as const, value: $tt_t3.value } };
      $tt_v3 = $tt_a5.value;
      break $tt_v3;
    }
    {
      const $tt_a6 = { value: { kind: "Ok" as const, value: 2 } };
      $tt_v3 = $tt_a6.value;
      break $tt_v3;
    }
  }
  return $tt_v3;
}
function elseIf(c: boolean, d: boolean, r: TResult<number, string>) {
  let $tt_v4: ({
    kind: "Ok";
    value: number;
}) | (Result.TErr<string>);
  $tt_v4: {
    if (c) { const $tt_a7 = { value: { kind: "Ok" as const, value: 3 } }; $tt_v4 = $tt_a7.value; break $tt_v4; }
    else if (d) {
      const $tt_t4 = r;
      if (!("value" in $tt_t4)) {
        $tt_v4 = $tt_t4;
        break $tt_v4;
      }
    }
    else if (!d) {
      const $tt_t5 = r;
      if (!("value" in $tt_t5)) {
        $tt_v4 = $tt_t5;
        break $tt_v4;
      }
      const $tt_a8 = { value: { kind: "Ok" as const, value: $tt_t5.value } };
      $tt_v4 = $tt_a8.value;
      break $tt_v4;
    }
    {
      const $tt_a9 = { value: { kind: "Ok" as const, value: 4 } };
      $tt_v4 = $tt_a9.value;
      break $tt_v4;
    }
  }
  return $tt_v4;
}
function nestedIf(c: boolean, d: boolean, r: TResult<number, string>) {
  let $tt_v5: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v5: {
    if (c) if (d) {
      const $tt_t6 = r;
      if (!("value" in $tt_t6)) {
        $tt_v5 = $tt_t6;
        break $tt_v5;
      }
    } else {
      const $tt_t7 = r;
      if (!("value" in $tt_t7)) {
        $tt_v5 = $tt_t7;
        break $tt_v5;
      }
      const $tt_a10 = { value: { kind: "Ok" as const, value: $tt_t7.value } };
      $tt_v5 = $tt_a10.value;
      break $tt_v5;
    }
    {
      const $tt_a11 = { value: { kind: "Ok" as const, value: 5 } };
      $tt_v5 = $tt_a11.value;
      break $tt_v5;
    }
  }
  return $tt_v5;
}
function loopBody(r: TResult<number, string>) {
  let $tt_v6: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v6: {
    for (;;) {
      const $tt_t8 = r;
      if (!("value" in $tt_t8)) {
        $tt_v6 = $tt_t8;
        break $tt_v6;
      }
      const $tt_a12 = { value: { kind: "Ok" as const, value: $tt_t8.value } };
      $tt_v6 = $tt_a12.value;
      break $tt_v6;
    }
  }
  return $tt_v6;
}
function whileBody(r: TResult<number, string>) {
  let turns = 0;
  let $tt_v7: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v7: {
    while (turns++ < 2) {
      const $tt_t9 = r;
      if (!("value" in $tt_t9)) {
        $tt_v7 = $tt_t9;
        break $tt_v7;
      }
    }
    {
      const $tt_a13 = { value: { kind: "Ok" as const, value: turns } };
      $tt_v7 = $tt_a13.value;
      break $tt_v7;
    }
  }
  return $tt_v7;
}
function labelBody(r: TResult<number, string>) {
  let $tt_v8: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v8: {
    done: {
      const $tt_t10 = r;
      if (!("value" in $tt_t10)) {
        $tt_v8 = $tt_t10;
        break $tt_v8;
      }
      const $tt_a14 = { value: { kind: "Ok" as const, value: $tt_t10.value } };
      $tt_v8 = $tt_a14.value;
      break $tt_v8;
    }
  }
  return $tt_v8;
}
function returnedMatch(c: boolean, s: S, r: TResult<number, string>) {
  let $tt_v9: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v9: {
    const $tt_t11 = r;
    if (!("value" in $tt_t11)) {
      $tt_v9 = $tt_t11;
      break $tt_v9;
    }
    const base = $tt_t11.value;
    if (c) {
      {
        const $tt_m = s;
        switch ($tt_m.kind) {
          case "A": {
            const { n } = $tt_m;
            const $tt_a15 = { value: { kind: "Ok" as const, value: n + base } };
            $tt_v9 = $tt_a15.value;
            break;
          }
          case "B": {
            const $tt_a16 = { value: { kind: "Ok" as const, value: -1 } };
            $tt_v9 = $tt_a16.value;
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      break $tt_v9;
    }
    {
      const $tt_a17 = { value: { kind: "Ok" as const, value: 6 } };
      $tt_v9 = $tt_a17.value;
      break $tt_v9;
    }
  }
  return $tt_v9;
}
for (const r of [ok(5), bad("no")]) {
  console.log(JSON.stringify([
    early(true, r), early(false, r), paren(true, r), statement(true, r),
    statement(false, r), elseBody(false, r), elseBody(true, r),
  ]));
  console.log(JSON.stringify([
    elseIf(true, false, r), elseIf(false, true, r), elseIf(false, false, r),
    nestedIf(true, true, r), nestedIf(true, false, r), nestedIf(false, true, r),
  ]));
  console.log(JSON.stringify([
    loopBody(r), whileBody(r), labelBody(r),
    returnedMatch(true, S.A(7), r), returnedMatch(true, S.B, r), returnedMatch(false, S.B, r),
  ]));
}
