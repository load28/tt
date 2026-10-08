//// [aSkippedBranchOrLaterArgumentMayHoldAnInlinePipeline.tt] ////
variant S { A, B }
const s: S = S.A as S;
function p(): S { return S.A; }
function F(x: number) { return x + 1; }
function f(...x: unknown[]) { return x; }
const g = { mm: (...x: unknown[]) => x };
const h: ((...x: unknown[]) => unknown[]) | undefined = (...x) => x;
function ternary(c: boolean) {
  const a = c ? (1 |> F) : match (p()) { A => 3, B => 0 };
  const b = c ? (1 |> F) : (match (p()) { A => 3, B => 0 } |> F);
  const d = c ? (1 |> F) : F(match (p()) { A => 3, B => 0 });
  return [a, b, d];
}
function conditionalTest() {
  return g.mm?.(match (p()) { A => 1, B => 2 }) ? (1 |> F) : f(match (p()) { A => 1, B => 2 });
}
function laterArguments(a: number | undefined, c: boolean) {
  return [
    a ?? match (s) { A => 1, B => 2 },
    g.mm?.(match (s) { A => 1, B => 2 }, (c ? 1 : (2 |> F))),
    h?.(match (s) { A => 1, B => 2 }, 1 |> F),
    c && f(match (s) { A => 1, B => 2 }, 1 |> F),
  ];
}
console.log(JSON.stringify([ternary(true), ternary(false), conditionalTest(), laterArguments(undefined, false), laterArguments(7, true)]));


//// [aSkippedBranchOrLaterArgumentMayHoldAnInlinePipeline.ts]
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
function p(): S { return S.A; }
function F(x: number) { return x + 1; }
function f(...x: unknown[]) { return x; }
const g = { mm: (...x: unknown[]) => x };
const h: ((...x: unknown[]) => unknown[]) | undefined = (...x) => x;
function ternary(c: boolean) {
  let $tt_v3: number;
  if (c) {
    $tt_v3 = F(1);
  } else {
    {
      const $tt_m = p();
      switch ($tt_m.kind) {
        case "A": {
          $tt_v3 = 3;
          break;
        }
        case "B": {
          $tt_v3 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  }
  
  const a = $tt_v3;
  let $tt_v8: number;
  if (c) {
    $tt_v8 = F(1);
  } else {
    do {
      let $tt_v5: number;
      let $tt_v42: number;
      {
        const $tt_m = p();
        switch ($tt_m.kind) {
          case "A": {
            $tt_v42 = 3;
            break;
          }
          case "B": {
            $tt_v42 = 0;
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      $tt_v5 = F($tt_v42);
      $tt_v8 = $tt_v5; break;
    } while (false);
  }
  
  const b = $tt_v8;
  let $tt_v13: number;
  if (c) {
    $tt_v13 = F(1);
  } else {
    let $tt_v10: number;
    const $tt_v12: typeof F = (F);
    {
      const $tt_m = p();
      switch ($tt_m.kind) {
        case "A": {
          $tt_v10 = 3;
          break;
        }
        case "B": {
          $tt_v10 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v13 = $tt_v12($tt_v10);
  }
  
  const d = $tt_v13;
  return [a, b, d];
}
function conditionalTest() {
  let $tt_v20: (unknown[]) | (undefined);
  let $tt_v21: (number) | (unknown[]);
  const $tt_v17 = (g.mm);
  if ($tt_v17 != null) {
    {
      const $tt_m = p();
      switch ($tt_m.kind) {
        case "A": {
          $tt_v20 = $tt_v17.call(g, 1);
          break;
        }
        case "B": {
          $tt_v20 = $tt_v17.call(g, 2);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v20 = undefined;
  }
  
  if ($tt_v20) {
    $tt_v21 = F(1);
  } else {
    let $tt_v16: number;
    const $tt_v19: typeof f = (f);
    {
      const $tt_m = p();
      switch ($tt_m.kind) {
        case "A": {
          $tt_v16 = 1;
          break;
        }
        case "B": {
          $tt_v16 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v21 = $tt_v19($tt_v16);
  }
  
  return $tt_v21;
}
function laterArguments(a: number | undefined, c: boolean) {
  let $tt_v38: number;
  let $tt_v39: (unknown[]) | (undefined);
  let $tt_v40: (unknown[]) | (undefined);
  let $tt_v41: (unknown[]) | (false);
  let $tt_v29: number | undefined;
  if (($tt_v29 = a) == null) {
    let $tt_v22: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v22 = 1;
          break;
        }
        case "B": {
          $tt_v22 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v38 = $tt_v29 ?? $tt_v22;
  } else {
    $tt_v38 = $tt_v29;
  }
  
  const $tt_v31 = ($tt_v38);
  const $tt_v30 = (g.mm);
  if ($tt_v30 != null) {
    let $tt_v23: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v23 = 1;
          break;
        }
        case "B": {
          $tt_v23 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v39 = $tt_v30.call(g, $tt_v23, c ? 1 : (F(2)));
  } else {
    $tt_v39 = undefined;
  }
  
  const $tt_v34 = ($tt_v39);
  const $tt_v33: typeof h = (h);
  if ($tt_v33 != null) {
    let $tt_v25: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v25 = 1;
          break;
        }
        case "B": {
          $tt_v25 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v40 = $tt_v33($tt_v25, F(1));
  } else {
    $tt_v40 = undefined;
  }
  
  const $tt_v37 = ($tt_v40);
  let $tt_v36: boolean;
  if ($tt_v36 = c) {
    let $tt_v27: number;
    const $tt_v35: typeof f = (f);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v27 = 1;
          break;
        }
        case "B": {
          $tt_v27 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v41 = $tt_v36 && $tt_v35($tt_v27, F(1));
  } else {
    $tt_v41 = $tt_v36;
  }
  
  return [
    $tt_v31,
    $tt_v34,
    $tt_v37,
    $tt_v41,
  ];
}
console.log(JSON.stringify([ternary(true), ternary(false), conditionalTest(), laterArguments(undefined, false), laterArguments(7, true)]));
