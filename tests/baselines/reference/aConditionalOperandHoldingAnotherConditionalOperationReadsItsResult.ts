//// [aConditionalOperandHoldingAnotherConditionalOperationReadsItsResult.tt] ////
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
variant G { E, F }
const ok = (value: number): R<number> => { console.log("ok", value); return { kind: "Ok", value }; };
const err = (error: string): R<number> => ({ kind: "Err", error });

function or(c: boolean, r: R<number>, g: G): R<number> {
    const v = (c && try r) || match (g) { E => 10, F => 20 };
    return { kind: "Ok", value: Number(v) };
}

function ternary(c: boolean, d: R<number>, e: R<number>): R<number> {
    const v = (c ? try d : 0) ? 2 : try e;
    return { kind: "Ok", value: v };
}

function nullish(c: boolean, r: R<number>, s: R<number>): R<number> {
    const v = (c && try r) ?? try s;
    return { kind: "Ok", value: Number(v) };
}

const show = (r: R<number>) => JSON.stringify(r);
console.log(show(or(true, ok(0), G.E)), show(or(true, ok(3), G.F)), show(or(false, ok(4), G.F)), show(or(true, err("x"), G.E)));
console.log(show(ternary(true, ok(1), ok(5))), show(ternary(true, ok(0), ok(6))), show(ternary(false, ok(1), err("y"))));
console.log(show(nullish(true, ok(7), ok(8))), show(nullish(false, ok(7), err("z"))));
export {};


//// [aConditionalOperandHoldingAnotherConditionalOperationReadsItsResult.ts]
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
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
type G =
  | { kind: "E" }
  | { kind: "F" };
const G = {
  E: { kind: "E" } as const,
  F: { kind: "F" } as const,
};
const ok = (value: number): R<number> => { console.log("ok", value); return { kind: "Ok", value }; };
const err = (error: string): R<number> => ({ kind: "Err", error });

function or(c: boolean, r: R<number>, g: G): R<number> {
    let $tt_v4: number | false;
    let $tt_v5: number;
    let $tt_v2: boolean;
    if ($tt_v2 = c) {
      let $tt_v0: number | false;
      const $tt_t0 = r;
      if (!("value" in $tt_t0)) {
        return $tt_t0;
      }
      $tt_v0 = $tt_t0.value;
      $tt_v4 = $tt_v2 && $tt_v0;
    } else {
      $tt_v4 = $tt_v2;
    }
    
    let $tt_v3: number | false;
    if ($tt_v3 = $tt_v4) {
      $tt_v5 = $tt_v3;
    } else {
      let $tt_v1: number;
      {
        const $tt_m = g;
        switch ($tt_m.kind) {
          case "E": {
            $tt_v1 = 10;
            break;
          }
          case "F": {
            $tt_v1 = 20;
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      $tt_v5 = $tt_v3 || $tt_v1;
    }
    
    const v = $tt_v5;
    return { kind: "Ok", value: Number(v) };
}

function ternary(c: boolean, d: R<number>, e: R<number>): R<number> {
    let $tt_v10: number;
    let $tt_v11: number;
    if (c) {
      const $tt_t1 = d;
      if (!("value" in $tt_t1)) {
        return $tt_t1;
      }
      $tt_v10 = $tt_t1.value;
    } else {
      $tt_v10 = 0;
    }
    
    if ($tt_v10) {
      $tt_v11 = 2;
    } else {
      const $tt_t2 = e;
      if (!("value" in $tt_t2)) {
        return $tt_t2;
      }
      $tt_v11 = $tt_t2.value;
    }
    
    const v = $tt_v11;
    return { kind: "Ok", value: v };
}

function nullish(c: boolean, r: R<number>, s: R<number>): R<number> {
    let $tt_v16: number | false;
    let $tt_v17: number | false;
    let $tt_v14: boolean;
    if ($tt_v14 = c) {
      let $tt_v12: number | false;
      const $tt_t3 = r;
      if (!("value" in $tt_t3)) {
        return $tt_t3;
      }
      $tt_v12 = $tt_t3.value;
      $tt_v16 = $tt_v14 && $tt_v12;
    } else {
      $tt_v16 = $tt_v14;
    }
    
    let $tt_v15: number | false;
    if (($tt_v15 = $tt_v16) == null) {
      let $tt_v13: number;
      const $tt_t4 = s;
      if (!("value" in $tt_t4)) {
        return $tt_t4;
      }
      $tt_v13 = $tt_t4.value;
      $tt_v17 = $tt_v15 ?? $tt_v13;
    } else {
      $tt_v17 = $tt_v15;
    }
    
    const v = $tt_v17;
    return { kind: "Ok", value: Number(v) };
}

const show = (r: R<number>) => JSON.stringify(r);
console.log(show(or(true, ok(0), G.E)), show(or(true, ok(3), G.F)), show(or(false, ok(4), G.F)), show(or(true, err("x"), G.E)));
console.log(show(ternary(true, ok(1), ok(5))), show(ternary(true, ok(0), ok(6))), show(ternary(false, ok(1), err("y"))));
console.log(show(nullish(true, ok(7), ok(8))), show(nullish(false, ok(7), err("z"))));
export {};
