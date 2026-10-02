//// [runtimeSeveralValuesInAResultReturnArgumentRunInTheReturnSPrelude.tt] ////

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
function r(n: number): R {
  trace.push("r" + n);
  return n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "bad" + n };
}
function pair(x: number, y: number): number { trace.push("pair"); return x * 10 + y; }
function id(n: number): number { return n; }
variant K { A, B }
function sum(x: number, y: number) { return result { return (try r(x)) + (try r(y)); }; }
function listed(x: number, y: number) { return result { return [try r(x), try r(y)]; }; }
function keyed(x: number, y: number) { return result { return { x: try r(x), y: try r(y) }; }; }
function called(x: number, y: number) { return result { return pair(try r(x), try r(y)); }; }
function matched(k: K, y: number) { return result { return match (k) { A => 1, B => 2 } + (try r(y)); }; }
function piped(x: number, y: number) { return result { return (x |> id) + (try r(y)); }; }
function chosen(x: number, y: number) { return result { return (try r(x)) > 1 ? (try r(y)) : 0; }; }
function guarded(c: boolean, x: number, y: number) {
  return result { if (c) return (try r(x)) + (try r(y)); return 0; };
}
console.log(JSON.stringify([sum(1, 2), sum(0, 2), sum(1, -1)]), JSON.stringify(trace));
trace.length = 0;
console.log(JSON.stringify([listed(1, 2), keyed(3, 4), called(1, 2), called(-2, 1)]), JSON.stringify(trace));
trace.length = 0;
console.log(JSON.stringify([matched(K.B, 3), piped(4, 5), chosen(2, 7), chosen(1, 7), guarded(true, 1, 2), guarded(false, 1, 2)]), JSON.stringify(trace));

export {};


//// [runtimeSeveralValuesInAResultReturnArgumentRunInTheReturnSPrelude.ts]
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
function r(n: number): R {
  trace.push("r" + n);
  return n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "bad" + n };
}
function pair(x: number, y: number): number { trace.push("pair"); return x * 10 + y; }
function id(n: number): number { return n; }
type K =
  | { kind: "A" }
  | { kind: "B" };
const K = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
function sum(x: number, y: number) { let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v0: {
  let $tt_v1: number;
  let $tt_v2: number;
  const $tt_t0 = r(x);
  if (!("value" in $tt_t0)) {
    $tt_v0 = $tt_t0;
    break $tt_v0;
  }
  $tt_v1 = $tt_t0.value;
  const $tt_t1 = r(y);
  if (!("value" in $tt_t1)) {
    $tt_v0 = $tt_t1;
    break $tt_v0;
  }
  $tt_v2 = $tt_t1.value;
  { const $tt_a0 = { value: { kind: "Ok" as const, value: ($tt_v1) + ($tt_v2) } }; $tt_v0 = $tt_a0.value; break $tt_v0; }
}
return $tt_v0; }
function listed(x: number, y: number) { let $tt_v3: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number[];
});
$tt_v3: {
  let $tt_v4: number;
  let $tt_v5: number;
  const $tt_t2 = r(x);
  if (!("value" in $tt_t2)) {
    $tt_v3 = $tt_t2;
    break $tt_v3;
  }
  $tt_v4 = $tt_t2.value;
  const $tt_t3 = r(y);
  if (!("value" in $tt_t3)) {
    $tt_v3 = $tt_t3;
    break $tt_v3;
  }
  $tt_v5 = $tt_t3.value;
  { const $tt_a1 = { value: { kind: "Ok" as const, value: [$tt_v4, $tt_v5] } }; $tt_v3 = $tt_a1.value; break $tt_v3; }
}
return $tt_v3; }
function keyed(x: number, y: number) { let $tt_v6: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: {
        x: number;
        y: number;
    };
});
$tt_v6: {
  let $tt_v7: number;
  let $tt_v8: number;
  const $tt_t4 = r(x);
  if (!("value" in $tt_t4)) {
    $tt_v6 = $tt_t4;
    break $tt_v6;
  }
  $tt_v7 = $tt_t4.value;
  const $tt_t5 = r(y);
  if (!("value" in $tt_t5)) {
    $tt_v6 = $tt_t5;
    break $tt_v6;
  }
  $tt_v8 = $tt_t5.value;
  { const $tt_a2 = { value: { kind: "Ok" as const, value: { x: $tt_v7, y: $tt_v8 } } }; $tt_v6 = $tt_a2.value; break $tt_v6; }
}
return $tt_v6; }
function called(x: number, y: number) { let $tt_v9: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v9: {
  let $tt_v10: number;
  let $tt_v11: number;
  const $tt_v12 = (pair);
  const $tt_t6 = r(x);
  if (!("value" in $tt_t6)) {
    $tt_v9 = $tt_t6;
    break $tt_v9;
  }
  $tt_v10 = $tt_t6.value;
  const $tt_t7 = r(y);
  if (!("value" in $tt_t7)) {
    $tt_v9 = $tt_t7;
    break $tt_v9;
  }
  $tt_v11 = $tt_t7.value;
  { const $tt_a3 = { value: { kind: "Ok" as const, value: $tt_v12($tt_v10, $tt_v11) } }; $tt_v9 = $tt_a3.value; break $tt_v9; }
}
return $tt_v9; }
function matched(k: K, y: number) { let $tt_v13: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v13: {
  let $tt_v14: number;
  let $tt_v15: number;
  {
    const $tt_m = k;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v14 = 1;
        break;
      }
      case "B": {
        $tt_v14 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const $tt_t8 = r(y);
  if (!("value" in $tt_t8)) {
    $tt_v13 = $tt_t8;
    break $tt_v13;
  }
  $tt_v15 = $tt_t8.value;
  { const $tt_a4 = { value: { kind: "Ok" as const, value: $tt_v14 + ($tt_v15) } }; $tt_v13 = $tt_a4.value; break $tt_v13; }
}
return $tt_v13; }
function piped(x: number, y: number) { let $tt_v16: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v16: {
  let $tt_v18: number;
  const $tt_v19 = ((($tt_v, $tt_f) => $tt_f($tt_v))(x, id));
  const $tt_t9 = r(y);
  if (!("value" in $tt_t9)) {
    $tt_v16 = $tt_t9;
    break $tt_v16;
  }
  $tt_v18 = $tt_t9.value;
  { const $tt_a5 = { value: { kind: "Ok" as const, value: ($tt_v19) + ($tt_v18) } }; $tt_v16 = $tt_a5.value; break $tt_v16; }
}
return $tt_v16; }
function chosen(x: number, y: number) { let $tt_v20: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v20: {
  let $tt_v21: number;
  let $tt_v24: number;
  const $tt_t10 = r(x);
  if (!("value" in $tt_t10)) {
    $tt_v20 = $tt_t10;
    break $tt_v20;
  }
  $tt_v21 = $tt_t10.value;
  if (($tt_v21) > 1) {
    const $tt_t11 = r(y);
    if (!("value" in $tt_t11)) {
      $tt_v20 = $tt_t11;
      break $tt_v20;
    }
    $tt_v24 = $tt_t11.value;
  } else {
    $tt_v24 = 0;
  }
  
  { const $tt_a6 = { value: { kind: "Ok" as const, value: $tt_v24 } }; $tt_v20 = $tt_a6.value; break $tt_v20; }
}
return $tt_v20; }
function guarded(c: boolean, x: number, y: number) {
  let $tt_v25: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v25: {
    if (c) {
      let $tt_v26: number;
      let $tt_v27: number;
      const $tt_t12 = r(x);
      if (!("value" in $tt_t12)) {
        $tt_v25 = $tt_t12;
        break $tt_v25;
      }
      $tt_v26 = $tt_t12.value;
      const $tt_t13 = r(y);
      if (!("value" in $tt_t13)) {
        $tt_v25 = $tt_t13;
        break $tt_v25;
      }
      $tt_v27 = $tt_t13.value;
      { const $tt_a7 = { value: { kind: "Ok" as const, value: ($tt_v26) + ($tt_v27) } }; $tt_v25 = $tt_a7.value; break $tt_v25; }
    } { const $tt_a8 = { value: { kind: "Ok" as const, value: 0 } }; $tt_v25 = $tt_a8.value; break $tt_v25; }
  }
  return $tt_v25;
}
console.log(JSON.stringify([sum(1, 2), sum(0, 2), sum(1, -1)]), JSON.stringify(trace));
trace.length = 0;
console.log(JSON.stringify([listed(1, 2), keyed(3, 4), called(1, 2), called(-2, 1)]), JSON.stringify(trace));
trace.length = 0;
console.log(JSON.stringify([matched(K.B, 3), piped(4, 5), chosen(2, 7), chosen(1, 7), guarded(true, 1, 2), guarded(false, 1, 2)]), JSON.stringify(trace));

export {};
