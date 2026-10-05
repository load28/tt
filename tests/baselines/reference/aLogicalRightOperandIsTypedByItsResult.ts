//// [aLogicalRightOperandIsTypedByItsResult.tt] ////
variant O { S(v: number), N }
declare const o: O;
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
declare function r(): R<number>;
declare function rs(): R<string>;
export function f(flag: true, no: false, n: number, cond: boolean, s: "a" | undefined): R<number> {
  const b0: true = true || try r();
  const b1: true = flag || try r();
  const b2: true = flag || match (o) { S(v) => v, N => 0 };
  const b3: false = no && try rs();
  const b4: number = n ?? try rs();
  const y: "a" | "b" | false = cond && match (o) { S => "a", N => "b" };
  const z: "a" | "b" = s ?? match (o) { S => "a", N => "b" };
  return { kind: "Ok", value: [b0, b1, b2, b3, b4, y, z].length };
}


//// [aLogicalRightOperandIsTypedByItsResult.ts]
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
type O =
  | { kind: "S"; v: number }
  | { kind: "N" };
const O = {
  S: (v: number): O => ({ kind: "S", v }),
  N: { kind: "N" } as const,
};
declare const o: O;
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
declare function r(): R<number>;
declare function rs(): R<string>;
export function f(flag: true, no: false, n: number, cond: boolean, s: "a" | undefined): R<number> {
  let $tt_v1: true;
  let $tt_v2: true;
  if ($tt_v2 = true) {
    $tt_v1 = $tt_v2;
  } else {
    let $tt_v0: number;
    const $tt_t0 = r();
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v0 = $tt_t0.value;
    $tt_v1 = $tt_v2 || $tt_v0;
  }
  
  const b0: true = $tt_v1;
  let $tt_v5: true;
  let $tt_v4: true;
  if ($tt_v4 = flag) {
    $tt_v5 = $tt_v4;
  } else {
    let $tt_v3: number;
    const $tt_t1 = r();
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v3 = $tt_t1.value;
    $tt_v5 = $tt_v4 || $tt_v3;
  }
  
  const b1: true = $tt_v5;
  let $tt_v8: true;
  let $tt_v7: true;
  if ($tt_v7 = flag) {
    $tt_v8 = $tt_v7;
  } else {
    let $tt_v6: number;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "S": {
          const { v } = $tt_m;
          $tt_v6 = v;
          break;
        }
        case "N": {
          $tt_v6 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v8 = $tt_v7 || $tt_v6;
  }
  
  const b2: true = $tt_v8;
  let $tt_v11: false;
  let $tt_v10: false;
  if ($tt_v10 = no) {
    let $tt_v9: string;
    const $tt_t2 = rs();
    if (!("value" in $tt_t2)) {
      return $tt_t2;
    }
    $tt_v9 = $tt_t2.value;
    $tt_v11 = $tt_v10 && $tt_v9;
  } else {
    $tt_v11 = $tt_v10;
  }
  
  const b3: false = $tt_v11;
  let $tt_v14: number;
  let $tt_v13: number;
  if (($tt_v13 = n) == null) {
    let $tt_v12: string;
    const $tt_t3 = rs();
    if (!("value" in $tt_t3)) {
      return $tt_t3;
    }
    $tt_v12 = $tt_t3.value;
    $tt_v14 = $tt_v13 ?? $tt_v12;
  } else {
    $tt_v14 = $tt_v13;
  }
  
  const b4: number = $tt_v14;
  let $tt_v17: "a" | "b" | false;
  let $tt_v16: boolean;
  if ($tt_v16 = cond) {
    let $tt_v15: "a" | "b" | false;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "S": {
          $tt_v15 = "a";
          break;
        }
        case "N": {
          $tt_v15 = "b";
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v17 = $tt_v16 && $tt_v15;
  } else {
    $tt_v17 = $tt_v16;
  }
  
  const y: "a" | "b" | false = $tt_v17;
  let $tt_v20: "a" | "b";
  let $tt_v19: "a" | undefined;
  if (($tt_v19 = s) == null) {
    let $tt_v18: "a" | "b";
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "S": {
          $tt_v18 = "a";
          break;
        }
        case "N": {
          $tt_v18 = "b";
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v20 = $tt_v19 ?? $tt_v18;
  } else {
    $tt_v20 = $tt_v19;
  }
  
  const z: "a" | "b" = $tt_v20;
  return { kind: "Ok", value: [b0, b1, b2, b3, b4, y, z].length };
}
