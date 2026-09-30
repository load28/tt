//// [forInitializerOperands.tt] ////
// Repro from TASK-601
import type { TResult } from "@tt/std";
import * as Result from "@tt/std/result";
variant O { A(n: number), B }
const log: string[] = [];
function flush(label: string, value: unknown) {
  console.log(`${label}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
function r(ok: boolean): TResult<number, string> {
  log.push("r");
  return ok ? Result.Ok(10) : Result.Err("r failed");
}
function g(n: number): number {
  log.push(`g(${n})`);
  return n * 2;
}
export function firstDeclarator(ok: boolean): TResult<number[], string> {
  const out: number[] = [];
  for (let x = try r(ok), i = 0; i < 3; i++) {
    log.push(`body ${i}`);
    out.push(x + i);
  }
  return Result.Ok(out);
}
export function operand(o: O): number[] {
  const out: number[] = [];
  for (let x = g(match (o) { A(n) => n, B => 0 }); x < 10; x += 3) {
    out.push(x);
  }
  return out;
}
export function assignment(o: O): number[] {
  const out: number[] = [];
  let x: number;
  let i: number;
  for (x = match (o) { A(n) => n, B => 5 }, i = 0; i < 2; i++) {
    out.push(x + i);
  }
  return out;
}
flush("firstDeclarator ok", firstDeclarator(true));
flush("firstDeclarator err", firstDeclarator(false));
flush("operand A(2)", operand(O.A(2)));
flush("operand B", operand(O.B));
flush("assignment A(7)", assignment(O.A(7)));
flush("assignment B", assignment(O.B));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [forInitializerOperands.ts]
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
// Repro from TASK-601
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const log: string[] = [];
function flush(label: string, value: unknown) {
  console.log(`${label}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
function r(ok: boolean): TResult<number, string> {
  log.push("r");
  return ok ? Result.Ok(10) : Result.Err("r failed");
}
function g(n: number): number {
  log.push(`g(${n})`);
  return n * 2;
}
export function firstDeclarator(ok: boolean): TResult<number[], string> {
  const out: number[] = [];
  let $tt_v0: number;
  const $tt_t0 = r(ok);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  for (let x = $tt_v0, i = 0; i < 3; i++) {
    log.push(`body ${i}`);
    out.push(x + i);
  }
  return Result.Ok(out);
}
export function operand(o: O): number[] {
  const out: number[] = [];
  let $tt_v1: number;
  const $tt_v2 = (g);
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v1 = $tt_v2(n);
        break;
      }
      case "B": {
        $tt_v1 = $tt_v2(0);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  for (let x = $tt_v1; x < 10; x += 3) {
    out.push(x);
  }
  return out;
}
export function assignment(o: O): number[] {
  const out: number[] = [];
  let x: number;
  let i: number;
  let $tt_v3: number;
  {
    const $tt_m = o;
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
  for (x = $tt_v3, i = 0; i < 2; i++) {
    out.push(x + i);
  }
  return out;
}
flush("firstDeclarator ok", firstDeclarator(true));
flush("firstDeclarator err", firstDeclarator(false));
flush("operand A(2)", operand(O.A(2)));
flush("operand B", operand(O.B));
flush("assignment A(7)", assignment(O.A(7)));
flush("assignment B", assignment(O.B));
