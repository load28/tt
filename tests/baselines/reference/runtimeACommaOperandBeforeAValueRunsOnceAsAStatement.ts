//// [runtimeACommaOperandBeforeAValueRunsOnceAsAStatement.tt] ////

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
function tick(): void { trace.push("tick"); }
function tock(): number { trace.push("tock"); return 7; }
function r(n: number): R { trace.push("r"); return { kind: "Ok", value: n }; }
variant K { A, B }
function pick(k: K) { trace.push("pick"); return k; }
function first(k: K) { return (tick(), match (pick(k)) { A => 1, B => 2 }); }
function several(k: K) { const x = (tick(), tock(), match (pick(k)) { A => 1, B => 2 }); return x + tock(); }
function guarded(c: boolean, k: K) { return c && (tick() /* effect */, match (pick(k)) { A => 1, B => 2 }); }
function propagated(): R { return r((tick(), try r(3))); }
console.log(first(K.B), JSON.stringify(trace));
trace.length = 0;
console.log(several(K.A), JSON.stringify(trace));
trace.length = 0;
console.log(guarded(false, K.A), guarded(true, K.B), JSON.stringify(trace));
trace.length = 0;
console.log(JSON.stringify(propagated()), JSON.stringify(trace));

export {};


//// [runtimeACommaOperandBeforeAValueRunsOnceAsAStatement.ts]
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

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
function tick(): void { trace.push("tick"); }
function tock(): number { trace.push("tock"); return 7; }
function r(n: number): R { trace.push("r"); return { kind: "Ok", value: n }; }
type K =
  | { kind: "A" }
  | { kind: "B" };
const K = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
function pick(k: K) { trace.push("pick"); return k; }
function first(k: K) { let $tt_v0: number;
(tick());
{
  const $tt_m = pick(k);
  switch ($tt_m.kind) {
    case "A": $tt_v0 = 0; break;
    case "B": $tt_v0 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
return ( ($tt_v0 === 0 ? 1 : 2)); }
function several(k: K) { let $tt_v2: number;
(tick());
(tock());
{
  const $tt_m = pick(k);
  switch ($tt_m.kind) {
    case "A": $tt_v2 = 0; break;
    case "B": $tt_v2 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
const x = (  ($tt_v2 === 0 ? 1 : 2)); return x + tock(); }
function guarded(c: boolean, k: K) { let $tt_v8: (number) | (false);
let $tt_v7: boolean;
if ($tt_v7 = c) {
  let $tt_v5: number;
  (tick());
  {
    const $tt_m = pick(k);
    switch ($tt_m.kind) {
      case "A": {
        $tt_v5 = 1;
        break;
      }
      case "B": {
        $tt_v5 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v8 = $tt_v7 &&  /* effect */ $tt_v5;
} else {
  $tt_v8 = $tt_v7;
}

return $tt_v8; }
function propagated(): R { let $tt_v9: number;
const $tt_v11: typeof r = (r);
(tick());
const $tt_t0 = r(3);
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v9 = $tt_t0.value;
return $tt_v11(( $tt_v9)); }
console.log(first(K.B), JSON.stringify(trace));
trace.length = 0;
console.log(several(K.A), JSON.stringify(trace));
trace.length = 0;
console.log(guarded(false, K.A), guarded(true, K.B), JSON.stringify(trace));
trace.length = 0;
console.log(JSON.stringify(propagated()), JSON.stringify(trace));

export {};
