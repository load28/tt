//// [shortCircuitTtOperands.tt] ////
// A tt value on the right of `&&`, `||`, `??`, `? :`, or an optional call is
// evaluated only when JavaScript reaches it; the left operand is evaluated once.
import type { TResult } from "@tt/std";
import * as Result from "@tt/std/result";
variant Flag { On, Off }
function flag(on: boolean): Flag {
  return on ? Flag.On : Flag.Off;
}
const log: string[] = [];
function note<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
function read(label: string): TResult<number, string> {
  log.push(label);
  return Result.Ok(7);
}
for (const left of [0, 1]) {
  flush(`${left} && match`, note("left", left) && match (note("scrutinee", flag(true))) { On => note("on", 2), Off => 3 });
  flush(`${left} || match`, note("left", left) || match (note("scrutinee", flag(false))) { On => 2, Off => note("off", 3) });
}
for (const left of [null, 0]) {
  flush(`${left} ?? match`, note("left", left) ?? match (note("scrutinee", flag(true))) { On => note("on", 4), Off => 5 });
}
for (const test of [true, false]) {
  flush(
    `${test} ? match : other`,
    note("test", test) ? match (note("scrutinee", flag(false))) { On => 1, Off => note("off", 2) } : note("otherwise", 0),
  );
}
class Runner {
  run(n: number) {
    log.push(this === runner ? "this is the runner" : "this lost");
    return n;
  }
}
const runner = new Runner();
const absent: { run?: (n: number) => number } = {};
const present: { run?: (n: number) => number } = runner;
flush("absent?.()", absent.run?.(match (note("argument scrutinee", flag(true))) { On => 1, Off => 2 }));
flush("present?.()", present.run?.(match (note("argument scrutinee", flag(true))) { On => 1, Off => 2 }));
const holder: { runner?: Runner } = {};
flush("missing receiver?.m()", holder.runner?.run(match (note("argument scrutinee", flag(false))) { On => 1, Off => 2 }));
function tries(left: number | null): TResult<number, string> {
  const value = note("left", left) ?? try read("read");
  return Result.Ok(value);
}
flush("null ?? try", tries(null));
flush("1 ?? try", tries(1));
function chain(a: boolean, b: boolean) {
  return note("a", a) && note("b", b) && match (note("scrutinee", flag(true))) { On => note("on", "reached"), Off => "off" };
}
flush("true && true && match", chain(true, true));
flush("true && false && match", chain(true, false));
flush("false && ... && match", chain(false, true));
export {};

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [shortCircuitTtOperands.ts]
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
// A tt value on the right of `&&`, `||`, `??`, `? :`, or an optional call is
// evaluated only when JavaScript reaches it; the left operand is evaluated once.
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
type Flag =
  | { kind: "On" }
  | { kind: "Off" };
const Flag = {
  On: { kind: "On" } as const,
  Off: { kind: "Off" } as const,
};
function flag(on: boolean): Flag {
  return on ? Flag.On : Flag.Off;
}
const log: string[] = [];
function note<T>(label: string, value: T): T {
  log.push(label);
  return value;
}
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
function read(label: string): TResult<number, string> {
  log.push(label);
  return Result.Ok(7);
}
for (const left of [0, 1]) {
  let $tt_v4: number;
  const $tt_v2 = (flush);
  const $tt_v3: string = (`${left} && match`);
  let $tt_v1: number;
  if ($tt_v1 = note("left", left)) {
    let $tt_v0: number;
    {
      const $tt_m = note("scrutinee", flag(true));
      switch ($tt_m.kind) {
        case "On": {
          $tt_v0 = note("on", 2);
          break;
        }
        case "Off": {
          $tt_v0 = 3;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v4 = $tt_v1 && $tt_v0;
  } else {
    $tt_v4 = $tt_v1;
  }
  
  $tt_v2($tt_v3, $tt_v4);
  let $tt_v9: number;
  const $tt_v7 = (flush);
  const $tt_v8: string = (`${left} || match`);
  let $tt_v6: number;
  if ($tt_v6 = note("left", left)) {
    $tt_v9 = $tt_v6;
  } else {
    let $tt_v5: number;
    {
      const $tt_m = note("scrutinee", flag(false));
      switch ($tt_m.kind) {
        case "On": {
          $tt_v5 = 2;
          break;
        }
        case "Off": {
          $tt_v5 = note("off", 3);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v9 = $tt_v6 || $tt_v5;
  }
  
  $tt_v7($tt_v8, $tt_v9);
}
for (const left of [null, 0]) {
  let $tt_v14: number;
  const $tt_v12 = (flush);
  const $tt_v13: string = (`${left} ?? match`);
  let $tt_v11: number | null;
  if (($tt_v11 = note("left", left)) == null) {
    let $tt_v10: number;
    {
      const $tt_m = note("scrutinee", flag(true));
      switch ($tt_m.kind) {
        case "On": {
          $tt_v10 = note("on", 4);
          break;
        }
        case "Off": {
          $tt_v10 = 5;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v14 = $tt_v11 ?? $tt_v10;
  } else {
    $tt_v14 = $tt_v11;
  }
  
  $tt_v12($tt_v13, $tt_v14);
}
for (const test of [true, false]) {
  let $tt_v19: number;
  const $tt_v17 = (flush);
  const $tt_v18: string = (`${test} ? match : other`);
  if (note("test", test)) {
    {
      const $tt_m = note("scrutinee", flag(false));
      switch ($tt_m.kind) {
        case "On": {
          $tt_v19 = 1;
          break;
        }
        case "Off": {
          $tt_v19 = note("off", 2);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v19 = note("otherwise", 0);
  }
  
  $tt_v17(
    $tt_v18,
    $tt_v19,
  );
}
class Runner {
  run(n: number) {
    log.push(this === runner ? "this is the runner" : "this lost");
    return n;
  }
}
const runner = new Runner();
const absent: { run?: (n: number) => number } = {};
const present: { run?: (n: number) => number } = runner;
let $tt_v24: (number) | (undefined);
const $tt_v22 = (flush);
const $tt_v21 = (absent.run);
if ($tt_v21 != null) {
  {
    const $tt_m = note("argument scrutinee", flag(true));
    switch ($tt_m.kind) {
      case "On": {
        $tt_v24 = $tt_v21.call(absent, 1);
        break;
      }
      case "Off": {
        $tt_v24 = $tt_v21.call(absent, 2);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
} else {
  $tt_v24 = undefined;
}

$tt_v22("absent?.()", $tt_v24);
let $tt_v29: (number) | (undefined);
const $tt_v27 = (flush);
const $tt_v26 = (present.run);
if ($tt_v26 != null) {
  {
    const $tt_m = note("argument scrutinee", flag(true));
    switch ($tt_m.kind) {
      case "On": {
        $tt_v29 = $tt_v26.call(present, 1);
        break;
      }
      case "Off": {
        $tt_v29 = $tt_v26.call(present, 2);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
} else {
  $tt_v29 = undefined;
}

$tt_v27("present?.()", $tt_v29);
const holder: { runner?: Runner } = {};
let $tt_v35: (number) | (undefined);
const $tt_v33 = (flush);
const $tt_v32 = (holder.runner);
if ($tt_v32 != null) {
  {
    const $tt_m = note("argument scrutinee", flag(false));
    switch ($tt_m.kind) {
      case "On": {
        $tt_v35 = $tt_v32?.run(1);
        break;
      }
      case "Off": {
        $tt_v35 = $tt_v32?.run(2);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
} else {
  $tt_v35 = undefined;
}

$tt_v33("missing receiver?.m()", $tt_v35);
function tries(left: number | null): TResult<number, string> {
  let $tt_v38: number;
  let $tt_v37: number | null;
  if (($tt_v37 = note("left", left)) == null) {
    let $tt_v36: number;
    const $tt_t0 = read("read");
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v36 = $tt_t0.value;
    $tt_v38 = $tt_v37 ?? $tt_v36;
  } else {
    $tt_v38 = $tt_v37;
  }
  
  const value = $tt_v38;
  return Result.Ok(value);
}
flush("null ?? try", tries(null));
flush("1 ?? try", tries(1));
function chain(a: boolean, b: boolean) {
  let $tt_v41: (string) | (false);
  let $tt_v40: boolean;
  if ($tt_v40 = note("a", a) && note("b", b)) {
    let $tt_v39: string;
    {
      const $tt_m = note("scrutinee", flag(true));
      switch ($tt_m.kind) {
        case "On": {
          $tt_v39 = note("on", "reached");
          break;
        }
        case "Off": {
          $tt_v39 = "off";
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v41 = $tt_v40 && $tt_v39;
  } else {
    $tt_v41 = $tt_v40;
  }
  
  return $tt_v41;
}
flush("true && true && match", chain(true, true));
flush("true && false && match", chain(true, false));
flush("false && ... && match", chain(false, true));
export {};
