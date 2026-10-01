//// [aLabeledLoopKeepsItsLabelOnTheLoopWhenItsHeaderHoistsAValue.tt] ////
variant S { A(n: number), B }
declare const s: S;
declare const c: boolean;
function f(xs: number[][]) {
  lbl: for (const q of match (s) { A(n) => xs[n], B => [] }) { if (c) continue lbl; }
  if (c) outer: inner: for (const q of match (s) { A(n) => xs[n], B => [] }) { continue outer; }
}


//// [aLabeledLoopKeepsItsLabelOnTheLoopWhenItsHeaderHoistsAValue.ts]
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
declare const s: S;
declare const c: boolean;
function f(xs: number[][]) {
  let $tt_v0: number[];
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = xs[n];
        break;
      }
      case "B": {
        const $tt_a0 = { value: [] };
        $tt_v0 = $tt_a0.value;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  lbl: for (const q of $tt_v0) { if (c) continue lbl; }
  if (c) {
    let $tt_v1: number[];
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v1 = xs[n];
          break;
        }
        case "B": {
          const $tt_a1 = { value: [] };
          $tt_v1 = $tt_a1.value;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    outer: inner: for (const q of $tt_v1) { continue outer; }
  }
}
