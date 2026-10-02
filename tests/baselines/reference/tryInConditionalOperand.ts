//// [tryInConditionalOperand.tt] ////
// A `try` nested in an operand of a conditional operation keeps the
// operation's evaluation: the operand runs only when the operation reaches
// it, its values run left to right with the source between them, and an
// `Err` leaves the function before anything after it. This holds for several
// values in one branch of `&&`, `||`, `??`, and `? :`, and for an optional
// call's argument that holds a value inside a larger expression.
import type { TResult } from "@tt/std";
const log: string[] = [];
function read(n: number): TResult<number, string> {
  log.push(`read ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `bad ${n}` };
}
function note<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function sink(label: string): ((...values: unknown[]) => string) | undefined {
  log.push(`callee ${label}`);
  return label === "none" ? undefined : (...values: unknown[]) => values.join("|");
}
function logical(flag: boolean, n: number): TResult<unknown, string> {
  const and = flag && (try read(n)) + note("middle", 10) + (try read(n + 1));
  const or = !flag || (try read(n)) * (try read(n + 2));
  const nullish = (flag ? null : "set") ?? (try read(n)) - (try read(n + 3));
  return { kind: "Ok", value: [and, or, nullish] };
}
function ternary(flag: boolean, n: number): TResult<unknown, string> {
  const both = flag ? (try read(n)) + (try read(n + 1)) : (try read(n + 2)) * 2;
  const one = flag ? note("plain", 0) : [try read(n), note("after", 1), try read(n + 1)];
  return { kind: "Ok", value: [both, one] };
}
function optional(label: string, n: number): TResult<unknown, string> {
  const scaled = sink(label)?.(note("first", 1), try read(n) * 2, note("last", 3));
  const member = sink(label)?.((try read(n)).toFixed(1), (try read(n)) + (try read(n + 1)));
  return { kind: "Ok", value: [scaled, member] };
}
for (const [flag, n] of [[true, 1], [false, 1], [true, -2], [false, -2]] as const) {
  log.length = 0;
  console.log(JSON.stringify(logical(flag, n)), log.join(","));
  log.length = 0;
  console.log(JSON.stringify(ternary(flag, n)), log.join(","));
}
for (const [label, n] of [["some", 2], ["none", 2], ["some", -1], ["none", -1]] as const) {
  log.length = 0;
  console.log(JSON.stringify(optional(label, n)), log.join(","));
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [tryInConditionalOperand.ts]
// A `try` nested in an operand of a conditional operation keeps the
// operation's evaluation: the operand runs only when the operation reaches
// it, its values run left to right with the source between them, and an
// `Err` leaves the function before anything after it. This holds for several
// values in one branch of `&&`, `||`, `??`, and `? :`, and for an optional
// call's argument that holds a value inside a larger expression.
import type { TResult } from "./tt/index.js";
const log: string[] = [];
function read(n: number): TResult<number, string> {
  log.push(`read ${n}`);
  return n >= 0 ? { kind: "Ok", value: n } : { kind: "Err", error: `bad ${n}` };
}
function note<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function sink(label: string): ((...values: unknown[]) => string) | undefined {
  log.push(`callee ${label}`);
  return label === "none" ? undefined : (...values: unknown[]) => values.join("|");
}
function logical(flag: boolean, n: number): TResult<unknown, string> {
  let $tt_v4: (number) | (false);
  let $tt_v2: boolean;
  if ($tt_v2 = flag) {
    let $tt_v0: number;
    const $tt_t0 = read(n);
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v0 = $tt_t0.value;
    let $tt_v1: number;
    const $tt_v3 = (($tt_v0) + note("middle", 10));
    const $tt_t1 = read(n + 1);
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v1 = $tt_t1.value;
    $tt_v4 = $tt_v2 && $tt_v3 + ($tt_v1);
  } else {
    $tt_v4 = $tt_v2;
  }
  
  const and = $tt_v4;
  let $tt_v8: (true) | (number);
  let $tt_v7: boolean;
  if ($tt_v7 = !flag) {
    $tt_v8 = $tt_v7;
  } else {
    let $tt_v5: number;
    const $tt_t2 = read(n);
    if (!("value" in $tt_t2)) {
      return $tt_t2;
    }
    $tt_v5 = $tt_t2.value;
    let $tt_v6: number;
    const $tt_t3 = read(n + 2);
    if (!("value" in $tt_t3)) {
      return $tt_t3;
    }
    $tt_v6 = $tt_t3.value;
    $tt_v8 = $tt_v7 || ($tt_v5) * ($tt_v6);
  }
  
  const or = $tt_v8;
  let $tt_v12: (number) | ("set");
  let $tt_v11: "set" | null;
  if (($tt_v11 = flag ? null : "set") == null) {
    let $tt_v9: number;
    const $tt_t4 = read(n);
    if (!("value" in $tt_t4)) {
      return $tt_t4;
    }
    $tt_v9 = $tt_t4.value;
    let $tt_v10: number;
    const $tt_t5 = read(n + 3);
    if (!("value" in $tt_t5)) {
      return $tt_t5;
    }
    $tt_v10 = $tt_t5.value;
    $tt_v12 = $tt_v11 ?? ($tt_v9) - ($tt_v10);
  } else {
    $tt_v12 = $tt_v11;
  }
  
  const nullish = $tt_v12;
  return { kind: "Ok", value: [and, or, nullish] };
}
function ternary(flag: boolean, n: number): TResult<unknown, string> {
  let $tt_v17: number;
  if (flag) {
    let $tt_v13: number;
    const $tt_t6 = read(n);
    if (!("value" in $tt_t6)) {
      return $tt_t6;
    }
    $tt_v13 = $tt_t6.value;
    let $tt_v14: number;
    const $tt_t7 = read(n + 1);
    if (!("value" in $tt_t7)) {
      return $tt_t7;
    }
    $tt_v14 = $tt_t7.value;
    $tt_v17 = ($tt_v13) + ($tt_v14);
  } else {
    let $tt_v15: number;
    const $tt_t8 = read(n + 2);
    if (!("value" in $tt_t8)) {
      return $tt_t8;
    }
    $tt_v15 = $tt_t8.value;
    $tt_v17 = ($tt_v15) * 2;
  }
  
  const both = $tt_v17;
  let $tt_v22: (number) | (number[]);
  if (flag) {
    $tt_v22 = note("plain", 0);
  } else {
    let $tt_v18: number;
    const $tt_t9 = read(n);
    if (!("value" in $tt_t9)) {
      return $tt_t9;
    }
    $tt_v18 = $tt_t9.value;
    let $tt_v19: number;
    const $tt_v21 = (note("after", 1));
    const $tt_t10 = read(n + 1);
    if (!("value" in $tt_t10)) {
      return $tt_t10;
    }
    $tt_v19 = $tt_t10.value;
    const $tt_a0 = { value: [$tt_v18, $tt_v21, $tt_v19] };
    $tt_v22 = $tt_a0.value;
  }
  
  const one = $tt_v22;
  return { kind: "Ok", value: [both, one] };
}
function optional(label: string, n: number): TResult<unknown, string> {
  let $tt_v26: (string) | (undefined);
  const $tt_v24 = (sink(label));
  if ($tt_v24 != null) {
    const $tt_v25 = (note("first", 1));
    let $tt_v23: number;
    const $tt_t11 = read(n);
    if (!("value" in $tt_t11)) {
      return $tt_t11;
    }
    $tt_v23 = $tt_t11.value;
    $tt_v26 = $tt_v24($tt_v25, $tt_v23 * 2, note("last", 3));
  } else {
    $tt_v26 = undefined;
  }
  
  const scaled = $tt_v26;
  let $tt_v32: (string) | (undefined);
  const $tt_v30 = (sink(label));
  if ($tt_v30 != null) {
    let $tt_v27: number;
    const $tt_t12 = read(n);
    if (!("value" in $tt_t12)) {
      return $tt_t12;
    }
    $tt_v27 = $tt_t12.value;
    let $tt_v28: number;
    const $tt_t13 = read(n);
    if (!("value" in $tt_t13)) {
      return $tt_t13;
    }
    $tt_v28 = $tt_t13.value;
    let $tt_v29: number;
    const $tt_t14 = read(n + 1);
    if (!("value" in $tt_t14)) {
      return $tt_t14;
    }
    $tt_v29 = $tt_t14.value;
    $tt_v32 = $tt_v30(($tt_v27).toFixed(1), ($tt_v28) + ($tt_v29));
  } else {
    $tt_v32 = undefined;
  }
  
  const member = $tt_v32;
  return { kind: "Ok", value: [scaled, member] };
}
for (const [flag, n] of [[true, 1], [false, 1], [true, -2], [false, -2]] as const) {
  log.length = 0;
  console.log(JSON.stringify(logical(flag, n)), log.join(","));
  log.length = 0;
  console.log(JSON.stringify(ternary(flag, n)), log.join(","));
}
for (const [label, n] of [["some", 2], ["none", 2], ["some", -1], ["none", -1]] as const) {
  log.length = 0;
  console.log(JSON.stringify(optional(label, n)), log.join(","));
}
