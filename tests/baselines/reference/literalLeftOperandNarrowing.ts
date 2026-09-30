//// [literalLeftOperandNarrowing.tt] ////
// Repro from TASK-626
import type { TResult } from "@tt/std";
variant M { A(n: number), B }
declare function read(): TResult<number, string>;
export function orFalse() {
  return result { const a = false || try read(); return a; };
}
export function andTrue(m: M) {
  const b = true && match (m) { A(n) => "a", B => "b" };
  return b;
}
export function nullishNull() {
  return result { const c = null ?? try read(); return c; };
}
export function nullishUndefined() {
  return result { const d = undefined ?? try read(); return d; };
}
export function numericLiterals(m: M) {
  return result {
    let e = 0 || try read();
    const f = 1 && match (m) { A(n) => `${n}`, B => "b" };
    return [e, f] as const;
  };
}
export function parenthesized(m: M) {
  let h = (false) || match (m) { A(n) => n, B => 0 };
  return h;
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [literalLeftOperandNarrowing.ts]
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
// Repro from TASK-626
import type { TResult } from "./tt/index.js";
type M =
  | { kind: "A"; n: number }
  | { kind: "B" };
const M = {
  A: (n: number): M => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare function read(): TResult<number, string>;
export function orFalse() {
  let $tt_v0: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v0: {
    let $tt_v2: number;
    let $tt_v3: false;
    if ($tt_v3 = false) {
      $tt_v2 = $tt_v3;
    } else {
      let $tt_v1: number;
      const $tt_t0 = read();
      if (!("value" in $tt_t0)) {
        $tt_v0 = $tt_t0;
        break $tt_v0;
      }
      $tt_v1 = $tt_t0.value;
      $tt_v2 = $tt_v1;
    }
    
    const a = $tt_v2; { const $tt_a0 = { value: { kind: "Ok" as const, value: a } }; $tt_v0 = $tt_a0.value; break $tt_v0; }
  }
  return $tt_v0;
}
export function andTrue(m: M) {
  let $tt_v5: string;
  let $tt_v6: true;
  if ($tt_v6 = true) {
    let $tt_v4: string;
    {
      const $tt_m = m;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v4 = "a";
          break;
        }
        case "B": {
          $tt_v4 = "b";
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v5 = $tt_v4;
  } else {
    $tt_v5 = $tt_v6;
  }
  
  const b = $tt_v5;
  return b;
}
export function nullishNull() {
  let $tt_v7: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v7: {
    let $tt_v9: number;
    let $tt_v10: null;
    if (($tt_v10 = null) == null) {
      let $tt_v8: number;
      const $tt_t1 = read();
      if (!("value" in $tt_t1)) {
        $tt_v7 = $tt_t1;
        break $tt_v7;
      }
      $tt_v8 = $tt_t1.value;
      $tt_v9 = $tt_v8;
    } else {
      $tt_v9 = $tt_v10;
    }
    
    const c = $tt_v9; { const $tt_a1 = { value: { kind: "Ok" as const, value: c } }; $tt_v7 = $tt_a1.value; break $tt_v7; }
  }
  return $tt_v7;
}
export function nullishUndefined() {
  let $tt_v11: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v11: {
    let $tt_v14: number;
    let $tt_v13: undefined;
    if (($tt_v13 = undefined) == null) {
      let $tt_v12: number;
      const $tt_t2 = read();
      if (!("value" in $tt_t2)) {
        $tt_v11 = $tt_t2;
        break $tt_v11;
      }
      $tt_v12 = $tt_t2.value;
      $tt_v14 = $tt_v12;
    } else {
      $tt_v14 = $tt_v13;
    }
    
    const d = $tt_v14; { const $tt_a2 = { value: { kind: "Ok" as const, value: d } }; $tt_v11 = $tt_a2.value; break $tt_v11; }
  }
  return $tt_v11;
}
export function numericLiterals(m: M) {
  let $tt_v15: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: readonly [
        number,
        string
    ];
});
  $tt_v15: {
    let $tt_v17: number;
    let $tt_v18: 0;
    if ($tt_v18 = 0) {
      $tt_v17 = $tt_v18;
    } else {
      let $tt_v16: number;
      const $tt_t3 = read();
      if (!("value" in $tt_t3)) {
        $tt_v15 = $tt_t3;
        break $tt_v15;
      }
      $tt_v16 = $tt_t3.value;
      $tt_v17 = $tt_v16;
    }
    
    let e = $tt_v17;
    let $tt_v20: string;
    let $tt_v21: 1;
    if ($tt_v21 = 1) {
      let $tt_v19: string;
      {
        const $tt_m = m;
        switch ($tt_m.kind) {
          case "A": {
            const { n } = $tt_m;
            $tt_v19 = `${n}`;
            break;
          }
          case "B": {
            $tt_v19 = "b";
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      $tt_v20 = $tt_v19;
    } else {
      $tt_v20 = $tt_v21;
    }
    
    const f = $tt_v20;
    {
      const $tt_a3 = { value: { kind: "Ok" as const, value: [e, f] as const } };
      $tt_v15 = $tt_a3.value;
      break $tt_v15;
    }
  }
  return $tt_v15;
}
export function parenthesized(m: M) {
  let $tt_v23: number;
  let $tt_v24: false;
  if ($tt_v24 = false) {
    $tt_v23 = $tt_v24;
  } else {
    let $tt_v22: number;
    {
      const $tt_m = m;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v22 = n;
          break;
        }
        case "B": {
          $tt_v22 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v23 = $tt_v22;
  }
  
  let h = $tt_v23;
  return h;
}
