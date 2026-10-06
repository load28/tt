//// [runtimeATryStatementInAResultBlockLowersTheValuesOfItsOperand.tt] ////

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
let r = (n: number): R => { trace.push("r" + n); return n > 15 ? { kind: "Ok", value: n } : { kind: "Err", error: "small" + n }; };
function swap(n: number): number {
  trace.push("swap");
  const first = r;
  r = (m) => { trace.push("late"); return first(m); };
  return n;
}
const id = <T,>(x: T) => x;
const g = (x: R): R => x;
function viaConst(v: number) { return result { const x = try r(match (v) { 1 => swap(10), _ => swap(20) }); return x; }; }
function viaLet(v: number) { return result { let x = try r(match (v) { 1 => 10, _ => 20 }); x += 1; return x; }; }
function propagateOnly(v: number) { return result { try r(match (v) { 1 => 10, _ => 20 }); return 0; }; }
function viaPipeline(v: number) { return result { const x = try (match (v) { 1 => r(10), _ => r(20) } |> id); return x; }; }
function viaCall(v: number) { return result { const x = try id(match (v) { 1 => r(10), _ => r(20) }); return x; }; }
function viaResult(v: number) {
  return result { const x = try r(result { const y = try r(v); return y; } |> g |> (q => q.kind === "Ok" ? q.value : 0)); return x; };
}
const base = r;
console.log(JSON.stringify([viaConst(1), viaConst(2)]), JSON.stringify(trace));
r = base;
console.log(JSON.stringify([viaLet(1), viaLet(2), propagateOnly(1), propagateOnly(2)]));
console.log(JSON.stringify([viaPipeline(1), viaPipeline(2), viaCall(1), viaCall(2), viaResult(20), viaResult(1)]));

export {};


//// [runtimeATryStatementInAResultBlockLowersTheValuesOfItsOperand.ts]
function $tt_expr<T>(run: () => T): T { return run(); }

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
let r = (n: number): R => { trace.push("r" + n); return n > 15 ? { kind: "Ok", value: n } : { kind: "Err", error: "small" + n }; };
function swap(n: number): number {
  trace.push("swap");
  const first = r;
  r = (m) => { trace.push("late"); return first(m); };
  return n;
}
const id = <T,>(x: T) => x;
const g = (x: R): R => x;
function viaConst(v: number) { let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v0: {
  let $tt_v1: number;
  const $tt_v2: typeof r = (r);
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: $tt_v1 = 0; break;
      default: $tt_v1 = 1; break;
    }
  }
  const $tt_t0 = $tt_v2(($tt_v1 === 0 ? swap(10) : swap(20)));
  if (!("value" in $tt_t0)) {
    $tt_v0 = $tt_t0;
    break $tt_v0;
  }
  const x = $tt_t0.value; { const $tt_a0 = { value: { kind: "Ok" as const, value: x } }; $tt_v0 = $tt_a0.value; break $tt_v0; }
}
return $tt_v0; }
function viaLet(v: number) { let $tt_v3: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v3: {
  let $tt_v4: number;
  const $tt_v5: typeof r = (r);
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: $tt_v4 = 0; break;
      default: $tt_v4 = 1; break;
    }
  }
  const $tt_t1 = $tt_v5(($tt_v4 === 0 ? 10 : 20));
  if (!("value" in $tt_t1)) {
    $tt_v3 = $tt_t1;
    break $tt_v3;
  }
  let x = $tt_t1.value; x += 1; { const $tt_a1 = { value: { kind: "Ok" as const, value: x } }; $tt_v3 = $tt_a1.value; break $tt_v3; }
}
return $tt_v3; }
function propagateOnly(v: number) { let $tt_v6: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v6: {
  let $tt_v7: number;
  const $tt_v8: typeof r = (r);
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: $tt_v7 = 0; break;
      default: $tt_v7 = 1; break;
    }
  }
  const $tt_t2 = $tt_v8(($tt_v7 === 0 ? 10 : 20));
  if (!("value" in $tt_t2)) {
    $tt_v6 = $tt_t2;
    break $tt_v6;
  } { const $tt_a2 = { value: { kind: "Ok" as const, value: 0 } }; $tt_v6 = $tt_a2.value; break $tt_v6; }
}
return $tt_v6; }
function viaPipeline(v: number) { let $tt_v9: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v9: {
  let $tt_v16: R;
  do {
    let $tt_v21: R;
    {
      const $tt_m = v;
      switch ($tt_m) {
        case 1: {
          $tt_v21 = r(10);
          break;
        }
        default: {
          $tt_v21 = r(20);
          break;
        }
      }
    }
    $tt_v16 = id($tt_v21);
    break;
  } while (false);
  $tt_v16 = ($tt_v16);
  const $tt_t3 = $tt_v16;
  if (!("value" in $tt_t3)) {
    $tt_v9 = $tt_t3;
    break $tt_v9;
  }
  const x = $tt_t3.value; { const $tt_a3 = { value: { kind: "Ok" as const, value: x } }; $tt_v9 = $tt_a3.value; break $tt_v9; }
}
return $tt_v9; }
function viaCall(v: number) { let $tt_v10: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v10: {
  let $tt_v11: number;
  const $tt_v12: typeof id = (id);
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: $tt_v11 = 0; break;
      default: $tt_v11 = 1; break;
    }
  }
  const $tt_t4 = $tt_v12(($tt_v11 === 0 ? r(10) : r(20)));
  if (!("value" in $tt_t4)) {
    $tt_v10 = $tt_t4;
    break $tt_v10;
  }
  const x = $tt_t4.value; { const $tt_a4 = { value: { kind: "Ok" as const, value: x } }; $tt_v10 = $tt_a4.value; break $tt_v10; }
}
return $tt_v10; }
function viaResult(v: number) {
  let $tt_v13: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v13: {
    let $tt_v14: number;
    const $tt_v15: typeof r = (r);
    do {
      const $tt_v19: R = ($tt_expr(() => {
        const $tt_t6 = r(v);
        if (!("value" in $tt_t6)) {
          return $tt_t6;
        }
        const y = $tt_t6.value; { return { kind: "Ok" as const, value: y }; }
        }));
      const $tt_v20: R = g($tt_v19);
      $tt_v14 = (q => q.kind === "Ok" ? q.value : 0)($tt_v20);
      break;
    } while (false);
    const $tt_t5 = $tt_v15($tt_v14);
    if (!("value" in $tt_t5)) {
      $tt_v13 = $tt_t5;
      break $tt_v13;
    }
    const x = $tt_t5.value; { const $tt_a5 = { value: { kind: "Ok" as const, value: x } }; $tt_v13 = $tt_a5.value; break $tt_v13; }
  }
  return $tt_v13;
}
const base = r;
console.log(JSON.stringify([viaConst(1), viaConst(2)]), JSON.stringify(trace));
r = base;
console.log(JSON.stringify([viaLet(1), viaLet(2), propagateOnly(1), propagateOnly(2)]));
console.log(JSON.stringify([viaPipeline(1), viaPipeline(2), viaCall(1), viaCall(2), viaResult(20), viaResult(1)]));

export {};
