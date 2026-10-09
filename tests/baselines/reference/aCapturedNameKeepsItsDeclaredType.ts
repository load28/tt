//// [aCapturedNameKeepsItsDeclaredType.tt] ////
variant V { A(n: number), B }
function assertPos(n: number): asserts n {
  if (n <= 0) throw new Error("not positive");
}
const checks: { assertPos: typeof assertPos } = { assertPos };
export function asserted(v: V) {
  assertPos(match (v) { A(n) => n, B => 1 });
  checks.assertPos(match (v) { A(n) => n, B => 1 });
  return "asserted";
}
export function keyed(v: V) {
  const sym: unique symbol = Symbol();
  const d = { [sym]: match (v) { A(n) => n, B => 0 } };
  const it = { [Symbol.iterator]: match (v) { A => function* () { yield 1; }, B => function* () { yield 2; } } };
  return [d[sym], ...it];
}
console.log(asserted(V.A(2)));
console.log(JSON.stringify(keyed(V.A(5))));


//// [aCapturedNameKeepsItsDeclaredType.ts]
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
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
function assertPos(n: number): asserts n {
  if (n <= 0) throw new Error("not positive");
}
const checks: { assertPos: typeof assertPos } = { assertPos };
export function asserted(v: V) {
  const $tt_v1: typeof assertPos = (assertPos);
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v1(n);
        break;
      }
      case "B": {
        $tt_v1(1);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        checks.assertPos(n);
        break;
      }
      case "B": {
        checks.assertPos(1);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
  return "asserted";
}
export function keyed(v: V) {
  const sym: unique symbol = Symbol();
  let $tt_v4: number;
  const $tt_v5: typeof sym = (sym);
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v4 = n;
        break;
      }
      case "B": {
        $tt_v4 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const d = { [$tt_v5]: $tt_v4 };
  let $tt_v6: number;
  const $tt_v7: typeof Symbol.iterator = (Symbol.iterator);
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": $tt_v6 = 0; break;
      case "B": $tt_v6 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  const it = { [$tt_v7]: ($tt_v6 === 0 ? function* () { yield 1; } : function* () { yield 2; }) };
  return [d[sym], ...it];
}
console.log(asserted(V.A(2)));
console.log(JSON.stringify(keyed(V.A(5))));
