//// [aMatchInsideAFunctionInAConditionalBranchIsNotMisplaced.tt] ////
variant S { A(n: number), B }
const s: S = S.A(3) as S; const c = true;
const v = c ? match (s) { A(n) => n, B => 0 } : () => match (s) { A => 1, B => 2 };
const w = !c ? match (s) { A(n) => n, B => 0 } : () => match (s) { A => 1, B => 2 };
const x = c ? (function () { return match (s) { A => 10, B => 20 }; })() : match (s) { A(n) => n * 2, B => 0 };
console.log(v, typeof w === "function" ? (w as () => number)() : w, x);


//// [aMatchInsideAFunctionInAConditionalBranchIsNotMisplaced.ts]
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
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const s: S = S.A(3) as S; const c = true;
let $tt_v2$v: (number) | (() => number);
if (c) {
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v2$v = n;
        break;
      }
      case "B": {
        $tt_v2$v = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
} else {
  const $tt_a0 = { value: ((void 0, () => {
    let $tt_v3: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v3 = 1;
          break;
        }
        case "B": {
          $tt_v3 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    return $tt_v3;
  })) };
  $tt_v2$v = $tt_a0.value;
}

const v = $tt_v2$v;
let $tt_v6$w: (number) | (() => number);
if (!c) {
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v6$w = n;
        break;
      }
      case "B": {
        $tt_v6$w = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
} else {
  const $tt_a1 = { value: ((void 0, () => {
    let $tt_v7: number;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v7 = 1;
          break;
        }
        case "B": {
          $tt_v7 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    return $tt_v7;
  })) };
  $tt_v6$w = $tt_a1.value;
}

const w = $tt_v6$w;
let $tt_v10$x: number;
if (c) {
  $tt_v10$x = ((function () { let $tt_v11: number;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v11 = 10;
        break;
      }
      case "B": {
        $tt_v11 = 20;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v11; })());
} else {
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v10$x = n * 2;
        break;
      }
      case "B": {
        $tt_v10$x = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
}

const x = $tt_v10$x;
console.log(v, typeof w === "function" ? (w as () => number)() : w, x);
