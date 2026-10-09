//// [aValueInsideAnArgumentIsNotTheWholeArgument.tt] ////
variant S { A, B }
const s: S = S.A as S;
const g = { mm: (...a: unknown[]) => a };
function f(...a: unknown[]) { return a; }
function main() {
  const b = g.mm?.(1, (match (s) { A => 0, B => 1 } ? "then" : "else"));
  const c = g.mm?.(match (s) { A => 0, B => 1 } ? "then" : "else");
  const d = f?.(match (s) { A => 0, B => 1 } ? "then" : "else");
  const e = f(match (s) { A => 0, B => 1 } ? "then" : "else");
  const h = g.mm?.(match (s) { A => 0, B => 1 } || "or");
  const i = g.mm?.(match (s) { A => 0, B => 1 } + 100);
  const j = g?.mm(match (s) { A => 0, B => 1 } ? "then" : "else");
  return JSON.stringify({ b, c, d, e, h, i, j });
}
console.log(main());


//// [aValueInsideAnArgumentIsNotTheWholeArgument.ts]
var $tt_show: (value: unknown) => string = function (value) {
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
};
type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const s: S = S.A as S;
const g = { mm: (...a: unknown[]) => a };
function f(...a: unknown[]) { return a; }
function main() {
  let $tt_v2: (unknown[]) | (undefined);
  const $tt_v1 = (g.mm);
  if ($tt_v1 != null) {
    let $tt_v0: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v0 = 0;
          break;
        }
        case "B": {
          $tt_v0 = 1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v2 = $tt_v1.call(g, 1, $tt_v0 ? "then" : "else");
  } else {
    $tt_v2 = undefined;
  }
  
  const b = $tt_v2;
  let $tt_v5: (unknown[]) | (undefined);
  const $tt_v4 = (g.mm);
  if ($tt_v4 != null) {
    let $tt_v3: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v3 = 0;
          break;
        }
        case "B": {
          $tt_v3 = 1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v5 = $tt_v4.call(g, $tt_v3 ? "then" : "else");
  } else {
    $tt_v5 = undefined;
  }
  
  const c = $tt_v5;
  let $tt_v8: (unknown[]) | (undefined);
  const $tt_v7: typeof f = (f);
  if ($tt_v7 != null) {
    let $tt_v6: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v6 = 0;
          break;
        }
        case "B": {
          $tt_v6 = 1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v8 = $tt_v7($tt_v6 ? "then" : "else");
  } else {
    $tt_v8 = undefined;
  }
  
  const d = $tt_v8;
  let $tt_v9: number;
  const $tt_v10: typeof f = (f);
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": $tt_v9 = 0; break;
      case "B": $tt_v9 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  const e = $tt_v10(($tt_v9 === 0 ? 0 : 1) ? "then" : "else");
  let $tt_v13: (unknown[]) | (undefined);
  const $tt_v12 = (g.mm);
  if ($tt_v12 != null) {
    let $tt_v11: string | number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v11 = 0;
          break;
        }
        case "B": {
          $tt_v11 = 1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v13 = $tt_v12.call(g, $tt_v11 || "or");
  } else {
    $tt_v13 = undefined;
  }
  
  const h = $tt_v13;
  let $tt_v16: (unknown[]) | (undefined);
  const $tt_v15 = (g.mm);
  if ($tt_v15 != null) {
    let $tt_v14: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v14 = 0;
          break;
        }
        case "B": {
          $tt_v14 = 1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v16 = $tt_v15.call(g, $tt_v14 + 100);
  } else {
    $tt_v16 = undefined;
  }
  
  const i = $tt_v16;
  let $tt_v19: (unknown[]) | (undefined);
  if (g != null) {
    let $tt_v17: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v17 = 0;
          break;
        }
        case "B": {
          $tt_v17 = 1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v19 = g?.mm($tt_v17 ? "then" : "else");
  } else {
    $tt_v19 = undefined;
  }
  
  const j = $tt_v19;
  return JSON.stringify({ b, c, d, e, h, i, j });
}
console.log(main());
