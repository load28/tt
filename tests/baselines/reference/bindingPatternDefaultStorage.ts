//// [bindingPatternDefaultStorage.tt] ////
// A tt value in the initializer of a declaration that destructures with a
// default is typed as TypeScript types it there: the binding pattern's
// implied type (`{ a?: 1 }` for `{ a = 1 }`) is a contextual type only,
// never a constraint, so the generated storage is not annotated with it.
// An annotated declaration still gives its type.
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant S { A(n: number), B }
function f(r: TResult<number, string>, s: S, flag: boolean): TResult<string, string> {
  const { a = 1 } = { a: try r };
  const [b = 2] = [try r];
  const { c = "x" } = { c: match (s) { A(n) => `n${n}`, B => "b" } };
  const [d = 0] = [match (s) { A(n) => n, B => 5 }];
  let { e = 1 } = { e: try r };
  const { n: { f = 1 } } = { n: { f: try r } };
  const [[g = 1]] = [[try r]];
  const { h = 1 } = { h: (flag ? try r : 3) };
  const { i = 1 } = { i: flag && try r };
  const { j = 1 }: { j?: number } = { j: try r };
  return Result.Ok(`${a},${b},${c},${d},${e},${f},${g},${h},${i},${j}`);
}
console.log(JSON.stringify(f(Result.Ok(7), S.A(3), true)), JSON.stringify(f(Result.Err("no"), S.B, false)));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [bindingPatternDefaultStorage.ts]
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
// A tt value in the initializer of a declaration that destructures with a
// default is typed as TypeScript types it there: the binding pattern's
// implied type (`{ a?: 1 }` for `{ a = 1 }`) is a contextual type only,
// never a constraint, so the generated storage is not annotated with it.
// An annotated declaration still gives its type.
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
type S =
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
function f(r: TResult<number, string>, s: S, flag: boolean): TResult<string, string> {
  let $tt_v0: number;
  const $tt_t0 = r;
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  const { a = 1 } = { a: $tt_v0 };
  let $tt_v1: number;
  const $tt_t1 = r;
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v1 = $tt_t1.value;
  const [b = 2] = [$tt_v1];
  let $tt_v2: string;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v2 = `n${n}`;
        break;
      }
      case "B": {
        $tt_v2 = "b";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const { c = "x" } = { c: $tt_v2 };
  let $tt_v3: number;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v3 = n;
        break;
      }
      case "B": {
        $tt_v3 = 5;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const [d = 0] = [$tt_v3];
  let $tt_v4: number;
  const $tt_t2 = r;
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  $tt_v4 = $tt_t2.value;
  let { e = 1 } = { e: $tt_v4 };
  let $tt_v5: number;
  const $tt_t3 = r;
  if (!("value" in $tt_t3)) {
    return $tt_t3;
  }
  $tt_v5 = $tt_t3.value;
  const { n: { f = 1 } } = { n: { f: $tt_v5 } };
  let $tt_v6: number;
  const $tt_t4 = r;
  if (!("value" in $tt_t4)) {
    return $tt_t4;
  }
  $tt_v6 = $tt_t4.value;
  const [[g = 1]] = [[$tt_v6]];
  let $tt_v9: number;
  if (flag) {
    const $tt_t5 = r;
    if (!("value" in $tt_t5)) {
      return $tt_t5;
    }
    $tt_v9 = $tt_t5.value;
  } else {
    $tt_v9 = 3;
  }
  
  const { h = 1 } = { h: ($tt_v9) };
  let $tt_v12: (number) | (false);
  let $tt_v11: boolean;
  if ($tt_v11 = flag) {
    let $tt_v10: number;
    const $tt_t6 = r;
    if (!("value" in $tt_t6)) {
      return $tt_t6;
    }
    $tt_v10 = $tt_t6.value;
    $tt_v12 = $tt_v10;
  } else {
    $tt_v12 = $tt_v11;
  }
  
  const { i = 1 } = { i: $tt_v12 };
  let $tt_v13: number | undefined;
  const $tt_t7 = r;
  if (!("value" in $tt_t7)) {
    return $tt_t7;
  }
  $tt_v13 = $tt_t7.value;
  const { j = 1 }: { j?: number } = { j: $tt_v13 };
  return Result.Ok(`${a},${b},${c},${d},${e},${f},${g},${h},${i},${j}`);
}
console.log(JSON.stringify(f(Result.Ok(7), S.A(3), true)), JSON.stringify(f(Result.Err("no"), S.B, false)));
