//// [conditionalOperationNarrowing.tt] ////
// Repro from TASK-595
import type { TResult } from "@tt/std";
import * as Result from "@tt/std/result";
variant O { A(n: number), B }
function g(n: number): TResult<number, string> {
  return n > 2 ? Result.Ok(n) : Result.Err(`short ${n}`);
}
export function h(o: O, cfg: { name?: string }) {
  return cfg.name ? match (o) { A(n) => cfg.name.slice(n), B => cfg.name } : "anon";
}
export function k(o: O, s: string | null) {
  return s && match (o) { A(n) => s.charAt(n), B => s };
}
export function l(o: O, init: { v: number } | undefined) {
  let u = init;
  u = init;
  const a = u ? match (o) { A(n) => u.v + n, B => u.v } : 0;
  const b = !u || match (o) { A(n) => u.v > n, B => u.v > 0 };
  const c = u !== undefined && u.v > 0 && match (o) { A(n) => u.v + n, B => 1 };
  return [a, b, c];
}
export function m(s: string | null) {
  return result { const x = s && try g(s.length); return x; };
}
let reads = 0;
const counted = {
  get name() {
    reads += 1;
    return "config";
  },
};
const log = (label: string, value: unknown) => {
  console.log(`${label}: ${JSON.stringify(value)} (name read ${reads} time(s))`);
  reads = 0;
};
log("h A(2) counted", h(O.A(2), counted));
log("h B counted", h(O.B, counted));
log("h A(1) unnamed", h(O.A(1), {}));
log("k A(1) text", k(O.A(1), "text"));
log("k B null", k(O.B, null));
log("l A(1) v=2", l(O.A(1), { v: 2 }));
log("l B undefined", l(O.B, undefined));
log("m long", m("long"));
log("m short", m("ab"));
log("m null", m(null));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [conditionalOperationNarrowing.ts]
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
// Repro from TASK-595
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
function g(n: number): TResult<number, string> {
  return n > 2 ? Result.Ok(n) : Result.Err(`short ${n}`);
}
export function h(o: O, cfg: { name?: string }) {
  let $tt_v2: string;
  if (cfg.name) {
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v2 = cfg.name.slice(n);
          break;
        }
        case "B": {
          $tt_v2 = cfg.name;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v2 = "anon";
  }
  
  return $tt_v2;
}
export function k(o: O, s: string | null) {
  let $tt_v5: string | null;
  let $tt_v4: string | null;
  if ($tt_v4 = s) {
    let $tt_v3: string;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v3 = s.charAt(n);
          break;
        }
        case "B": {
          $tt_v3 = s;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v5 = $tt_v3;
  } else {
    $tt_v5 = $tt_v4;
  }
  
  return $tt_v5;
}
export function l(o: O, init: { v: number } | undefined) {
  let u = init;
  u = init;
  let $tt_v8: number;
  if (u) {
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v8 = u.v + n;
          break;
        }
        case "B": {
          $tt_v8 = u.v;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v8 = 0;
  }
  
  const a = $tt_v8;
  let $tt_v11: boolean;
  let $tt_v10: boolean;
  if ($tt_v10 = !u) {
    $tt_v11 = $tt_v10;
  } else {
    let $tt_v9: boolean;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v9 = u.v > n;
          break;
        }
        case "B": {
          $tt_v9 = u.v > 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v11 = $tt_v9;
  }
  
  const b = $tt_v11;
  let $tt_v14: (number) | (false);
  let $tt_v13: boolean;
  if ($tt_v13 = u !== undefined && u.v > 0) {
    let $tt_v12: number;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v12 = u.v + n;
          break;
        }
        case "B": {
          $tt_v12 = 1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v14 = $tt_v12;
  } else {
    $tt_v14 = $tt_v13;
  }
  
  const c = $tt_v14;
  return [a, b, c];
}
export function m(s: string | null) {
  let $tt_v15: (Result.TErr<string>) | ({
    kind: "Ok";
    value: string | number | null;
});
  $tt_v15: {
    let $tt_v18: (number) | (string | null);
    let $tt_v17: string | null;
    if ($tt_v17 = s) {
      let $tt_v16: number;
      const $tt_t0 = g(s.length);
      if (!("value" in $tt_t0)) {
        $tt_v15 = $tt_t0;
        break $tt_v15;
      }
      $tt_v16 = $tt_t0.value;
      $tt_v18 = $tt_v16;
    } else {
      $tt_v18 = $tt_v17;
    }
    
    const x = $tt_v18; { const $tt_a0 = { value: { kind: "Ok" as const, value: x } }; $tt_v15 = $tt_a0.value; break $tt_v15; }
  }
  return $tt_v15;
}
let reads = 0;
const counted = {
  get name() {
    reads += 1;
    return "config";
  },
};
const log = (label: string, value: unknown) => {
  console.log(`${label}: ${JSON.stringify(value)} (name read ${reads} time(s))`);
  reads = 0;
};
log("h A(2) counted", h(O.A(2), counted));
log("h B counted", h(O.B, counted));
log("h A(1) unnamed", h(O.A(1), {}));
log("k A(1) text", k(O.A(1), "text"));
log("k B null", k(O.B, null));
log("l A(1) v=2", l(O.A(1), { v: 2 }));
log("l B undefined", l(O.B, undefined));
log("m long", m("long"));
log("m short", m("ab"));
log("m null", m(null));
