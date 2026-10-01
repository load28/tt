//// [main.tt] ////
import type { TResult } from "@tt/std";
const log: string[] = [];
function read(n: number): TResult<number, string> {
  log.push(`read ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `bad ${n}` };
}
function test(label: string, on: boolean): boolean {
  log.push(label);
  return on;
}
function sink(on: boolean): ((value: unknown) => string) | undefined {
  log.push("callee");
  return on ? (value: unknown) => `got ${JSON.stringify(value)}` : undefined;
}
function and(on: boolean, n: number) {
  return test("and", on) && result { const v = try read(n) * 2; return v; };
}
function or(on: boolean, n: number) {
  return test("or", on) || result { const v = 1 + try read(n); return v; };
}
function nullish(on: boolean, n: number) {
  return (test("nullish", on) ? null : "set") ?? result { const v = String(try read(n)); return v; };
}
function ternary(on: boolean, n: number) {
  return test("ternary", on) ? result { const v = [try read(n), try read(n + 1)]; return v; } : "no";
}
function optional(on: boolean, n: number) {
  return sink(on)?.(result { const v = 2 * try read(n); return v; });
}
function arm(on: boolean, n: number): TResult<unknown, string> {
  const v = test("arm", on) && match (n) { 0 => 0, _ => { const d = 2 * try read(n); return d; } };
  return { kind: "Ok", value: v };
}
function later(on: boolean, n: number) {
  const f = test("later", on) && match (n) { 0 => () => 0, _ => () => 2 * Number(match (n) { 1 => 3, _ => 4 }) };
  return f ? f() : f;
}
const runs: [string, (on: boolean, n: number) => unknown][] = [
  ["and", and],
  ["or", or],
  ["nullish", nullish],
  ["ternary", ternary],
  ["optional", optional],
  ["arm", arm],
  ["later", later],
];
for (const [name, run] of runs) {
  for (const [on, n] of [[true, 1], [false, 1], [true, -1], [false, -1], [true, 0]] as const) {
    log.length = 0;
    console.log(name, on, n, JSON.stringify(run(on, n)), log.join(","));
  }
}

//// [twin.ts] ////
type TResult<T, E> = { kind: "Ok"; value: T } | { kind: "Err"; error: E };
class Failure {
  constructor(readonly result: unknown) {}
}
function unwrap<T>(result: TResult<T, string>): T {
  if ("value" in result) return result.value;
  throw new Failure(result);
}
function block<T>(body: () => T): unknown {
  try {
    return { kind: "Ok", value: body() };
  } catch (error) {
    if (error instanceof Failure) return error.result;
    throw error;
  }
}
const log: string[] = [];
function read(n: number): TResult<number, string> {
  log.push(`read ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `bad ${n}` };
}
function test(label: string, on: boolean): boolean {
  log.push(label);
  return on;
}
function sink(on: boolean): ((value: unknown) => string) | undefined {
  log.push("callee");
  return on ? (value: unknown) => `got ${JSON.stringify(value)}` : undefined;
}
function and(on: boolean, n: number) {
  return test("and", on) && block(() => unwrap(read(n)) * 2);
}
function or(on: boolean, n: number) {
  return test("or", on) || block(() => 1 + unwrap(read(n)));
}
function nullish(on: boolean, n: number) {
  return (test("nullish", on) ? null : "set") ?? block(() => String(unwrap(read(n))));
}
function ternary(on: boolean, n: number) {
  return test("ternary", on) ? block(() => [unwrap(read(n)), unwrap(read(n + 1))]) : "no";
}
function optional(on: boolean, n: number) {
  return sink(on)?.(block(() => 2 * unwrap(read(n))));
}
function arm(on: boolean, n: number): unknown {
  try {
    const v = test("arm", on) && (n === 0 ? 0 : 2 * unwrap(read(n)));
    return { kind: "Ok", value: v };
  } catch (error) {
    if (error instanceof Failure) return error.result;
    throw error;
  }
}
function later(on: boolean, n: number) {
  const f = test("later", on) && (n === 0 ? () => 0 : () => 2 * Number(n === 1 ? 3 : 4));
  return f ? f() : f;
}
const runs: [string, (on: boolean, n: number) => unknown][] = [
  ["and", and],
  ["or", or],
  ["nullish", nullish],
  ["ternary", ternary],
  ["optional", optional],
  ["arm", arm],
  ["later", later],
];
for (const [name, run] of runs) {
  for (const [on, n] of [[true, 1], [false, 1], [true, -1], [false, -1], [true, 0]] as const) {
    log.length = 0;
    console.log(name, on, n, JSON.stringify(run(on, n)), log.join(","));
  }
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [main.ts]
import type { TResult } from "./tt/index.js";
const log: string[] = [];
function read(n: number): TResult<number, string> {
  log.push(`read ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `bad ${n}` };
}
function test(label: string, on: boolean): boolean {
  log.push(label);
  return on;
}
function sink(on: boolean): ((value: unknown) => string) | undefined {
  log.push("callee");
  return on ? (value: unknown) => `got ${JSON.stringify(value)}` : undefined;
}
function and(on: boolean, n: number) {
  let $tt_v2: (import("./tt/index.js").TErr<string> | {
    kind: "Ok";
    value: number;
}) | (false);
  let $tt_v1: boolean;
  if ($tt_v1 = test("and", on)) {
    let $tt_v0: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number;
});
    $tt_v0: {
      let $tt_v3: number;
      const $tt_t0 = read(n);
      if (!("value" in $tt_t0)) {
        $tt_v0 = $tt_t0;
        break $tt_v0;
      }
      $tt_v3 = $tt_t0.value;
      const v = $tt_v3 * 2; { const $tt_a0 = { value: { kind: "Ok" as const, value: v } }; $tt_v0 = $tt_a0.value; break $tt_v0; }
    }
    $tt_v2 = $tt_v1 && $tt_v0;
  } else {
    $tt_v2 = $tt_v1;
  }
  
  return $tt_v2;
}
function or(on: boolean, n: number) {
  let $tt_v6: (true) | (import("./tt/index.js").TErr<string> | {
    kind: "Ok";
    value: number;
});
  let $tt_v5: boolean;
  if ($tt_v5 = test("or", on)) {
    $tt_v6 = $tt_v5;
  } else {
    let $tt_v4: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number;
});
    $tt_v4: {
      let $tt_v7: number;
      const $tt_t1 = read(n);
      if (!("value" in $tt_t1)) {
        $tt_v4 = $tt_t1;
        break $tt_v4;
      }
      $tt_v7 = $tt_t1.value;
      const v = 1 + $tt_v7; { const $tt_a1 = { value: { kind: "Ok" as const, value: v } }; $tt_v4 = $tt_a1.value; break $tt_v4; }
    }
    $tt_v6 = $tt_v5 || $tt_v4;
  }
  
  return $tt_v6;
}
function nullish(on: boolean, n: number) {
  let $tt_v10: (import("./tt/index.js").TErr<string> | {
    kind: "Ok";
    value: string;
}) | ("set");
  let $tt_v9: "set" | null;
  if (($tt_v9 = test("nullish", on) ? null : "set") == null) {
    let $tt_v8: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: string;
});
    $tt_v8: {
      let $tt_v11: number;
      const $tt_v12 = (String);
      const $tt_t2 = read(n);
      if (!("value" in $tt_t2)) {
        $tt_v8 = $tt_t2;
        break $tt_v8;
      }
      $tt_v11 = $tt_t2.value;
      const v = $tt_v12($tt_v11); { const $tt_a2 = { value: { kind: "Ok" as const, value: v } }; $tt_v8 = $tt_a2.value; break $tt_v8; }
    }
    $tt_v10 = $tt_v9 ?? $tt_v8;
  } else {
    $tt_v10 = $tt_v9;
  }
  
  return $tt_v10;
}
function ternary(on: boolean, n: number) {
  let $tt_v15: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number[];
}) | (string);
  if (test("ternary", on)) {
    $tt_y_v13: {
      let $tt_v16: number;
      let $tt_v17: number;
      const $tt_t3 = read(n);
      if (!("value" in $tt_t3)) {
        $tt_v15 = $tt_t3;
        break $tt_y_v13;
      }
      $tt_v16 = $tt_t3.value;
      const $tt_t4 = read(n + 1);
      if (!("value" in $tt_t4)) {
        $tt_v15 = $tt_t4;
        break $tt_y_v13;
      }
      $tt_v17 = $tt_t4.value;
      const v = [$tt_v16, $tt_v17]; { const $tt_a3 = { value: { kind: "Ok" as const, value: v } }; $tt_v15 = $tt_a3.value; break $tt_y_v13; }
    }
  } else {
    $tt_v15 = "no";
  }
  
  return $tt_v15;
}
function optional(on: boolean, n: number) {
  let $tt_v20: (string) | (undefined);
  const $tt_v19 = (sink(on));
  if ($tt_v19 != null) {
    let $tt_v18: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number;
});
    $tt_v18: {
      let $tt_v21: number;
      const $tt_t5 = read(n);
      if (!("value" in $tt_t5)) {
        $tt_v18 = $tt_t5;
        break $tt_v18;
      }
      $tt_v21 = $tt_t5.value;
      const v = 2 * $tt_v21; { const $tt_a4 = { value: { kind: "Ok" as const, value: v } }; $tt_v18 = $tt_a4.value; break $tt_v18; }
    }
    $tt_v20 = $tt_v19($tt_v18);
  } else {
    $tt_v20 = undefined;
  }
  
  return $tt_v20;
}
function arm(on: boolean, n: number): TResult<unknown, string> {
  let $tt_v24: (number) | (false);
  let $tt_v23: boolean;
  if ($tt_v23 = test("arm", on)) {
    let $tt_v22: number;
    {
      const $tt_m = n;
      switch ($tt_m) {
        case 0: {
          $tt_v22 = 0;
          break;
        }
        default: {
          let $tt_v25: number;
          const $tt_t6 = read(n);
          if (!("value" in $tt_t6)) {
            return $tt_t6;
          }
          $tt_v25 = $tt_t6.value;
          const d = 2 * $tt_v25; $tt_v22 = d; break;
        }
      }
    }
    $tt_v24 = $tt_v23 && $tt_v22;
  } else {
    $tt_v24 = $tt_v23;
  }
  
  const v = $tt_v24;
  return { kind: "Ok", value: v };
}
function later(on: boolean, n: number) {
  let $tt_v28: (() => number) | (false);
  let $tt_v27: boolean;
  if ($tt_v27 = test("later", on)) {
    let $tt_v26: () => number;
    {
      const $tt_m = n;
      switch ($tt_m) {
        case 0: {
          const $tt_a5 = { value: () => 0 };
          $tt_v26 = $tt_a5.value;
          break;
        }
        default: {
          const $tt_a6 = { value: (() => {
            let $tt_v29: number;
            const $tt_v30 = (Number);
            {
              const $tt_m = n;
              switch ($tt_m) {
                case 1: $tt_v29 = 0; break;
                default: $tt_v29 = 1; break;
              }
            }
            return 2 * $tt_v30(($tt_v29 === 0 ? 3 : 4));
          }) };
          $tt_v26 = $tt_a6.value;
          break;
        }
      }
    }
    $tt_v28 = $tt_v27 && $tt_v26;
  } else {
    $tt_v28 = $tt_v27;
  }
  
  const f = $tt_v28;
  return f ? f() : f;
}
const runs: [string, (on: boolean, n: number) => unknown][] = [
  ["and", and],
  ["or", or],
  ["nullish", nullish],
  ["ternary", ternary],
  ["optional", optional],
  ["arm", arm],
  ["later", later],
];
for (const [name, run] of runs) {
  for (const [on, n] of [[true, 1], [false, 1], [true, -1], [false, -1], [true, 0]] as const) {
    log.length = 0;
    console.log(name, on, n, JSON.stringify(run(on, n)), log.join(","));
  }
}
\ No newline at end of file

//// [twin.ts]
type TResult<T, E> = { kind: "Ok"; value: T } | { kind: "Err"; error: E };
class Failure {
  constructor(readonly result: unknown) {}
}
function unwrap<T>(result: TResult<T, string>): T {
  if ("value" in result) return result.value;
  throw new Failure(result);
}
function block<T>(body: () => T): unknown {
  try {
    return { kind: "Ok", value: body() };
  } catch (error) {
    if (error instanceof Failure) return error.result;
    throw error;
  }
}
const log: string[] = [];
function read(n: number): TResult<number, string> {
  log.push(`read ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `bad ${n}` };
}
function test(label: string, on: boolean): boolean {
  log.push(label);
  return on;
}
function sink(on: boolean): ((value: unknown) => string) | undefined {
  log.push("callee");
  return on ? (value: unknown) => `got ${JSON.stringify(value)}` : undefined;
}
function and(on: boolean, n: number) {
  return test("and", on) && block(() => unwrap(read(n)) * 2);
}
function or(on: boolean, n: number) {
  return test("or", on) || block(() => 1 + unwrap(read(n)));
}
function nullish(on: boolean, n: number) {
  return (test("nullish", on) ? null : "set") ?? block(() => String(unwrap(read(n))));
}
function ternary(on: boolean, n: number) {
  return test("ternary", on) ? block(() => [unwrap(read(n)), unwrap(read(n + 1))]) : "no";
}
function optional(on: boolean, n: number) {
  return sink(on)?.(block(() => 2 * unwrap(read(n))));
}
function arm(on: boolean, n: number): unknown {
  try {
    const v = test("arm", on) && (n === 0 ? 0 : 2 * unwrap(read(n)));
    return { kind: "Ok", value: v };
  } catch (error) {
    if (error instanceof Failure) return error.result;
    throw error;
  }
}
function later(on: boolean, n: number) {
  const f = test("later", on) && (n === 0 ? () => 0 : () => 2 * Number(n === 1 ? 3 : 4));
  return f ? f() : f;
}
const runs: [string, (on: boolean, n: number) => unknown][] = [
  ["and", and],
  ["or", or],
  ["nullish", nullish],
  ["ternary", ternary],
  ["optional", optional],
  ["arm", arm],
  ["later", later],
];
for (const [name, run] of runs) {
  for (const [on, n] of [[true, 1], [false, 1], [true, -1], [false, -1], [true, 0]] as const) {
    log.length = 0;
    console.log(name, on, n, JSON.stringify(run(on, n)), log.join(","));
  }
}
