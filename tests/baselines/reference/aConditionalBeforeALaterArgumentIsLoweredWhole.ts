//// [aConditionalBeforeALaterArgumentIsLoweredWhole.tt] ////
const calls: string[] = [];
variant V { A, B }
function f(...xs: number[]): number { calls.push("f"); return xs.reduce((a, b) => a + b, 0); }
function a(): number { calls.push("a"); return 1; }
function b(): number { calls.push("b"); return 10; }
export function g(c: boolean, v: V) {
  return f(a(), c ? match (v) { A => { calls.push("A"); return 100; }, B => 200 } : match (v) { A => { calls.push("A2"); return 300; }, B => 400 }, b());
}
console.log(g(true, V.A), g(false, V.B), JSON.stringify(calls));


//// [aConditionalBeforeALaterArgumentIsLoweredWhole.ts]
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
const calls: string[] = [];
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
function f(...xs: number[]): number { calls.push("f"); return xs.reduce((a, b) => a + b, 0); }
function a(): number { calls.push("a"); return 1; }
function b(): number { calls.push("b"); return 10; }
export function g(c: boolean, v: V) {
  let $tt_v5: number;
  const $tt_v3: typeof f = (f);
  const $tt_v4: number = (a());
  if (c) {
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": {
          calls.push("A"); $tt_v5 = 100; break;
        }
        case "B": {
          $tt_v5 = 200;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": {
          calls.push("A2"); $tt_v5 = 300; break;
        }
        case "B": {
          $tt_v5 = 400;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  }
  
  return $tt_v3($tt_v4, $tt_v5, b());
}
console.log(g(true, V.A), g(false, V.B), JSON.stringify(calls));
